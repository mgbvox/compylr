//! The compiler's Python-facing entry point.
//!
//! The compilation logic is exercised directly, without an interpreter, because it is ordinary
//! Rust; the exception *types* are exercised under a real interpreter, because "a syntax error and
//! a subset rejection are distinguishable" is a claim about Python classes and cannot be checked
//! any other way.

use compylr::bridge::{CompileFailure, compile};

const ADD: &str = "def add(a: int, b: int) -> int:\n    return a + b\n";

#[test]
fn one_source_compiles_to_target_source_ir_and_a_fingerprint() {
    let compiled = compile(&[ADD.to_string()], "rust").expect("must compile");

    assert!(compiled.target_source.contains("pub mod generated"));
    assert!(
        compiled.target_source.contains("#[pymodule]"),
        "the target source must include the bindings that make it importable"
    );
    assert!(compiled.ir_artifact.contains("\"add\""));
    assert_ne!(compiled.fingerprint, 0);
    assert_eq!(compiled.function_names, ["add"]);
    assert!(compiled.module_name.starts_with("compylr_generated_"));
    assert!(compiled.manifest.contains("pyo3"));
}

#[test]
fn an_empty_collection_of_sources_succeeds_with_an_empty_unit() {
    // A project can legitimately have nothing marked yet; that is not an error.
    let compiled = compile(&[], "rust").expect("an empty unit must compile");
    assert!(compiled.function_names.is_empty());
}

#[test]
fn source_text_needs_no_file_behind_it() {
    // What `inspect.getsource` hands back: text with no path, possibly never written to disk.
    let compiled = compile(
        &["def f(a: int) -> int:\n    return a\n".to_string()],
        "rust",
    )
    .expect("text-only source must compile");
    assert_eq!(compiled.function_names, ["f"]);
}

#[test]
fn sources_are_assembled_into_one_unit_so_cross_source_calls_resolve() {
    let caller = "def caller(a: int) -> int:\n    return callee(a)\n".to_string();
    let callee = "def callee(a: int) -> int:\n    return a * 2\n".to_string();

    let forward = compile(&[caller.clone(), callee.clone()], "rust").expect("must compile");
    let backward = compile(&[callee, caller], "rust").expect("order must not matter");

    assert_eq!(forward.function_names, ["callee", "caller"]);
    assert_eq!(
        forward.fingerprint, backward.fingerprint,
        "resolution and fingerprinting must not depend on the order sources arrive"
    );
}

#[test]
fn a_call_to_a_function_in_no_source_is_rejected() {
    let failure = compile(
        &["def caller(a: int) -> int:\n    return missing(a)\n".to_string()],
        "rust",
    )
    .expect_err("an unresolved call must fail");
    match failure {
        CompileFailure::Unsupported { message, .. } => assert!(message.contains("missing")),
        other => panic!("expected an unsupported-program failure, got {other:?}"),
    }
}

#[test]
fn duplicate_function_names_across_sources_are_reported() {
    let failure = compile(&[ADD.to_string(), ADD.to_string()], "rust")
        .expect_err("a duplicate name must fail");
    match failure {
        CompileFailure::Unsupported { message, .. } => assert!(message.contains("add")),
        other => panic!("expected an unsupported-program failure, got {other:?}"),
    }
}

#[test]
fn a_syntax_error_is_distinguishable_from_a_subset_rejection() {
    let syntax = compile(&["def broken(:\n".to_string()], "rust").expect_err("must fail");
    assert!(matches!(syntax, CompileFailure::Syntax { .. }));

    let unsupported = compile(
        &["def loops(a: int) -> int:\n    while a:\n        pass\n    return a\n".to_string()],
        "rust",
    )
    .expect_err("must fail");
    assert!(matches!(unsupported, CompileFailure::Unsupported { .. }));
}

