//! Frontends resolve by name, the same three ways backends do.
//!
//! The asymmetry this replaces was real: a backend could be asked for by name and answer
//! "implemented", "reserved", or "unknown", while the source language was whatever was compiled
//! in. A compiler that intends to accept more than one source language cannot have one of them
//! be the default by construction.

use compylr_core::frontend::FrontendError;
use compylr_registry::frontends as frontend;

/// A source lowered under Python's own stance, which is what an unconfigured project resolves to.
fn py_source(text: &str) -> compylr_core::Source {
    compylr_core::Source::new(
        text,
        compylr_ir::Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR),
    )
}

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
        .lower(&[py_source("def double(n: int) -> int:\n    return n * 2\n")])
        .expect("a supported program must lower");
    assert_eq!(unit.functions().count(), 1);
}

/// Multiple sources assemble into one unit, which is the arrangement the decorator produces.
#[test]
fn sources_assemble_into_one_unit_across_files() {
    let frontend = frontend::lookup("python").unwrap();
    let unit = frontend
        .lower(&[
            py_source("def double(n: int) -> int:\n    return n * 2\n"),
            py_source("def quadruple(n: int) -> int:\n    return double(double(n))\n"),
        ])
        .expect("a call across sources must type");
    assert_eq!(unit.functions().count(), 2);
}

#[test]
fn a_syntax_failure_and_a_subset_rejection_are_different_kinds() {
    let frontend = frontend::lookup("python").unwrap();
    let syntax = frontend
        .lower(&[py_source("def broken(:\n")])
        .expect_err("must fail");
    let unsupported = frontend
        .lower(&[py_source("def f(a):\n    return a\n")])
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
        .lower(&[py_source("def f(a: int) -> int:\n    return a + \"x\"\n")])
        .expect_err("must fail");
    assert_eq!(error.line(), 2);
    assert!(error.column() > 1, "{error}");
    assert!(error.to_string().starts_with("2:"), "{error}");
}

