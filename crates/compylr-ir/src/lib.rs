//! compylr's intermediate representation.
//!
//! The one thing every frontend and every backend shares, and therefore the one place where a
//! leak in either direction is fatal: a Python spelling here would be inherited by every
//! backend's diagnostics, and a Rust type here would have to be ignored by every other target.
//!
//! The crate graph is what enforces this rather than discipline. No parser and no target
//! toolchain is among this crate's dependencies, so neither can be named from inside it.

pub mod artifact;
pub mod ir;

pub use artifact::ArtifactError;
pub use ir::*;
