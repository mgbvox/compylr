//! Fixture-driven tests over real Python files.
//!
//! The unit tests inside each module cover behaviour with inline sources. These tests exercise
//! the same rules through files on disk, which is how the compiler will actually be fed, and
//! snapshot the lowered IR so that an unintended change in shape shows up as a diff rather than
//! as a silently different tree.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use compylr_diagnostics::error::LowerErrorKind;
use compylr_frontend_python::frontend::parse_file;
use compylr_frontend_python::lower::lower_source_members;
use compylr_ir::{Function, Unit};

mod support;
use support::drivers;

/// The workspace root, which the fixture tree hangs off.
///
/// `CARGO_MANIFEST_DIR` is this crate's directory, two levels down since the host binding moved
/// under `crates/` alongside every other crate.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate lives at <root>/crates/<name>")
        .to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    workspace_root().join("python/fixtures")
}

fn lower_fixture(path: &Path) -> Result<Vec<Function>, compylr_diagnostics::error::LowerError> {
    let parsed = parse_file(path).expect("fixture must parse as valid Python");
    lower_source_members(&parsed, python_stance()).map(|(functions, _)| functions)
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
    // Read from the directory rather than listed, so a fixture added later is snapshotted rather
    // than quietly uncovered.
    let mut names: Vec<String> = std::fs::read_dir(fixtures_dir().join("accepted"))
        .expect("accepted fixtures directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".py") && !name.starts_with("cross_source_"))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "there must be accepted fixtures");

    for name in names {
        let functions = accepted(&name);
        insta::assert_debug_snapshot!(name, functions);
    }
}

#[test]
fn default_behavior_fixture_fingerprints_are_stable() {
    // This is the permanent form of the one-time before/after comparison. The structural fixture
    // snapshots did not move when behavior selection replaced the frontend constants, and these
    // fingerprints make that equivalence a compact baseline that future changes must review.
    let mut names: Vec<String> = std::fs::read_dir(fixtures_dir().join("accepted"))
        .expect("accepted fixtures directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".py"))
        .collect();
    names.sort();

    let fingerprints: BTreeMap<String, u64> = names
        .into_iter()
        .map(|name| {
            let path = fixtures_dir().join("accepted").join(&name);
            let parsed = parse_file(&path).expect("fixture must parse as valid Python");
            let (functions, classes) = lower_source_members(&parsed, python_stance())
                .unwrap_or_else(|error| panic!("{name} should lower, but failed: {error}"));
            let mut unit = Unit::new();
            for class in classes {
                unit.add_class(class).expect("fixture names are unique");
            }
            for function in functions {
                unit.add_function(function)
                    .expect("fixture names are unique");
            }
            (name, unit.fingerprint())
        })
        .collect();

    insta::assert_debug_snapshot!(fingerprints);
}

/// What the corpus records each rejected program is refused *for*.
///
/// Every file in `python/fixtures/rejected/` appears here, and
/// `every_rejected_fixture_is_covered_by_the_table` derives that requirement from the directory
/// rather than from a count. It used to be a count, and twelve fixtures had drifted out of the
/// table without anything noticing -- a fixture with no recorded rejection is one whose refusal
/// nothing is asserting.
const REJECTIONS: &[(&str, LowerErrorKind)] = &[
    (
        "missing_param_annotation.py",
        LowerErrorKind::MissingAnnotation,
    ),
    (
        "missing_return_annotation.py",
        LowerErrorKind::MissingAnnotation,
    ),
    (
        "unsupported_type_complex.py",
        LowerErrorKind::UnsupportedType,
    ),
    ("unsupported_generic.py", LowerErrorKind::UnsupportedType),
    ("none_parameter.py", LowerErrorKind::UnsupportedType),
    ("type_parameters.py", LowerErrorKind::UnsupportedType),
    (
        "constructor_early_return.py",
        LowerErrorKind::UnsupportedConstruct,
    ),
    ("decorated.py", LowerErrorKind::UnsupportedConstruct),
    ("async_function.py", LowerErrorKind::UnsupportedConstruct),
    ("varargs.py", LowerErrorKind::UnsupportedConstruct),
    ("kwargs.py", LowerErrorKind::UnsupportedConstruct),
    ("default_value.py", LowerErrorKind::UnsupportedConstruct),
    ("keyword_only.py", LowerErrorKind::UnsupportedConstruct),
    ("non_boolean_test.py", LowerErrorKind::TypeMismatch),
    ("non_boolean_loop_test.py", LowerErrorKind::TypeMismatch),
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
    ("redeclare_local.py", LowerErrorKind::Reassignment),
    ("conflicting_annotation.py", LowerErrorKind::TypeMismatch),
    ("bare_expression.py", LowerErrorKind::UnsupportedConstruct),
    ("trailing_string.py", LowerErrorKind::UnsupportedConstruct),
    ("wrong_arity.py", LowerErrorKind::ArityMismatch),
    ("wrong_argument_type.py", LowerErrorKind::TypeMismatch),
    ("missing_return.py", LowerErrorKind::MissingReturn),
    ("float_dict_key.py", LowerErrorKind::UnsupportedType),
    ("bare_list_annotation.py", LowerErrorKind::UnsupportedType),
    ("mismatched_literal.py", LowerErrorKind::TypeMismatch),
    (
        "computed_tuple_index.py",
        LowerErrorKind::UnsupportedConstruct,
    ),
    ("slicing.py", LowerErrorKind::UnsupportedConstruct),
    ("reserved_len.py", LowerErrorKind::UnsupportedConstruct),
    ("reserved_range.py", LowerErrorKind::UnsupportedConstruct),
    (
        "range_outside_loop.py",
        LowerErrorKind::UnsupportedConstruct,
    ),
    (
        "break_outside_loop.py",
        LowerErrorKind::LoopControlOutsideLoop,
    ),
    ("branch_bound_name.py", LowerErrorKind::Unresolved),
    ("one_branch_returns.py", LowerErrorKind::MissingReturn),
    ("append_to_mapping.py", LowerErrorKind::TypeMismatch),
    (
        "assign_into_parameter.py",
        LowerErrorKind::UnsupportedConstruct,
    ),
    ("assign_into_tuple.py", LowerErrorKind::TypeMismatch),
    ("class_inherits.py", LowerErrorKind::UnsupportedConstruct),
    (
        "class_without_init.py",
        LowerErrorKind::UnsupportedConstruct,
    ),
    ("membership_type_mismatch.py", LowerErrorKind::TypeMismatch),
    (
        "method_without_self.py",
        LowerErrorKind::UnsupportedConstruct,
    ),
    ("mutate_alias.py", LowerErrorKind::UnsupportedConstruct),
    ("mutate_parameter.py", LowerErrorKind::UnsupportedConstruct),
    (
        "unannotated_attribute.py",
        LowerErrorKind::MissingAnnotation,
    ),
    ("undeclared_attribute.py", LowerErrorKind::Unresolved),
    (
        "unsupported_method.py",
        LowerErrorKind::UnsupportedConstruct,
    ),
];

#[test]
fn every_rejected_fixture_fails_with_the_expected_kind() {
    for (name, expected) in REJECTIONS {
        assert_eq!(
            rejected(name),
            *expected,
            "wrong diagnostic kind for {name}"
        );
    }
}

/// Every rejected program is **still** refused.
///
/// The inverted guard. Growing the accepted subset is a decision, and this is what makes it one:
/// a program here that begins to lower fails the suite, and the failure is cleared by moving it
/// into `accepted/` and giving it a driver -- never by editing an allowance. Without this, a
/// construct becoming supported is invisible, and the corpus quietly stops recording what the
/// subset refuses.
#[test]
fn every_rejected_fixture_is_still_refused() {
    let recorded: BTreeMap<&str, LowerErrorKind> = REJECTIONS.iter().copied().collect();
    let mut lowered = Vec::new();

    for name in rejected_names() {
        let path = fixtures_dir().join("rejected").join(&name);
        if lower_fixture(&path).is_ok() {
            let kind = recorded.get(name.as_str()).map_or_else(
                || "with no recorded rejection".to_string(),
                |kind| format!("recorded as {}", kind.code()),
            );
            lowered.push(format!("{name} ({kind})"));
        }
    }

    assert!(
        lowered.is_empty(),
        "these programs are in the rejected corpus but now lower successfully: {lowered:?}\n\
         if a construct became supported, move its program into python/fixtures/accepted/ and \
         give it a driver -- see python/fixtures/rejected/README.md"
    );
}

/// Every file in the rejected corpus has a recorded rejection.
///
/// Derived from the directory, not counted. A count cannot tell you *which* fixture drifted out,
/// and it did not: twelve had.
#[test]
fn every_rejected_fixture_is_covered_by_the_table() {
    let recorded: BTreeSet<String> = REJECTIONS
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();
    let on_disk: BTreeSet<String> = rejected_names().into_iter().collect();

    let untabled: Vec<&String> = on_disk.difference(&recorded).collect();
    assert!(
        untabled.is_empty(),
        "these rejected fixtures have no recorded rejection: {untabled:?}"
    );
    let missing: Vec<&String> = recorded.difference(&on_disk).collect();
    assert!(
        missing.is_empty(),
        "the table records rejections for files that are not there: {missing:?}"
    );
}

/// The rejected corpus, read from the directory.
fn rejected_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixtures_dir().join("rejected"))
        .expect("rejected fixtures directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".py"))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "there must be rejected fixtures");
    names
}

#[test]
fn entrypoint_is_rejected() {
    // python/entrypoint.py sits outside the subset twice over: `def main():` carries no return
    // annotation, and the file ends in an `if __name__ == '__main__':` guard. Because
    // diagnostics report the first violation in source order, the missing annotation is what
    // surfaces -- the guard is never reached. Asserting the earlier one keeps this test honest
    // about ordering; main_guard.py covers the guard rule on its own.
    let path = workspace_root().join("python/entrypoint.py");
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
        let parsed = compylr_frontend_python::frontend::parse_source(source).unwrap();
        lower_source_members(&parsed, python_stance()).unwrap().0
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

/// Python's own stance, which is what an unconfigured compilation resolves to.
///
/// Read from the frontend's declaration rather than rebuilt here, so these tests lower under the
/// same bundle the pipeline uses.
fn python_stance() -> compylr_ir::Behavior {
    compylr_ir::Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR)
}

/// Every accepted fixture states which calls exercise it, and nothing states calls for a fixture
/// that is not there.
///
/// The corpus's value is entirely in its coverage, and coverage that is not checked decays. Both
/// lists are read from their directories: a literal list drifted once already and hid a real
/// defect.
#[test]
fn every_accepted_fixture_has_exactly_one_driver() {
    let fixtures = drivers::accepted_stems();
    let declared = drivers::driver_stems();
    assert!(!fixtures.is_empty(), "there must be accepted fixtures");

    let undriven: Vec<&String> = fixtures.iter().filter(|f| !declared.contains(f)).collect();
    assert!(
        undriven.is_empty(),
        "these accepted fixtures have no driver, so they prove nothing: {undriven:?}\n\
         add python/fixtures/drivers/<name>.py for each"
    );

    let orphaned: Vec<&String> = declared.iter().filter(|d| !fixtures.contains(d)).collect();
    assert!(
        orphaned.is_empty(),
        "these drivers name no accepted fixture: {orphaned:?}"
    );
}

/// A driver reaches every member its fixture defines.
///
/// The members come from the lowered unit rather than from a list beside the check, so what is
/// demanded cannot drift from what the fixture actually declares. A member no driver calls
/// contributes nothing to the evidence, which is the whole point of the corpus.
#[test]
fn every_driver_reaches_every_member_its_fixture_defines() {
    let Some(loaded) = drivers::load_all() else {
        eprintln!("skipping: no python3 on PATH to read the drivers");
        return;
    };

    let mut missing: Vec<String> = Vec::new();
    for stem in drivers::accepted_stems() {
        let driver = loaded
            .get(&stem)
            .unwrap_or_else(|| panic!("{stem} has no driver; the companion test names it"));

        let path = fixtures_dir().join("accepted").join(format!("{stem}.py"));
        let parsed = parse_file(&path).expect("fixture must parse as valid Python");
        let (functions, classes) = lower_source_members(&parsed, python_stance())
            .unwrap_or_else(|error| panic!("{stem} should lower, but failed: {error}"));

        let defined = functions
            .iter()
            .map(|f| f.name.clone())
            .chain(classes.iter().map(|c| c.name.clone()));
        for member in defined {
            if !driver.members.contains(&member) {
                missing.push(format!("{stem}.{member}"));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "these members are defined by a fixture and called by no driver: {missing:?}"
    );
}
