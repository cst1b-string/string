//! Defines error types for the [crate::socket] module.

use protocol::prost::DecodeError;
use thiserror::Error;

use crate::peer::PeerError;

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
    /// Packet decoding failed.
    #[error("Failed to decode packet")]
    DecodeFail(#[from] DecodeError),
}
