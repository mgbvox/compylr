//! compylr — transpiles a strict, fully annotated Python subset.
//!
//! The pipeline is deliberately split so that each stage can be tested on its own:
//!
//! ```text
//! source text ──frontend──> ruff AST ──lower──> compylr IR ──backend──> target code
//! ```
//!
//! The pipeline reaches generated Rust, including the PyO3 bindings that make it callable from
//! Python. The IR is independent of both Python and any target language: a backend chooses
//! concrete type spellings and operator syntax, so Rust, Go, C++, or TypeScript backends can all
//! consume the same tree.
//!
//! Two distinct PyO3 roles meet in this crate and are worth keeping apart. [`bridge`] exposes
//! *the compiler* to Python as `compylr._core`; [`backend::bindings`] *generates* PyO3 code onto
//! the user's compiled functions. They are different crates at runtime with different lifecycles.

pub mod backend;
pub mod bridge;
pub mod error;
pub mod frontend;
pub mod ir;
pub mod lower;
pub mod span;

pub use error::{ArtifactError, FrontendError, LowerError};
pub use ir::{BinOp, Expr, Function, Literal, Param, Stmt, Ty, Unit};
pub use span::Span;
