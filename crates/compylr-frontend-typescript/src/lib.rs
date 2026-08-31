//! The TypeScript frontend: oxc for parsing, and lowering for the supported subset.
//!
//! The only crate in the workspace that depends on a TypeScript parser, which is what makes "a
//! backend cannot name TypeScript" a property of the build rather than a convention — the same
//! rule `compylr-frontend-python` establishes for Python.
//!
//! Its existence is what makes the IR's source-neutrality checkable. Members accepted by both
//! this corpus and the Python one under the same name are compared node by node, and the
//! recorded divergence score is what would fail if either frontend started encoding its own
//! language's habits into the tree.
//!
//! Where the two languages genuinely differ, they are allowed to: TypeScript declines a
//! parameter reassignment that Python accepts, and has no three-clause equivalent for some
//! `range()` loops. Those are recorded as unpaired members rather than smoothed over, because a
//! frontend that quietly invented a construct to make the score look better would be defeating
//! the measurement.

#![allow(
    clippy::too_many_arguments,
    clippy::collapsible_if,
    clippy::type_complexity,
    clippy::single_match
)]

pub mod error;
pub mod frontend;
pub mod lower;
pub mod spelling;

pub use frontend::TypeScriptFrontend;
