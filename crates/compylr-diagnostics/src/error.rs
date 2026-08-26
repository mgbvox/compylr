//! Located diagnostics, shared by every frontend and every backend.
//!
//! Hand-written rather than derived from a macro crate: the impls are short, and this crate
//! sits below the IR, so a dependency added here would reach everything.
//!
//! Callers must be able to branch on *what went wrong* without matching on message text, since
//! message wording is presentation and changes freely. That is why the type exposes an explicit
//! kind alongside its human-readable rendering.
//!
//! The kinds are deliberately not Python's. "Missing annotation", "unresolved name", and
//! "arity mismatch" are categories any frontend for a statically annotated subset produces, and
//! [`crate::error::LowerError`] is raised by the IR's own validation as well as by lowering.

use std::error::Error;
use std::fmt;

use crate::span::Span;

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
    /// A bare annotation may name a class supplied by another source in the complete unit.
    ///
    /// Single-source validation may defer this category. Complete-unit lowering never does: the
    /// same category then identifies an unknown or misspelled class at its original annotation.
    UnresolvedClassAnnotation,
    /// A borrow-only instance parameter was used where an owned value would be required.
    BorrowedInstanceEscape,
    /// `break` or `continue` appeared with no enclosing loop.
    ///
    /// Distinct from [`Self::UnsupportedConstruct`]: the construct is supported, it is only in the
    /// wrong place, and saying "unsupported" would send the user looking for an alternative that
    /// does not need to exist.
    LoopControlOutsideLoop,
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
            Self::UnresolvedClassAnnotation => "unresolved_class_annotation",
            Self::BorrowedInstanceEscape => "borrowed_instance_escape",
            Self::LoopControlOutsideLoop => "loop_control_outside_loop",
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
            Self::LoopControlOutsideLoop => "loop control outside a loop",
            Self::MissingReturn => "missing return",
            Self::UndeterminedBinding => "undetermined binding type",
            Self::UnresolvedClassAnnotation => "unresolved class annotation",
            Self::BorrowedInstanceEscape => "borrowed instance escape",
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

#[cfg(test)]
mod tests {
    use super::*;

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
            LowerErrorKind::UndeterminedBinding,
            LowerErrorKind::UnresolvedClassAnnotation,
            LowerErrorKind::BorrowedInstanceEscape,
            LowerErrorKind::LoopControlOutsideLoop,
            LowerErrorKind::MissingReturn,
        ];
        let mut labels: Vec<&str> = kinds.iter().map(|k| k.label()).collect();
        labels.sort_unstable();
        let count = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), count, "labels must be distinct");
    }
}
