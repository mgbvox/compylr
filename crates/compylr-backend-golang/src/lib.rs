//! The Go backend: rendering IR as Go source.

pub mod compat;
pub mod emit;
pub mod golang;
pub mod types;

pub use golang::GoBackend;
