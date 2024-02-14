//! Defines the packet format used for communication between peers.

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use protocol::{packet::v1::Packet, prost::DecodeError, try_decode_packet};
use std::io::{self, Cursor, Error, Read, Write};

use crate::error::PacketError;

/// The magic number used to identify packets sent over the network.
const MAGIC: u32 = 0x010203;

/// The minimum size of an encoded [NetworkPacket].
/// 4 bytes for MAGIC number, 1 for packet_type, 4 for seq_number, 4 for compressed_data_length, 4 for uncompressed_data_length
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
    pub packet_type: NetworkPacketType,
    /// The sequence number of the packet.
    pub seq_number: u32,
    /// The compressed length of the packet
    pub compressed_data_length: u32,
    /// The length of the packet
    pub uncompressed_data_length: u32,
    /// The packet data. This is empty for SYN, ACK, SYNACK, and HEARTBEAT packets.
    pub data: Vec<u8>,
}

/// An enumeration of the different types of network packets.
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum NetworkPacketType {
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

impl From<u8> for NetworkPacketType {
    fn from(value: u8) -> Self {
        match value {
            0 => NetworkPacketType::Syn,
            1 => NetworkPacketType::Ack,
            2 => NetworkPacketType::SynAck,
            3 => NetworkPacketType::Heartbeat,
            4 => NetworkPacketType::Data,
            _ => NetworkPacketType::Invalid,
        }
    }
}

impl NetworkPacket {
    /// Create a new packet with the given type, sequence number, and data.
    /// NOTE: compresses the sent data
    pub fn new<Data>(
        packet_type: NetworkPacketType,
        seq_number: u32,
        data: Data,
    ) -> io::Result<Self>
    where
        Data: AsRef<[u8]>,
    {
        // compress the data using Gzip
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        let _ = e.write_all(data.as_ref());
        let compressed_data = e.finish()?;
        Ok(Self {
            packet_type,
            seq_number,
            compressed_data_length: compressed_data.len() as u32,
            uncompressed_data_length: data.as_ref().len() as u32,
            data: compressed_data,
        })
    }

    /// Encode the packet into a byte buffer.
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(MIN_PACKET_SIZE);

        // write header
        buf.write_u24::<BigEndian>(MAGIC)?;
        buf.write_u8(self.packet_type as u8)?;
        buf.write_u32::<BigEndian>(self.seq_number)?;
        buf.write_u32::<BigEndian>(self.compressed_data_length)?;
        buf.write_u32::<BigEndian>(self.uncompressed_data_length)?;

        // write data
        buf.write_all(&self.data)?;

        Ok(buf)
    }

    /// Decode a packet from the given byte buffer.
    pub fn decode<Data>(bytes: Data) -> Result<NetworkPacket, PacketError>
    where
        Data: AsRef<[u8]>,
    {
        let bytes = bytes.as_ref();

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
        let compressed_data_length = reader.read_u32::<BigEndian>()?;
        let uncompressed_data_length = reader.read_u32::<BigEndian>()?;

        if (uncompressed_data_length as usize) == 0 {
            return Ok(NetworkPacket {
                packet_type,
                seq_number,
                compressed_data_length,
                uncompressed_data_length,
                data: vec![],
            });
        }

        // read compressed data
        let mut compressed_data = vec![0; compressed_data_length as usize];
        reader.read_exact(&mut compressed_data)?;

        Ok(NetworkPacket {
            packet_type,
            seq_number,
            compressed_data_length,
            uncompressed_data_length,
            data: compressed_data,
        })
    }

    /// decompress the data
    pub fn decompress(&self) -> io::Result<Vec<u8>> {
        let mut data = vec![0; self.uncompressed_data_length as usize];
        let mut gz_decoder = GzDecoder::new(&self.data[..]);
        gz_decoder.read_exact(&mut data)?;
        Ok(data)
    }
}

impl TryFrom<NetworkPacket> for Packet {
    type Error = DecodeError;

    fn try_from(value: NetworkPacket) -> Result<Self, Self::Error> {
        try_decode_packet(value.data)
    }
}
