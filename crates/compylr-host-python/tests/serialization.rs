//! The IR's on-disk artifact form.
//!
//! An artifact is only useful if it says the same thing every time and loses nothing that
//! matters. Two properties carry that, and both are easy to break by accident:
//!
//! * **Determinism.** The same unit must serialize byte-identically regardless of the order
//!   functions were added or how the source was formatted. The formatting test is the one that
//!   fails the moment spans leak into the artifact, since a span is a byte offset and comments
//!   move byte offsets.
//! * **Fidelity.** Every construct survives a round trip, and float literals survive *bit*-exactly
//!   rather than approximately — `-0.0` and `0.0` are different literals.

use compylr_diagnostics::span::Span;
use compylr_frontend_python::frontend::parse_source;
use compylr_frontend_python::lower::lower_source;
use compylr_ir::{BinOp, Checked, DivMode, Expr, Function, Literal, Param, Stmt, Ty, Unit};

/// Lower source text into a unit, panicking with the diagnostic if it does not lower.
fn unit_from(source: &str) -> Unit {
    let parsed = parse_source(source).expect("fixture must parse");
    let functions = lower_source(&parsed, python_stance())
        .unwrap_or_else(|e| panic!("should lower: {}", e.render(source)));
    let mut unit = Unit::new();
    for function in functions {
        unit.add_function(function).expect("names must be unique");
    }
    unit
}

/// Spans are diagnostic-only and deliberately absent from the artifact, so comparing a
/// round-tripped unit against its original means comparing everything *except* spans — which is
/// exactly the IR's own definition of structure.
fn without_spans(unit: &Unit) -> Vec<Function> {
    unit.functions()
        .map(|f| Function {
            doc: None,
            span: Span::default(),
            ..f.clone()
        })
        .collect()
}

/// A unit exercising every type, statement form, and expression form in one place.
fn every_construct() -> Unit {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "helper".into(),
        params: vec![Param {
            name: "n".into(),
            ty: Ty::Int,
        }],
        ret: Ty::Int,
        body: vec![Stmt::Return(Expr::name("n"))],
        doc: None,
        span: Span::default(),
    })
    .unwrap();
    unit.add_function(Function {
        name: "everything".into(),
        params: vec![
            Param {
                name: "i".into(),
                ty: Ty::Int,
            },
            Param {
                name: "f".into(),
                ty: Ty::Float,
            },
            Param {
                name: "b".into(),
                ty: Ty::Bool,
            },
            Param {
                name: "s".into(),
                ty: Ty::Str,
            },
        ],
        ret: Ty::Float,
        body: vec![
            Stmt::Bind {
                name: "an_int".into(),
                ty: Ty::Int,
                value: Expr::int(3),
            },
            Stmt::Bind {
                name: "a_float".into(),
                ty: Ty::Float,
                value: Expr::float(1.3),
            },
            Stmt::Bind {
                name: "a_bool".into(),
                ty: Ty::Bool,
                value: Expr::bool(true),
            },
            Stmt::Bind {
                name: "a_str".into(),
                ty: Ty::Str,
                value: Expr::string("x"),
            },
            Stmt::Bind {
                name: "negated".into(),
                ty: Ty::Int,
                value: Expr::Neg {
                    value: Box::new(Expr::name("i")),
                    checked: Checked::Reported,
                },
            },
            Stmt::Bind {
                name: "promoted".into(),
                ty: Ty::Float,
                value: Expr::to_float(Expr::name("i")),
            },
            Stmt::Bind {
                name: "called".into(),
                ty: Ty::Int,
                value: Expr::Call {
                    callee: "helper".into(),
                    args: vec![Expr::name("i")],
                },
            },
            Stmt::Return(Expr::binary(
                BinOp::Div {
                    mode: DivMode::Exact,
                    checked: Checked::Reported,
                },
                Expr::to_float(Expr::name("i")),
                Expr::name("f"),
            )),
        ],
        doc: None,
        span: Span::default(),
    })
    .unwrap();
    unit.add_function(Function {
        name: "nothing".into(),
        params: Vec::new(),
        ret: Ty::Unit,
        body: vec![Stmt::ReturnUnit],
        doc: None,
        span: Span::default(),
    })
    .unwrap();
    unit
}

