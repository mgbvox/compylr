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

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::ir::Unit;

pub mod bindings;
pub mod rust;

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
    fn emit(&self, unit: &Unit) -> Result<GeneratedFiles, BackendError>;

    /// Render a unit as a Python extension module, bindings included.
    ///
    /// Separate from [`Backend::emit`] because exposing a target to Python is a target-specific
    /// concern: PyO3 is meaningless to a TypeScript backend, which would reach Python — if ever —
    /// by an entirely different route. The default refuses rather than pretending, so a backend
    /// that gains code generation without gaining bindings cannot silently appear to support the
    /// decorator.
    fn emit_python_extension(&self, _unit: &Unit) -> Result<GeneratedFiles, BackendError> {
        Err(BackendError::Unsupported {
            detail: format!(
                "the '{}' backend can generate source but cannot yet expose it to Python",
                self.name()
            ),
        })
    }

    /// The build manifest for a generated crate, when the target needs one.
    fn build_manifest(&self, _unit: &Unit) -> Result<String, BackendError> {
        Err(BackendError::Unsupported {
            detail: format!("the '{}' backend defines no build manifest", self.name()),
        })
    }
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

/// One entry in the registry.
///
/// The three-way answer lives in `backend`: `Some` is implemented, `None` is a name compylr has
/// reserved for a planned target, and absence from [`REGISTRY`] entirely is an unknown name.
struct Entry {
    /// The name this entry is selected by.
    name: &'static str,
    /// The backend, or `None` when the name is reserved but not yet implemented.
    backend: Option<&'static dyn Backend>,
}

/// The registry, in the order [`names`] reports.
const REGISTRY: &[Entry] = &[
    Entry {
        name: "rust",
        backend: Some(&rust::RustBackend),
    },
    Entry {
        name: "typescript",
        backend: None,
    },
    Entry {
        name: "go",
        backend: None,
    },
    Entry {
        name: "cpp",
        backend: None,
    },
];

/// Every backend name compylr recognizes, implemented or not.
pub fn names() -> Vec<&'static str> {
    REGISTRY.iter().map(|entry| entry.name).collect()
}

/// Every backend name that can compile today.
pub fn implemented_names() -> Vec<String> {
    REGISTRY
        .iter()
        .filter(|entry| entry.backend.is_some())
        .map(|entry| entry.name.to_string())
        .collect()
}

/// Resolve a backend name.
///
/// Lookup is exact: a backend name comes from a configuration value someone typed deliberately,
/// and normalising case would raise the question of what else to normalise.
pub fn lookup(name: &str) -> Result<&'static dyn Backend, BackendError> {
    match REGISTRY.iter().find(|entry| entry.name == name) {
        Some(entry) => match entry.backend {
            Some(backend) => Ok(backend),
            None => Err(BackendError::NotImplemented {
                backend: name.to_string(),
            }),
        },
        None => Err(BackendError::Unknown {
            backend: name.to_string(),
            available: implemented_names(),
        }),
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
