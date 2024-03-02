//! Defines the UDP socket abstraction and first-layer packet format used for communication between peers.

pub mod error;
mod packet;

use std::{
    collections::{hash_map::Entry, HashMap},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
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
    peer::{Peer, PeerState, CHANNEL_SIZE},
    try_break, try_continue,
};

// re-export types
pub use self::error::{SocketError, SocketPacketDecodeError};
pub use self::packet::{
    SocketPacket, SocketPacketType, MIN_SOCKET_PACKET_SIZE, UDP_MAX_DATAGRAM_SIZE,
};

use string_protocol::crypto;

use pgp::composed::SignedSecretKey;

/// Number of peers to send gossip to
const GOSSIP_COUNT: usize = 3;

pub enum GossipAction {
    /// Send a normal unencrypted packet to some peers via gossip
    Send,
    /// Same as above, but encrypted. This should be the common case
    SendEncrypted,
    /// We received a gossip packet, please forward it
    Forward,
    /// Actually not a gossip, just send directly
    SendDirect,
}

pub struct Gossip {
    /// What to do with the gossip
    pub action: GossipAction,
    /// When this gossip is received from, is None if sending from current node
    pub addr: Option<SocketAddr>,
    /// Gossip packet to forward (either this or the one below)
    pub packet: Option<ProtocolPacket>,
    /// Gossip message to forward (either this or the one above)
    pub message: Option<MessageType>,
    /// Destination to send to; not needed when forwarding
    pub dest: Option<String>,
    ///
    pub dest_sockaddr: Option<SocketAddr>,
}

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
    /// Channel used to send gossip
    pub gossip_tx: mpsc::Sender<Gossip>,
}

impl Socket {
    /// Create a new `Socket` that is bound to the given address. This method also
    /// starts the background tasks that handle sending and receiving packets.
    pub async fn bind(addr: SocketAddr, secret_key: SignedSecretKey) -> Result<Self, SocketError> {
        // bind socket
        let socket: Arc<_> = UdpSocket::bind(addr)
            .await
            .map_err(SocketError::IoError)?
            .into();

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

        // start the perodic worker
        span!(tracing::Level::INFO, "socket::periodic")
            .in_scope(|| start_periodic_worker(peers.clone()));

        if secret_key.details.users.len() != 1 {
            // Why do we have a weird number of users
            panic!("Invalid number of users in secret key - programming error")
        }

        let username = Crypto::get_pubkey_username(secret_key.into());

        Ok(Self {
            inner: socket,
            peers,
            crypto,
            username,
            gossip_tx,
        })
    }

    /// Add a new peer to the list of connections, returning a channel for receiving
    /// packets from the peer.
    pub async fn add_peer(
        &mut self,
        addr: SocketAddr,
        fingerprint: Vec<u8>,
        initiate: bool,
    ) -> (mpsc::Sender<ProtocolPacket>, mpsc::Receiver<ProtocolPacket>) {
        let (peer, app_inbound_rx, net_outbound_rx) = Peer::new(
            addr,
            self.crypto.clone(),
            self.peers.clone(),
            self.username.clone(),
            self.gossip_tx.clone(),
            fingerprint,
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

fn start_gossip_worker(
    mut gossip_rx: mpsc::Receiver<Gossip>,
    peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
) {
    tokio::spawn(async move {
        loop {
            trace!("start gossip task loop");

            // receive gossip
            let Gossip {
                action,
                addr: skip,
                packet,
                message,
                dest,
                dest_sockaddr,
            } = match gossip_rx.recv().await {
                Some(gossip) => gossip,
                None => break,
            };

            if let GossipAction::SendDirect = action {
                let mut peers_obj = peers.write().await;
                let peer = peers_obj.get_mut(&dest_sockaddr.unwrap());
                if peer.is_none() {
                    continue;
                }
                let peer_ = peer.unwrap();
                let peername = peer_.peername.clone();
                let _ = peer_
                    .send_gossip_single(message.unwrap().clone(), peername.unwrap())
                    .await;
                continue;
            }

            // Selects at most 3 peers randomly from list of peers - should
            // probably employ round robin here.
            let targets: Vec<_> = peers
                .read()
                .await
                .keys()
                // skip if included
                .filter(|addr| skip.map(|skip_addr| skip_addr != **addr).unwrap_or(true))
                .cloned()
                .choose_multiple(&mut OsRng, GOSSIP_COUNT);

            // we have no targets!
            if targets.is_empty() {
                continue;
            }

            for target in targets {
                let mut peers_write = peers.write().await;
                let target_peer = peers_write.get_mut(&target);
                let target_peer_ = target_peer.expect("No such peer");
                let res = match action {
                    GossipAction::Send => {
                        trace!("sending gossip {:?}", message);
                        target_peer_
                            .send_gossip_single(message.clone().unwrap(), dest.clone().unwrap())
                            .await
                    }
                    GossipAction::SendEncrypted => {
                        trace!("sending encrypted gossip {:?}", packet);
                        target_peer_
                            .send_gossip_single_encrypted(
                                packet.clone().unwrap(),
                                dest.clone().unwrap(),
                            )
                            .await
                    }
                    GossipAction::Forward => {
                        trace!("forwarding gossip {:?}", packet);
                        target_peer_.send_packet(packet.clone().unwrap()).await
                    }
                    GossipAction::SendDirect => unreachable!(),
                };
                if res.is_ok() {}
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
