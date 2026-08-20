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

#[cfg(test)]
mod tests {
    use super::*;
    use compylr_diagnostics::error::LowerErrorKind;
    use compylr_diagnostics::span::Span;

    /// Every failure has to render, because each one becomes a message somebody reads while
    /// holding a file they cannot open.
    #[test]
    fn every_failure_renders_and_says_which_it_is() {
        let json = serde_json::from_str::<serde_json::Value>("{").expect_err("not valid JSON");
        let duplicate = LowerError::new(
            LowerErrorKind::DuplicateFunction,
            "'f' is defined twice",
            Span::default(),
        );

        let cases = [
            (ArtifactError::Json(json), "readable"),
            (
                ArtifactError::UnsupportedVersion {
                    found: 9,
                    expected: 3,
                },
                "9",
            ),
            (
                ArtifactError::FingerprintMismatch {
                    recorded: "aaaa".to_string(),
                    computed: "bbbb".to_string(),
                },
                "corrupt",
            ),
            (
                ArtifactError::DuplicateFunction(Box::new(duplicate)),
                "duplicate",
            ),
        ];

        let mut rendered = Vec::new();
        for (error, expected) in cases {
            let text = error.to_string();
            assert!(text.contains(expected), "{expected} missing from: {text}");
            rendered.push(text);
        }

        let count = rendered.len();
        rendered.sort();
        rendered.dedup();
        assert_eq!(rendered.len(), count, "failures must be distinguishable");
    }

    /// An unsupported version has to name both, or the reader cannot tell which way to move.
    #[test]
    fn an_unsupported_version_names_the_one_found_and_the_one_expected() {
        let error = ArtifactError::UnsupportedVersion {
            found: 2,
            expected: 3,
        };
        let text = error.to_string();
        assert!(text.contains('2') && text.contains('3'), "{text}");
    }

    /// A mismatch quotes both fingerprints, since the whole content of the diagnostic is that they
    /// differ.
    #[test]
    fn a_fingerprint_mismatch_quotes_both() {
        let error = ArtifactError::FingerprintMismatch {
            recorded: "0123456789abcdef".to_string(),
            computed: "fedcba9876543210".to_string(),
        };
        let text = error.to_string();
        assert!(text.contains("0123456789abcdef"), "{text}");
        assert!(text.contains("fedcba9876543210"), "{text}");
    }

    /// The two failures that wrap another error expose it; the two that do not, do not.
    #[test]
    fn only_the_wrapping_failures_report_a_cause() {
        let json: ArtifactError = serde_json::from_str::<serde_json::Value>("{")
            .expect_err("not valid JSON")
            .into();
        assert!(json.source().is_some(), "a parse failure wraps serde's");

        let duplicate = ArtifactError::DuplicateFunction(Box::new(LowerError::new(
            LowerErrorKind::DuplicateFunction,
            "'f' is defined twice",
            Span::default(),
        )));
        assert!(duplicate.source().is_some());

        assert!(
            ArtifactError::UnsupportedVersion {
                found: 1,
                expected: 3
            }
            .source()
            .is_none()
        );
        assert!(
            ArtifactError::FingerprintMismatch {
                recorded: "a".into(),
                computed: "b".into(),
            }
            .source()
            .is_none()
        );
    }
}
