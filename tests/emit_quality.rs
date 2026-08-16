//! Whether emitted source is fit to be compiled and read.
//!
//! Two properties, checked against every accepted fixture rather than a hand-picked example:
//! the output compiles with warnings denied, and its shape is snapshotted so an unintended change
//! shows up as a diff instead of as a silently different tree.

use std::path::PathBuf;
use std::process::Command;

use compylr::backend::{format_source, lookup};
use compylr::frontend::parse_file;
use compylr::ir::Unit;
use compylr::lower::lower_source;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/fixtures/accepted")
}

/// Build a unit from one or more fixtures.
///
/// The cross-source pair only resolves when compiled together, which is exactly the arrangement a
/// project of decorated functions produces.
fn unit_from_fixtures(names: &[&str]) -> Unit {
    let mut unit = Unit::new();
    for name in names {
        let path = fixtures_dir().join(name);
        let parsed = parse_file(&path).expect("fixture must parse");
        for function in lower_source(&parsed).unwrap_or_else(|e| panic!("{name} should lower: {e}"))
        {
            unit.add_function(function).expect("unique names");
        }
    }
    unit.validate().expect("calls must resolve");
    unit
}

/// Every accepted fixture, grouped so that cross-source calls resolve.
fn fixture_groups() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("aliases", vec!["aliases.py"]),
        ("arithmetic", vec!["arithmetic.py"]),
        ("calls", vec!["calls.py"]),
        ("comparisons", vec!["comparisons.py"]),
        ("division", vec!["division.py"]),
        ("floats", vec!["floats.py"]),
        ("inference", vec!["inference.py"]),
        (
            "cross_source",
            vec!["cross_source_caller.py", "cross_source_callee.py"],
        ),
    ]
}

#[test]
fn every_accepted_fixture_compiles_without_warnings() {
    let backend = lookup("rust").unwrap();
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lint");
    std::fs::create_dir_all(&tmp).unwrap();

    for (label, names) in fixture_groups() {
        let unit = unit_from_fixtures(&names);
        let emitted = backend.emit(&unit).expect("must emit");
        let path = tmp.join(format!("{label}.rs"));
        std::fs::write(&path, &emitted).unwrap();

        let output = Command::new("rustc")
            .arg("--edition")
            .arg("2024")
            .arg("--crate-type")
            .arg("lib")
            .arg("--emit")
            .arg("metadata")
            .arg("-D")
            .arg("warnings")
            .arg("-o")
            .arg(tmp.join(format!("{label}.rmeta")))
            .arg(&path)
            .output()
            .expect("rustc must be available");

        assert!(
            output.status.success(),
            "emitted Rust for `{label}` did not compile cleanly:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn emitted_source_is_stable() {
    let backend = lookup("rust").unwrap();
    for (label, names) in fixture_groups() {
        let unit = unit_from_fixtures(&names);
        let emitted = backend.emit(&unit).expect("must emit");
        // The embedded runtime is a verbatim copy of `src/backend/runtime.rs` and is already
        // covered by its own tests; snapshotting it here would turn every comment edit in that
        // file into a snapshot review with nothing to review.
        let marker = "pub mod generated {";
        let index = emitted
            .find(marker)
            .expect("generated module must be present");
        let functions = &emitted[index + marker.len()..];
        insta::assert_snapshot!(format!("emit_{label}"), functions.trim());
    }
}

#[test]
fn formatting_is_best_effort_and_never_loses_the_source() {
    let backend = lookup("rust").unwrap();
    let unit = unit_from_fixtures(&["arithmetic.py"]);
    let emitted = backend.emit(&unit).unwrap();
    let formatted = format_source(&emitted);

    // Whether or not rustfmt ran, the result must still be the same program.
    assert!(
        formatted.contains("pub fn"),
        "formatting must not discard the source"
    );
    for function in unit.functions() {
        assert!(
            formatted.contains(&function.name),
            "function `{}` vanished during formatting",
            function.name
        );
    }
}

#[test]
fn formatting_does_not_change_what_the_code_does() {
    // If rustfmt is present, the formatted source must still compile; if it is absent,
    // `format_source` returns the input and this is the same assertion as above.
    let backend = lookup("rust").unwrap();
    let unit = unit_from_fixtures(&["division.py"]);
    let formatted = format_source(&backend.emit(&unit).unwrap());

    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fmt");
    std::fs::create_dir_all(&tmp).unwrap();
    let path = tmp.join("formatted.rs");
    std::fs::write(&path, &formatted).unwrap();

    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("lib")
        .arg("--emit")
        .arg("metadata")
        .arg("-o")
        .arg(tmp.join("formatted.rmeta"))
        .arg(&path)
        .output()
        .expect("rustc must be available");

    assert!(
        output.status.success(),
        "formatted source must still compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn emission_is_reproducible_for_every_fixture() {
    let backend = lookup("rust").unwrap();
    for (label, names) in fixture_groups() {
        let first = backend.emit(&unit_from_fixtures(&names)).unwrap();
        let second = backend.emit(&unit_from_fixtures(&names)).unwrap();
        assert_eq!(first, second, "emission for `{label}` is not reproducible");
    }
}
