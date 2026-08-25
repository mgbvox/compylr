//! The host bridge table, keyed by `(source, target)`.
//!
//! A list rather than two nested tables, because the entries are pairs and there is no reason to
//! privilege one axis over the other. It is also the honest shape: this table is the N x M cost
//! made visible, and a nested structure would suggest a factoring that does not exist.

use compylr_bridge_python_rust::PythonRustBridge;
use compylr_bridge_typescript_golang::TypeScriptGoBridge;
use compylr_core::bridge::{BridgeError, HostBridge};

/// One entry in the registry.
///
/// No `Option` here, unlike the frontend and backend tables. A reserved *pair* would be a
/// promise about a combination rather than about a language, and there is nothing useful to say
/// about one: what a user needs to know is that the pair is not bridged today, which is what
/// absence already says.
struct Entry {
    bridge: &'static dyn HostBridge,
}

/// Every registered pair.
const REGISTRY: &[Entry] = &[
    Entry {
        bridge: &PythonRustBridge,
    },
    Entry {
        bridge: &TypeScriptGoBridge,
    },
];

/// Every `(source, target)` pair that can be called across today.
pub fn pairs() -> Vec<(String, String)> {
    REGISTRY
        .iter()
        .map(|entry| {
            (
                entry.bridge.source().to_string(),
                entry.bridge.target().to_string(),
            )
        })
        .collect()
}

/// Resolve the bridge for a pair.
///
/// Failure names both languages, because the useful thing to say is which combination is missing
/// — not that a target is unavailable, which would be false when the backend is implemented.
pub fn lookup(source: &str, target: &str) -> Result<&'static dyn HostBridge, BridgeError> {
    REGISTRY
        .iter()
        .find(|entry| entry.bridge.source() == source && entry.bridge.target() == target)
        .map(|entry| entry.bridge)
        .ok_or_else(|| BridgeError::Unbridged {
            source: source.to_string(),
            target: target.to_string(),
        })
}