/// The frontend declares what its source language needs preserved on the way to a target.
#[test]
fn the_python_frontend_declares_what_it_requires() {
    use compylr_ir::Guarantee;
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

/// Each language declares its own stance, on every axis, and mentions no other language.
///
/// The two declarations are the whole of the N + M property: a third language costs one
/// declaration and no edit to any existing one. Nothing but a test notices when a declaration
/// starts reaching for a name it should not know.
mod declared_stances {
    use compylr_core::{Axis, Behavior};
    use compylr_ir::{Checked, IndexOrigin, RemSign, Rounding, TextUnits};

    use super::*;

    #[test]
    fn both_endpoints_answer_for_every_axis() {
        let source = frontend::lookup("python").unwrap().behavior();
        let target = compylr_registry::backends::lookup("rust")
            .unwrap()
            .behavior();

        // `stance` is an exhaustive match, so an axis with no field cannot compile. What this
        // adds is that *both* sides answer: a component somehow given a partial bundle would
        // fail here rather than at the first program that used the missing axis.
        for axis in Axis::ALL {
            assert_eq!(source.stance(axis).axis(), axis);
            assert_eq!(target.stance(axis).axis(), axis);
        }
    }

    /// Python's stance, axis by axis, spelled out rather than compared against another bundle.
    ///
    /// Written as literals on purpose. Comparing the declaration against something derived from
    /// the same declaration would pass however wrong both were; these are the answers a Python
    /// programmer would give, checked against the answers the frontend gives.
    #[test]
    fn python_declares_pythons_meanings() {
        let python = frontend::lookup("python").unwrap().behavior();

        assert_eq!(python.integer_overflow, Checked::Reported);
        assert_eq!(python.integer_division.rounding, Rounding::TowardNegInf);
        assert_eq!(python.integer_division.checked, Checked::Reported);
        assert_eq!(python.exact_division, Checked::Reported);
        assert_eq!(python.remainder.sign, RemSign::Divisor);
        assert_eq!(python.remainder.checked, Checked::Reported);
        assert_eq!(python.sequence_index.origin, IndexOrigin::FromEitherEnd);
        assert_eq!(python.sequence_index.checked, Checked::Reported);
        assert_eq!(python.text_length, TextUnits::CodePoints);
    }

    /// Rust's stance, likewise.
    #[test]
    fn rust_declares_rusts_meanings() {
        let rust = compylr_registry::backends::lookup("rust")
            .unwrap()
            .behavior();

        assert_eq!(rust.integer_overflow, Checked::Unchecked);
        assert_eq!(rust.integer_division.rounding, Rounding::TowardZero);
        assert_eq!(rust.integer_division.checked, Checked::Unchecked);
        assert_eq!(rust.exact_division, Checked::Unchecked);
        assert_eq!(rust.remainder.sign, RemSign::Dividend);
        assert_eq!(rust.remainder.checked, Checked::Unchecked);
        assert_eq!(rust.sequence_index.origin, IndexOrigin::FromStart);
        assert_eq!(rust.sequence_index.checked, Checked::Unchecked);
        assert_eq!(rust.text_length, TextUnits::Utf8Bytes);
    }

    /// The two disagree on every axis, which is what makes all six worth having.
    ///
    /// An axis the two languages agreed on would be a setting with one value — not a choice, and
    /// not something a user could meaningfully ask for.
    #[test]
    fn the_pair_compylr_ships_disagrees_on_every_axis() {
        let python = frontend::lookup("python").unwrap().behavior();
        let rust = compylr_registry::backends::lookup("rust")
            .unwrap()
            .behavior();

        for axis in Axis::ALL {
            assert_ne!(
                python.stance(axis),
                rust.stance(axis),
                "{axis} is declared identically by both languages, so it is not a choice"
            );
        }
    }

    /// A declaration describes one language, so resolving to it produces exactly it.
    ///
    /// The property that would break first if a declaration ever started hedging toward the
    /// other language: resolving every axis to Python must reproduce Python's bundle unchanged.
    #[test]
    fn resolving_to_one_language_reproduces_its_declaration() {
        let python = frontend::lookup("python").unwrap().behavior();
        let rust = compylr_registry::backends::lookup("rust")
            .unwrap()
            .behavior();

        assert_eq!(Behavior::of(python).axes(), python);
        assert_eq!(Behavior::of(rust).axes(), rust);
    }
}

/// The Python frontend declares Python's meanings on every operator it lowers.
///
/// Asserted on the *declaration*, not on the variant name. A test that checked for a variant
/// called `FloorDiv` would pass whatever that variant happened to mean, which is the failure
/// mode the change exists to remove.
mod declared_meanings {
    use super::*;
    use compylr_ir::{BinOp, Checked, DivMode, Expr, RemSign, Rounding, Stmt};

    fn operator_of(source: &str) -> BinOp {
        let frontend = frontend::lookup("python").unwrap();
        let unit = frontend.lower(&[py_source(source)]).expect("must lower");
        match &unit.get("op").expect("the fixture defines op").body[0] {
            Stmt::Return(Expr::Binary { op, .. }) => *op,
            other => panic!("unexpected body: {other:?}"),
        }
    }

    #[test]
    fn floor_division_declares_rounding_toward_negative_infinity() {
        assert_eq!(
            operator_of("def op(a: int, b: int) -> int:\n    return a // b\n"),
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
                checked: Checked::Reported,
            }
        );
    }

    #[test]
    fn remainder_declares_the_sign_of_the_divisor() {
        assert_eq!(
            operator_of("def op(a: int, b: int) -> int:\n    return a % b\n"),
            BinOp::Rem {
                sign: RemSign::Divisor,
                checked: Checked::Reported,
            }
        );
    }

    #[test]
    fn true_division_declares_exact_division() {
        assert_eq!(
            operator_of("def op(a: int, b: int) -> float:\n    return a / b\n"),
            BinOp::Div {
                mode: DivMode::Exact,
                checked: Checked::Reported,
            }
        );
    }

    /// Container operations declare Python's readings too.
    ///
    /// Asserted on the declaration rather than the variant, for the same reason the arithmetic
    /// ones are: a test that checked for a variant named `Subscript` would pass whatever that
    /// variant happened to mean.
    #[test]
    fn subscripting_declares_counting_from_either_end() {
        use compylr_ir::IndexOrigin;
        let frontend = frontend::lookup("python").unwrap();
        let unit = frontend
            .lower(&[py_source(
                "def op(xs: list[int], i: int) -> int:\n    return xs[i]\n",
            )])
            .expect("must lower");
        match &unit.get("op").unwrap().body[0] {
            Stmt::Return(Expr::Subscript { origin, .. }) => {
                assert_eq!(*origin, IndexOrigin::FromEitherEnd);
            }
            other => panic!("unexpected body: {other:?}"),
        }
    }

    #[test]
    fn length_declares_code_points() {
        use compylr_ir::TextUnits;
        let frontend = frontend::lookup("python").unwrap();
        let unit = frontend
            .lower(&[py_source("def op(s: str) -> int:\n    return len(s)\n")])
            .expect("must lower");
        match &unit.get("op").unwrap().body[0] {
            Stmt::Return(Expr::Len { units, .. }) => assert_eq!(*units, TextUnits::CodePoints),
            other => panic!("unexpected body: {other:?}"),
        }
    }

    /// A mapping read declares an origin too, and it means nothing there.
    ///
    /// Worth pinning: the field is inert for a mapping, and a frontend that left it at some other
    /// value would be relying on nobody reading it. Declaring one reading everywhere is what makes
    /// that safe.
    #[test]
    fn a_mapping_read_still_declares_one_origin() {
        use compylr_ir::IndexOrigin;
        let frontend = frontend::lookup("python").unwrap();
        let unit = frontend
            .lower(&[py_source(
                "def op(d: dict[str, int], k: str) -> int:\n    return d[k]\n",
            )])
            .expect("must lower");
        match &unit.get("op").unwrap().body[0] {
            Stmt::Return(Expr::Subscript { origin, .. }) => {
                assert_eq!(*origin, IndexOrigin::FromEitherEnd);
            }
            other => panic!("unexpected body: {other:?}"),
        }
    }

    /// A lowered unit says which frontend produced it and what that language needs preserved.
    #[test]
    fn a_lowered_unit_records_its_origin() {
        use compylr_ir::Guarantee;
        let frontend = frontend::lookup("python").unwrap();
        let unit = frontend
            .lower(&[py_source(
                "def op(a: int, b: int) -> int:\n    return a + b\n",
            )])
            .unwrap();

        let origin = unit.origin().expect("a lowered unit is claimed");
        assert_eq!(origin.frontend, "python");
        assert!(
            unit.requires()
                .contains(&Guarantee::IntegerOverflowReported)
        );
    }
}
