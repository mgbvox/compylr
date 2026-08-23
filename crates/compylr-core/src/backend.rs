//! Target backends: the stage that turns IR into source text for a concrete language.
//!
//! Everything language-specific lives below this module. The IR names no target and the lowering
//! pass names no target; a backend is where `int` finally becomes `i64` and where Python's
//! operator semantics have to be reproduced in whatever the target actually offers.
//!
//! Backends are looked up by name through a **registry** rather than selected from an enum of
//! implemented backends, because the answer to "can you compile to X?" has three cases, not two:
//!
//! * implemented — compile with it,
//! * reserved — a target compylr intends to support but does not yet,
//! * unknown — not a backend name at all, most likely a typo.
//!
//! Folding the middle case into the last one would tell a user asking for TypeScript that no such
//! target exists, which is both false and discouraging. An enum could not express `Reserved`
//! without carrying a second list beside it.
//!
//! The registry *table* is not here. It lives in `compylr-registry`, which is allowed to name
//! every backend; this crate defines what a backend is, and a crate that defines an interface
//! cannot depend on the crates implementing it.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use compylr_ir::{Guarantee, LanguageBehavior, Unit};

use crate::negotiation::TargetOption;

/// The files of a generated crate, keyed by path relative to the crate root.
///
/// Paths are relative so the caller decides where the crate lands; ordered so that emitting the
/// same unit twice iterates identically.
pub type GeneratedFiles = BTreeMap<String, String>;

/// A target language backend.
///
/// `Debug` is required so that a lookup result can be unwrapped in tests and reported in
/// diagnostics without every call site having to special-case a trait object.
pub trait Backend: fmt::Debug + Send + Sync {
    /// The registry name this backend is selected by.
    fn name(&self) -> &'static str;

    /// The semantic guarantees this backend preserves.
    ///
    /// Required rather than defaulted, because a default of "none" would make every backend
    /// unusable with every frontend and a default of "everything" would make the check
    /// meaningless. A new backend has to answer the question.
    fn preserves(&self) -> &'static [Guarantee];

    /// What **this language** means, on every behavior axis.
    ///
    /// Required rather than defaulted, for the reason [`Backend::preserves`] is: any default
    /// would be some other language's answers, and a backend that inherited them would be
    /// offering a "native" operator that is not its own.
    ///
    /// Describes this language only. This is the half of the negotiation that says what a user
    /// gets when they ask for the target's meaning — not what the source meant, and not how the
    /// two relate, which is resolution's business and needs both.
    fn behavior(&self) -> &'static LanguageBehavior;

    /// Transformations this backend offers that cost a guarantee.
    ///
    /// Defaulted to none: a backend that trades nothing for speed is a coherent backend, and the
    /// empty answer is the safe one.
    fn options(&self) -> &'static [TargetOption] {
        &[]
    }

    /// Apply meaning-preserving cleanup to emitted files.
    ///
    /// Separate from [`Backend::emit`] and applied by whoever writes the files out, because
    /// emission must stay a pure function of the unit — that is what makes its output
    /// byte-reproducible and therefore safe to key a rebuild cache on. Shelling out to a
    /// formatter inside it would make the result depend on which formatter happens to be
    /// installed.
    ///
    /// Unconditional by contract: anything here changes how source reads and not what it does, so
    /// there is nothing for a frontend to object to. A transformation that would change meaning
    /// belongs in [`Backend::options`], where it can be refused.
    fn post_process(&self, files: GeneratedFiles) -> GeneratedFiles {
        files
    }

    /// Render a unit as the files of a crate, keyed by relative path.
    ///
    /// A map rather than one string, because generated source is written to disk to be *read*,
    /// and one file that opens with two hundred identical lines in every project buries the dozen
    /// the reader came for. A [`BTreeMap`] rather than a hash map so iteration order is
    /// deterministic, for the same reason [`Unit`] holds its functions in one.
    ///
    /// Emission is still a pure function of the unit: no I/O, no filesystem, no environment.
    /// Returning a set of files must not become *writing* files — that is the build pipeline's
    /// job, and keeping it there is what makes the determinism guarantee mean anything.
    ///
    /// Note what is *not* here: making the result callable from a host language. That is a
    /// property of the (source, target) pair rather than of the target, so it belongs to a host
    /// bridge. A method here would mean every backend growing one per source language.
    fn emit(&self, unit: &Unit) -> Result<GeneratedFiles, BackendError>;
}

/// Format generated source with `rustfmt`, falling back to the input unchanged.
///
/// Deliberately **not** part of [`Backend::emit`]. Emission is a pure function of the unit, which
/// is what makes its output byte-reproducible and therefore safe to key a rebuild cache on;
/// shelling out to a formatter inside it would make the result depend on which rustfmt happens to
/// be installed. Formatting is instead an explicit, cosmetic step applied when the source is
/// written out for a human to read.
///
/// Failure is not an error. `rustfmt` ships with the toolchain but can be absent from a minimal
/// install, and unformatted source compiles identically — so a missing formatter costs
/// readability and nothing else.
pub fn format_source(source: &str) -> String {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    let Ok(mut child) = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .arg("--emit")
        .arg("stdout")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return source.to_string();
    };

    if let Some(stdin) = child.stdin.as_mut()
        && stdin.write_all(source.as_bytes()).is_err()
    {
        return source.to_string();
    }

    match child.wait_with_output() {
        Ok(output) if output.status.success() => {
            String::from_utf8(output.stdout).unwrap_or_else(|_| source.to_string())
        }
        _ => source.to_string(),
    }
}

/// A failure selecting or running a backend.
#[derive(Debug)]
pub enum BackendError {
    /// The name is reserved for a target that is planned but not built.
    NotImplemented {
        /// Name that was requested.
        backend: String,
    },
    /// The name is not in the registry at all.
    Unknown {
        /// Name that was requested.
        backend: String,
        /// Names that would have worked.
        available: Vec<String>,
    },
    /// The backend could not render the unit.
    ///
    /// This is a compylr bug rather than a user error: lowering has already established that the
    /// program is inside the supported subset, so a backend that cannot render it is missing a
    /// case.
    Unsupported {
        /// What could not be rendered.
        detail: String,
    },
}

impl BackendError {
    /// Whether this is a reserved-but-unimplemented target.
    ///
    /// Exposed so callers branch on the case rather than on message wording, which is
    /// presentation and changes freely.
    pub fn is_not_implemented(&self) -> bool {
        matches!(self, Self::NotImplemented { .. })
    }

    /// Whether the name is not a backend at all.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented { backend } => write!(
                f,
                "the '{backend}' backend is not implemented yet; it is a planned target"
            ),
            Self::Unknown { backend, available } => write!(
                f,
                "'{backend}' is not a known backend; available backends: {}",
                available.join(", ")
            ),
            Self::Unsupported { detail } => {
                write!(f, "backend cannot render this program: {detail}")
            }
        }
    }
}

impl Error for BackendError {}
