//! Defines the UDP socket abstraction and first-layer packet format used for communication between peers.

pub mod error;
mod gossip;
mod packet;

use std::{
    collections::{hash_map::Entry, HashMap},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use pgp::composed::SignedSecretKey;
use string_protocol::crypto;
use string_protocol::{MessageType, ProtocolPacket};
use stunclient::StunClient;
use tokio::{
    net::UdpSocket,
    sync::{mpsc, RwLock},
};
use tracing::{debug, error, span, trace};

use self::gossip::start_gossip_worker;
use crate::{
    crypto::{Crypto, DoubleRatchet},
    maybe_break, maybe_continue,
    peer::{Peer, PeerState, CHANNEL_SIZE},
    try_break, try_continue,
};

// re-export types
pub use self::error::{SocketError, SocketPacketDecodeError};
pub use self::gossip::{Gossip, GossipAction};
pub use self::packet::{
    SocketPacket, SocketPacketType, MIN_SOCKET_PACKET_SIZE, UDP_MAX_DATAGRAM_SIZE,
};

/// A wrapper around the [UdpSocket] type that provides a higher-level interface for sending and
/// receiving packets from multiple peers.
#[derive(Debug)]
pub struct Socket {
    /// The inner [UdpSocket] used for sending and receiving packets.
    pub inner: Arc<UdpSocket>,
    /// A map of connections to other peers.
    pub peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
    /// Crypto object, contains ratchets for other nodes
    pub crypto: Arc<RwLock<Crypto>>,
    /// Username used to identify this current node
    pub username: String,
    /// Channel used to send gossip
    pub gossip_tx: mpsc::Sender<Gossip>,
    /// How our socket is seen externally
    pub external: SocketAddr,
    /// Channel used to unify inbound packets
    pub unified_inbound_tx: mpsc::Sender<(Vec<u8>, ProtocolPacket)>,
}

impl Socket {
    /// Create a new `Socket` that is bound to the given address. This method also
    /// starts the background tasks that handle sending and receiving packets.
    pub async fn bind(
        addr: SocketAddr,
        secret_key: SignedSecretKey,
    ) -> Result<(Self, mpsc::Receiver<(Vec<u8>, ProtocolPacket)>), SocketError> {
        // bind socket

        let raw_socket = UdpSocket::bind(addr).await.map_err(SocketError::IoError)?;

        let google_stun = StunClient::with_google_stun_server();
        let external = google_stun
            .query_external_address_async(&raw_socket)
            .await
            .map_err(|_| SocketError::StunError)?;

        let socket: Arc<_> = raw_socket.into();

        // create peers map
        let peers = Arc::new(RwLock::new(HashMap::new()));

        let crypto = Arc::new(RwLock::new(Crypto::new(secret_key.clone())));

        let (gossip_tx, gossip_rx) = mpsc::channel(CHANNEL_SIZE);

        // start the outbound worker
        span!(tracing::Level::INFO, "socket::outbound")
            .in_scope(|| start_outbound_worker(socket.clone(), peers.clone()));

        // start the gossip worker
        span!(tracing::Level::INFO, "socket::gossip")
            .in_scope(|| start_gossip_worker(gossip_rx, peers.clone()));

        // create the unified inbound channel
        let (unified_inbound_tx, unified_inbound_rx) = mpsc::channel(CHANNEL_SIZE);

        // start the perodic worker
        span!(tracing::Level::INFO, "socket::periodic")
            .in_scope(|| start_periodic_worker(peers.clone()));

        if secret_key.details.users.len() != 1 {
            // Why do we have a weird number of users
            panic!("Invalid number of users in secret key - programming error")
        }

        let username = Crypto::get_pubkey_username(secret_key.into());

        Ok((
            Self {
                inner: socket,
                peers,
                crypto,
                username,
                gossip_tx,
                external,
                unified_inbound_tx,
            },
            unified_inbound_rx,
        ))
    }

    /// Add a new peer to the list of connections, returning a channel for receiving
    /// packets from the peer.
    pub async fn add_peer(
        &mut self,
        addr: SocketAddr,
        fingerprint: Vec<u8>,
        initiate: bool,
    ) -> mpsc::Sender<ProtocolPacket> {
        let (peer, app_inbound_rx, net_outbound_rx) = Peer::new(
            addr,
            self.crypto.clone(),
            self.peers.clone(),
            self.username.clone(),
            self.gossip_tx.clone(),
            fingerprint.clone(),
			self.curr_time.clone(),
            initiate,
        )?;

        let app_outbound_tx = peer.app_outbound_tx.clone();

        // spawn the inbound peer task
        span!(
            tracing::Level::INFO,
            "socket::inbound",
            ?peer.remote_addr,
        )
        .in_scope(|| {
            spawn_inbound_peer_task(self.inner.clone(), peer.remote_addr, net_outbound_rx)
        });

        // spawn the application worker
        span!(
            tracing::Level::INFO,
            "socket::application",
            ?peer.remote_addr,
        )
        .in_scope(|| {
            start_application_worker(fingerprint, app_inbound_rx, self.unified_inbound_tx.clone())
        });

        // insert the peer into the connections map - done in a separate block to avoid holding the
        // lock for too long
        {
            let mut connections = self.peers.write().await;
            connections.insert(addr, peer);
        }

        app_outbound_tx
    }

    pub async fn get_peer_state(&mut self, addr: SocketAddr) -> Option<PeerState> {
        let connections = self.peers.read().await;
        if !connections.contains_key(&addr) {
            None
        } else {
            let state = { *connections[&addr].state.read().await };
            Some(state)
        }
    }

    /// Send a packet to the given peer.
    pub async fn send_packet(
        &mut self,
        destination: SocketAddr,
        packet: ProtocolPacket,
    ) -> Result<(), SocketError> {
        // lookup the peer
        let mut peers = self.peers.write().await;
        let peer = peers.get_mut(&destination).ok_or(SocketError::Unknown)?;
        peer.send_packet(packet).await?;

        Ok(())
    }

    /// Sends non-encrypted message to a random group of peers
    /// Use this to send key exchange messages
    pub async fn send_gossip(
        &self,
        message: MessageType,
        destination: String,
    ) -> Result<(), SocketError> {
        self.gossip_tx
            .send(Gossip {
                action: GossipAction::Send,
                addr: None,
                packet: None,
                message: Some(message),
                dest: Some(destination),
                dest_sockaddr: None,
            })
            .await
            .map_err(|err| SocketError::GossipSendError(Box::new(err)))
    }

    /// Sends encrypted packet to random group of peers
    /// Use this to send a ProtocolPacket containing a PktMessage,
    /// which contains the message data
    pub async fn send_gossip_encrypted(
        &self,
        packet: ProtocolPacket,
        destination: String,
    ) -> Result<(), SocketError> {
        self.gossip_tx
            .send(Gossip {
                action: GossipAction::SendEncrypted,
                addr: None,
                packet: Some(packet),
                message: None,
                dest: Some(destination),
                dest_sockaddr: None,
            })
            .await
            .map_err(|err| SocketError::GossipSendError(Box::new(err)))
    }

    pub async fn get_node_cert(&mut self, destination: String) -> Result<(), SocketError> {
        {
            let mut crypto_obj = self.crypto.write().await;
            crypto_obj.insert_pubkey_reply_to(destination.clone(), None);
        }
        self.send_gossip(
            MessageType::PubKeyRequest(crypto::v1::PubKeyRequest {}),
            destination,
        )
        .await?;
        Ok(())
    }

    /// Attempt to establish a DR ratchet with destination node
    /// Since this is done by gossip, it may not succeed if the node is down
    /// or inexistent
    pub async fn start_dr(&mut self, destination: String) -> Result<(), SocketError> {
        let mut crypto = self.crypto.write().await;
        match crypto.ratchets.entry(destination.clone()) {
            Entry::Occupied(_) => Err(SocketError::RatchetExists),
            Entry::Vacant(entry) => {
                let mut dr = DoubleRatchet::new_initiator();
                let kex_msg = dr.generate_kex_message();
                entry.insert(dr);
                drop(crypto);
                self.send_gossip(MessageType::KeyExchange(kex_msg), destination)
                    .await?;
                Ok(())
            }
        }
    }
}

/// Start the outbound network worker.
fn start_outbound_worker(socket: Arc<UdpSocket>, peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>) {
    tokio::spawn(async move {
        let mut buf = [0; UDP_MAX_DATAGRAM_SIZE];
        loop {
            trace!("start outbound worker loop");
            // wait for socket to be readable
            try_break!(socket.readable().await, "Error reading from socket");

            // receive packet
            let (size, addr) = try_continue!(
                socket.recv_from(&mut buf).await,
                "Error reading from network"
            );

            // see if we know this peer
            let mut peers = peers.write().await;
            let peer = maybe_continue!(peers.get_mut(&addr), "Unknown peer");

            // decode network packet
            let packet = try_continue!(SocketPacket::decode(&buf[..size]), "Error decoding packet");

            // forward to peer
            debug!(?peer.remote_addr, "forward packet to peer");

            if let Err(e) = peer.net_inbound_tx.send(packet).await {
                eprintln!("Error forwarding packet to peer: {:?}", e);
            }
        }
    });
}

/// Starts the background tasks that handle receiving
fn spawn_inbound_peer_task(
    socket: Arc<UdpSocket>,
    destination: SocketAddr,
    mut net_outbound_rx: mpsc::Receiver<SocketPacket>,
) {
    tokio::spawn(async move {
        loop {
            trace!("start inbound peer task loop");

            // receive packet from peer
            let packet = match net_outbound_rx.recv().await {
                Some(packet) => packet,
                None => break,
            };

            // encode packet
            let bytes = try_continue!(packet.encode(), "Error encoding packet");

            // send to network
            debug!(?destination, len = bytes.len(), "send packet to network");
            if let Err(e) = socket.send_to(&bytes, destination).await {
                eprintln!("Error sending packet to network: {:?}", e);
            }
        }
    });
}

/// Starts a background worker than can do certain chores at regular intervals
fn start_periodic_worker(_peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(5000)).await;
        }
    });
}

/// Starts the background tasks that handle receiving packets from all peers and
/// forwarding them to the unified inbound channel
fn start_application_worker(
    fingerprint: Vec<u8>,
    mut application_inbound_rx: mpsc::Receiver<ProtocolPacket>,
    unified_inbound_tx: mpsc::Sender<(Vec<u8>, ProtocolPacket)>,
) {
    tokio::spawn(async move {
        loop {
            trace!("start application worker loop");

            // receive packet from peer
            let packet = maybe_break!(
                application_inbound_rx.recv().await,
                "Error receiving packet from peer"
            );

            // send to unified inbound channel
            debug!(?fingerprint, "forward packet to unified inbound channel");
            try_break!(
                unified_inbound_tx.send((fingerprint.clone(), packet)).await,
                "Error forwarding packet to unified inbound channel"
            );
        }
    });
}
