//! `compylr._core`: the compiler, exposed to Python.
//!
//! This is the seam between the two languages. Above it, Python decides *what* to compile and
//! *when*; below it, everything is the Rust pipeline. Compiling in-process rather than shelling
//! out matters for one concrete reason: diagnostics stay structured. A subprocess would have to
//! format an error into text and have Python parse it back, and the location would be the first
//! thing lost.
//!
//! The work is done by plain Rust functions with the PyO3 wrappers kept thin on top, so the
//! compilation logic can be tested without a Python interpreter and the wrappers have nothing in
//! them that could disagree with what they wrap.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use crate::backend::{self, BackendError, GeneratedFiles};
use crate::error::{LowerError, SourceError};
use crate::frontend::{self, FrontendError, LoweringError, parse_source};
use crate::lower::lower_source_members;

/// The source language every compilation currently uses.
///
/// Named rather than assumed: the frontend is resolved through the registry like the backend is,
/// so a second source language is a registry entry rather than a branch here.
const SOURCE_LANGUAGE: &str = "python";

/// Everything a successful compilation produces.
#[derive(Debug, Clone)]
pub struct Compiled {
    /// Generated target files, keyed by path relative to the crate root.
    ///
    /// A mapping rather than one string: a backend emits a crate, and the paths are relative so
    /// the caller decides where it lands.
    pub target_sources: GeneratedFiles,
    /// The IR, serialized for inspection.
    pub ir_artifact: String,
    /// Fingerprint of the compiled unit.
    pub fingerprint: u64,
    /// Name of the extension module this unit compiles to.
    pub module_name: String,
    /// Build manifest for the generated crate.
    pub manifest: String,
    /// Names of the functions in the unit, in the unit's deterministic order.
    pub function_names: Vec<String>,
}

/// Why a compilation did not produce a unit.
#[derive(Debug)]
pub enum CompileFailure {
    /// The source is not valid Python.
    Syntax {
        /// What the parser objected to.
        message: String,
        /// 1-based line.
        line: usize,
        /// 1-based column.
        column: usize,
    },
    /// The source parses but is outside the supported subset.
    Unsupported {
        /// What lowering objected to.
        message: String,
        /// Stable identifier for the category, so callers can branch without matching prose.
        code: &'static str,
        /// 1-based line.
        line: usize,
        /// 1-based column.
        column: usize,
    },
    /// The requested backend cannot be used.
    Backend(BackendError),
    /// The requested source language cannot be used.
    Frontend(FrontendError),
}

impl CompileFailure {
    fn from_lower(error: &LowerError, source: &str) -> Self {
        let at = error.span().line_column(source);
        Self::Unsupported {
            message: error.message().to_string(),
            code: error.kind().code(),
            line: at.line,
            column: at.column,
        }
    }
}

/// A frontend's failure is already located, so it maps across without needing the source text.
impl From<LoweringError> for CompileFailure {
    fn from(error: LoweringError) -> Self {
        match error {
            LoweringError::Syntax {
                message,
                line,
                column,
            } => Self::Syntax {
                message,
                line,
                column,
            },
            LoweringError::Unsupported {
                message,
                code,
                line,
                column,
            } => Self::Unsupported {
                message,
                code,
                line,
                column,
            },
        }
    }
}

