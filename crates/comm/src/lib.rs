//! # string-comms
//!
//! This crate contains the communication code for string

pub mod crypto;
pub mod peer;
pub mod socket;

pub use peer::Peer;
pub use socket::Socket;
