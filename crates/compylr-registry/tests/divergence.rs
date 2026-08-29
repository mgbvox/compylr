//! Cross-language IR divergence, recorded and ratcheted.
//!
//! Two frontends that lower the same program should produce the same IR. Nothing measured whether
//! they do, which meant the shared middle end could drift apart one frontend change at a time. This
//! records the divergence of every member both corpora define, and refuses to let it move without
//! the recorded table moving with it.
//!
//! It lives in `compylr-registry` because it needs two frontends at once, and this is the one crate
//! permitted to know them all. `compylr-host-python` owns the corpus tests but does not depend on
//! the TypeScript frontend, and `crate_boundaries.rs` exists to refuse that edge.
//!
//! Members pair **by name**. Pairing whole files by stem was the original plan and is wrong: the
//! two corpora share filenames without sharing programs, so a file-level score would be dominated
//! by members one corpus simply does not define, and driving it down would mean writing fixtures
//! rather than fixing the compiler.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use compylr_core::{BehaviorRequest, LanguagePair, Source, diff, resolve};
use compylr_ir::Unit;
use compylr_registry::{backends, frontends};

/// Where the measured table is kept.
const RECORDED: &str = "tests/divergence.recorded";

/// Set this to rewrite the recorded table from a real run.
const UPDATE: &str = "UPDATE_DIVERGENCE";

/// The two corpora being compared, as `(frontend, target, directory, extension)`.
///
/// The target is named because lowering takes a negotiated behavior, not because it can change the
/// answer — `backend_independence.rs` is what establishes that it cannot.
const CORPORA: [(&str, &str, &str, &str); 2] = [
    ("python", "rust", "frontends/python/fixtures/accepted", "py"),
    (
        "typescript",
        "go",
        "frontends/typescript/fixtures/accepted",
        "ts",
    ),
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate lives at <root>/crates/<name>")
        .to_path_buf()
}

/// Lower every accepted fixture of one corpus into a single unit.
///
/// One unit, as a real project is built. `Unit::add_function` refuses a duplicate, so this is also
/// what enforces that a member name is unique across a corpus — the property the pairing relies on.
fn corpus(frontend_name: &str, target: &str, directory: &str, extension: &str) -> Unit {
    let frontend = frontends::lookup(frontend_name).expect("frontend is registered");
    let backend = backends::lookup(target).expect("backend is registered");

    let mut known: Vec<&str> = frontends::names();
    known.extend(backends::names());
    known.sort_unstable();
    known.dedup();

    let behavior = resolve(
        &BehaviorRequest::default(),
        &LanguagePair {
            source: frontend.name(),
            source_behavior: frontend.behavior(),
            target: backend.name(),
            target_behavior: backend.behavior(),
            known: &known,
        },
        None,
    )
    .expect("the pair resolves");

    // Read from the directory rather than from a list. Both fixture enumerations in this
    // repository are derived for the same reason: a list drifts, and a drifted list once hid a
    // fixture that had been producing code that did not compile.
    let path = workspace_root().join(directory);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|file| file.extension().is_some_and(|found| found == extension))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no {extension} fixtures in {directory}");

    let mut whole = Unit::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("fixture is readable");
        let unit = frontend
            .lower(&[Source::new(text, behavior)])
            .unwrap_or_else(|error| {
                panic!(
                    "{} is an accepted fixture but did not lower: {error}",
                    file.display()
                )
            });
        for function in unit.functions() {
            whole
                .add_function(function.clone())
                .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        }
        for class in unit.classes() {
            whole
                .add_class(class.clone())
                .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        }
    }
    whole
}

/// Every member name a unit defines.
fn members(unit: &Unit) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = unit
        .functions()
        .map(|function| function.name.clone())
        .collect();
    names.extend(unit.classes().map(|class| class.name.clone()));
    names
}

/// A unit holding only the named members.
///
/// Restricting both sides to the shared names is what makes the score a score *about pairs*: a
/// member the other corpus does not define is missing coverage, which the table reports separately,
/// and counting it as divergence would mean a corpus scored better by staying small.
fn restricted(unit: &Unit, keep: &BTreeSet<String>) -> Unit {
    let mut narrowed = Unit::new();
    for name in keep {
        if let Some(function) = unit.get(name) {
            narrowed
                .add_function(function.clone())
                .expect("names came from this unit");
        } else if let Some(class) = unit.class(name) {
            narrowed
                .add_class(class.clone())
                .expect("names came from this unit");
        }
    }
    narrowed
}

