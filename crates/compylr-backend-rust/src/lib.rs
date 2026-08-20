//! The Rust backend: IR to Rust source.
//!
//! Knows nothing about which language the IR came from, and nothing about which language will
//! call the result. Exposing generated Rust to a host is the job of a bridge crate, one per
//! (source, target) pair.

pub mod runtime;
pub mod rust;

pub use rust::{RustBackend, rust_ident, rust_ty};
