//! # prisma
//!
//! This crate contains all the generated code for the Prisma client, allowing upstream services to interface
//! with PostgreSQL in a type-safe and Rust-friendly manner.

/// The generated `prisma` module, which exports the generated Prisma client and its associated types.
mod prisma;

/// Re-export of the generated Prisma client.
pub use prisma::*;
