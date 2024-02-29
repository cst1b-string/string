//! # string-protocol
//!
//! This crate contains the protocol definition for the string protocol.

use std::io::{self, Read, Write};

use prost::{DecodeError, EncodeError, Message};
use thiserror::Error;

/// Utility macro to quickly define a module for a protocol.
macro_rules! include_protocol {
    ($name:literal, $version:ident) => {
        #[doc=concat!("Documentation for version", stringify!($version), "of the", $name, "protocol.")]
        pub mod $version {
            include!(concat!(
                env!("OUT_DIR"),
                "/str.",
                $name,
                ".",
                stringify!($version),
                ".rs",
            ));
        }
    };
}

/// Defines the user buffer types and data.
pub mod users {
    include_protocol!("users", v1);
}

/// Defines the messages buffer types and data.
pub mod messages {
    include_protocol!("messages", v1);
}

/// Defines the crypto buffer types and data.
pub mod crypto {
    include_protocol!("crypto", v1);
}

/// Defines the channel buffer types and data.
pub mod channels {
    include_protocol!("channels", v1);
}

/// Defines the network buffer types and data.
pub mod network {
    include_protocol!("network", v1);
}

/// Defines the packet buffer types and data.
pub mod packet {
    include_protocol!("packet", v1);
}

// Define the types for gossip packets
pub mod gossip {
    include_protocol!("gossip", v1);
}

pub mod prost {
    pub use prost::*;
}

/// A type alias for [packet::v1::Packet], useful for disambiguating packet formants between network layers.
pub type ProtocolPacket = packet::v1::Packet;

/// A type alias for [packet::v1::packet::PacketType], useful for disambiguating packet formants between network layers.
pub type ProtocolPacketType = packet::v1::packet::PacketType;

/// A type alias for [crypto::v1::signed_packet_internal::MessageType], useful for disambiguating message types in Gossip packets
pub type MessageType = crypto::v1::signed_packet_internal::MessageType;

pub type AttachmentType = messages::v1::message_attachment::AttachmentType;

/// An error that can occur when decoding a packet.
#[derive(Debug, Error)]
pub enum PacketDecodeError {
    #[error("failed to decode packet")]
    DecodeError(#[from] DecodeError),
    #[error("encountered an IO error while decoding packet")]
    IoError(#[from] io::Error),
}

/// An error that can occur when encoding a packet.
#[derive(Debug, Error)]
pub enum PacketEncodeError {
    #[error("failed to encode packet")]
    EncodeError(#[from] EncodeError),
    #[error("encountered an IO error while encoding packet")]
    IoError(#[from] io::Error),
}

/// Attempt to decode a packet from the given buffer.
pub fn try_decode_packet<Data>(buf: Data) -> Result<ProtocolPacket, PacketDecodeError>
where
    Data: AsRef<[u8]>,
{
    // decompress data - generously allocate 2x the size of the compressed data
    let mut decoder = flate2::read::GzDecoder::new(buf.as_ref());
    let mut buf = Vec::with_capacity(buf.as_ref().len() * 2);
    decoder.read_to_end(&mut buf)?;
    // decode packet
    Ok(packet::v1::Packet::decode(&*buf)?)
}

/// Attempt to encode a packet into a buffer.
pub fn try_encode_packet(packet: &ProtocolPacket) -> Result<Vec<u8>, PacketEncodeError> {
    // encode packet
    let mut buf = Vec::new();
    packet.encode(&mut buf)?;
    // compress data
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&buf)?;
    Ok(encoder.finish()?)
}

/// Attempt to encode a SignedPacketInternal for signing purposes
pub fn try_encode_internal_packet(packet: &crypto::v1::SignedPacketInternal) -> Result<Vec<u8>, PacketEncodeError>
{
    let mut buf = Vec::new();
    packet.encode(&mut buf)?;
    Ok(buf)
}
