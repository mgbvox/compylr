//! The dependency graph, asserted.
//!
//! Every claim this workspace makes about modularity reduces to one thing: a crate cannot name
//! what it does not depend on. `compylr-backend-rust` cannot grow a Python spelling because no
//! Python parser is reachable from it, and `compylr-ir` cannot grow a Rust type for the same
//! reason.
//!
//! That is only true while the manifests say so, and a manifest is one line away from not saying
//! so. A reviewer will not notice `ruff_python_ast` appearing in a backend's dependency list;
//! this test will. It reads the manifests rather than the resolved graph, because the resolved
//! graph would need cargo metadata and the interesting mistake is always a direct edge.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The direct dependency names declared by a crate's manifest, dev-dependencies excluded.
///
/// Dev-dependencies are excluded deliberately: they do not reach a consumer, so a test helper
/// pulling in a parser says nothing about what the crate can name in its own source.
fn direct_dependencies(crate_name: &str) -> BTreeSet<String> {
    let manifest = repo_root()
        .join("crates")
        .join(crate_name)
        .join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", manifest.display()));

    let mut names = BTreeSet::new();
    let mut in_dependencies = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]";
            continue;
        }
        if !in_dependencies || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((left, _)) = trimmed.split_once('=') else {
            continue;
        };
        // `name.workspace = true` and `name = { ... }` both name the crate before the first dot.
        let name = left.trim().split('.').next().unwrap_or_default().trim();
        if !name.is_empty() {
            names.insert(name.to_string());
        }
    }
    names
}

fn every_crate() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(repo_root().join("crates"))
        .expect("crates/ must exist")
        .flatten()
        .filter(|entry| entry.path().join("Cargo.toml").exists())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

const PARSERS: [&str; 4] = [
    "ruff_python_ast",
    "ruff_python_parser",
    "ruff_source_file",
    "ruff_text_size",
];

/// Only the Python frontend may depend on a Python parser.
///
/// This is the load-bearing edge. The moment a second crate can parse Python, "the IR is
/// independent of the source language" becomes a claim rather than a fact.
#[test]
fn only_the_python_frontend_depends_on_a_python_parser() {
    for name in every_crate() {
        let deps = direct_dependencies(&name);
        let parsers: Vec<&&str> = PARSERS.iter().filter(|p| deps.contains(**p)).collect();
        if name == "compylr-frontend-python" {
            assert!(
                !parsers.is_empty(),
                "the Python frontend must be the crate that parses Python"
            );
        } else {
            assert!(
                parsers.is_empty(),
                "{name} depends on {parsers:?}; only compylr-frontend-python may parse Python"
            );
        }
    }
}

/// Nothing below the assembly layer may depend on PyO3.
///
/// Generating PyO3 code is emitting text and needs no PyO3 dependency. The only crate that links
/// it is the one that *is* a Python extension module — the root `compylr` package.
#[test]
fn no_workspace_crate_links_pyo3() {
    for name in every_crate() {
        assert!(
            !direct_dependencies(&name).contains("pyo3"),
            "{name} depends on pyo3; only the root `compylr` package, which is the extension \
             module itself, may link it"
        );
    }
}

/// Diagnostics sits below everything, so it may depend on nothing.
#[test]
fn diagnostics_depends_on_nothing() {
    assert!(
        direct_dependencies("compylr-diagnostics").is_empty(),
        "compylr-diagnostics must stay dependency-free: whatever it pulls in reaches every crate"
    );
}

/// The IR may not depend on a frontend, a backend, or the component model.
///
/// The arrow points the other way: implementations consume the IR. An edge here would make the
/// shared model depend on one language's idea of what a program is.
#[test]
fn the_ir_depends_on_no_implementation() {
    let deps = direct_dependencies("compylr-ir");
    for forbidden in [
        "compylr-core",
        "compylr-frontend-python",
        "compylr-backend-rust",
        "compylr-bridge-python-rust",
        "compylr-registry",
    ] {
        assert!(
            !deps.contains(forbidden),
            "compylr-ir depends on {forbidden}; the IR is consumed by implementations, not the \
             other way round"
        );
    }
}

