//! # string-comms
//!
//! This crate contains the communication code for string

use std::{
    collections::VecDeque,
    io::{self, Cursor, Read, Write},
    net::SocketAddr,
    sync::Arc,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use connection::{NetworkPacketType, Peer, PeerState};
use error::{PacketError, SocketError};
use protocol::{packet::v1::Packet, prost::DecodeError, try_decode_packet};
use tokio::{net::UdpSocket, sync::RwLock};

mod connection;
mod error;

/// The magic number used to identify packets sent over the network.
const MAGIC: u32 = 0x010203;

/// The minimum size of an encoded [NetworkPacket].
const MIN_PACKET_SIZE: usize = 4 + 1 + 4 + 4;

/// A UDP packet sent over the network. These packets have the following format:
///
/// A header, consisting of:
/// - 4 bytes: Magic number (0x010203)
/// - 1 byte: Packet type (0 = SYN, 1 = ACK, 2 = SYNACK, 3 = HEARTBEAT, 4 = DATA)
/// - 4 bytes: Sequence number
/// - 4 bytes: Length of the data
///
/// Then arbitrary-length data, as defined by the protocol.
pub struct NetworkPacket {
    /// The type of packet.
    packet_type: NetworkPacketType,
    /// The sequence number of the packet.
    seq_number: u32,
    /// The length of the packet
    data_length: u32,
    /// The packet data. This is empty for SYN, ACK, SYNACK, and HEARTBEAT packets.
    data: Vec<u8>,
}

/// A wrapper around the [UdpSocket] type that provides a higher-level interface for sending and
/// receiving packets from multiple peers.
pub struct Socket {
    /// The inner [UdpSocket] used for sending and receiving packets.
    inner: Arc<UdpSocket>,
    /// A list of connections to other peers.
    peers: Arc<RwLock<Vec<Peer>>>,
}

impl Socket {
    /// Create a new `Socket` that is bound to the given address.
    pub async fn bind(addr: SocketAddr) -> Result<Self, SocketError> {
        // bind to the target address
        let socket = UdpSocket::bind(addr).await.map_err(SocketError::IoError)?;
        Ok(Self {
            inner: Arc::new(socket),
            peers: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Add a new peer to the list of connections.
    pub async fn add_peer(&mut self, addr: SocketAddr) {
        let mut connections = self.peers.write().await;
        connections.push(addr.into());
    }

    /// Spawns a new task to handle heartbeat events.
    pub async fn spawn_network_task(&self) {
        let peers = self.peers.clone();

        tokio::spawn(async move {
            loop {
                for peer in peers.read().await.iter() {
                    match peer.state {
                        PeerState::Disconnected => todo!(),
                        PeerState::Connecting => todo!(),
                        PeerState::Connected => todo!(),
                    }
                }
            }
        });
    }

    /// Send a packet to the given peer.
    pub async fn send_packet(
        &self,
        packet: &Packet,
        peer_addr: SocketAddr,
    ) -> Result<(), SocketError> {
        // encode protocol message, then wrap in frame
        let buf = protocol::try_encode_packet(&packet).map_err(|e| SocketError::EncodeFail(e))?;
        let buf = NetworkPacket::new(NetworkPacketType::Data, 0, &buf).encode()?;

        // send datagram
        self.inner
            .send_to(&buf, peer_addr)
            .await
            .map_err(SocketError::IoError)?;

        Ok(())
    }
}

impl NetworkPacket {
    /// Create a new packet with the given type, sequence number, and data.
    pub fn new<Data>(packet_type: NetworkPacketType, seq_number: u32, data: Data) -> Self
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
    pub fn from_bytes(bytes: &[u8]) -> Result<NetworkPacket, PacketError> {
        // check minimum packet length

        if bytes.len() < MIN_PACKET_SIZE {
            return Err(PacketError::BadSize);
        }

        // create reader
        let mut reader = Cursor::new(bytes);

        // check magic number
        let magic = reader.read_u24::<BigEndian>()?;
        if magic != MAGIC {
            return Err(PacketError::BadMagic);
        }

        // read packet header
        let packet_type = reader.read_u8()?.into();
        let seq_number = reader.read_u32::<BigEndian>()?;
        let data_length = reader.read_u32::<BigEndian>()?;

        if (data_length as usize) == 0 {
            return Ok(NetworkPacket {
                packet_type,
                seq_number,
                data_length,
                data: vec![],
            });
        }

        // read data
        let mut data = vec![0; data_length as usize];
        reader.read_exact(&mut data)?;

        Ok(NetworkPacket {
            packet_type,
            seq_number,
            data_length,
            data,
        })
    }
}

impl TryFrom<NetworkPacket> for Packet {
    type Error = DecodeError;

    fn try_from(value: NetworkPacket) -> Result<Self, Self::Error> {
        try_decode_packet(value.data)
    }
}
