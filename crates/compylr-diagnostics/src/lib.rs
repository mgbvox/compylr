//! Source locations and located diagnostics.
//!
//! The bottom of the workspace: every other crate depends on this one, and this one depends on
//! nothing. That is deliberate. A frontend's parser, a backend's toolchain, or a serialization
//! format pulled in here would become a dependency of the entire compiler.

pub mod error;
pub mod span;

pub use error::{LowerError, LowerErrorKind};
pub use span::{LineColumn, Span};
