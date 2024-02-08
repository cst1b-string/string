//! Defines the UDP socket abstraction and first-layer packet format used for communication between peers.

use std::{
    cmp::Ordering,
    collections::HashMap,
    io::{self, Cursor, Read, Write},
    net::SocketAddr,
    sync::Arc,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use protocol::{prost::DecodeError, try_decode_packet, ProtocolPacket};
use thiserror::Error;
use tokio::{
    net::UdpSocket,
    sync::{mpsc, RwLock},
};

use crate::peer::{Peer, PeerError};

/// The magic number used to identify packets sent over the network.
const MAGIC: u32 = 0x010203;

/// The minimum size of an encoded [NetworkPacket].
const MIN_PACKET_SIZE: usize = 4 + 1 + 4 + 4;

/// The maximum size of a UDP datagram.
const UDP_MAX_DATAGRAM_SIZE: usize = 65_507;

/// A wrapper around the [UdpSocket] type that provides a higher-level interface for sending and
/// receiving packets from multiple peers.
pub struct Socket {
    /// The inner [UdpSocket] used for sending and receiving packets.
    inner: Arc<UdpSocket>,
    /// A map of connections to other peers.
    peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
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
    pub async fn bind(addr: SocketAddr) -> Result<Self, SocketError> {
        // bind socket
        let socket: Arc<_> = UdpSocket::bind(addr)
            .await
            .map_err(SocketError::IoError)?
            .into();

        // create peers map
        let peers = Arc::new(RwLock::new(HashMap::new()));

        // start the outbound worker
        start_outbound_worker(socket.clone(), peers.clone());

        Ok(Self {
            inner: socket,
            peers,
        })
    }

    /// Add a new peer to the list of connections, returning a channel for receiving
    /// packets from the peer.
    pub async fn add_peer(
        &mut self,
        addr: SocketAddr,
    ) -> (mpsc::Sender<ProtocolPacket>, mpsc::Receiver<ProtocolPacket>) {
        let (peer, app_inbound_rx, net_outbound_rx) = Peer::new(addr, true);
        let app_outbound_tx = peer.app_outbound_tx.clone();

        // spawn the inbound peer task
        spawn_inbound_peer_task(self.inner.clone(), peer.destination, net_outbound_rx);

        // insert the peer into the connections map - done in a separate block to avoid holding the
        // lock for too long
        {
            let mut connections = self.peers.write().await;
            connections.insert(addr, peer);
        }

        (app_outbound_tx, app_inbound_rx)
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
}

/// Start the outbound network worker.
fn start_outbound_worker(socket: Arc<UdpSocket>, peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>) {
    tokio::spawn(async move {
        let mut buf = [0; UDP_MAX_DATAGRAM_SIZE];
        loop {
            // wait for socket to be readable
            if let Err(e) = socket.readable().await {
                eprintln!("Error reading from socket: {:?}", e);
                break;
            }

            // receive packet
            let (size, addr) = match socket.recv_from(&mut buf).await {
                Ok((size, addr)) => (size, addr),
                Err(e) => {
                    eprintln!("Error reading from network: {:?}", e);
                    continue;
                }
            };

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
    /// The sequence number of the packet.
    pub seq_number: u32,
    /// The length of the packet
    pub data_length: u32,
    /// The packet data. This is empty for SYN, ACK, SYNACK, and HEARTBEAT packets.
    pub data: Vec<u8>,
}

impl PartialEq for SocketPacket {
    fn eq(&self, other: &Self) -> bool {
        self.packet_type == other.packet_type && self.seq_number == other.seq_number
    }
}

impl Eq for SocketPacket {}

impl PartialOrd for SocketPacket {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.seq_number.cmp(&other.seq_number))
    }
}

impl Ord for SocketPacket {
    fn cmp(&self, other: &Self) -> Ordering {
        self.seq_number.cmp(&other.seq_number)
    }
}

/// An enumeration of the different types of network packets.
#[derive(Clone, Copy, PartialEq, Eq)]
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
    pub fn new<Data>(packet_type: SocketPacketType, seq_number: u32, data: Data) -> Self
    where
        Data: AsRef<[u8]>,
    {
        Self {
            packet_type,
            seq_number,
            data_length: data.as_ref().len() as u32,
            data: Vec::from(data.as_ref()),
        }
    }

    /// Encode the packet into a byte buffer.
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(MIN_PACKET_SIZE);

        // write header
        buf.write_u24::<BigEndian>(MAGIC)?;
        buf.write_u8(self.packet_type as u8)?;
        buf.write_u32::<BigEndian>(self.seq_number)?;
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
        if bytes.len() < MIN_PACKET_SIZE {
            return Err(SocketPacketDecodeError::BadSize);
        }

        // create reader
        let mut reader = Cursor::new(bytes);

        // check magic number
        let magic = reader.read_u24::<BigEndian>()?;
        if magic != MAGIC {
            return Err(SocketPacketDecodeError::BadMagic);
        }

        // read packet header
        let packet_type = reader.read_u8()?.into();
        let seq_number = reader.read_u32::<BigEndian>()?;
        let data_length = reader.read_u32::<BigEndian>()?;

        if (data_length as usize) == 0 {
            return Ok(SocketPacket {
                packet_type,
                seq_number,
                data_length,
                data: vec![],
            });
        }

        // read data
        let mut data = vec![0; data_length as usize];
        reader.read_exact(&mut data)?;

        Ok(SocketPacket {
            packet_type,
            seq_number,
            data_length,
            data,
        })
    }
}

impl TryFrom<SocketPacket> for ProtocolPacket {
    type Error = DecodeError;

    fn try_from(value: SocketPacket) -> Result<Self, Self::Error> {
        try_decode_packet(value.data)
    }
}