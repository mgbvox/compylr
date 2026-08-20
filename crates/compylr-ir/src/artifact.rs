//! Failures reading or writing the IR's on-disk artifact form.
//!
//! Kept beside the IR rather than in the shared diagnostics crate because these are about the
//! *artifact*, and the artifact is the IR's own serialized shape.

use std::error::Error;
use std::fmt;

use compylr_diagnostics::error::LowerError;

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
