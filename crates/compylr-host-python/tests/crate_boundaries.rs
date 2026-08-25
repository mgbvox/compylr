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
    // `CARGO_MANIFEST_DIR` is this crate's directory, two levels down since the host binding
    // moved under `crates/` alongside every other crate.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate lives at <root>/crates/<name>")
        .to_path_buf()
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

const TS_PARSERS: [&str; 4] = ["oxc_allocator", "oxc_ast", "oxc_parser", "oxc_span"];

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

/// Only the TypeScript frontend may depend on a TypeScript parser.
#[test]
fn only_the_typescript_frontend_depends_on_a_typescript_parser() {
    for name in every_crate() {
        let deps = direct_dependencies(&name);
        let parsers: Vec<&&str> = TS_PARSERS.iter().filter(|p| deps.contains(**p)).collect();
        if name == "compylr-frontend-typescript" {
            assert!(
                !parsers.is_empty(),
                "the TypeScript frontend must be the crate that parses TypeScript"
            );
        } else {
            assert!(
                parsers.is_empty(),
                "{name} depends on {parsers:?}; only compylr-frontend-typescript may parse TypeScript"
            );
        }
    }
}

/// Only a host binding may link a host language's runtime.
///
/// `compylr-host-python` links PyO3 because it *is* a Python extension module. Nothing else may:
/// generating PyO3 code is emitting text and needs no dependency on it, and a crate below the host
/// layer that linked one would be a crate that only works when that language is present.
///
/// Stated over the `compylr-host-*` prefix rather than over one crate's name, so that a
/// `compylr-host-typescript` linking napi-rs passes for the same reason and neither is special.
#[test]
fn only_a_host_binding_links_a_host_runtime() {
    const HOST_RUNTIMES: [&str; 3] = ["pyo3", "napi", "wasm-bindgen"];

    for name in every_crate() {
        let is_host = name.starts_with("compylr-host-");
        let deps = direct_dependencies(&name);
        let linked: Vec<&&str> = HOST_RUNTIMES
            .iter()
            .filter(|r| deps.contains(**r))
            .collect();

        if is_host {
            assert!(
                !linked.is_empty(),
                "{name} is a host binding and links no host runtime, so it cannot be one"
            );
        } else {
            assert!(
                linked.is_empty(),
                "{name} depends on {linked:?}; only a `compylr-host-*` crate may link a host \
                 language's runtime"
            );
        }
    }
}

/// Exactly one crate is the host binding for a given language.
///
/// A second one would mean two answers to "how does Python call this", and the registries have no
/// way to choose between them.
#[test]
fn each_host_language_has_one_binding() {
    let hosts: Vec<String> = every_crate()
        .into_iter()
        .filter(|name| name.starts_with("compylr-host-"))
        .collect();
    assert!(!hosts.is_empty(), "there must be at least one host binding");

    let mut sorted = hosts.clone();
    sorted.sort();
    let count = sorted.len();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        count,
        "host bindings must name distinct languages"
    );
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

/// The Go backend may not depend on a host bridge and knows no host runtime.
#[test]
fn the_golang_backend_knows_no_host_language() {
    let deps = direct_dependencies("compylr-backend-golang");
    assert!(
        !deps.contains("compylr-bridge-typescript-golang"),
        "the bridge depends on the backend, never the reverse"
    );

    let source = read_crate_source("compylr-backend-golang");
    for spelling in ["napi", "pyo3", "koffi"] {
        assert!(
            !source.contains(spelling),
            "the Go backend mentions {spelling}; exposing generated code to a host belongs to \
             a bridge, one per (source, target) pair"
        );
    }
}

/// The `(python, rust)` bridge generates PyO3 source; it does not parse Python and does not
/// link PyO3.
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

