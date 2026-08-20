//! The component model: what a frontend, a backend, and a host bridge are, and how one is found.
//!
//! This crate defines interfaces and knows no implementation of them. The concrete tables live
//! in `compylr-registry`, which may depend on both this crate and every implementation — putting
//! them here would require core to depend on the crates that depend on core.

pub mod backend;

pub use backend::{Backend, BackendError, GeneratedFiles, format_source};
