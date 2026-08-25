//! The TypeScript frontend: turning TypeScript source text into IR.

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