#[test]
fn a_unit_is_written_and_read_back() {
    let unit = every_construct();
    let json = unit.to_json().expect("unit must serialize");
    let restored = Unit::from_json(&json).expect("artifact must deserialize");

    assert_eq!(
        without_spans(&unit),
        without_spans(&restored),
        "round trip must preserve every construct"
    );
}

#[test]
fn the_artifact_describes_every_construct() {
    let unit = every_construct();
    let json = unit.to_json().unwrap();

    // Each of these appears only if its construct made it into the artifact.
    for construct in [
        "Int",
        "Float",
        "Bool",
        "Str",
        "Unit",
        "Literal",
        "Name",
        "Neg",
        "ToFloat",
        "Binary",
        "Call",
        "Return",
        "ReturnUnit",
        "Bind",
        "Div",
    ] {
        assert!(
            json.contains(construct),
            "artifact does not represent `{construct}`"
        );
    }

    let restored = Unit::from_json(&json).unwrap();
    assert_eq!(restored.len(), 3);
}

#[test]
fn fingerprint_survives_a_round_trip() {
    let unit = every_construct();
    let restored = Unit::from_json(&unit.to_json().unwrap()).unwrap();
    assert_eq!(
        unit.fingerprint(),
        restored.fingerprint(),
        "a fingerprint computed over structure must not change across a round trip"
    );
}

#[test]
fn float_literals_survive_bit_exactly() {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "floats".into(),
        params: Vec::new(),
        ret: Ty::Float,
        body: vec![
            Stmt::Bind {
                name: "positive_zero".into(),
                ty: Ty::Float,
                value: Expr::float(0.0),
            },
            Stmt::Bind {
                name: "negative_zero".into(),
                ty: Ty::Float,
                value: Expr::float(-0.0),
            },
            Stmt::Bind {
                name: "third".into(),
                ty: Ty::Float,
                value: Expr::float(1.0 / 3.0),
            },
            Stmt::Return(Expr::float(f64::MAX)),
        ],
        doc: None,
        span: Span::default(),
    })
    .unwrap();

    let restored = Unit::from_json(&unit.to_json().unwrap()).unwrap();
    let function = restored.get("floats").expect("function must survive");

    let literal = |index: usize| match &function.body[index] {
        Stmt::Bind { value, .. } => match value {
            Expr::Literal(literal) => literal.clone(),
            other => panic!("expected a literal, found {other:?}"),
        },
        other => panic!("expected a binding, found {other:?}"),
    };

    // `0.0 == -0.0` in IEEE-754, so equality alone would not catch a lost sign bit. The
    // literals must differ, which is only true if the bit pattern round-tripped.
    let positive = literal(0);
    let negative = literal(1);
    assert_ne!(
        positive, negative,
        "negative zero must stay distinguishable from positive zero"
    );
    assert!(positive.as_f64().unwrap().is_sign_positive());
    assert!(negative.as_f64().unwrap().is_sign_negative());

    assert_eq!(literal(2), Literal::float(1.0 / 3.0));
    match &function.body[3] {
        Stmt::Return(Expr::Literal(literal)) => assert_eq!(literal.as_f64().unwrap(), f64::MAX),
        other => panic!("expected a return of a literal, found {other:?}"),
    }
}

#[test]
fn the_artifact_carries_no_target_language_information() {
    let unit = every_construct();
    let json = unit.to_json().unwrap();

    for spelling in ["i64", "f64", "String", "usize", "&str", "Vec<"] {
        assert!(
            !json.contains(spelling),
            "artifact leaks the Rust spelling `{spelling}`; the IR must stay target-neutral"
        );
    }
}

#[test]
fn repeated_serialization_is_byte_identical() {
    let unit = every_construct();
    assert_eq!(unit.to_json().unwrap(), unit.to_json().unwrap());
}

