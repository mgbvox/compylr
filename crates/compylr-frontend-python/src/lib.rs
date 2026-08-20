//! The Python frontend: ruff for parsing, and lowering for the supported subset.
//!
//! The only crate in the workspace that depends on a Python parser, which is what makes
//! "a backend cannot name Python" a property of the build rather than a convention.

pub mod component;
pub mod error;
pub mod frontend;
pub mod lower;
pub mod spelling;

pub use component::PythonFrontend;
pub use error::SourceError;
pub use frontend::{parse_file, parse_source};
pub use spelling::{PythonOperator, PythonTypeName};

use compylr_diagnostics::span::Span;
use ruff_text_size::TextRange;

/// Convert a parser range into a [`Span`].
///
/// A free function rather than a `From` impl on [`Span`], because the impl would have to live
/// beside one of the two types — and putting it beside `Span` would give the shared diagnostics
/// crate a dependency on a Python parser, which is the thing the split exists to prevent.
pub(crate) fn span_of(range: TextRange) -> Span {
    Span::new(range.start().to_u32(), range.end().to_u32())
}
