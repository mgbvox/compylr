//! compylr — transpiles a strict, fully annotated Python subset.
//!
//! The pipeline is deliberately split so that each stage can be tested on its own:
//!
//! ```text
//! source text ──frontend──> parse tree ──lower──> compylr IR ──backend──> target code
//! ```
//!
//! Each stage is a separate crate in this workspace, and the dependency edges between them are
//! what keep the split honest: `compylr-ir` cannot name a Python construct because no parser is
//! among its dependencies, and `compylr-backend-rust` cannot name Python for the same reason.
//! See the crate manifests under `crates/` for the shape.
//!
//! This crate is the assembly of those parts, plus the PyO3 layer that exposes *the compiler* to
//! Python as `compylr._core`. Note the second, entirely separate PyO3 role in the workspace:
//! `compylr-bridge-python-rust` *generates* PyO3 code onto the user's compiled functions. They
//! are different crates producing different artifacts with different lifecycles.
//!
//! The module paths here are a facade over the workspace crates so that a caller — and the test
//! suite — reaches the compiler through one name.

pub use compylr_diagnostics::span;
pub use compylr_ir as ir;

/// Diagnostics: the shared located-error type, plus the Python frontend's parse failures.
pub mod error {
    pub use compylr_diagnostics::error::{LowerError, LowerErrorKind};
    pub use compylr_frontend_python::error::SourceError;
}

/// Source language frontends, their registry, and the Python parser.
pub mod frontend {
    pub use compylr_core::frontend::{Frontend, FrontendError, LoweringError};
    pub use compylr_frontend_python::frontend::*;
    pub use compylr_registry::frontends::{implemented_names, lookup, names};
}

/// Lowering a Python tree into IR.
pub mod lower {
    pub use compylr_frontend_python::lower::*;
}

/// Target backends, their registry, and the code they generate.
pub mod backend {
    pub use compylr_core::backend::{Backend, BackendError, GeneratedFiles, format_source};
    pub use compylr_registry::backends::{implemented_names, lookup, names};

    /// The Rust backend.
    pub mod rust {
        pub use compylr_backend_rust::rust::*;
    }

    /// PyO3 generation for the `(python, rust)` pair.
    pub mod bindings {
        pub use compylr_bridge_python_rust::bindings::*;
    }
}

/// Host bridges, keyed by `(source, target)`, and their registry.
///
/// Named `bridge_registry` rather than `bridge` because [`bridge`] is already the PyO3 layer that
/// exposes *the compiler* to Python. The two are different things and sharing a name would keep
/// the confusion alive.
pub mod bridge_registry {
    pub use compylr_core::bridge::{BridgeError, HostArtifact, HostBridge};
    pub use compylr_registry::bridges::{lookup, pairs};
}

pub mod bridge;

pub use compylr_core::Guarantee;
pub use compylr_diagnostics::error::LowerError;
pub use compylr_diagnostics::span::Span;
pub use compylr_frontend_python::error::SourceError;
pub use compylr_ir::{ArtifactError, BinOp, Expr, Function, Literal, Param, Stmt, Ty, Unit};
