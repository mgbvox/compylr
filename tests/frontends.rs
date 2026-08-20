//! Frontends resolve by name, the same three ways backends do.
//!
//! The asymmetry this replaces was real: a backend could be asked for by name and answer
//! "implemented", "reserved", or "unknown", while the source language was whatever was compiled
//! in. A compiler that intends to accept more than one source language cannot have one of them
//! be the default by construction.

use compylr::frontend::{self, FrontendError};

#[test]
fn python_is_implemented() {
    let frontend = frontend::lookup("python").expect("python must be implemented");
    assert_eq!(frontend.name(), "python");
}

#[test]
fn a_reserved_frontend_reports_itself_as_planned() {
    let error = frontend::lookup("go").expect_err("go is not implemented yet");
    assert!(error.is_not_implemented(), "{error}");
    assert!(!error.is_unknown());
    assert!(
        error.to_string().contains("planned"),
        "a reserved name should read as planned, got: {error}"
    );
}

#[test]
fn an_unknown_frontend_lists_what_would_have_worked() {
    let error = frontend::lookup("perl").expect_err("perl is not a compylr frontend");
    assert!(error.is_unknown(), "{error}");
    assert!(!error.is_not_implemented());
    assert!(
        error.to_string().contains("python"),
        "an unknown name should say what does work, got: {error}"
    );
}

/// The two failures must be distinguishable without reading the message.
///
/// Message wording is presentation. A caller that branches on it is broken by a rewording, which
/// is precisely the kind of change nobody expects to break anything.
#[test]
fn the_two_failures_are_distinguishable_by_kind() {
    let reserved = frontend::lookup("cpp").expect_err("cpp is reserved");
    let unknown = frontend::lookup("cobol").expect_err("cobol is not a name");

    let classify = |error: &FrontendError| {
        if error.is_not_implemented() {
            "reserved"
        } else if error.is_unknown() {
            "unknown"
        } else {
            "other"
        }
    };
    assert_eq!(classify(&reserved), "reserved");
    assert_eq!(classify(&unknown), "unknown");
}

#[test]
fn every_reserved_name_is_listed_but_only_some_can_compile() {
    let all = frontend::names();
    let implemented = frontend::implemented_names();

    assert!(all.contains(&"python"));
    assert!(implemented.contains(&"python".to_string()));
    assert!(
        all.len() > implemented.len(),
        "reserving a name is what makes 'planned' a distinct answer"
    );
    for name in &implemented {
        assert!(
            all.contains(&name.as_str()),
            "{name} can compile but is not in the registry"
        );
    }
}

/// The frontend is what turns source text into a unit, with no caller doing the assembly.
#[test]
fn the_frontend_lowers_source_text_into_a_unit() {
    let frontend = frontend::lookup("python").unwrap();
    let unit = frontend
        .lower(&["def double(n: int) -> int:\n    return n * 2\n".to_string()])
        .expect("a supported program must lower");
    assert_eq!(unit.functions().count(), 1);
}

/// Multiple sources assemble into one unit, which is the arrangement the decorator produces.
#[test]
fn sources_assemble_into_one_unit_across_files() {
    let frontend = frontend::lookup("python").unwrap();
    let unit = frontend
        .lower(&[
            "def double(n: int) -> int:\n    return n * 2\n".to_string(),
            "def quadruple(n: int) -> int:\n    return double(double(n))\n".to_string(),
        ])
        .expect("a call across sources must type");
    assert_eq!(unit.functions().count(), 2);
}

#[test]
fn a_syntax_failure_and_a_subset_rejection_are_different_kinds() {
    let frontend = frontend::lookup("python").unwrap();
    let syntax = frontend
        .lower(&["def broken(:\n".to_string()])
        .expect_err("must fail");
    let unsupported = frontend
        .lower(&["def f(a):\n    return a\n".to_string()])
        .expect_err("must fail");

    assert!(syntax.is_syntax(), "{syntax}");
    assert!(!unsupported.is_syntax(), "{unsupported}");
    assert_eq!(unsupported.code(), Some("missing_annotation"));
    assert_eq!(syntax.code(), None);
}

/// A lowering failure locates itself, so a caller does not need the source text to report it.
#[test]
fn a_failure_carries_a_resolved_line_and_column() {
    let frontend = frontend::lookup("python").unwrap();
    let error = frontend
        .lower(&["def f(a: int) -> int:\n    return a + \"x\"\n".to_string()])
        .expect_err("must fail");
    assert_eq!(error.line(), 2);
    assert!(error.column() > 1, "{error}");
    assert!(error.to_string().starts_with("2:"), "{error}");
}

/// The frontend declares what its source language needs preserved on the way to a target.
#[test]
fn the_python_frontend_declares_what_it_requires() {
    use compylr::Guarantee;
    let frontend = frontend::lookup("python").unwrap();
    let required = frontend.requires();

    for guarantee in [
        Guarantee::IntegerOverflowReported,
        Guarantee::DivisionByZeroReported,
        Guarantee::FloatOrderPreserved,
    ] {
        assert!(
            required.contains(&guarantee),
            "Python needs {guarantee} preserved for a compiled function to still mean what the \
             source meant"
        );
    }
}