/// Measure the two corpora and render the table a run produces.
fn measure() -> String {
    let [
        (left_name, left_target, left_dir, left_ext),
        (right_name, right_target, right_dir, right_ext),
    ] = CORPORA;
    let left = corpus(left_name, left_target, left_dir, left_ext);
    let right = corpus(right_name, right_target, right_dir, right_ext);

    let left_members = members(&left);
    let right_members = members(&right);
    let shared: BTreeSet<String> = left_members.intersection(&right_members).cloned().collect();

    let found = diff::divergence(
        &diff::normalize(&restricted(&left, &shared)),
        &diff::normalize(&restricted(&right, &shared)),
    );

    let scores: BTreeMap<&str, u32> = found
        .members()
        .iter()
        .map(|member| (member.name(), member.score()))
        .collect();

    let only_left: Vec<&String> = left_members.difference(&right_members).collect();
    let only_right: Vec<&String> = right_members.difference(&left_members).collect();

    let mut out = String::new();
    out.push_str(&format!(
        "# Cross-language IR divergence between the {left_name} and {right_name} accepted corpora.\n"
    ));
    out.push_str("#\n");
    out.push_str("# GENERATED. Regenerate from a real run with:\n");
    out.push_str(&format!(
        "#   {UPDATE}=1 cargo test -p compylr-registry --test divergence\n"
    ));
    out.push_str("#\n");
    out.push_str(
        "# A member is listed when BOTH corpora define it, under the same name. The check\n",
    );
    out.push_str(
        "# recomputes rather than trusting this file, and requires it to match exactly: a\n",
    );
    out.push_str(
        "# score that rises fails, a score that falls fails until it is recorded, and a\n",
    );
    out.push_str("# value edited by hand fails. The only way the numbers move is a real run.\n");
    out.push_str("#\n");
    out.push_str(&format!("# total {}\n", found.score()));
    out.push_str(&format!("# pairs {}\n", shared.len()));
    out.push('\n');
    for (name, score) in &scores {
        out.push_str(&format!("{name} {score}\n"));
    }
    out.push('\n');
    out.push_str(
        "# Members only one corpus defines. Not divergence -- missing coverage. Listed so\n",
    );
    out.push_str(
        "# that dropping a pair, which would lower the total by measuring less, is a diff.\n",
    );
    for name in &only_left {
        out.push_str(&format!("# only-{left_name} {name}\n"));
    }
    for name in &only_right {
        out.push_str(&format!("# only-{right_name} {name}\n"));
    }
    out
}

/// Parse the `<member> <score>` lines out of a rendered table.
fn scores_in(table: &str) -> BTreeMap<String, u32> {
    table
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| {
            let (name, score) = line.rsplit_once(' ')?;
            Some((name.to_string(), score.parse().ok()?))
        })
        .collect()
}

/// The recorded table matches what a run produces.
///
/// One byte comparison covers every rule at once: a raised score, a lowered one that nobody
/// recorded, a hand-edited number, and a pair quietly dropped from the corpus.
#[test]
fn the_recorded_divergence_is_current() {
    let measured = measure();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(RECORDED);

    if std::env::var(UPDATE).is_ok() {
        std::fs::write(&path, &measured).expect("the recorded table is writable");
        return;
    }

    let recorded = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "cannot read {}: {error}\nrun `{UPDATE}=1 cargo test -p compylr-registry --test divergence` to create it",
            path.display()
        )
    });

    if recorded == measured {
        return;
    }

    // Say what moved before saying that something did. A failure that only reports "the file
    // differs" sends the reader to a diff to work out whether the news is good or bad.
    let was = scores_in(&recorded);
    let now = scores_in(&measured);
    let mut report = String::new();
    for (name, score) in &now {
        match was.get(name) {
            Some(before) if before < score => {
                report.push_str(&format!("  {name}: rose from {before} to {score}\n"));
            }
            Some(before) if before > score => {
                report.push_str(&format!("  {name}: fell from {before} to {score}\n"));
            }
            Some(_) => {}
            None => report.push_str(&format!("  {name}: newly paired at {score}\n")),
        }
    }
    for name in was.keys() {
        if !now.contains_key(name) {
            report.push_str(&format!("  {name}: no longer paired\n"));
        }
    }
    if report.is_empty() {
        report.push_str("  the scores are unchanged; the coverage lists moved\n");
    }

    panic!(
        "the recorded cross-language divergence is out of date:\n{report}\n\
         If this is an improvement, record it:\n  \
         {UPDATE}=1 cargo test -p compylr-registry --test divergence"
    );
}

/// Every member both corpora define is one the differ can actually compare.
///
/// Guards the case where the table is trivially satisfied because nothing pairs at all: a corpus
/// that shares no member with the other would record an empty table and pass forever.
#[test]
fn the_corpora_share_members_to_compare() {
    let [
        (left_name, left_target, left_dir, left_ext),
        (right_name, right_target, right_dir, right_ext),
    ] = CORPORA;
    let left = members(&corpus(left_name, left_target, left_dir, left_ext));
    let right = members(&corpus(right_name, right_target, right_dir, right_ext));
    let shared = left.intersection(&right).count();

    assert!(
        shared > 0,
        "the two accepted corpora share no member name, so nothing is being compared"
    );
}
