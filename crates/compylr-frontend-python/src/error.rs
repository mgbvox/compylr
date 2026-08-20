//! Failures specific to reading Python source.
//!
//! Separate from [`compylr_diagnostics::error::LowerError`], which describes a *program* being
//! outside the supported subset. These describe the input never becoming a tree at all, and they
//! wrap the parser's own error type — which is why they live in the frontend rather than in the
//! shared diagnostics crate.
//!
//! Named `SourceError` rather than `FrontendError` because the latter now means "no such
//! frontend" in [`compylr_core`], symmetrically with `BackendError`. Two types with one name,
//! one of them about a missing component and the other about a malformed file, is a confusion
//! worth a rename.

use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use ruff_python_parser::ParseError;

use compylr_diagnostics::span::Span;

use crate::span_of;

/// A failure while turning Python source into a parse tree.
#[derive(Debug)]
pub enum SourceError {
    /// The source could not be read from disk.
    Io {
        /// Path that was requested.
        path: PathBuf,
        /// Underlying operating-system error.
        source: io::Error,
    },
    /// The source was read but is not valid Python.
    Syntax {
        /// Parser description of the problem.
        message: String,
        /// Where parsing gave up.
        span: Span,
    },
}

impl SourceError {
    /// Build an I/O failure that remembers which path was requested.
    ///
    /// The path cannot come from [`io::Error`] itself, so it is threaded in here rather than
    /// through a `From` impl — an I/O error that cannot say which file it was about is not
    /// worth reporting.
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Whether this failure came from reading the source.
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io { .. })
    }

    /// Whether this failure came from parsing the source.
    pub fn is_syntax(&self) -> bool {
        matches!(self, Self::Syntax { .. })
    }

    /// Location of a syntax failure, if this is one.
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Io { .. } => None,
            Self::Syntax { span, .. } => Some(*span),
        }
    }

    /// Path involved in an I/O failure, if this is one.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io { path, .. } => Some(path.as_path()),
            Self::Syntax { .. } => None,
        }
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // `source` is rendered via Display, not Debug, so the message stays readable.
            Self::Io { path, source } => {
                write!(f, "could not read {}: {source}", path.display())
            }
            Self::Syntax { message, span } => {
                write!(f, "invalid Python syntax at {span}: {message}")
            }
        }
    }
}

impl Error for SourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Syntax { .. } => None,
        }
    }
}

impl From<ParseError> for SourceError {
    fn from(error: ParseError) -> Self {
        Self::Syntax {
            message: error.error.to_string(),
            span: span_of(error.location),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_failure_names_the_path_and_is_distinguishable() {
        let error = SourceError::io(
            "/tmp/missing.py",
            io::Error::new(io::ErrorKind::NotFound, "no such file"),
        );
        assert!(error.is_io());
        assert!(!error.is_syntax());
        assert_eq!(error.path().unwrap().to_str().unwrap(), "/tmp/missing.py");
        assert!(error.to_string().contains("/tmp/missing.py"));
        assert!(error.source().is_some());
    }

    #[test]
    fn io_display_uses_the_causes_display_not_debug() {
        let error = SourceError::io(
            "/tmp/x.py",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );
        let rendered = error.to_string();
        assert!(rendered.contains("permission denied"));
        // Debug formatting of io::Error contains `Custom {` / `kind:`; Display must not.
        assert!(!rendered.contains("kind:"));
    }

    #[test]
    fn syntax_failure_carries_a_span_and_is_distinguishable() {
        let error = SourceError::Syntax {
            message: "unexpected token".to_string(),
            span: Span::new(4, 7),
        };
        assert!(error.is_syntax());
        assert!(!error.is_io());
        assert_eq!(error.span(), Some(Span::new(4, 7)));
        assert!(error.path().is_none());
    }
}
