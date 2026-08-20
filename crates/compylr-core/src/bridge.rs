//! Host bridges: what makes generated target code callable from a source language.
//!
//! Indexed by the **pair** `(source, target)`, not by either language alone, and this is the one
//! place in the design where the modularity does not come for free.
//!
//! A frontend and a backend compose N + M because they meet at the IR and never see each other.
//! A bridge cannot: a calling convention is a negotiation between two runtimes — who owns the
//! memory, how an error signals, how a string encodes, how one language's collector interacts
//! with the other's allocator. Neither side can decide that alone, so neither side can own the
//! code. Python calling Rust is PyO3; Python calling Go is cgo and a C array someone has to free;
//! TypeScript calling Rust is napi-rs. Nothing carries over.
//!
//! So the cost is N x M, and the design's job is to keep it *visible* rather than to pretend it
//! is not there. Two consequences follow:
//!
//! * A missing pair is a fourth honest answer, beside implemented/reserved/unknown: compylr can
//!   generate the target, and cannot call it back from this source language.
//! * The trait is shaped so that a canonical-C-ABI hub — one bridge registered for many pairs —
//!   could be implemented *behind* it later, collapsing N x M back to N + M at the cost of a
//!   marshalling layer. That trade is deferred, not foreclosed.

use std::error::Error;
use std::fmt;

use compylr_ir::Unit;

use crate::backend::{BackendError, GeneratedFiles};

/// Everything a host needs to build and load a compiled unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostArtifact {
    /// Generated files, keyed by path relative to the artifact root.
    ///
    /// Includes the target's own output: a bridge builds on a backend rather than beside it, so
    /// the caller receives one buildable tree rather than two halves to merge.
    pub files: GeneratedFiles,
    /// The build manifest for the generated project.
    pub manifest: String,
    /// The name the host language loads the built artifact under.
    ///
    /// Not a stable or user-facing name: it encodes build identity, which is what allows a
    /// rebuilt artifact to be loaded by a process that already loaded its predecessor.
    pub loaded_as: String,
}

/// A bridge between one source language and one target language.
pub trait HostBridge: fmt::Debug + Send + Sync {
    /// The source language that will call the compiled code.
    fn source(&self) -> &'static str;

    /// The target language the compiled code is written in.
    fn target(&self) -> &'static str;

    /// Generate everything needed to build and load the unit from the source language.
    fn emit(&self, unit: &Unit) -> Result<HostArtifact, BackendError>;
}

/// Why a callable artifact could not be produced.
#[derive(Debug)]
pub enum BridgeError {
    /// No bridge is registered for this pair.
    ///
    /// Deliberately not reported as an unknown or unimplemented *target*: the target may be
    /// perfectly well implemented, and saying otherwise sends someone looking for a backend that
    /// is already there.
    Unbridged {
        /// Language that wanted to call.
        source: String,
        /// Language that was generated.
        target: String,
    },
    /// The bridge exists but could not render the unit.
    Emission(BackendError),
}

impl BridgeError {
    /// Whether the pair simply has no bridge.
    pub fn is_unbridged(&self) -> bool {
        matches!(self, Self::Unbridged { .. })
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unbridged { source, target } => write!(
                f,
                "compylr can generate {target} but cannot yet call it from {source}: \
                 no host bridge is registered for the ({source}, {target}) pair"
            ),
            Self::Emission(error) => write!(f, "{error}"),
        }
    }
}

impl Error for BridgeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Unbridged { .. } => None,
            Self::Emission(error) => Some(error),
        }
    }
}

impl From<BackendError> for BridgeError {
    fn from(error: BackendError) -> Self {
        Self::Emission(error)
    }
}
