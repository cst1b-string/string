//! Defines the UDP socket abstraction and first-layer packet format used for communication between peers.

use std::{
    cmp::{self, Ordering},
    collections::{hash_map, HashMap},
    io::{self, Cursor, Read, Write},
    net::SocketAddr,
    sync::Arc,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use protocol::{prost::DecodeError, try_decode_packet, MessageType, ProtocolPacket};
use rand::{rngs::OsRng, seq::IteratorRandom};
use thiserror::Error;
use tokio::{
    net::UdpSocket,
    sync::{mpsc, RwLock},
};
use tracing::{debug, error, span, trace};

use crate::{
    crypto::{Crypto, DoubleRatchet},
    peer::{Peer, PeerError, PeerState},
    try_continue,
};

/// The magic number used to identify packets sent over the network.
pub const SOCKET_PACKET_MAGIC_NUMBER: u32 = 0x010203;

/// The minimum size of an encoded [SocketPacket].
// 3 (Magic) + 1 (Packet type) + 4 (Packet number) + 4 (Chunk number) + 4 (Data length)
pub const MIN_SOCKET_PACKET_SIZE: usize = 3 + 1 + 4 + 4 + 4;

/// The maximum size of a UDP datagram.
pub const UDP_MAX_DATAGRAM_SIZE: usize = 65_507;

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

/// An enumeration of possible errors that can occur when working with [Socket].
#[derive(Error, Debug)]
pub enum SocketError {
    /// An unknown error occurred.
    #[error("Unknown error")]
    Unknown,
    /// A connection to a peer timed ou.t
    #[error("Connection timed out")]
    ConnectionTimeout,
    /// A connection to a peer already exists.
    #[error("Already connected")]
    ConnectionExists,
    /// A connection to a peer is dead.
    #[error("Not connected")]
    ConnectionDead,
    /// An IO operation failed.
    #[error("Encountered an IO error")]
    IoError(#[from] std::io::Error),
    /// A packet failed to encode.
    #[error("Failed to encode packet")]
    EncodeFail(#[from] protocol::prost::EncodeError),
    /// A peer operation failed.
    #[error("Failed to process peer operation")]
    PeerError(#[from] PeerError),
    /// Trying to start a ratchet when it exists
    #[error("Ratchet exists")]
    RatchetExists,
    /// Tried to send gossip, but 0 peers connected
    #[error("No peer for gossip")]
    NoPeer,
}

/// An enumeration of possible errors that can occur when working with [ProtocolPacket]s.
#[derive(Error, Debug)]
pub enum SocketPacketDecodeError {
    /// An unknown error occurred.
    #[error("Unknown error")]
    Unknown,
    /// The magic number in the packet was incorrect.
    #[error("Magic number incorrect")]
    BadMagic,
    /// An unknown packet type was encountered.
    #[error("Unknown packet type")]
    BadPacketType,
    /// The packet was too small to be valid.
    #[error("Packet too small")]
    BadSize,
    /// An IO operation failed.
    #[error("Encountered an IO error")]
    IoError(#[from] std::io::Error),
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
            {
                let mut peers = self.peers.write().await;
                let target_peer = peers.get_mut(&target);
                if (target_peer
                    .expect("No such peer")
                    .send_gossip_single(message.clone(), destination.clone())
                    .await)
                    .is_ok()
                {}
            }
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
            {
                let mut peers_write = peers.write().await;
                let target_peer = peers_write.get_mut(&target);
                if (target_peer
                    .expect("No such peer")
                    .send_gossip_single_encrypted(packet.clone(), destination.clone())
                    .await)
                    .is_ok()
                {}
            }
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
            {
                let mut peers_write = self.peers.write().await;
                let target_peer = peers_write.get_mut(&target);
                if (target_peer
                    .expect("No such peer")
                    .send_packet(packet.clone())
                    .await)
                    .is_ok()
                {}
            }
        }
        Ok(())
    }

    /// Attempt to establish a DR ratchet with destination node
    /// Since this is done by gossip, it may not succeed if the node is down
    /// or inexistent
    pub async fn start_dr(&mut self, destination: String) -> Result<(), SocketError> {
        let mut crypto = self.crypto.write().await;
        match crypto.ratchets.entry(destination.clone()) {
            hash_map::Entry::Occupied(_) => Err(SocketError::RatchetExists),
            hash_map::Entry::Vacant(entry) => {
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
            if let Err(e) = socket.readable().await {
                eprintln!("Error reading from socket: {:?}", e);
                break;
            }

            // receive packet
            let (size, addr) = try_continue!(
                socket.recv_from(&mut buf).await,
                "Error reading from network: {:?}",
                err
            );
            // see if we know this peer
            let mut peers = peers.write().await;
            let peer = match peers.get_mut(&addr) {
                Some(peer) => peer,
                None => {
                    eprintln!("Unknown peer: {:?}", addr);
                    continue;
                }
            };

            // decode network packet
            let packet = match SocketPacket::decode(&buf[..size]) {
                Ok(packet) => packet,
                Err(e) => {
                    eprintln!("Error decoding packet: {:?}", e);
                    continue;
                }
            };

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
            let bytes = match packet.encode() {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Error encoding packet: {:?}", e);
                    continue;
                }
            };

            // send to network
            debug!(?destination, len = bytes.len(), "send packet to network");
            if let Err(e) = socket.send_to(&bytes, destination).await {
                eprintln!("Error sending packet to network: {:?}", e);
            }
        }
    });
}

/// A UDP packet sent over the network. These packets have the following format:
///
/// A header, consisting of:
/// - 4 bytes: Magic number (0x010203)
/// - 1 byte: Packet type (0 = SYN, 1 = ACK, 2 = SYNACK, 3 = HEARTBEAT, 4 = DATA)
/// - 4 bytes: Sequence number
/// - 4 bytes: Length of the data
///
/// Then arbitrary-length data, as defined by the protocol.
pub struct SocketPacket {
    /// The type of packet.
    pub packet_type: SocketPacketType,
    /// The sequence of the underlying [ProtocolPacket].
    pub packet_number: u32,
    /// The chunk number of the packet. This is only used for data packets.
    pub chunk_number: u32,
    /// The length of the packet
    pub data_length: u32,
    /// The packet data. This is empty for SYN, ACK, SYNACK, and HEARTBEAT packets.
    pub data: Vec<u8>,
}

impl PartialEq for SocketPacket {
    fn eq(&self, other: &Self) -> bool {
        self.packet_type == other.packet_type
            && self.packet_number == other.packet_number
            && self.chunk_number == other.chunk_number
    }
}

impl Eq for SocketPacket {}

// TODO: Justify that packet_number and chunk_number will be unique.

impl PartialOrd for SocketPacket {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        //  enforces lexicographic ordering, with packet number taking precedence
        if self.packet_number == other.packet_number {
            Some(self.chunk_number.cmp(&other.chunk_number))
        } else {
            Some(self.packet_number.cmp(&other.packet_number))
        }
    }
}

impl Ord for SocketPacket {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.packet_number == other.packet_number {
            self.chunk_number.cmp(&other.chunk_number)
        } else {
            self.packet_number.cmp(&other.packet_number)
        }
    }
}

