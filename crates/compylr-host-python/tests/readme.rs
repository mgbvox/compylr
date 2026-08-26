//! Keeps `README.md` honest.
//!
//! A README drifts the moment it is only prose someone promises to update. These tests turn the
//! parts that *can* drift mechanically into assertions, so `cargo test` fails when the code and
//! the README disagree.
//!
//! Deliberately checked: facts derived from the code and the repo layout — the type table, the
//! operator list, the capability list, and every path the README points at.
//!
//! Deliberately **not** checked: anything that churns without meaning, such as a test count.
//! An assertion that fails every time a test is added trains people to edit the README without
//! reading it, which is worse than no check at all.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use compylr_frontend_python::{PythonOperator, PythonTypeName};
use compylr_ir::{Axis, BinOp, Checked, DivMode, RemSign, Rounding, Ty};

fn repo_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is this crate's directory, two levels down since the host binding
    // moved under `crates/` alongside every other crate.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate lives at <root>/crates/<name>")
        .to_path_buf()
}

fn readme() -> String {
    std::fs::read_to_string(repo_root().join("README.md")).expect("README.md must exist at root")
}

/// Every supported type must appear in the README, spelled as inline code.
#[test]
fn readme_documents_every_type() {
    let text = readme();
    for ty in [Ty::Int, Ty::Float, Ty::Bool, Ty::Str, Ty::Unit] {
        let token = format!("`{}`", ty.python_name());
        assert!(
            text.contains(&token),
            "README does not document the type {token}; add it to the subset table"
        );
    }
}

/// Every supported operator must appear in the README, spelled as inline code.
///
/// Spelled the way a Python programmer writes it, which since the operators started carrying
/// their semantics is the frontend's rendering rather than the IR's — the IR now says "integer
/// division rounding toward negative infinity", which is right for a diagnostic in no particular
/// language and wrong for a README about Python.
#[test]
fn readme_documents_every_operator() {
    let text = readme();
    let ops = [
        BinOp::Add {
            checked: Checked::Reported,
        },
        BinOp::Sub {
            checked: Checked::Reported,
        },
        BinOp::Mul {
            checked: Checked::Reported,
        },
        BinOp::Div {
            mode: DivMode::Exact,
            checked: Checked::Reported,
        },
        BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardNegInf),
            checked: Checked::Reported,
        },
        BinOp::Rem {
            sign: RemSign::Divisor,
            checked: Checked::Reported,
        },
        BinOp::Eq,
        BinOp::NotEq,
        BinOp::Lt,
        BinOp::LtE,
        BinOp::Gt,
        BinOp::GtE,
    ];
    for op in ops {
        let token = format!("`{}`", op.python_symbol());
        assert!(
            text.contains(&token),
            "README does not document the operator {token}; add it to the subset section"
        );
    }
}

#[test]
fn readme_behavior_table_lists_exactly_the_compiler_axes() {
    let text = readme();
    let section = text
        .split("### Behavior axes")
        .nth(1)
        .expect("README must contain a Behavior axes section")
        .split("\n### ")
        .next()
        .expect("Behavior axes section must have a body");
    let documented: BTreeSet<&str> = section
        .lines()
        .filter(|line| line.starts_with("| "))
        .filter_map(|line| line.split('|').nth(2))
        .map(|cell| cell.trim().trim_matches('`'))
        .filter(|axis| !matches!(*axis, "IR axis" | "---"))
        .collect();
    let actual: BTreeSet<&str> = Axis::ALL.into_iter().map(Axis::code).collect();

    assert_eq!(
        documented, actual,
        "README behavior table must list exactly the compiler's behavior axes"
    );
}

/// Every capability with a main spec must be listed, so a new one cannot land unmentioned.
#[test]
fn readme_lists_every_capability() {
    let text = readme();
    let specs = repo_root().join("openspec/specs");
    let capabilities: BTreeSet<String> = std::fs::read_dir(&specs)
        .expect("openspec/specs must exist")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    assert!(
        !capabilities.is_empty(),
        "expected at least one capability under openspec/specs"
    );
    for capability in &capabilities {
        assert!(
            text.contains(&format!("`{capability}`")),
            "README does not mention capability `{capability}`; add it to the status table"
        );
    }
}

/// Every workspace crate must appear in the layout section.
///
/// The unit that matters is the crate, not the file: a crate is where a dependency edge is
/// declared, and the edges are what keep languages out of shared code. A reader who cannot see
/// the full set from the README cannot tell which crate a new language would touch.
#[test]
fn readme_layout_covers_every_crate() {
    let text = readme();
    let crates = repo_root().join("crates");
    let mut seen = 0;
    for entry in std::fs::read_dir(&crates)
        .expect("crates must exist")
        .flatten()
    {
        if !entry.path().join("Cargo.toml").exists() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(
            text.contains(&name),
            "README layout does not mention crates/{name}; add it or drop the crate"
        );
        seen += 1;
    }
    assert!(seen >= 5, "expected several crates, found {seen}");
}

