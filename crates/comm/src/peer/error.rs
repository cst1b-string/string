use crate::{
	crypto::{DoubleRatchetError, SigningError},
	socket::SocketPacket
};

use tokio::sync::mpsc::error::SendError;
use string_protocol::{ProtocolPacket, PacketDecodeError, PacketEncodeError};
use thiserror::Error;
use rsntp::{ConversionError, SynchronizationError};
use prost_types::TimestampError;

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
	// Failure in converting to [Timestamp] because its out of range
	#[error("Failure in converting to [Timestamp] because its out of range")]
	TimeStampFail(#[from] TimestampError),
	// Failure in converting to [Timestamp] because its out of range
	#[error("Failure in internal timestamp conversion")]
	ConvertFail(#[from] ConversionError),
	// Failure in time synchronisation
	#[error("Failure in time synchronization")]
	SynchronizationFail(#[from] SynchronizationError),
}