/// An enumeration of the different types of network packets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum SocketPacketType {
    /// Packets sent by the initiating peer.
    Syn,
    /// Packets sent by a receiving peer.
    Ack,
    /// Packets sent by the initiating peer after receiving an ACK. Once this is sent, the connection is established.
    SynAck,
    /// Packets sent by either peer to keep the connection alive. This is done to avoid stateful firewalls from dropping the connection.
    Heartbeat,
    /// Actual communication data
    Data,
    /// An invalid packet.
    Invalid,
}

impl From<u8> for SocketPacketType {
    fn from(value: u8) -> Self {
        match value {
            0 => SocketPacketType::Syn,
            1 => SocketPacketType::Ack,
            2 => SocketPacketType::SynAck,
            3 => SocketPacketType::Heartbeat,
            4 => SocketPacketType::Data,
            _ => SocketPacketType::Invalid,
        }
    }
}

impl SocketPacket {
    /// Create a new packet with the given type, sequence number, and data.
    pub fn new<Data>(
        packet_type: SocketPacketType,
        packet_number: u32,
        chunk_number: u32,
        data: Data,
    ) -> Self
    where
        Data: AsRef<[u8]>,
    {
        Self {
            packet_type,
            packet_number,
            chunk_number,
            data_length: data.as_ref().len() as u32,
            data: Vec::from(data.as_ref()),
        }
    }

    /// Encode the packet into a byte buffer.
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(MIN_SOCKET_PACKET_SIZE);

        // write header
        buf.write_u24::<BigEndian>(SOCKET_PACKET_MAGIC_NUMBER)?;
        buf.write_u8(self.packet_type as u8)?;
        buf.write_u32::<BigEndian>(self.packet_number)?;
        buf.write_u32::<BigEndian>(self.chunk_number)?;
        buf.write_u32::<BigEndian>(self.data_length)?;

        // write data
        buf.write_all(&self.data)?;

        Ok(buf)
    }

    /// Decode a packet from the given byte buffer.
    pub fn decode<Data>(bytes: Data) -> Result<SocketPacket, SocketPacketDecodeError>
    where
        Data: AsRef<[u8]>,
    {
        let bytes = bytes.as_ref();

        // check minimum packet length
        if bytes.len() < MIN_SOCKET_PACKET_SIZE {
            return Err(SocketPacketDecodeError::BadSize);
        }

        // create reader
        let mut reader = Cursor::new(bytes);

        // check magic number
        let magic = reader.read_u24::<BigEndian>()?;
        if magic != SOCKET_PACKET_MAGIC_NUMBER {
            return Err(SocketPacketDecodeError::BadMagic);
        }

        // read packet header
        let packet_type = reader.read_u8()?.into();
        let packet_number = reader.read_u32::<BigEndian>()?;
        let chunk_number = reader.read_u32::<BigEndian>()?;
        let data_length = reader.read_u32::<BigEndian>()?;

        if (data_length as usize) == 0 {
            return Ok(SocketPacket::new(
                packet_type,
                packet_number,
                chunk_number,
                vec![],
            ));
        }

        // read data
        let mut data = vec![0; data_length as usize];
        reader.read_exact(&mut data)?;

        Ok(SocketPacket::new(
            packet_type,
            packet_number,
            chunk_number,
            data,
        ))
    }
}

impl TryFrom<SocketPacket> for ProtocolPacket {
    type Error = DecodeError;

    fn try_from(value: SocketPacket) -> Result<Self, Self::Error> {
        try_decode_packet(value.data)
    }
}
