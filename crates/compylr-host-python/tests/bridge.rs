//! The compiler's Python-facing entry point.
//!
//! The compilation logic is exercised directly, without an interpreter, because it is ordinary
//! Rust; the exception *types* are exercised under a real interpreter, because "a syntax error and
//! a subset rejection are distinguishable" is a claim about Python classes and cannot be checked
//! any other way.

use compylr::{CompileFailure, compile};

/// A source lowered under Python's own stance, which is what an unconfigured project resolves to.
fn py_source(text: &str) -> compylr_core::Source {
    compylr_core::Source::new(
        text,
        compylr_ir::Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR),
    )
}

const ADD: &str = "def add(a: int, b: int) -> int:\n    return a + b\n";

#[test]
fn one_source_compiles_to_target_source_ir_and_a_fingerprint() {
    let compiled = compile(&[py_source(ADD)], "rust").expect("must compile");

    assert!(compiled.target_sources.contains_key("src/generated.rs"));
    assert!(
        compiled.target_sources["src/lib.rs"].contains("#[pymodule]"),
        "the crate root must include the registration that makes it importable"
    );
    assert!(
        compiled.target_sources.keys().all(|p| !p.starts_with('/')),
        "paths must be relative, so the caller decides where the crate lands"
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
        &[py_source("def f(a: int) -> int:\n    return a\n")],
        "rust",
    )
    .expect("text-only source must compile");
    assert_eq!(compiled.function_names, ["f"]);
}

#[test]
fn sources_are_assembled_into_one_unit_so_cross_source_calls_resolve() {
    let caller = py_source("def caller(a: int) -> int:\n    return callee(a)\n");
    let callee = py_source("def callee(a: int) -> int:\n    return a * 2\n");

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
        &[py_source(
            "def caller(a: int) -> int:\n    return missing(a)\n",
        )],
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
    let failure =
        compile(&[py_source(ADD), py_source(ADD)], "rust").expect_err("a duplicate name must fail");
    match failure {
        CompileFailure::Unsupported { message, .. } => assert!(message.contains("add")),
        other => panic!("expected an unsupported-program failure, got {other:?}"),
    }
}

#[test]
fn a_syntax_error_is_distinguishable_from_a_subset_rejection() {
    let syntax = compile(&[py_source("def broken(:\n")], "rust").expect_err("must fail");
    assert!(matches!(syntax, CompileFailure::Syntax { .. }));

    let unsupported = compile(
        &[py_source(
            "def loops(a: int) -> int:\n    while a:\n        pass\n    return a\n",
        )],
        "rust",
    )
    .expect_err("must fail");
    assert!(matches!(unsupported, CompileFailure::Unsupported { .. }));
}

