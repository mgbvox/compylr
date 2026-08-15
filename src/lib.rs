//! compylr — transpiles a strict, fully annotated Python subset.
//!
//! The pipeline is deliberately split so that each stage can be tested on its own:
//!
//! ```text
//! source text ──frontend──> ruff AST ──lower──> compylr IR ──backend──> target code
//! ```
//!
//! This crate currently implements everything up to the IR. The IR is independent of both
//! Python and any target language: a backend chooses concrete type spellings and operator
//! syntax, so Rust, Go, C++, or TypeScript backends can all consume the same tree.

pub mod backend;
pub mod error;
pub mod frontend;
pub mod ir;
pub mod lower;
pub mod span;

pub use error::{ArtifactError, FrontendError, LowerError};
pub use ir::{BinOp, Expr, Function, Literal, Param, Stmt, Ty, Unit};
pub use span::Span;