/// Compile Python source texts into a target artifact.
///
/// Sources arrive as **text**, not paths: the decorator obtains them by introspecting a live
/// function object, and there may be no file that corresponds.
///
/// Every source is assembled into one unit before emitting, which is what lets a function in one
/// source call a function in another — the arrangement a project of separately decorated functions
/// always produces. Because callee resolution happens over the assembled unit rather than during
/// lowering, the result does not depend on the order the sources arrive.
pub fn compile(sources: &[String], backend_name: &str) -> Result<Compiled, CompileFailure> {
    // Resolved first so that an unusable backend is reported before any parsing work, and so the
    // error is about the backend rather than about whichever source happened to be malformed.
    //
    // The value is not used: the files come from the (python, rust) bridge, since a calling
    // convention belongs to the pair. Resolution is still what rejects an unusable target name.
    let _backend = backend::lookup(backend_name).map_err(CompileFailure::Backend)?;
    let frontend = frontend::lookup(SOURCE_LANGUAGE).map_err(CompileFailure::Frontend)?;

    // Parsing, gathering signatures across sources, and lowering are all the frontend's, because
    // they are Python's typing rules rather than the pipeline's. What comes back is a unit.
    let unit = frontend.lower(sources)?;

    // Resolves calls across every source at once. Reported against the first source because a
    // unit assembled from many texts has no single source a span indexes into.
    unit.validate()
        .map_err(|error| CompileFailure::Unsupported {
            message: error.message().to_string(),
            code: error.kind().code(),
            line: 1,
            column: 1,
        })?;

    // Generating the target source is the backend's job; making it callable from Python is the
    // (python, rust) bridge's, because a calling convention belongs to the pair rather than to
    // either language alone.
    let target_sources = compylr_bridge_python_rust::emit_python_extension(&unit)
        .map_err(CompileFailure::Backend)?;
    let manifest = compylr_bridge_python_rust::build_manifest(&unit);
    let ir_artifact = unit.to_json().map_err(|error| {
        CompileFailure::Backend(BackendError::Unsupported {
            detail: format!("could not serialize the IR: {error}"),
        })
    })?;

    Ok(Compiled {
        target_sources,
        ir_artifact,
        fingerprint: unit.fingerprint(),
        module_name: crate::backend::bindings::module_name(&unit),
        manifest,
        function_names: unit.functions().map(|f| f.name.clone()).collect(),
    })
}

create_exception!(
    _core,
    CompylrError,
    PyException,
    "Base class for every compylr failure."
);
create_exception!(
    _core,
    CompilationError,
    CompylrError,
    "A program could not be compiled. Carries `line` and `column`."
);
create_exception!(
    _core,
    SourceSyntaxError,
    CompilationError,
    "The source is not valid Python."
);
create_exception!(
    _core,
    UnsupportedProgramError,
    CompilationError,
    "The source is valid Python but outside compylr's supported subset."
);
create_exception!(
    _core,
    BackendNotAvailableError,
    CompylrError,
    "The requested backend is unknown, or reserved but not implemented."
);

impl CompileFailure {
    /// Turn a failure into the Python exception that describes it.
    ///
    /// The location is attached as attributes as well as being in the message, so callers can act
    /// on it without parsing text.
    pub fn into_py_err(self, py: Python<'_>) -> PyErr {
        let (err, location) = match self {
            Self::Syntax {
                message,
                line,
                column,
            } => (
                SourceSyntaxError::new_err(format!("{line}:{column}: {message}")),
                Some((line, column, None)),
            ),
            Self::Unsupported {
                message,
                code,
                line,
                column,
            } => (
                UnsupportedProgramError::new_err(format!("{line}:{column}: {message}")),
                Some((line, column, Some(code))),
            ),
            Self::Backend(error) => (BackendNotAvailableError::new_err(error.to_string()), None),
            // Rendered as the base type rather than getting one of its own. The source language
            // is a compiled-in constant that is always implemented, so reaching here means
            // compylr is misconfigured against itself — not something a user's program caused,
            // and not worth an exception class nobody can catch meaningfully.
            Self::Frontend(error) => (CompylrError::new_err(error.to_string()), None),
        };

        if let Some((line, column, code)) = location {
            let value = err.value(py);
            let _ = value.setattr("line", line);
            let _ = value.setattr("column", column);
            // The category, so a caller can act on *which* rule was broken without matching on
            // message text. The decorator uses this to defer one specific case.
            let _ = value.setattr("code", code);
        }
        err
    }
}