/// The `(typescript, go)` bridge generates CGo/JS loader source; it does not parse TypeScript
/// and does not link napi.
#[test]
fn the_typescript_golang_bridge_neither_parses_ts_nor_links_napi() {
    let deps = direct_dependencies("compylr-bridge-typescript-golang");
    for parser in TS_PARSERS {
        assert!(
            !deps.contains(parser),
            "the bridge depends on {parser}; it reads the IR, not the user's source"
        );
    }
    assert!(
        !deps.contains("napi"),
        "the bridge emits JS/CGo loader as text and has no reason to link napi"
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

/// A stance declaration describes its own language and names no other.
///
/// This is the route the behavior model most plausibly leaks along. A declaration is a list of
/// what operations mean, and it would read perfectly naturally to write "…, unlike Python's" into
/// the Rust backend's — at which point the backend knows a source language, and a third language
/// costs an edit to every existing declaration instead of none.
///
/// Checked on the source with comments stripped, because the *prose* around a declaration may
/// legitimately compare: the point of `sequence_index: FromStart` is lost without saying that
/// somebody else counts from the end. What may not happen is code depending on it.
#[test]
fn a_stance_declaration_names_only_its_own_language() {
    for (crate_name, own, foreign) in [
        ("compylr-backend-rust", "rust", ["python", "typescript"]),
        ("compylr-frontend-python", "python", ["rust", "typescript"]),
    ] {
        let source = strip_comments(&read_crate_source(crate_name));
        for other in foreign {
            assert!(
                !source.contains(&format!("\"{other}\"")),
                "{crate_name} names '{other}'; it declares what {own} means and nothing about \
                 any other language — resolution is what holds two at once"
            );
        }
    }
}

/// Behavior resolution names no concrete language either.
///
/// This is the one place a pairwise table would be easy to write and hard to notice: resolution
/// holds two languages at once, so `if source == "python"` would compile, work, and quietly turn
/// the N + M property into N x M. The names arrive as strings on a `LanguagePair` and the
/// declarations arrive with them, which is what keeps a third language from costing an edit here.
///
/// The test module is excluded because its fixtures are two invented languages — which is itself
/// deliberate: reaching for the real declarations would let resolution pass by agreeing with them
/// coincidentally, even if it had stopped consulting them.
#[test]
fn behavior_resolution_names_no_concrete_language() {
    let source = read_crate_source("compylr-core");
    let resolution = source
        .split("mod tests")
        .next()
        .expect("splitting on a marker always yields a first part");
    let resolution = strip_comments(resolution);

    for language in ["python", "rust", "typescript", "cpp", "golang"] {
        assert!(
            !resolution.contains(&format!("\"{language}\"")),
            "compylr-core names '{language}'; resolution must read the two declarations it is \
             handed rather than know which languages exist"
        );
    }
}

/// Emission must not touch the filesystem or run anything.
///
/// Turning IR into text is a pure function of the unit, and that is not a style preference: it is
/// what makes the output byte-reproducible, which is what makes it safe to key a rebuild cache on.
/// A formatter invoked inside `emit` would make the result depend on which rustfmt is installed,
/// and two machines would disagree about whether a project needs rebuilding.
///
/// Asserted structurally, because the property is "cannot", not "did not on this run". The
/// exception is `format_source` and `post_process`, which are the *post*-emission hook and are
/// applied by whoever writes the files out.
#[test]
fn emission_reads_and_writes_nothing() {
    let source = strip_comments(&read_crate_source("compylr-backend-rust"));

    // `format_source` lives in core and is called only from `post_process`; everything else that
    // could reach the outside world would have to appear here.
    for escape in [
        "std::fs",
        "File::",
        "Command::new",
        "std::env::",
        "include_str!(\"/",
    ] {
        assert!(
            !source.contains(escape),
            "the Rust backend mentions {escape}; emission is a pure function of the unit, and a \
             backend that reads the environment cannot have byte-reproducible output"
        );
    }

    // The one filesystem-adjacent thing it does is embed the runtime at *compile* time, which
    // happens once when compylr is built rather than when a unit is emitted.
    assert!(
        source.contains("include_str!"),
        "the runtime is embedded at compile time; if that stopped being true this test is checking \
         the wrong thing"
    );
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