#[test]
fn addition_order_does_not_affect_the_artifact() {
    let source = concat!(
        "def alpha(a: int) -> int:\n    return a + 1\n\n",
        "def beta(b: int) -> int:\n    return b * 2\n\n",
        "def gamma(c: int) -> int:\n    return c - 3\n",
    );
    let parsed = parse_source(source).unwrap();
    let functions = lower_source(&parsed, python_stance()).unwrap();

    let build = |reverse: bool| {
        let mut functions = functions.clone();
        if reverse {
            functions.reverse();
        }
        let mut unit = Unit::new();
        for function in functions {
            unit.add_function(function).unwrap();
        }
        unit.to_json().unwrap()
    };

    assert_eq!(build(false), build(true));
}

#[test]
fn formatting_changes_do_not_affect_the_artifact() {
    // This is the test that fails if `Span` is serialized: comments and indentation move byte
    // offsets without changing meaning.
    let plain = "def add(a: int, b: int) -> int:\n    return a + b\n";
    let noisy = concat!(
        "# a leading comment\n",
        "def add(a: int, b: int) -> int:\n",
        "\n",
        "        # an indented comment\n",
        "        return a + b\n",
    );

    assert_eq!(
        unit_from(plain).to_json().unwrap(),
        unit_from(noisy).to_json().unwrap(),
        "an artifact must describe meaning, not source layout"
    );
}

#[test]
fn a_changed_body_changes_the_artifact() {
    // The mirror of the test above: determinism must not be achieved by discarding detail.
    let before = unit_from("def add(a: int, b: int) -> int:\n    return a + b\n");
    let after = unit_from("def add(a: int, b: int) -> int:\n    return a - b\n");
    assert_ne!(before.to_json().unwrap(), after.to_json().unwrap());
}

#[test]
fn an_empty_unit_round_trips() {
    let unit = Unit::new();
    let restored = Unit::from_json(&unit.to_json().unwrap()).unwrap();
    assert!(restored.is_empty());
    assert_eq!(unit.fingerprint(), restored.fingerprint());
}

#[test]
fn a_corrupted_artifact_is_rejected() {
    // The artifact records the fingerprint it was written with. Recomputing it on load turns a
    // truncated or hand-edited file into an error rather than a silently wrong unit.
    let unit = every_construct();
    let json = unit.to_json().unwrap();
    let tampered = json.replace("\"helper\"", "\"helperr\"");
    assert_ne!(json, tampered, "the test must actually modify the artifact");

    assert!(
        Unit::from_json(&tampered).is_err(),
        "an artifact whose contents disagree with its recorded fingerprint must be rejected"
    );
}

#[test]
fn malformed_json_is_rejected() {
    assert!(Unit::from_json("{not json").is_err());
}

/// An artifact written before checking modes existed is refused rather than reinterpreted.
///
/// No reader for the previous version is kept, and that is the decision rather than an omission:
/// the only thing a version 3 artifact could mean is "every failure reported", and a migration
/// asserting that would be more code than the single rebuild it saves. What matters is that the
/// refusal *says both numbers* — a user whose first run after upgrading is slow deserves to be
/// told why, and "unsupported version" alone does not.
#[test]
fn an_artifact_from_the_previous_version_is_refused_naming_both_versions() {
    let json = every_construct().to_json().unwrap();
    assert!(
        json.contains("\"version\": 4"),
        "the current artifact must be version 4; found: {}",
        json.lines().take(3).collect::<Vec<_>>().join(" ")
    );

    let previous = json.replace("\"version\": 4", "\"version\": 3");
    let error = Unit::from_json(&previous).expect_err("a version 3 artifact must be refused");

    let message = error.to_string();
    assert!(
        message.contains('3') && message.contains('4'),
        "the refusal must name the version found and the version expected; got: {message}"
    );
}

/// Two divisions that differ only in declared rounding are two different programs.
///
/// This is the whole change reduced to one assertion. Before, `//` *was* flooring and there was
/// nowhere to say otherwise; a frontend that meant truncation had no way to express it and would
/// have been silently given Python's answer. If these two units ever fingerprint alike, the mode
/// has stopped being part of the program and a build cache will hand back the wrong one.
mod declared_semantics {
    use super::*;
    use compylr_ir::{DivMode, IndexOrigin, RemSign, Rounding, TextUnits};