#[test]
fn diagnostics_carry_their_location() {
    // The rejection is on the third line; a diagnostic that lost that would send a user hunting.
    let failure = compile(
        &[py_source(
            "def f(a: int) -> int:\n    b = a + 1\n    return \"x\"\n",
        )],
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
    match compile(&[py_source(ADD)], "typescript") {
        Err(CompileFailure::Backend(error)) => assert!(error.is_not_implemented()),
        other => panic!("reserved backend should fail as unimplemented, got {other:?}"),
    }
    match compile(&[py_source(ADD)], "nonesuch") {
        Err(CompileFailure::Backend(error)) => assert!(error.is_unknown()),
        other => panic!("unknown backend should fail as unknown, got {other:?}"),
    }
}

#[test]
fn an_unusable_backend_is_reported_before_the_source_is_even_parsed() {
    // The source below is not valid Python. Asking for an unusable backend must still report the
    // backend, since that is the thing the caller got wrong.
    match compile(&[py_source("def broken(:\n")], "nonesuch") {
        Err(CompileFailure::Backend(_)) => {}
        other => panic!("expected the backend failure to win, got {other:?}"),
    }
}

#[test]
fn the_fingerprint_ignores_formatting_but_follows_meaning() {
    let plain = compile(&[py_source(ADD)], "rust").unwrap();
    let noisy = compile(
        &[py_source(concat!(
            "# a leading comment\n",
            "def add(a: int, b: int) -> int:\n",
            "\n",
            "        # an indented comment\n",
            "        return a + b\n",
        ))],
        "rust",
    )
    .unwrap();
    assert_eq!(
        plain.fingerprint, noisy.fingerprint,
        "comments and reformatting must not trigger a rebuild"
    );
    assert_eq!(plain.module_name, noisy.module_name);

    let changed = compile(
        &[py_source(
            "def add(a: int, b: int) -> int:\n    return a - b\n",
        )],
        "rust",
    )
    .unwrap();
    assert_ne!(plain.fingerprint, changed.fingerprint);
}

/// The exception hierarchy, checked under a real interpreter.
mod python_exceptions {
    use super::*;
    use compylr::compile;
    use pyo3::Python;
    use pyo3::types::{PyAnyMethods, PyStringMethods, PyTypeMethods};

    /// Compile something that must fail, and return the exception's class name.
    fn failure_class(source: &str) -> String {
        Python::attach(|py| {
            let failure = compile(&[py_source(source)], "rust").expect_err("must fail");
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
            let failure = compile(&[py_source(source)], "rust").expect_err("must fail");
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
                let failure = compile(&[py_source(source)], "rust").expect_err("must fail");
                let err = failure.into_py_err(py);
                assert!(
                    err.is_instance_of::<compylr::CompylrError>(py),
                    "a caller must be able to handle any compylr failure with one except clause"
                );
                assert!(err.is_instance_of::<compylr::CompilationError>(py));
            }
        });
    }

    #[test]
    fn a_backend_failure_is_not_a_compilation_error() {
        // Asking for an unimplemented target is a configuration mistake, not a bad program, and
        // handling one should not accidentally swallow the other.
        Python::attach(|py| {
            let failure = compile(
                &[py_source(
                    "def add(a: int, b: int) -> int:\n    return a + b\n",
                )],
                "typescript",
            )
            .expect_err("must fail");
            let err = failure.into_py_err(py);
            assert!(err.is_instance_of::<compylr::CompylrError>(py));
            assert!(!err.is_instance_of::<compylr::CompilationError>(py));
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
        let compiled = compile(&[py_source(DOUBLE), py_source(USES)], "rust")
            .expect("a cross-source call must be typed, not demand an annotation");
        assert_eq!(compiled.function_names, ["double", "uses"]);
    }

    #[test]
    fn source_order_does_not_matter() {
        let forward = compile(&[py_source(DOUBLE), py_source(USES)], "rust").unwrap();
        let backward = compile(&[py_source(USES), py_source(DOUBLE)], "rust").unwrap();
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
            &[py_source(
                "def f(n: int) -> int:\n    b = nowhere(n)\n    return b\n",
            )],
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
            &[py_source("def f(n: int) -> int:\n    return n ** 2\n")],
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

/// A behavior travels with each source, not with the call.
///
/// The property that makes mixed behavior within one project work at all: the decorator captures
/// each marked member as its own source, so a per-call setting could not express a project whose
/// members differ. What follows from that is that a call *between* two of them is an ordinary
/// call, because the meanings ride on the nodes rather than on anything the call has to know.
mod behavior_per_source {
    use compylr_core::{BehaviorRequest, Source};
    use compylr_ir::{BinOp, Checked, DivMode, Expr, Rounding, Stmt};

    use super::*;

    fn rust_source(text: &str) -> Source {
        let behavior =
            compylr::resolve_behavior(&BehaviorRequest::language("rust"), "python", "rust")
                .expect("both languages of the pair resolve");
        Source::new(text, behavior)
    }

    const FLOOR: &str = "def floor_it(a: int, b: int) -> int:\n    return a // b\n";

    fn rounding_of(unit: &compylr_ir::Unit, name: &str) -> (Rounding, Checked) {
        match &unit.get(name).expect("the fixture defines it").body[0] {
            Stmt::Return(Expr::Binary {
                op:
                    BinOp::Div {
                        mode: DivMode::Integer(rounding),
                        checked,
                    },
                ..
            }) => (*rounding, *checked),
            other => panic!("unexpected body: {other:?}"),
        }
    }

    /// Two sources, two behaviors, one unit — and each function keeps its own meanings.
    #[test]
    fn each_source_keeps_the_behavior_it_was_given() {
        let unit = compylr_registry::frontends::lookup("python")
            .unwrap()
            .lower(&[
                py_source(FLOOR),
                rust_source("def truncate_it(a: int, b: int) -> int:\n    return a // b\n"),
            ])
            .expect("must lower");

        assert_eq!(
            rounding_of(&unit, "floor_it"),
            (Rounding::TowardNegInf, Checked::Reported)
        );
        assert_eq!(
            rounding_of(&unit, "truncate_it"),
            (Rounding::TowardZero, Checked::Unchecked)
        );
    }

    #[test]
    fn an_omitted_behavior_is_the_source_languages_stance() {
        let inherited = compylr::resolve_behavior(&BehaviorRequest::inherit(), "python", "rust")
            .expect("must resolve");
        assert_eq!(
            inherited.axes(),
            &compylr_frontend_python::component::PYTHON_BEHAVIOR
        );
    }

    /// A call across the boundary types and resolves exactly as a same-behavior call would.
    #[test]
    fn a_cross_behavior_call_resolves() {
        let compiled = compile(
            &[
                py_source("def outer(a: int) -> int:\n    return inner(a)\n"),
                rust_source("def inner(a: int) -> int:\n    return a + 1\n"),
            ],
            "rust",
        )
        .expect("a call between two behaviors is an ordinary call");

        assert_eq!(compiled.function_names, ["inner", "outer"]);
    }

    /// The same source under two behaviors is two different programs, so two fingerprints.
    ///
    /// This is what makes a behavior change rebuild without any new machinery: the modes are part
    /// of what the program computes, so they reach the rebuild key the way everything else does.
    #[test]
    fn the_same_source_under_two_behaviors_fingerprints_differently() {
        let under_python = compile(&[py_source(FLOOR)], "rust").unwrap();
        let under_rust = compile(&[rust_source(FLOOR)], "rust").unwrap();

        assert_ne!(under_python.fingerprint, under_rust.fingerprint);
    }
}
