//! Where implementations are registered.
//!
//! Every other crate is either an interface or an implementation of one; this is the single
//! place that knows the full set. Adding a language means adding a crate and one entry here —
//! in [`frontends`] if it is a source language, in [`backends`] if it is a target, and both if
//! it is both.

pub mod backends;
pub mod frontends;

pub use compylr_core::backend::{BackendError, GeneratedFiles, format_source};
pub use compylr_core::frontend::{Frontend, FrontendError, LoweringError};