/// Every repo path the README points at must exist.
///
/// This is the check that catches the common failure: a rename lands, and the README keeps
/// pointing at a file that is no longer there.
///
/// The roots are the directories that exist *at the workspace root*. `tests/` is not one of them
/// any more — the integration suite moved under `crates/compylr-host-python/` with the package it
/// belongs to — so a bare `tests/` in the layout block is a nested entry rather than a path, and
/// is left alone.
#[test]
fn readme_references_only_paths_that_exist() {
    let text = readme();
    let roots = ["crates/", "scripts/", "frontends/python/", "openspec/", "vendored/"];
    let mut checked = 0;

    for raw in text.split(|c: char| c.is_whitespace() || c == '`' || c == '(' || c == ')') {
        let token = raw.trim_end_matches([',', '.', ':', ';', '"', '\'']);
        if !roots.iter().any(|r| token.starts_with(r)) {
            continue;
        }
        // Skip glob-ish or prose fragments.
        if token.contains('*') || token.is_empty() {
            continue;
        }
        let path: &Path = token.as_ref();
        assert!(
            repo_root().join(path).exists(),
            "README references `{token}`, which does not exist"
        );
        checked += 1;
    }

    assert!(
        checked >= 5,
        "expected the README to reference several real paths, found {checked} — \
         the extraction may have broken"
    );
}

/// The README's claim about the backend must match reality **in both directions**.
///
/// The status section is the first thing a reader trusts, and whether Rust is emitted is the
/// single most damaging thing it could get wrong.
///
/// This originally asserted only that a missing backend was disclosed, which meant it fell silent
/// the moment a backend landed — going quiet exactly when the README first became wrong. A
/// one-directional check protects nothing after the transition it was written for, so the
/// existing-backend case is asserted too.
#[test]
fn readme_status_matches_reality() {
    let text = readme();
    let backend_exists = repo_root().join("crates/compylr-backend-rust").exists();

    if backend_exists {
        assert!(
            !text.contains("no backend"),
            "a backend exists, so the README must not still say there is none"
        );
        assert!(
            text.contains("Rust source"),
            "a backend exists, so the README status section should say Rust is emitted"
        );
    } else {
        assert!(
            text.contains("not built") || text.contains("no backend"),
            "no backend exists, so the README status section must say so"
        );
    }
}

/// The README must not claim an importable Python package exists before one does.
///
/// This is the same failure mode one stage later: the backend landing makes it tempting to say
/// the decorator works, and a reader who tries `import compylr` on that basis gets a
/// `ModuleNotFoundError` rather than a compiled function.
#[test]
fn readme_does_not_promise_a_python_package_that_does_not_exist() {
    let text = readme();
    let package_exists = repo_root().join("pyproject.toml").exists()
        && repo_root().join("frontends/python/compylr/__init__.py").exists();

    if !package_exists {
        assert!(
            text.contains("does not work") || text.contains("no Python package"),
            "there is no installable Python package, so the README must say so plainly"
        );
    }
}

/// The generated subset matrix is addressable, populated, and rests on fixtures that exist.
///
/// The mechanical half of "the documented subset is generated from the corpus". What the table
/// *claims* is checked by `scripts/update_subset.py --check`, which regenerates it; what this
/// checks is that the block is still there to be regenerated, and that every fixture it names is
/// still in the corpus. Moving or renaming a marker is what breaks the generator, and it would
/// otherwise break it silently.
#[test]
fn readme_carries_a_generated_subset_matrix() {
    let text = readme();
    let opening = "<!-- subset:matrix -->";
    let closing = "<!-- /subset:matrix -->";

    assert_eq!(
        text.matches(opening).count(),
        1,
        "expected exactly one {opening}"
    );
    assert_eq!(
        text.matches(closing).count(),
        1,
        "expected exactly one {closing}"
    );

    let start = text.find(opening).expect("the opening marker") + opening.len();
    let end = text.find(closing).expect("the closing marker");
    assert!(end > start, "{closing} appears before {opening}");
    let block = text[start..end].trim();

    assert!(
        block.contains("| Form | Kind | Exercised by |"),
        "the subset matrix is empty; run ./scripts/update_subset.py"
    );

    // Every fixture the table names must still be in the corpus. A renamed fixture would
    // otherwise leave the README pointing at a file nobody can open.
    let accepted = repo_root().join("frontends/python/fixtures/accepted");
    for line in block.lines().filter(|line| line.starts_with("| `")) {
        let named = line
            .rsplit('|')
            .nth(1)
            .expect("a row ends with the fixture that exercises it")
            .trim()
            .trim_matches('`');
        assert!(
            accepted.join(named).exists(),
            "the subset matrix names {named}, which is not in frontends/python/fixtures/accepted/"
        );
    }
}
