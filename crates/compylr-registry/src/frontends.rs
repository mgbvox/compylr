//! The frontend table.
//!
//! Deliberately the same shape as [`crate::backends`]. Asking "can you read X?" has the same
//! three answers as asking "can you write X?", and the reserved names are the same list —
//! compylr's supported languages are supported in both directions or the pipeline is only half
//! modular.

use compylr_core::frontend::{Frontend, FrontendError};
use compylr_frontend_python::PythonFrontend;

/// One entry in the registry.
///
/// The three-way answer lives in `frontend`: `Some` is implemented, `None` is a name compylr has
/// reserved for a planned source language, and absence from [`REGISTRY`] entirely is an unknown
/// name.
struct Entry {
    /// The name this entry is selected by.
    name: &'static str,
    /// The frontend, or `None` when the name is reserved but not yet implemented.
    frontend: Option<&'static dyn Frontend>,
}

/// The registry, in the order [`names`] reports.
const REGISTRY: &[Entry] = &[
    Entry {
        name: "python",
        frontend: Some(&PythonFrontend),
    },
    Entry {
        name: "typescript",
        frontend: None,
    },
    Entry {
        name: "go",
        frontend: None,
    },
    Entry {
        name: "cpp",
        frontend: None,
    },
];

/// Every frontend name compylr recognizes, implemented or not.
pub fn names() -> Vec<&'static str> {
    REGISTRY.iter().map(|entry| entry.name).collect()
}

/// Every frontend name that can compile today.
pub fn implemented_names() -> Vec<String> {
    REGISTRY
        .iter()
        .filter(|entry| entry.frontend.is_some())
        .map(|entry| entry.name.to_string())
        .collect()
}

/// Resolve a frontend name.
///
/// Lookup is exact, for the same reason backend lookup is: a name comes from a configuration
/// value someone typed deliberately, and normalising case would raise the question of what else
/// to normalise.
pub fn lookup(name: &str) -> Result<&'static dyn Frontend, FrontendError> {
    match REGISTRY.iter().find(|entry| entry.name == name) {
        Some(entry) => match entry.frontend {
            Some(frontend) => Ok(frontend),
            None => Err(FrontendError::NotImplemented {
                frontend: name.to_string(),
            }),
        },
        None => Err(FrontendError::Unknown {
            frontend: name.to_string(),
            available: implemented_names(),
        }),
    }
}
