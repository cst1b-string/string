//! Defines the various error types used in this crate.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SocketError {
    #[error("Unknown error")]
    Unknown,
    #[error("Connection timed out")]
    ConnectionTimeout,
    #[error("Already connected")]
    ConnectionExists,
    #[error("Not connected")]
    ConnectionDead,
    #[error("Encountered an IO error")]
    IoError(#[from] std::io::Error),
    #[error("Failed to encode packet")]
    EncodeFail(#[from] protocol::prost::EncodeError),
}

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("Unknown error")]
    Unknown,
    #[error("Magic number incorrect")]
    BadMagic,
    #[error("Unknown packet type")]
    BadPacketType,
    #[error("Packet too small")]
    BadSize,
    #[error("Encountered an IO error")]
    IoError(#[from] std::io::Error),
}
