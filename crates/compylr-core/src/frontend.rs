//! What a source language is, from the compiler's point of view.
//!
//! A frontend turns source text into a [`Unit`] and says nothing else about itself except its
//! name and what its language needs preserved. Everything specific to a language — its parser,
//! its spellings, its idea of what `/` means — lives inside the implementation and reaches no
//! other crate.
//!
//! Note what the trait takes: *texts*, plural, not a path. A project's decorated functions arrive
//! as separate sources that call each other, so assembling them into one unit is the frontend's
//! job rather than the caller's. A caller that had to gather signatures across sources itself
//! would be doing the source language's typing rules on the language's behalf.

use std::error::Error;
use std::fmt;

use compylr_ir::{Behavior, Guarantee, LanguageBehavior, Unit};

/// A source language frontend.
///
/// `Debug` is required for the same reason it is on a backend: a lookup result should be
/// unwrappable in a test and reportable in a diagnostic without every call site special-casing a
/// trait object.
pub trait Frontend: fmt::Debug + Send + Sync {
    /// The registry name this frontend is selected by.
    fn name(&self) -> &'static str;

    /// The guarantees this language needs preserved for a translation to mean what the source
    /// meant.
    ///
    /// Declared rather than assumed, because the alternative is that every backend hardcodes one
    /// language's expectations and the second frontend silently inherits them.
    fn requires(&self) -> &'static [Guarantee];

    /// What **this language** means, on every behavior axis.
    ///
    /// Required rather than defaulted, for the reason [`Frontend::requires`] is: a default of
    /// "everything reported" would be one language's answers handed to every other, and a
    /// frontend that inherited them would look correct until someone compiled a negative index.
    ///
    /// Describes this language only. A frontend that declared anything about a *target* would be
    /// the first entry in a table costing N x M, which is the whole reason the two sides declare
    /// separately and something else resolves them.
    fn behavior(&self) -> &'static LanguageBehavior;

    /// Lower source texts into a single unit.
    ///
    /// Every source is available to every other: a call from one to another must type, which is
    /// the arrangement a project of separately marked functions always produces. The result does
    /// not depend on the order the sources arrive in.
    ///
    /// The behavior rides on each [`Source`] rather than on the call, because it is a property of
    /// the *member*: a project may mark one function for the source language's meanings and its
    /// neighbour for the target's, and a call between them is an ordinary call. A per-call
    /// setting could not express that at all.
    fn lower(&self, sources: &[Source]) -> Result<Unit, LoweringError>;
}

/// One source text and what its operations mean.
///
/// Paired rather than passed alongside, so the two cannot come apart. Lowering a source under
/// somebody else's behavior is not an error any type could catch once the two are separate lists
/// indexed in parallel — and the failure would be a program that computes different answers with
/// nothing in its source saying so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// The text to lower.
    pub text: String,
    /// Which language supplies the meaning of each operation in it.
    pub behavior: Behavior,
}

impl Source {
    /// A source and the behavior it lowers under.
    pub fn new(text: impl Into<String>, behavior: Behavior) -> Self {
        Self {
            text: text.into(),
            behavior,
        }
    }
}

/// Why a frontend could not produce a unit.
///
/// Located in lines and columns rather than byte offsets, because the frontend is the last thing
/// that holds the source text. A caller downstream of it has a `Unit` and no way to resolve an
/// offset, so handing one back would make the location unusable exactly where it is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoweringError {
    /// The source never became a tree.
    Syntax {
        /// What the parser objected to.
        message: String,
        /// 1-based line.
        line: usize,
        /// 1-based column.
        column: usize,
    },
    /// The source parsed but is outside the language subset compylr accepts.
    Unsupported {
        /// What lowering objected to.
        message: String,
        /// Stable identifier for the category, so callers branch without matching prose.
        code: &'static str,
        /// 1-based line.
        line: usize,
        /// 1-based column.
        column: usize,
    },
}

impl LoweringError {
    /// Whether the source failed to parse at all.
    pub fn is_syntax(&self) -> bool {
        matches!(self, Self::Syntax { .. })
    }

    /// The rejection category, or `None` for a syntax failure.
    ///
    /// A syntax failure has no category because there is nothing to categorise: the parser did
    /// not get far enough to say which rule was broken.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::Syntax { .. } => None,
            Self::Unsupported { code, .. } => Some(code),
        }
    }

    /// Description of the problem, without its location.
    pub fn message(&self) -> &str {
        match self {
            Self::Syntax { message, .. } | Self::Unsupported { message, .. } => message,
        }
    }

    /// 1-based line the offending construct is on.
    pub fn line(&self) -> usize {
        match self {
            Self::Syntax { line, .. } | Self::Unsupported { line, .. } => *line,
        }
    }

    /// 1-based column the offending construct starts at.
    pub fn column(&self) -> usize {
        match self {
            Self::Syntax { column, .. } | Self::Unsupported { column, .. } => *column,
        }
    }
}

impl fmt::Display for LoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line(), self.column(), self.message())
    }
}

impl Error for LoweringError {}

/// A failure selecting a frontend.
///
/// Deliberately the same three-way shape as [`crate::backend::BackendError`]'s selection
/// variants: asking "can you read X?" has the same three answers as asking "can you write X?",
/// and answering one of them differently would be a difference with no reason behind it.
#[derive(Debug)]
pub enum FrontendError {
    /// The name is reserved for a source language that is planned but not built.
    NotImplemented {
        /// Name that was requested.
        frontend: String,
    },
    /// The name is not in the registry at all.
    Unknown {
        /// Name that was requested.
        frontend: String,
        /// Names that would have worked.
        available: Vec<String>,
    },
}

impl FrontendError {
    /// Whether this is a reserved-but-unimplemented source language.
    pub fn is_not_implemented(&self) -> bool {
        matches!(self, Self::NotImplemented { .. })
    }

    /// Whether the name is not a frontend at all.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented { frontend } => write!(
                f,
                "the '{frontend}' frontend is not implemented yet; it is a planned source language"
            ),
            Self::Unknown {
                frontend,
                available,
            } => write!(
                f,
                "'{frontend}' is not a known frontend; available frontends: {}",
                available.join(", ")
            ),
        }
    }
}

impl Error for FrontendError {}
