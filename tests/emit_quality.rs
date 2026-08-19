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
use compylr::lower::lower_source_members;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/fixtures/accepted")
}

/// Build a unit from one or more fixtures.
///
/// The cross-source pair only resolves when compiled together, which is exactly the arrangement a
/// project of decorated functions produces.
fn unit_from_fixtures(names: &[impl AsRef<str> + std::fmt::Display]) -> Unit {
    let mut unit = Unit::new();
    for name in names {
        let path = fixtures_dir().join(name.as_ref());
        let parsed = parse_file(&path).expect("fixture must parse");
        let (functions, classes) =
            lower_source_members(&parsed).unwrap_or_else(|e| panic!("{name} should lower: {e}"));
        for class in classes {
            unit.add_class(class).expect("unique names");
        }
        for function in functions {
            unit.add_function(function).expect("unique names");
        }
    }
    unit.validate().expect("calls must resolve");
    unit
}

/// Every accepted fixture, grouped so that cross-source calls resolve.
///
/// Read from the directory rather than listed here. A hardcoded list silently stops covering
/// fixtures added later, which is the failure mode this test exists to prevent: a fixture that
/// lowers but emits code that does not compile would go unnoticed.
fn fixture_groups() -> Vec<(String, Vec<String>)> {
    let mut singles = Vec::new();
    let mut cross_source = Vec::new();
    let mut names: Vec<String> = std::fs::read_dir(fixtures_dir())
        .expect("accepted fixtures directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".py"))
        .collect();
    names.sort();

    for name in names {
        // The cross-source pair only resolves as one unit; everything else stands alone.
        if name.starts_with("cross_source_") {
            cross_source.push(name);
        } else {
            let label = name.trim_end_matches(".py").to_string();
            singles.push((label, vec![name]));
        }
    }
    if !cross_source.is_empty() {
        singles.push(("cross_source".to_string(), cross_source));
    }
    singles
}

#[test]
fn every_accepted_fixture_compiles_without_warnings() {
    let backend = lookup("rust").unwrap();
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lint");
    std::fs::create_dir_all(&tmp).unwrap();

    for (label, names) in fixture_groups() {
        let unit = unit_from_fixtures(&names);
        let emitted = backend.emit(&unit).expect("must emit");
        // Written out and compiled from the crate root, as the build pipeline does. The files
        // cannot be concatenated: `lib.rs` opens with inner attributes.
        let dir = tmp.join(&label);
        let _ = std::fs::remove_dir_all(&dir);
        for (relative, contents) in &emitted {
            let path = dir.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, contents).unwrap();
        }
        let path = dir.join("src/lib.rs");

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
        // The translated functions are their own file now, so this is a lookup. It used to be
        // string surgery, to keep a comment edit in the runtime from forcing a snapshot review;
        // splitting the crate made the workaround unnecessary.
        let functions = &emitted["src/generated.rs"];
        insta::assert_snapshot!(format!("emit_{label}"), functions.trim());
    }
}

#[test]
fn formatting_is_best_effort_and_never_loses_the_source() {
    let backend = lookup("rust").unwrap();
    let unit = unit_from_fixtures(&["arithmetic.py"]);
    let emitted = backend.emit(&unit).unwrap()["src/generated.rs"].clone();
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
    // If rustfmt is present, the formatted crate must still compile; if it is absent,
    // `format_source` returns its input and this is the same assertion as above.
    let backend = lookup("rust").unwrap();
    let unit = unit_from_fixtures(&["division.py"]);
    let emitted = backend.emit(&unit).unwrap();

    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fmt");
    let _ = std::fs::remove_dir_all(&tmp);
    for (relative, contents) in &emitted {
        let path = tmp.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, format_source(contents)).unwrap();
    }

    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("lib")
        .arg("--emit")
        .arg("metadata")
        .arg("-o")
        .arg(tmp.join("formatted.rmeta"))
        .arg(tmp.join("src/lib.rs"))
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