/// The result of a successful compilation, as seen from Python.
// `skip_from_py_object`: this type is only ever handed *out* to Python, never accepted back, so
// deriving a conversion from Python would be dead surface area.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "compylr._core",
    name = "CompiledUnit"
)]
#[derive(Debug, Clone)]
pub struct PyCompiledUnit {
    /// Generated target files, keyed by path relative to the crate root.
    #[pyo3(get)]
    pub target_sources: std::collections::BTreeMap<String, String>,
    /// The IR, serialized for inspection.
    #[pyo3(get)]
    pub ir_artifact: String,
    /// Fingerprint of the compiled unit, as a hex string.
    #[pyo3(get)]
    pub fingerprint: String,
    /// Name of the extension module this unit compiles to.
    #[pyo3(get)]
    pub module_name: String,
    /// Build manifest for the generated crate.
    #[pyo3(get)]
    pub manifest: String,
    /// Names of the functions in the unit.
    #[pyo3(get)]
    pub function_names: Vec<String>,
}

impl From<Compiled> for PyCompiledUnit {
    fn from(compiled: Compiled) -> Self {
        Self {
            target_sources: compiled.target_sources,
            ir_artifact: compiled.ir_artifact,
            fingerprint: format!("{:016x}", compiled.fingerprint),
            module_name: compiled.module_name,
            manifest: compiled.manifest,
            function_names: compiled.function_names,
        }
    }
}

/// Compile source texts for a backend.
#[pyfunction]
#[pyo3(signature = (sources, backend = "rust"))]
fn compile_unit(py: Python<'_>, sources: Vec<String>, backend: &str) -> PyResult<PyCompiledUnit> {
    match compile(&sources, backend) {
        Ok(compiled) => Ok(compiled.into()),
        Err(failure) => Err(failure.into_py_err(py)),
    }
}

/// Check that a single function's source is inside the supported subset.
///
/// Used by the decorator to fail at the point a function is marked rather than at first call.
/// This deliberately does **not** resolve calls: a decorated function may legitimately call one
/// that has not been marked yet, and resolving here would make acceptance depend on decoration
/// order.
#[pyfunction]
fn validate_source(py: Python<'_>, source: &str) -> PyResult<Vec<String>> {
    let parsed = parse_source(source).map_err(|error| match error {
        SourceError::Syntax { message, span } => {
            let at = span.line_column(source);
            CompileFailure::Syntax {
                message,
                line: at.line,
                column: at.column,
            }
            .into_py_err(py)
        }
        SourceError::Io { path, source } => {
            SourceSyntaxError::new_err(format!("could not read {}: {source}", path.display()))
        }
    })?;

    let (functions, classes) = lower_source_members(&parsed)
        .map_err(|error| CompileFailure::from_lower(&error, source).into_py_err(py))?;
    Ok(functions
        .into_iter()
        .map(|f| f.name)
        .chain(classes.into_iter().map(|c| c.name))
        .collect())
}

/// Every backend name compylr recognizes, implemented or not.
#[pyfunction]
fn backend_names() -> Vec<String> {
    backend::names().into_iter().map(str::to_string).collect()
}

/// Every backend name that can compile today.
#[pyfunction]
fn implemented_backends() -> Vec<String> {
    backend::implemented_names()
}

/// Resolve a backend name, raising if it cannot be used.
#[pyfunction]
fn check_backend(name: &str) -> PyResult<()> {
    backend::lookup(name)
        .map(|_| ())
        .map_err(|error| BackendNotAvailableError::new_err(error.to_string()))
}

/// The native half of compylr.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCompiledUnit>()?;
    m.add_function(wrap_pyfunction!(compile_unit, m)?)?;
    m.add_function(wrap_pyfunction!(validate_source, m)?)?;
    m.add_function(wrap_pyfunction!(backend_names, m)?)?;
    m.add_function(wrap_pyfunction!(implemented_backends, m)?)?;
    m.add_function(wrap_pyfunction!(check_backend, m)?)?;

    let py = m.py();
    m.add("CompylrError", py.get_type::<CompylrError>())?;
    m.add("CompilationError", py.get_type::<CompilationError>())?;
    m.add("SourceSyntaxError", py.get_type::<SourceSyntaxError>())?;
    m.add(
        "UnsupportedProgramError",
        py.get_type::<UnsupportedProgramError>(),
    )?;
    m.add(
        "BackendNotAvailableError",
        py.get_type::<BackendNotAvailableError>(),
    )?;
    Ok(())
}
