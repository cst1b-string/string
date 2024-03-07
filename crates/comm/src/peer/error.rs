use crate::{
    crypto::{DoubleRatchetError, SigningError},
    socket::SocketPacket,
};

use string_protocol::{PacketDecodeError, PacketEncodeError, ProtocolPacket};
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

/// An enumeration of possible errors that can occur when working with peers.
#[derive(Error, Debug)]
pub enum PeerError {
    // Failed to send packet between threads.
    #[error("Failed to send packet to network thread")]
    NetworkSendFail(#[from] SendError<SocketPacket>),
    // Failed to send packet between threads.
    #[error("Failed to send packet to application")]
    ApplicationSendFail(#[from] SendError<ProtocolPacket>),
    // Failed to decode decrypted packet
    #[error("Failed to decode decrypted packet")]
    DecodeFail(#[from] PacketDecodeError),
    // Failed to encode packet for encryption
    #[error("Failed to encode packet for encryption")]
    EncodeFail(#[from] PacketEncodeError),
    // Failure in double ratchet
    #[error("Failure in double ratchet")]
    DRFail(#[from] DoubleRatchetError),
    // Generic error with signature
    #[error("Failure in signature verification")]
    SigFail(#[from] SigningError),
    /// The packet we received does not conform to some format
    #[error("Bad packet")]
    BadPacket,
}
