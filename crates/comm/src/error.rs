//! Defines the various error types used in this crate.

use protocol::packet::v1::Packet;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::packet::NetworkPacket;

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
    /// A peer operation failed.
    #[error("Failed to process peer operation")]
    PeerError(#[from] PeerError),
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

/// An enumeration of possible errors that can occur when working with peers.
#[derive(Error, Debug)]
pub enum PeerError {
    // Failed to send packet between threads.
    #[error("Failed to send packet to network thread")]
    NetworkSendFail(#[from] SendError<NetworkPacket>),
    // Failed to send packet between threads.
    #[error("Failed to send packet to application")]
    ApplicationSendFail(#[from] SendError<Packet>),
}
