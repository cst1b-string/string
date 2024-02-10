//! # string-comms
//!
//! This crate contains the communication code for string

pub mod crypto;
pub mod peer;
pub mod socket;
pub mod util;

pub use peer::Peer;
pub use socket::Socket;

/// The default port for the socket.
pub const DEFAULT_PORT: u16 = 54321;
