//! The PyO3 layer wrapping compiled functions.
//!
//! Kept apart from `rust.rs` on purpose: that module knows how to translate IR into Rust and
//! nothing about Python, so the same translation could feed a non-Python consumer. This module is
//! the only place where Rust values become Python values and where a [`RuntimeError`] becomes an
//! exception.
//!
//! [`RuntimeError`]: super::runtime::RuntimeError
//!
//! The wrappers are thin by design. Each one calls the pure function, maps its error, and returns
//! — no logic that could disagree with the translated body.

use std::fmt::Write as _;

use super::BackendError;
use super::rust::{RustBackend, rust_ident, rust_ty};
use crate::backend::Backend;
use crate::ir::{Ty, Unit};

/// Prefix for the extension module a unit compiles to.
const MODULE_PREFIX: &str = "compylr_generated_";

/// The Python module name a unit compiles to.
///
/// The fingerprint is part of the name because CPython cannot reliably re-import an extension
/// module under a name already present in `sys.modules` — the shared object is already mapped, and
/// reload semantics for extension modules are not supported. A fixed name would therefore make a
/// rebuild inside a running process succeed and then be unloadable. Each build producing a
/// distinct module sidesteps that entirely.
///
/// This is safe only because the name is never user-facing: callers reach compiled functions
/// through the objects they decorated, never by importing this module themselves.
pub fn module_name(unit: &Unit) -> String {
    format!("{MODULE_PREFIX}{:016x}", unit.fingerprint())
}

/// Emit a complete extension-module crate source for a unit.
///
/// The output is the pure translation from [`RustBackend`] with a binding layer appended, so the
/// two stay separable and the translated bodies are identical whether or not Python is involved.
pub fn emit_extension(unit: &Unit) -> Result<String, BackendError> {
    let mut out = RustBackend.emit(unit)?;
    out.push('\n');
    out.push_str(&emit_binding_layer(unit)?);
    Ok(out)
}

/// The PyO3 wrappers and the module registration.
fn emit_binding_layer(unit: &Unit) -> Result<String, BackendError> {
    let mut out = String::new();
    out.push_str(PREAMBLE);

    for (index, function) in unit.functions().enumerate() {
        let params = function
            .params
            .iter()
            .map(|p| format!("{}: {}", rust_ident(&p.name), rust_ty(p.ty)))
            .collect::<Vec<_>>()
            .join(", ");
        let args = function
            .params
            .iter()
            .map(|p| rust_ident(&p.name))
            .collect::<Vec<_>>()
            .join(", ");

        // The wrapper is named positionally rather than after the function. A Python name can be
        // a Rust keyword, and `#[pyo3(name = ...)]` is what restores the name Python sees, so the
        // Rust-side identifier only has to be unique.
        let _ = writeln!(out, "#[pyfunction]");
        let _ = writeln!(out, "#[pyo3(name = {:?})]", function.name);
        let _ = writeln!(
            out,
            "fn __compylr_export_{index}({params}) -> PyResult<{}> {{",
            rust_ty(function.ret)
        );
        let _ = writeln!(
            out,
            "    generated::{}({args}).map_err(__compylr_to_py_err)",
            rust_ident(&function.name)
        );
        out.push_str("}\n\n");
    }

    let _ = writeln!(out, "#[pymodule]");
    let _ = writeln!(
        out,
        "fn {}(m: &Bound<'_, PyModule>) -> PyResult<()> {{",
        module_name(unit)
    );
    for index in 0..unit.len() {
        let _ = writeln!(
            out,
            "    m.add_function(wrap_pyfunction!(__compylr_export_{index}, m)?)?;"
        );
    }
    out.push_str("    Ok(())\n}\n");

    // A unit type is only valid as a return, so nothing above can have produced a `()` parameter.
    debug_assert!(
        unit.functions()
            .all(|f| f.params.iter().all(|p| p.ty != Ty::Unit))
    );
    Ok(out)
}

/// Imports and the error mapping shared by every wrapper.
///
/// The mapping is where compylr's promise of "same semantics" is actually kept at the boundary:
/// code that already catches `ZeroDivisionError` around a function keeps working when that
/// function is compiled, because the compiled version raises the same thing.
const PREAMBLE: &str = r#"use pyo3::exceptions::{PyOverflowError, PyZeroDivisionError};
use pyo3::prelude::*;

/// Map a compiled function's failure onto the exception Python raises for the same condition.
fn __compylr_to_py_err(error: runtime::RuntimeError) -> PyErr {
    match error {
        runtime::RuntimeError::DivisionByZero => {
            PyZeroDivisionError::new_err("division by zero")
        }
        runtime::RuntimeError::Overflow => {
            PyOverflowError::new_err("integer arithmetic overflowed a 64-bit signed integer")
        }
    }
}

"#;

/// `Cargo.toml` for the generated crate.
///
/// The crate depends on `pyo3` and nothing else — deliberately not on compylr, which will not
/// exist on the machine where this is built.
pub fn cargo_manifest(unit: &Unit, pyo3_version: &str) -> String {
    let name = module_name(unit);
    format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         \n\
         [lib]\n\
         name = \"{name}\"\n\
         crate-type = [\"cdylib\"]\n\
         \n\
         [dependencies]\n\
         pyo3 = {{ version = \"{pyo3_version}\", features = [\"abi3-py311\", \"extension-module\"] }}\n"
    )
}
