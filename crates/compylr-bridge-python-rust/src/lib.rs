//! The host bridge for the `(python, rust)` pair.
//!
//! Generates the PyO3 layer that makes a compiled unit importable from Python. Note that this
//! crate *emits* PyO3 source as text; it does not depend on PyO3 itself. The other PyO3 role in
//! this workspace — exposing the compiler to Python as `compylr._core` — belongs to the root
//! `compylr` crate and is a different artifact with a different lifecycle.

pub mod bindings;

pub use bindings::{cargo_manifest, emit_extension, module_name};

use compylr_core::backend::{BackendError, GeneratedFiles};
use compylr_core::bridge::{BuildKey, HostArtifact, HostBridge};
use compylr_ir::Unit;

/// The `(python, rust)` bridge.
#[derive(Debug)]
pub struct PythonRustBridge;

impl HostBridge for PythonRustBridge {
    fn source(&self) -> &'static str {
        "python"
    }

    fn target(&self) -> &'static str {
        "rust"
    }

    fn emit(&self, unit: &Unit, key: &BuildKey) -> Result<HostArtifact, BackendError> {
        Ok(HostArtifact {
            files: emit_extension(unit, key)?,
            manifest: cargo_manifest(key, PYO3_VERSION),
            loaded_as: module_name(key),
        })
    }
}

/// Path of the file holding the Python-boundary wrappers.
///
/// Owned here rather than by the Rust backend: the file exists because Python is calling, and a
/// target that no Python ever calls has no such file to name.
pub const BINDINGS_PATH: &str = "src/bindings.rs";

/// PyO3 version generated crates depend on.
///
/// Pinned to match the API the emitted bindings are written against: letting a generated crate
/// float to a different major version would produce code that does not compile. It lives with
/// the bridge rather than with the Rust backend, because a target that no Python ever calls has
/// no reason to know PyO3 exists.
pub const PYO3_VERSION: &str = "0.29.2";

/// Generate the extension module that exposes `unit` to Python.
pub fn emit_python_extension(unit: &Unit, key: &BuildKey) -> Result<GeneratedFiles, BackendError> {
    emit_extension(unit, key)
}

/// The build manifest for the generated crate.
pub fn build_manifest(key: &BuildKey) -> String {
    cargo_manifest(key, PYO3_VERSION)
}