/// The component model may not depend on an implementation of itself.
///
/// This is why the registry is its own crate: a table naming every backend has to live somewhere
/// that can see them, and that somewhere cannot be the crate defining what a backend is.
#[test]
fn core_depends_on_no_implementation() {
    let deps = direct_dependencies("compylr-core");
    for forbidden in [
        "compylr-frontend-python",
        "compylr-backend-rust",
        "compylr-bridge-python-rust",
        "compylr-registry",
    ] {
        assert!(
            !deps.contains(forbidden),
            "compylr-core depends on {forbidden}; core defines interfaces and must not know an \
             implementation"
        );
    }
}

/// A backend may not depend on a host bridge, and may not name a source language.
#[test]
fn the_rust_backend_knows_no_host_language() {
    let deps = direct_dependencies("compylr-backend-rust");
    assert!(
        !deps.contains("compylr-bridge-python-rust"),
        "the bridge depends on the backend, never the reverse: a target that no Python calls \
         must not need the Python bridge to build"
    );

    let source = read_crate_source("compylr-backend-rust");
    for spelling in ["pyo3", "PyO3", "#[pyclass]", "#[pyfunction]"] {
        assert!(
            !source.contains(spelling),
            "the Rust backend mentions {spelling}; exposing generated code to a host belongs to \
             a bridge, one per (source, target) pair"
        );
    }
}

/// The `(python, rust)` bridge generates PyO3 source; it does not parse Python and does not
/// link PyO3.
///
/// Both are easy to reach for and both would be wrong. Reading the user's Python here would mean
/// the binding layer depended on something the IR does not carry, which is exactly what would
/// not survive a second frontend; linking PyO3 would confuse generating a boundary with being
/// one.
#[test]
fn the_python_rust_bridge_neither_parses_python_nor_links_pyo3() {
    let deps = direct_dependencies("compylr-bridge-python-rust");
    for parser in PARSERS {
        assert!(
            !deps.contains(parser),
            "the bridge depends on {parser}; it reads the IR, not the user's source"
        );
    }
    assert!(
        !deps.contains("pyo3"),
        "the bridge emits PyO3 source as text and has no reason to link it"
    );
}

/// The component model may not spell a source language's constructs either.
///
/// Core defines what a frontend *is*. A Python keyword appearing here would mean the interface
/// had been shaped around one implementation of itself, which is the failure this crate exists
/// to prevent.
#[test]
fn core_names_no_source_language_syntax() {
    let source = strip_comments(&read_crate_source("compylr-core"));
    for spelling in [
        "\"def \"",
        "\"elif\"",
        "ruff_",
        "\"lambda\"",
        "\"__init__\"",
    ] {
        assert!(
            !source.contains(spelling),
            "compylr-core mentions {spelling}; the component model must not be shaped around \
             one source language"
        );
    }
}

/// The IR's own source may not spell a Python construct.
///
/// The manifest check above makes this impossible to do by *calling* a parser; this catches the
/// remaining route, which is writing the spelling out as a string.
#[test]
fn the_ir_source_names_no_python_syntax() {
    let source = strip_comments(&read_crate_source("compylr-ir"));
    for spelling in ["\"def \"", "\"elif\"", "ruff_", "\"lambda\""] {
        assert!(
            !source.contains(spelling),
            "the IR mentions {spelling}; a Python spelling here is inherited by every backend"
        );
    }
}

/// Concatenate every `.rs` file in a crate, tests included.
fn read_crate_source(crate_name: &str) -> String {
    let mut out = String::new();
    collect(
        &repo_root().join("crates").join(crate_name).join("src"),
        &mut out,
    );
    out
}

fn collect(dir: &Path, out: &mut String) {
    for entry in std::fs::read_dir(dir)
        .expect("crate src must exist")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push_str(&std::fs::read_to_string(&path).expect("source must be readable"));
            out.push('\n');
        }
    }
}

/// Drop `//` lines so that prose explaining a rule does not trip the rule.
fn strip_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}