#[test]
fn diagnostics_carry_their_location() {
    // The rejection is on the third line; a diagnostic that lost that would send a user hunting.
    let failure = compile(
        &["def f(a: int) -> int:\n    b = a + 1\n    return \"x\"\n".to_string()],
        "rust",
    )
    .expect_err("must fail");
    match failure {
        CompileFailure::Unsupported { line, column, .. } => {
            assert_eq!(line, 3, "wrong line");
            assert!(
                column > 1,
                "column should point into the line, got {column}"
            );
        }
        other => panic!("expected an unsupported-program failure, got {other:?}"),
    }
}

#[test]
fn the_backend_registry_surfaces_through_the_bridge() {
    match compile(&[ADD.to_string()], "typescript") {
        Err(CompileFailure::Backend(error)) => assert!(error.is_not_implemented()),
        other => panic!("reserved backend should fail as unimplemented, got {other:?}"),
    }
    match compile(&[ADD.to_string()], "nonesuch") {
        Err(CompileFailure::Backend(error)) => assert!(error.is_unknown()),
        other => panic!("unknown backend should fail as unknown, got {other:?}"),
    }
}

#[test]
fn an_unusable_backend_is_reported_before_the_source_is_even_parsed() {
    // The source below is not valid Python. Asking for an unusable backend must still report the
    // backend, since that is the thing the caller got wrong.
    match compile(&["def broken(:\n".to_string()], "nonesuch") {
        Err(CompileFailure::Backend(_)) => {}
        other => panic!("expected the backend failure to win, got {other:?}"),
    }
}

#[test]
fn the_fingerprint_ignores_formatting_but_follows_meaning() {
    let plain = compile(&[ADD.to_string()], "rust").unwrap();
    let noisy = compile(
        &[concat!(
            "# a leading comment\n",
            "def add(a: int, b: int) -> int:\n",
            "\n",
            "        # an indented comment\n",
            "        return a + b\n",
        )
        .to_string()],
        "rust",
    )
    .unwrap();
    assert_eq!(
        plain.fingerprint, noisy.fingerprint,
        "comments and reformatting must not trigger a rebuild"
    );
    assert_eq!(plain.module_name, noisy.module_name);

    let changed = compile(
        &["def add(a: int, b: int) -> int:\n    return a - b\n".to_string()],
        "rust",
    )
    .unwrap();
    assert_ne!(plain.fingerprint, changed.fingerprint);
}

/// The exception hierarchy, checked under a real interpreter.
mod python_exceptions {
    use compylr::bridge::compile;
    use pyo3::Python;
    use pyo3::types::{PyAnyMethods, PyStringMethods, PyTypeMethods};

    /// Compile something that must fail, and return the exception's class name.
    fn failure_class(source: &str) -> String {
        Python::attach(|py| {
            let failure = compile(&[source.to_string()], "rust").expect_err("must fail");
            failure
                .into_py_err(py)
                .value(py)
                .get_type()
                .name()
                .expect("class name")
                .to_string_lossy()
                .into_owned()
        })
    }

    /// The `line` and `column` attributes of a failure's exception.
    fn failure_location(source: &str) -> (Option<usize>, Option<usize>) {
        Python::attach(|py| {
            let failure = compile(&[source.to_string()], "rust").expect_err("must fail");
            let err = failure.into_py_err(py);
            let value = err.value(py);
            (
                value.getattr("line").ok().and_then(|v| v.extract().ok()),
                value.getattr("column").ok().and_then(|v| v.extract().ok()),
            )
        })
    }

    #[test]
    fn a_syntax_error_and_a_subset_rejection_are_different_classes() {
        let syntax = failure_class("def broken(:\n");
        let unsupported =
            failure_class("def loops(a: int) -> int:\n    while a:\n        pass\n    return a\n");

        assert_eq!(syntax, "SourceSyntaxError");
        assert_eq!(unsupported, "UnsupportedProgramError");
    }

