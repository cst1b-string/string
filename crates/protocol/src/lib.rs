//! # string-protocol
//!
//! This crate contains the protocol definition for the string protocol.

/// Defines user types for the protocol.
pub mod users {
    include!(concat!(env!("OUT_DIR"), "/string.users.rs"));
}
