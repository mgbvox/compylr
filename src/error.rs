//! Error types for the frontend and the lowering pass.
//!
//! Both are hand-written rather than derived from a macro crate: there are only two of them,
//! the impls are short, and it keeps the dependency surface at "the vendored ruff tree".
//!
//! Callers must be able to branch on *what went wrong* without matching on message text, since
//! message wording is presentation and changes freely. That is why both types expose an
//! explicit kind alongside their human-readable rendering.

use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use ruff_python_parser::ParseError;

use crate::span::Span;

/// A failure while turning Python source into a parse tree.
#[derive(Debug)]
pub enum FrontendError {
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

impl FrontendError {
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

impl fmt::Display for FrontendError {
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

impl Error for FrontendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Syntax { .. } => None,
        }
    }
}

impl From<ParseError> for FrontendError {
    fn from(error: ParseError) -> Self {
        Self::Syntax {
            message: error.error.to_string(),
            span: Span::from(error.location),
        }
    }
}

/// What category of rule a lowering diagnostic violated.
///
/// Tests and callers branch on this instead of on message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LowerErrorKind {
    /// A parameter, return, or local lacked a required type annotation.
    MissingAnnotation,
    /// An annotation was outside the supported set.
    UnsupportedType,
    /// A statement, expression, operator, or function form is not in the subset.
    UnsupportedConstruct,
    /// A name or call target did not resolve.
    Unresolved,
    /// A call passed the wrong number of arguments.
    ArityMismatch,
    /// A declared type disagreed with the value being bound.
    TypeMismatch,
    /// A name was assigned more than once.
    Reassignment,
    /// An integer literal did not fit the supported integer type.
    LiteralOutOfRange,
    /// Two functions in one unit share a name.
    DuplicateFunction,
    /// A binding's initializer type could not be determined from this source alone.
    ///
    /// Distinct from [`Self::MissingAnnotation`], which means the user genuinely omitted
    /// something required. This one may be resolvable with more context: the initializer calls a
    /// function defined in another source, whose signature is known once every source is
    /// assembled. Callers that will later see the whole program can defer it; callers that will
    /// not must report it.
    UndeterminedBinding,
    /// A function declared a return type but its body never returns a value.
    ///
    /// Distinct from [`Self::TypeMismatch`]: nothing disagrees about types here, the value is
    /// simply absent, and telling the user their types conflict would send them looking in the
    /// wrong place.
    MissingReturn,
}

impl LowerErrorKind {
    /// A stable identifier for this category, for callers that branch on it.
    ///
    /// Deliberately separate from [`Self::label`]: the label is prose and free to be reworded,
    /// while anything acting on a category needs a value that does not move under it.
    pub fn code(self) -> &'static str {
        match self {
            Self::MissingAnnotation => "missing_annotation",
            Self::UnsupportedType => "unsupported_type",
            Self::UnsupportedConstruct => "unsupported_construct",
            Self::Unresolved => "unresolved",
            Self::ArityMismatch => "arity_mismatch",
            Self::TypeMismatch => "type_mismatch",
            Self::Reassignment => "reassignment",
            Self::LiteralOutOfRange => "literal_out_of_range",
            Self::DuplicateFunction => "duplicate_function",
            Self::UndeterminedBinding => "undetermined_binding",
            Self::MissingReturn => "missing_return",
        }
    }

    /// Short human-readable label for this category.
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingAnnotation => "missing type annotation",
            Self::UnsupportedType => "unsupported type",
            Self::UnsupportedConstruct => "unsupported construct",
            Self::Unresolved => "unresolved name",
            Self::ArityMismatch => "wrong number of arguments",
            Self::TypeMismatch => "type mismatch",
            Self::Reassignment => "unsupported reassignment",
            Self::LiteralOutOfRange => "integer literal out of range",
            Self::MissingReturn => "missing return",
            Self::UndeterminedBinding => "undetermined binding type",
            Self::DuplicateFunction => "duplicate function",
        }
    }
}

/// A located diagnostic produced while lowering or validating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerError {
    kind: LowerErrorKind,
    message: String,
    span: Span,
}