    #[test]
    fn a_compilation_error_carries_line_and_column_as_attributes() {
        let (line, column) =
            failure_location("def f(a: int) -> int:\n    b = a + 1\n    return \"x\"\n");
        assert_eq!(
            line,
            Some(3),
            "callers must be able to read the location without parsing the message"
        );
        assert!(column.is_some_and(|c| c > 1), "column: {column:?}");
    }

    #[test]
    fn every_compilation_failure_shares_one_catchable_base() {
        Python::attach(|py| {
            for source in [
                "def broken(:\n",
                "def loops(a: int) -> int:\n    while a:\n        pass\n    return a\n",
            ] {
                let failure = compile(&[source.to_string()], "rust").expect_err("must fail");
                let err = failure.into_py_err(py);
                assert!(
                    err.is_instance_of::<compylr::bridge::CompylrError>(py),
                    "a caller must be able to handle any compylr failure with one except clause"
                );
                assert!(err.is_instance_of::<compylr::bridge::CompilationError>(py));
            }
        });
    }

    #[test]
    fn a_backend_failure_is_not_a_compilation_error() {
        // Asking for an unimplemented target is a configuration mistake, not a bad program, and
        // handling one should not accidentally swallow the other.
        Python::attach(|py| {
            let failure = compile(
                &["def add(a: int, b: int) -> int:\n    return a + b\n".to_string()],
                "typescript",
            )
            .expect_err("must fail");
            let err = failure.into_py_err(py);
            assert!(err.is_instance_of::<compylr::bridge::CompylrError>(py));
            assert!(!err.is_instance_of::<compylr::bridge::CompilationError>(py));
        });
    }
}

/// Typing calls that cross source boundaries.
///
/// The decorator captures each function with `inspect.getsource`, so every decorated function is
/// its own source and a call between two of them is a call across sources. Signatures are
/// therefore gathered from every source before any is lowered — without that, the inference this
/// compiler advertises would work everywhere except the arrangement its main interface produces.
mod cross_source_inference {
    use super::*;

    const DOUBLE: &str = "def double(n: int) -> int:\n    return n * 2\n";
    const USES: &str =
        "def uses(n: int) -> int:\n    doubled = double(n)\n    return doubled + 1\n";

    #[test]
    fn a_call_into_another_source_is_typed() {
        let compiled = compile(&[DOUBLE.to_string(), USES.to_string()], "rust")
            .expect("a cross-source call must be typed, not demand an annotation");
        assert_eq!(compiled.function_names, ["double", "uses"]);
    }

    #[test]
    fn source_order_does_not_matter() {
        let forward = compile(&[DOUBLE.to_string(), USES.to_string()], "rust").unwrap();
        let backward = compile(&[USES.to_string(), DOUBLE.to_string()], "rust").unwrap();
        assert_eq!(
            forward.fingerprint, backward.fingerprint,
            "signatures are gathered before any body is lowered, so arrival order cannot matter"
        );
    }

    #[test]
    fn a_callee_in_no_source_is_still_reported() {
        // Deferring is not the same as ignoring: once every source is present, a binding that
        // still cannot be typed is an error.
        let failure = compile(
            &["def f(n: int) -> int:\n    b = nowhere(n)\n    return b\n".to_string()],
            "rust",
        )
        .expect_err("a callee that exists nowhere must fail");
        match failure {
            CompileFailure::Unsupported { code, .. } => {
                assert_eq!(code, "undetermined_binding");
            }
            other => panic!("expected an unsupported-program failure, got {other:?}"),
        }
    }

    #[test]
    fn the_failure_category_is_machine_readable() {
        // The decorator branches on this to decide what to defer, so it must not be prose.
        let failure = compile(
            &["def f(n: int) -> int:\n    while n:\n        pass\n    return n\n".to_string()],
            "rust",
        )
        .expect_err("must fail");
        match failure {
            CompileFailure::Unsupported { code, .. } => {
                assert_eq!(code, "unsupported_construct");
                assert_ne!(code, "undetermined_binding");
            }
            other => panic!("unexpected failure: {other:?}"),
        }
    }
}
