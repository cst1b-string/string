//! Defines the [SocketPacket] type and related functionality.

use std::{
    cmp::Ordering,
    io::{self, Cursor, Read, Write},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
// use flate2::read::GzDecoder;
// use string_protocol::{try_decode_packet, ProtocolPacket};

use super::error::SocketPacketDecodeError;

/// The magic number used to identify packets sent over the network.
pub const SOCKET_PACKET_MAGIC_NUMBER: u32 = 0x010203;

/// The minimum size of an encoded [SocketPacket].
// 3 (Magic) + 1 (Packet type) + 4 (Packet number) + 4 (Chunk number) + 4 (Data length)
pub const MIN_SOCKET_PACKET_SIZE: usize = 3 + 1 + 4 + 4 + 4;

/// The maximum size of a UDP datagram.
pub const UDP_MAX_DATAGRAM_SIZE: usize = 40_000;

/// A UDP packet sent over the network. These packets have the following format:
///
/// A header, consisting of:
/// - 4 bytes: Magic number (0x010203)
/// - 1 byte: Packet type (0 = SYN, 1 = ACK, 2 = SYNACK, 3 = HEARTBEAT, 4 = DATA)
/// - 4 bytes: Sequence number
/// - 4 bytes: Length of the data
///
/// Then arbitrary-length data, as defined by the protocol.
#[derive(Clone, Debug)]
pub struct SocketPacket {
    /// The type of packet.
    pub packet_type: SocketPacketType,
    /// The sequence of the underlying [ProtocolPacket].
    pub packet_number: u32,
    /// The chunk number of the packet. This is only used for data packets.
    pub chunk_number: u32,
    /// The length of the data within the packet.
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
    ) -> io::Result<Self>
    where
        Data: AsRef<[u8]>,
    {
        let data = data.as_ref().to_owned();
        Ok(Self {
            packet_type,
            packet_number,
            chunk_number,
            data_length: data.len() as u32,
            data,
        })
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

    /// Create an empty packet with the given type, sequence number, and chunk number. Useful for non-data packets.
    pub fn empty(packet_type: SocketPacketType, packet_number: u32, chunk_number: u32) -> Self {
        Self {
            packet_type,
            packet_number,
            chunk_number,
            data: vec![],
            data_length: 0,
        }
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
            )?);
        }

        // read data
        let mut data = vec![0; data_length as usize];
        reader.read_exact(&mut data)?;

        Ok(SocketPacket::new(
            packet_type,
            packet_number,
            chunk_number,
            data,
        )?)
    }

    // Decompress the data.
    // pub fn decompress(&self) -> std::io::Result<Vec<u8>> {
    //     let mut data = Vec::new();
    //     let mut gz_decoder = GzDecoder::new(self.data.as_slice());
    //     gz_decoder.read_to_end(&mut data)?;
    //     Ok(data)
    // }
}

// impl TryFrom<SocketPacket> for ProtocolPacket {
//     type Error = SocketPacketDecodeError;

//     fn try_from(value: SocketPacket) -> Result<Self, Self::Error> {
//         Ok(try_decode_packet(value.decompress()?)?)
//     }
// }
