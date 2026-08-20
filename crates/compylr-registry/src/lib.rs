//! Where implementations are registered.
//!
//! Every other crate is either an interface or an implementation of one; this is the single
//! place that knows the full set. Adding a language means adding a crate and one entry here.
//!
//! Resolution reports three cases, never two — implemented, reserved, and unknown — because
//! telling someone that a target compylr has committed to does not exist is both false and
//! discouraging. See [`compylr_core::backend`] for what a backend is.

use compylr_backend_rust::rust::RustBackend;
use compylr_core::backend::Backend;

pub use compylr_core::backend::{BackendError, GeneratedFiles, format_source};

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
        backend: Some(&RustBackend),
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
