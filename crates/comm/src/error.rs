//! Defines the various error types used in this crate.

use thiserror::Error;

/// An enumeration of possible errors that can occur when working with the socket.
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
}

/// An enumeration of possible errors that can occur when working with packets.
#[derive(Error, Debug)]
pub enum PacketError {
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
