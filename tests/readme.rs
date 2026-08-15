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

use compylr::ir::{BinOp, Ty};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
#[test]
fn readme_documents_every_operator() {
    let text = readme();
    let ops = [
        BinOp::Add,
        BinOp::Sub,
        BinOp::Mul,
        BinOp::TrueDiv,
        BinOp::FloorDiv,
        BinOp::Mod,
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

/// Every library module must appear in the layout section.
#[test]
fn readme_layout_covers_every_module() {
    let text = readme();
    let src = repo_root().join("src");
    for entry in std::fs::read_dir(&src).expect("src must exist").flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // lib.rs and main.rs are plumbing, not concepts a reader needs the layout to explain.
        if !name.ends_with(".rs") || name == "lib.rs" || name == "main.rs" {
            continue;
        }
        assert!(
            text.contains(&name),
            "README layout does not mention src/{name}; add it or drop the module"
        );
    }
}

/// Every repo path the README points at must exist.
///
/// This is the check that catches the common failure: a rename lands, and the README keeps
/// pointing at a file that is no longer there.
#[test]
fn readme_references_only_paths_that_exist() {
    let text = readme();
    let roots = [
        "src/",
        "scripts/",
        "python/",
        "openspec/",
        "tests/",
        "vendored/",
    ];
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

/// The README must not claim a backend exists while none does.
///
/// The status section is the first thing a reader trusts, and "it emits Rust" is the single
/// most damaging thing it could get wrong.
#[test]
fn readme_status_matches_reality() {
    let text = readme();
    let backend_exists =
        repo_root().join("src/codegen.rs").exists() || repo_root().join("src/backend").exists();
    if !backend_exists {
        assert!(
            text.contains("not built") || text.contains("no backend"),
            "no backend exists, so the README status section must say so"
        );
    }
}
