//! # string-protocol
//!
//! This crate contains the protocol definition for the string protocol.

use prost::{DecodeError, EncodeError, Message};

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

pub mod prost {
    pub use prost::*;
}

/// A type alias for [packet::v1::Packet], useful for disambiguating packet formants between network layers.
pub type ProtocolPacket = packet::v1::Packet;

/// A type alias for [packet::v1::packet::PacketType], useful for disambiguating packet formants between network layers.
pub type ProtocolPacketType = packet::v1::packet::PacketType;

/// Attempt to decode a packet from the given buffer.
pub fn try_decode_packet<Data>(buf: Data) -> Result<packet::v1::Packet, DecodeError>
where
    Data: AsRef<[u8]>,
{
    packet::v1::Packet::decode(buf.as_ref())
}

/// Attempt to encode a packet into a buffer.
pub fn try_encode_packet(packet: &packet::v1::Packet) -> Result<Vec<u8>, EncodeError> {
    let mut buf = Vec::new();
    packet.encode(&mut buf)?;
    Ok(buf)
}