    fn unit_dividing(op: BinOp) -> Unit {
        let mut unit = Unit::new();
        unit.add_function(Function {
            name: "op".to_string(),
            params: vec![
                Param {
                    name: "a".to_string(),
                    ty: Ty::Int,
                },
                Param {
                    name: "b".to_string(),
                    ty: Ty::Int,
                },
            ],
            ret: Ty::Int,
            body: vec![Stmt::Return(Expr::Binary {
                op,
                left: Box::new(Expr::name("a")),
                right: Box::new(Expr::name("b")),
            })],
            doc: None,
            span: Span::default(),
        })
        .unwrap();
        unit
    }

    #[test]
    fn rounding_modes_fingerprint_differently() {
        let flooring = unit_dividing(BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardNegInf),
            checked: Checked::Reported,
        });
        let truncating = unit_dividing(BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardZero),
            checked: Checked::Reported,
        });
        assert_ne!(
            flooring.fingerprint(),
            truncating.fingerprint(),
            "the mode is part of what the program computes, so it must reach the rebuild key"
        );
    }

    #[test]
    fn remainder_conventions_fingerprint_differently() {
        let divisor = unit_dividing(BinOp::Rem {
            sign: RemSign::Divisor,
            checked: Checked::Reported,
        });
        let dividend = unit_dividing(BinOp::Rem {
            sign: RemSign::Dividend,
            checked: Checked::Reported,
        });
        assert_ne!(divisor.fingerprint(), dividend.fingerprint());
    }

    #[test]
    fn a_declared_mode_survives_the_artifact() {
        for op in [
            BinOp::Div {
                mode: DivMode::Exact,
                checked: Checked::Reported,
            },
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
                checked: Checked::Reported,
            },
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardZero),
                checked: Checked::Reported,
            },
            BinOp::Rem {
                sign: RemSign::Divisor,
                checked: Checked::Reported,
            },
            BinOp::Rem {
                sign: RemSign::Dividend,
                checked: Checked::Reported,
            },
        ] {
            let unit = unit_dividing(op);
            let restored = Unit::from_json(&unit.to_json().unwrap()).expect("round trip");
            match &restored.get("op").unwrap().body[0] {
                Stmt::Return(Expr::Binary { op: restored, .. }) => assert_eq!(*restored, op),
                other => panic!("unexpected body: {other:?}"),
            }
        }
    }

    /// The producing frontend and its requirements survive too.
    ///
    /// An artifact read back from disk has no frontend to ask, and the requirements are what a
    /// backend checks before it is allowed to optimize. Losing them on the way through the file
    /// would mean a cached build silently escaped the check the fresh one passed.
    #[test]
    fn the_origin_survives_the_artifact() {
        use compylr_ir::Guarantee;
        // A reported remainder, so the unit derives a division-by-zero requirement to carry.
        let mut unit = unit_dividing(BinOp::Rem {
            sign: RemSign::Divisor,
            checked: Checked::Reported,
        });
        unit.set_origin("python");
        assert!(unit.requires().contains(&Guarantee::DivisionByZeroReported));

        let restored = Unit::from_json(&unit.to_json().unwrap()).expect("round trip");
        assert_eq!(
            restored.origin().map(|o| o.frontend.as_str()),
            Some("python")
        );
        assert_eq!(restored.requires(), unit.requires());
        assert_eq!(restored.fingerprint(), unit.fingerprint());
    }

    fn unit_indexing(origin: IndexOrigin) -> Unit {
        let mut unit = Unit::new();
        unit.add_function(Function {
            name: "read".to_string(),
            params: vec![
                Param {
                    name: "xs".to_string(),
                    ty: Ty::List(Box::new(Ty::Int)),
                },
                Param {
                    name: "i".to_string(),
                    ty: Ty::Int,
                },
            ],
            ret: Ty::Int,
            body: vec![Stmt::Return(Expr::Subscript {
                base: Box::new(Expr::name("xs")),
                index: Box::new(Expr::name("i")),
                origin,
                checked: Checked::Reported,
            })],
            doc: None,
            span: Span::default(),
        })
        .unwrap();
        unit
    }

    fn unit_measuring(units: TextUnits) -> Unit {
        let mut unit = Unit::new();
        unit.add_function(Function {
            name: "size".to_string(),
            params: vec![Param {
                name: "s".to_string(),
                ty: Ty::Str,
            }],
            ret: Ty::Int,
            body: vec![Stmt::Return(Expr::Len {
                value: Box::new(Expr::name("s")),
                units,
            })],
            doc: None,
            span: Span::default(),
        })
        .unwrap();
        unit
    }

    /// Two programs that index differently are two different programs.
    #[test]
    fn index_origins_fingerprint_differently() {
        assert_ne!(
            unit_indexing(IndexOrigin::FromEitherEnd).fingerprint(),
            unit_indexing(IndexOrigin::FromStart).fingerprint(),
            "the origin decides what `xs[-1]` returns, so it must reach the rebuild key"
        );
    }

    #[test]
    fn text_units_fingerprint_differently() {
        let prints = [
            unit_measuring(TextUnits::CodePoints).fingerprint(),
            unit_measuring(TextUnits::Utf8Bytes).fingerprint(),
            unit_measuring(TextUnits::Utf16Units).fingerprint(),
        ];
        let mut sorted = prints;
        sorted.sort_unstable();
        let count = sorted.len();
        let mut deduped = sorted.to_vec();
        deduped.dedup();
        assert_eq!(deduped.len(), count, "all three readings must be distinct");
    }

    #[test]
    fn a_declared_container_mode_survives_the_artifact() {
        for origin in [IndexOrigin::FromEitherEnd, IndexOrigin::FromStart] {
            let unit = unit_indexing(origin);
            let restored = Unit::from_json(&unit.to_json().unwrap()).expect("round trip");
            match &restored.get("read").unwrap().body[0] {
                Stmt::Return(Expr::Subscript { origin: back, .. }) => assert_eq!(*back, origin),
                other => panic!("unexpected body: {other:?}"),
            }
        }

        for units in [
            TextUnits::CodePoints,
            TextUnits::Utf8Bytes,
            TextUnits::Utf16Units,
        ] {
            let unit = unit_measuring(units);
            let restored = Unit::from_json(&unit.to_json().unwrap()).expect("round trip");
            match &restored.get("size").unwrap().body[0] {
                Stmt::Return(Expr::Len { units: back, .. }) => assert_eq!(*back, units),
                other => panic!("unexpected body: {other:?}"),
            }
        }
    }

    /// A unit nobody claimed fingerprints as it always did.
    ///
    /// Hand-built units exist — test fixtures, and the backend conformance corpus. Making the
    /// origin mandatory would have forced each of them to invent a source language they do not
    /// have.
    #[test]
    fn an_unclaimed_unit_carries_no_origin() {
        let unit = unit_dividing(BinOp::Add {
            checked: Checked::Reported,
        });
        assert!(unit.origin().is_none());
        assert!(unit.requires().is_empty());
        assert!(!unit.to_json().unwrap().contains("origin"));
    }

    /// Whether the program defines a failure is part of what the program computes.
    ///
    /// The same shape the rounding mode already had, and here for the same reason: two units that
    /// disagree about what `a + b` does on overflow are two different programs, and a rebuild key
    /// that could not tell them apart would hand back the wrong artifact.
    mod checking_mode {
        use super::*;

        /// Every operation that can fail, in both modes.
        fn both_modes(build: impl Fn(Checked) -> BinOp) -> (Unit, Unit) {
            (
                unit_dividing(build(Checked::Reported)),
                unit_dividing(build(Checked::Unchecked)),
            )
        }

        #[test]
        fn the_mode_is_readable_off_every_operation_that_can_fail() {
            let operations: Vec<BinOp> = vec![
                BinOp::Add {
                    checked: Checked::Unchecked,
                },
                BinOp::Sub {
                    checked: Checked::Unchecked,
                },
                BinOp::Mul {
                    checked: Checked::Unchecked,
                },
                BinOp::Div {
                    mode: DivMode::Integer(Rounding::TowardZero),
                    checked: Checked::Unchecked,
                },
                BinOp::Rem {
                    sign: RemSign::Dividend,
                    checked: Checked::Unchecked,
                },
            ];
            for op in operations {
                let read = match op {
                    BinOp::Add { checked }
                    | BinOp::Sub { checked }
                    | BinOp::Mul { checked }
                    | BinOp::Div { checked, .. }
                    | BinOp::Rem { checked, .. } => checked,
                    other => panic!("{other:?} cannot fail"),
                };
                assert_eq!(read, Checked::Unchecked);
            }

            // Negation and subscripting carry it too, on their own forms.
            let negation = Expr::Neg {
                value: Box::new(Expr::name("a")),
                checked: Checked::Unchecked,
            };
            let Expr::Neg { checked, .. } = negation else {
                unreachable!()
            };
            assert_eq!(checked, Checked::Unchecked);

            let read = Expr::Subscript {
                base: Box::new(Expr::name("xs")),
                index: Box::new(Expr::name("i")),
                origin: compylr_ir::IndexOrigin::FromStart,
                checked: Checked::Unchecked,
            };
            let Expr::Subscript { checked, .. } = read else {
                unreachable!()
            };
            assert_eq!(checked, Checked::Unchecked);
        }

        #[test]
        fn two_nodes_differing_only_in_the_mode_are_distinguishable() {
            let (reported, unchecked) = both_modes(|checked| BinOp::Add { checked });
            assert_ne!(
                reported.functions().next().unwrap().body,
                unchecked.functions().next().unwrap().body
            );
        }

        /// The mode composes with the modes already on a node rather than replacing them.
        ///
        /// `Div { mode: Integer(TowardNegInf), checked: Unchecked }` is a real combination — a
        /// flooring division whose zero divisor is undefined — and it must be distinct from all
        /// three of its neighbours, or one of the two axes has collapsed into the other.
        #[test]
        fn the_mode_is_independent_of_the_rounding_and_the_sign() {
            let mut seen = std::collections::HashSet::new();
            for rounding in [Rounding::TowardNegInf, Rounding::TowardZero] {
                for checked in [Checked::Reported, Checked::Unchecked] {
                    let unit = unit_dividing(BinOp::Div {
                        mode: DivMode::Integer(rounding),
                        checked,
                    });
                    assert!(
                        seen.insert(unit.fingerprint()),
                        "{rounding:?} with {checked:?} collided with an earlier combination"
                    );
                }
            }
            assert_eq!(seen.len(), 4, "two independent axes give four combinations");

            let mut seen = std::collections::HashSet::new();
            for sign in [RemSign::Divisor, RemSign::Dividend] {
                for checked in [Checked::Reported, Checked::Unchecked] {
                    let unit = unit_dividing(BinOp::Rem { sign, checked });
                    assert!(seen.insert(unit.fingerprint()));
                }
            }
            assert_eq!(seen.len(), 4);
        }

        #[test]
        fn a_declared_mode_survives_the_artifact() {
            for op in [
                BinOp::Add {
                    checked: Checked::Unchecked,
                },
                BinOp::Div {
                    mode: DivMode::Integer(Rounding::TowardNegInf),
                    checked: Checked::Unchecked,
                },
                BinOp::Rem {
                    sign: RemSign::Divisor,
                    checked: Checked::Unchecked,
                },
            ] {
                let unit = unit_dividing(op);
                let restored = Unit::from_json(&unit.to_json().unwrap()).expect("round trip");
                match &restored.get("op").unwrap().body[0] {
                    Stmt::Return(Expr::Binary { op: back, .. }) => assert_eq!(*back, op),
                    other => panic!("unexpected body: {other:?}"),
                }
            }
        }

        #[test]
        fn two_units_differing_only_in_the_mode_fingerprint_differently() {
            let (reported, unchecked) = both_modes(|checked| BinOp::Add { checked });
            assert_ne!(
                reported.fingerprint(),
                unchecked.fingerprint(),
                "the mode is part of what the program computes, so it must reach the rebuild key"
            );
        }
    }
}

/// Python's own stance, which is what an unconfigured compilation resolves to.
///
/// Read from the frontend's declaration rather than rebuilt here, so these tests lower under the
/// same bundle the pipeline uses.
fn python_stance() -> compylr_ir::Behavior {
    compylr_ir::Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR)
}
