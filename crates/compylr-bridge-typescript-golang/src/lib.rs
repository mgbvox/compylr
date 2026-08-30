//! The host bridge for the `(typescript, go)` pair.
//!
//! Generates the loader that makes a compiled unit callable from a TypeScript runtime. Like
//! `compylr-bridge-python-rust`, it *emits* that layer as text and does not itself link the
//! runtime it targets.
//!
//! This crate is the concrete reason bridges are keyed by the pair rather than by the target.
//! Nothing in `compylr-bridge-python-rust` carries over to it: PyO3 negotiates ownership,
//! exceptions, and string encoding one way, and a Node-API addon over cgo negotiates them
//! another. Sharing a backend does not mean sharing a calling convention, which is why bridges
//! cost N x M where frontends and backends cost N + M.

pub mod bridge;

pub use bridge::TypeScriptGoBridge;
