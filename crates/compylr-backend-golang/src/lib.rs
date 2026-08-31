//! The Go backend: IR to Go source.
//!
//! Like every backend, it knows nothing about which language the IR came from and nothing about
//! which language will call the result. It is the reason the IR's target-neutrality is a
//! measurement rather than a claim: it consumes the same tree the Rust backend does, unchanged,
//! and the two disagree only in the spellings they choose (`int` becomes `i64` there and `int64`
//! here) and in the semantics each node already declares.
//!
//! Emission reads the mode a node carries — rounding, checking, index origin, text units — and
//! never the operation's name. A backend that read the name would be silently wrong whenever the
//! resolved behavior took the other language's stance.

pub mod compat;
pub mod emit;
pub mod golang;
pub mod types;

pub use golang::GoBackend;
