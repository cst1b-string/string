//! # string-comms
//!
//! This crate contains the communication code for string

pub mod peer;
pub mod socket;
pub mod crypto;

pub use peer::Peer;
pub use socket::Socket;
