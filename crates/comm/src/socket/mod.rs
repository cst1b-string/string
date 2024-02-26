//! Defines the UDP socket abstraction and first-layer packet format used for communication between peers.

pub mod error;
mod packet;

use std::{
    collections::{hash_map::Entry, HashMap},
    net::SocketAddr,
    sync::Arc,
};

use rand::{rngs::OsRng, seq::IteratorRandom};
use string_protocol::{MessageType, ProtocolPacket};
use tokio::{
    net::UdpSocket,
    sync::{mpsc, RwLock},
};
use tracing::{debug, error, span, trace};

use crate::{
    crypto::{Crypto, DoubleRatchet},
    maybe_continue,
    peer::{Peer, PeerState},
    try_break, try_continue,
};

// re-export types
pub use self::error::{SocketError, SocketPacketDecodeError};
pub use self::packet::{
    SocketPacket, SocketPacketType, MIN_SOCKET_PACKET_SIZE, UDP_MAX_DATAGRAM_SIZE,
};

/// Number of peers to send gossip to
const GOSSIP_COUNT: usize = 3;

/// A wrapper around the [UdpSocket] type that provides a higher-level interface for sending and
/// receiving packets from multiple peers.
pub struct Socket {
    /// The inner [UdpSocket] used for sending and receiving packets.
    pub inner: Arc<UdpSocket>,
    /// A map of connections to other peers.
    pub peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
    /// Crypto object, contains ratchets for other nodes
    pub crypto: Arc<RwLock<Crypto>>,
    /// Username used to identify this current node
    pub username: String,
}

impl Socket {
    /// Create a new `Socket` that is bound to the given address. This method also
    /// starts the background tasks that handle sending and receiving packets.
    pub async fn bind(addr: SocketAddr, username: String) -> Result<Self, SocketError> {
        // bind socket
        let socket: Arc<_> = UdpSocket::bind(addr)
            .await
            .map_err(SocketError::IoError)?
            .into();

        // create peers map
        let peers = Arc::new(RwLock::new(HashMap::new()));

        let crypto = Arc::new(RwLock::new(Crypto::new()));

        // start the outbound worker
        span!(tracing::Level::INFO, "socket::outbound")
            .in_scope(|| start_outbound_worker(socket.clone(), peers.clone()));

        Ok(Self {
            inner: socket,
            peers,
            crypto,
            username,
        })
    }

    /// Add a new peer to the list of connections, returning a channel for receiving
    /// packets from the peer.
    pub async fn add_peer(
        &mut self,
        addr: SocketAddr,
        initiate: bool,
    ) -> (mpsc::Sender<ProtocolPacket>, mpsc::Receiver<ProtocolPacket>) {
        let (peer, app_inbound_rx, net_outbound_rx) = Peer::new(
            addr,
            self.crypto.clone(),
            self.peers.clone(),
            self.username.clone(),
            initiate,
        );

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
        // insert the peer into the connections map - done in a separate block to avoid holding the
        // lock for too long
        {
            let mut connections = self.peers.write().await;
            connections.insert(addr, peer);
        }

        (app_outbound_tx, app_inbound_rx)
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

    /// Selects at most 3 peers randomly from list of peers - should
    /// probably employ round robin here.
    pub async fn select_gossip_peers(
        &self,
        skip: Option<SocketAddr>,
    ) -> Result<Vec<SocketAddr>, SocketError> {
        let targets: Vec<_> = self
            .peers
            .read()
            .await
            .keys()
            // skip if included
            .filter(|addr| skip.map(|skip_addr| skip_addr != **addr).unwrap_or(true))
            .cloned()
            .choose_multiple(&mut OsRng, GOSSIP_COUNT);

        // we have no targets!
        if targets.is_empty() {
            return Err(SocketError::NoPeer);
        }

        Ok(targets)
    }

    /// Sends non-encrypted message to a random group of peers
    /// Use this to send key exchange messages
    pub async fn send_gossip(
        &self,
        message: MessageType,
        destination: String,
    ) -> Result<(), SocketError> {
        let gossip_targets = self.select_gossip_peers(None).await?;
        for target in gossip_targets {
            let mut peers = self.peers.write().await;
            let target_peer = peers.get_mut(&target);
            if (target_peer
                .expect("No such peer")
                .send_gossip_single(message.clone(), destination.clone())
                .await)
                .is_ok()
            {}
        }
        Ok(())
    }

    /// Sends encrypted packet to random group of peers
    /// Use this to send a ProtocolPacket containing a PktMessage,
    /// which contains the message data
    pub async fn send_gossip_encrypted(
        &self,
        packet: ProtocolPacket,
        peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
        destination: String,
    ) -> Result<(), SocketError> {
        let gossip_targets = self.select_gossip_peers(None).await?;
        for target in gossip_targets {
            let mut peers_write = peers.write().await;
            let target_peer = peers_write.get_mut(&target);
            if (target_peer
                .expect("No such peer")
                .send_gossip_single_encrypted(packet.clone(), destination.clone())
                .await)
                .is_ok()
            {}
        }
        Ok(())
    }

    /// Forwards a received gossip packet as is to a random group of peers
    /// Internal use only for forwarding gossip packets not intended for us
    pub async fn forward_gossip(
        &self,
        packet: ProtocolPacket,
        skip: SocketAddr,
    ) -> Result<(), SocketError> {
        let gossip_targets = self.select_gossip_peers(Some(skip)).await?;
        for target in gossip_targets {
            let mut peers_write = self.peers.write().await;
            let target_peer = peers_write.get_mut(&target);
            if (target_peer
                .expect("No such peer")
                .send_packet(packet.clone())
                .await)
                .is_ok()
            {}
        }
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
            let _ = try_break!(socket.readable().await, "Error reading from socket");

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
