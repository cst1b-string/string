//! # lighthouse-prisma
//!
//! This crate contains all the generated code for the Prisma client, allowing upstream services to interface
//! with PostgreSQL in a type-safe and Rust-friendly manner.

/// The generated `prisma` module, which exports the generated Prisma client and its associated types.
#[allow(clippy::all, unused_imports)]
pub mod prisma;

/// Re-export of the generated Prisma client.
pub use prisma::*;

/// Re-export of `prisma-client-rust`.
pub mod client {
    pub use prisma_client_rust::*;
}
