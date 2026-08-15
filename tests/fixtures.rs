//! Fixture-driven tests over real Python files.
//!
//! The unit tests inside each module cover behaviour with inline sources. These tests exercise
//! the same rules through files on disk, which is how the compiler will actually be fed, and
//! snapshot the lowered IR so that an unintended change in shape shows up as a diff rather than
//! as a silently different tree.

use std::path::{Path, PathBuf};

use compylr::error::LowerErrorKind;
use compylr::frontend::parse_file;
use compylr::ir::{Function, Unit};
use compylr::lower::lower_source;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/fixtures")
}

fn lower_fixture(path: &Path) -> Result<Vec<Function>, compylr::error::LowerError> {
    let parsed = parse_file(path).expect("fixture must parse as valid Python");
    lower_source(&parsed)
}

fn accepted(name: &str) -> Vec<Function> {
    let path = fixtures_dir().join("accepted").join(name);
    lower_fixture(&path).unwrap_or_else(|error| panic!("{name} should lower, but failed: {error}"))
}

fn rejected(name: &str) -> LowerErrorKind {
    let path = fixtures_dir().join("rejected").join(name);
    match lower_fixture(&path) {
        Ok(_) => panic!("{name} should have been rejected but lowered successfully"),
        Err(error) => error.kind(),
    }
}

#[test]
fn accepted_fixtures_lower_to_stable_ir() {
    for name in [
        "arithmetic.py",
        "comparisons.py",
        "aliases.py",
        "calls.py",
        "inference.py",
        "floats.py",
        "division.py",
    ] {
        let functions = accepted(name);
        insta::assert_debug_snapshot!(name, functions);
    }
}

#[test]
fn every_rejected_fixture_fails_with_the_expected_kind() {
    let cases: &[(&str, LowerErrorKind)] = &[
        (
            "missing_param_annotation.py",
            LowerErrorKind::MissingAnnotation,
        ),
        (
            "missing_return_annotation.py",
            LowerErrorKind::MissingAnnotation,
        ),
        (
            "unannotated_from_call.py",
            LowerErrorKind::MissingAnnotation,
        ),
        (
            "unsupported_type_complex.py",
            LowerErrorKind::UnsupportedType,
        ),
        ("unsupported_generic.py", LowerErrorKind::UnsupportedType),
        ("none_parameter.py", LowerErrorKind::UnsupportedType),
        ("type_parameters.py", LowerErrorKind::UnsupportedType),
        ("decorated.py", LowerErrorKind::UnsupportedConstruct),
        ("async_function.py", LowerErrorKind::UnsupportedConstruct),
        ("varargs.py", LowerErrorKind::UnsupportedConstruct),
        ("kwargs.py", LowerErrorKind::UnsupportedConstruct),
        ("default_value.py", LowerErrorKind::UnsupportedConstruct),
        ("keyword_only.py", LowerErrorKind::UnsupportedConstruct),
        ("if_statement.py", LowerErrorKind::UnsupportedConstruct),
        ("while_loop.py", LowerErrorKind::UnsupportedConstruct),
        ("import_statement.py", LowerErrorKind::UnsupportedConstruct),
        ("class_definition.py", LowerErrorKind::UnsupportedConstruct),
        ("exponentiation.py", LowerErrorKind::UnsupportedConstruct),
        ("str_plus_int.py", LowerErrorKind::TypeMismatch),
        ("boolean_arithmetic.py", LowerErrorKind::TypeMismatch),
        ("negate_string.py", LowerErrorKind::TypeMismatch),
        ("compare_unrelated.py", LowerErrorKind::TypeMismatch),
        ("narrowing_annotation.py", LowerErrorKind::TypeMismatch),
        ("return_type_conflict.py", LowerErrorKind::TypeMismatch),
        ("return_from_unit.py", LowerErrorKind::TypeMismatch),
        ("main_guard.py", LowerErrorKind::UnsupportedConstruct),
        ("big_integer.py", LowerErrorKind::LiteralOutOfRange),
        ("unbound_name.py", LowerErrorKind::Unresolved),
        ("alias_of_unbound.py", LowerErrorKind::Unresolved),
        ("rebind_local.py", LowerErrorKind::Reassignment),
        ("conflicting_annotation.py", LowerErrorKind::TypeMismatch),
    ];

    for (name, expected) in cases {
        assert_eq!(
            rejected(name),
            *expected,
            "wrong diagnostic kind for {name}"
        );
    }
}

#[test]
fn every_rejected_fixture_is_covered_by_the_table() {
    // Guards against adding a fixture and forgetting to assert on it, which would leave a
    // rejection rule silently untested.
    let dir = fixtures_dir().join("rejected");
    let count = std::fs::read_dir(&dir)
        .expect("rejected fixtures directory must exist")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "py"))
        .count();
    assert_eq!(count, 31, "update the rejection table when adding fixtures");
}

#[test]
fn entrypoint_is_rejected() {
    // python/entrypoint.py sits outside the subset twice over: `def main():` carries no return
    // annotation, and the file ends in an `if __name__ == '__main__':` guard. Because
    // diagnostics report the first violation in source order, the missing annotation is what
    // surfaces -- the guard is never reached. Asserting the earlier one keeps this test honest
    // about ordering; main_guard.py covers the guard rule on its own.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/entrypoint.py");
    let error = lower_fixture(&path).expect_err("entrypoint.py should be rejected");
    assert_eq!(error.kind(), LowerErrorKind::MissingAnnotation);
    assert!(error.message().contains("main"));
}

#[test]
fn calls_resolve_across_separately_lowered_sources() {
    let caller = accepted("cross_source_caller.py");
    let callee = accepted("cross_source_callee.py");

    // Add the caller first: resolution must not depend on arrival order.
    let mut unit = Unit::new();
    for function in caller.into_iter().chain(callee) {
        unit.add_function(function).unwrap();
    }
    unit.validate()
        .expect("a call across two sources should resolve once both are in the unit");
    assert_eq!(unit.len(), 2);
}

#[test]
fn formatting_differences_do_not_change_fingerprints() {
    let plain = "def add(a: int, b: int) -> int:\n    return a + b\n";
    // Same function: extra comments, a blank line, and a wider indent.
    let decorated_with_noise = concat!(
        "# a leading comment\n",
        "def add(a: int, b: int) -> int:\n",
        "\n",
        "        # an indented comment\n",
        "        return a + b\n",
    );

    let lower_text = |source: &str| {
        let parsed = compylr::frontend::parse_source(source).unwrap();
        lower_source(&parsed).unwrap()
    };

    let a = lower_text(plain);
    let b = lower_text(decorated_with_noise);
    assert_eq!(
        a[0].fingerprint(),
        b[0].fingerprint(),
        "comments, blank lines, and indentation width must not affect the fingerprint"
    );

    // A real change to the body must move it.
    let changed = lower_text("def add(a: int, b: int) -> int:\n    return a - b\n");
    assert_ne!(a[0].fingerprint(), changed[0].fingerprint());
}

#[test]
fn unit_fingerprint_is_stable_across_addition_order() {
    let build = |reverse: bool| {
        let mut functions = accepted("arithmetic.py");
        if reverse {
            functions.reverse();
        }
        let mut unit = Unit::new();
        for function in functions {
            unit.add_function(function).unwrap();
        }
        unit
    };
    assert_eq!(build(false).fingerprint(), build(true).fingerprint());
}