impl LowerError {
    /// Build a diagnostic of `kind` describing `message` at `span`.
    pub fn new(kind: LowerErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
        }
    }

    /// Category of rule that was violated.
    pub fn kind(&self) -> LowerErrorKind {
        self.kind
    }

    /// Human-readable description, without location.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Where the offending construct appears.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Render with a `line:column` location resolved against the original source.
    ///
    /// The diagnostic itself stores only byte offsets so it stays cheap to compare and free of
    /// borrows; turning those into a line and column needs the text, so it happens on demand.
    pub fn render(&self, source: &str) -> String {
        format!("{}: {}", self.span.line_column(source), self.message)
    }
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}: {}",
            self.kind.label(),
            self.span,
            self.message
        )
    }
}

impl Error for LowerError {}

/// A failure while reading or writing the IR's on-disk artifact form.
///
/// Kept separate from [`LowerError`] because these describe a *file* being wrong rather than a
/// *program* being wrong: none of them carry a source location, and none of them are something a
/// user fixes by editing Python.
#[derive(Debug)]
pub enum ArtifactError {
    /// The bytes are not valid JSON, or do not describe a unit.
    Json(serde_json::Error),
    /// The artifact was written by an incompatible version of the format.
    UnsupportedVersion {
        /// Version recorded in the artifact.
        found: u32,
        /// Version this build understands.
        expected: u32,
    },
    /// The artifact's contents disagree with the fingerprint recorded alongside them.
    ///
    /// This catches truncation and hand-editing. Without it a corrupted artifact would load as
    /// a valid but different unit, and the rebuild cache would then happily reuse a build that
    /// does not correspond to any source.
    FingerprintMismatch {
        /// Fingerprint the artifact claims.
        recorded: String,
        /// Fingerprint its contents actually produce.
        computed: String,
    },
    /// The artifact lists two functions of the same name.
    DuplicateFunction(Box<LowerError>),
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(source) => write!(f, "artifact is not a readable IR document: {source}"),
            Self::UnsupportedVersion { found, expected } => write!(
                f,
                "artifact format version {found} is not supported; this build reads version {expected}"
            ),
            Self::FingerprintMismatch { recorded, computed } => write!(
                f,
                "artifact is corrupt: it records fingerprint {recorded} but its contents produce {computed}"
            ),
            Self::DuplicateFunction(source) => {
                write!(f, "artifact contains a duplicate function: {source}")
            }
        }
    }
}

impl Error for ArtifactError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(source) => Some(source),
            Self::DuplicateFunction(source) => Some(source),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for ArtifactError {
    fn from(source: serde_json::Error) -> Self {
        Self::Json(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_failure_names_the_path_and_is_distinguishable() {
        let error = FrontendError::io(
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
        let error = FrontendError::io(
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
        let error = FrontendError::Syntax {
            message: "unexpected token".to_string(),
            span: Span::new(4, 7),
        };
        assert!(error.is_syntax());
        assert!(!error.is_io());
        assert_eq!(error.span(), Some(Span::new(4, 7)));
        assert!(error.path().is_none());
    }

    #[test]
    fn lower_error_exposes_kind_message_and_span() {
        let error = LowerError::new(
            LowerErrorKind::MissingAnnotation,
            "parameter 'a' needs a type annotation",
            Span::new(8, 9),
        );
        assert_eq!(error.kind(), LowerErrorKind::MissingAnnotation);
        assert!(error.message().contains('a'));
        assert_eq!(error.span(), Span::new(8, 9));
        assert!(error.to_string().contains("missing type annotation"));
    }

    #[test]
    fn lower_error_renders_line_and_column() {
        let source = "def f():\n    x = 1\n";
        let error = LowerError::new(
            LowerErrorKind::MissingAnnotation,
            "x needs a type",
            Span::new(13, 14),
        );
        let rendered = error.render(source);
        assert!(
            rendered.starts_with("2:"),
            "expected line 2, got {rendered}"
        );
        assert!(rendered.contains("x needs a type"));
    }

    #[test]
    fn every_kind_has_a_distinct_label() {
        let kinds = [
            LowerErrorKind::MissingAnnotation,
            LowerErrorKind::UnsupportedType,
            LowerErrorKind::UnsupportedConstruct,
            LowerErrorKind::Unresolved,
            LowerErrorKind::ArityMismatch,
            LowerErrorKind::TypeMismatch,
            LowerErrorKind::Reassignment,
            LowerErrorKind::LiteralOutOfRange,
            LowerErrorKind::DuplicateFunction,
        ];
        let mut labels: Vec<&str> = kinds.iter().map(|k| k.label()).collect();
        labels.sort_unstable();
        let count = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), count, "labels must be distinct");
    }
}
