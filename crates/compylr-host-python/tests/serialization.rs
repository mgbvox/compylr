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
use compylr_ir::{BinOp, DivMode, Expr, Function, Literal, Param, Stmt, Ty, Unit};

/// Lower source text into a unit, panicking with the diagnostic if it does not lower.
fn unit_from(source: &str) -> Unit {
    let parsed = parse_source(source).expect("fixture must parse");
    let functions =
        lower_source(&parsed).unwrap_or_else(|e| panic!("should lower: {}", e.render(source)));
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
                value: Expr::Neg(Box::new(Expr::name("i"))),
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
    let functions = lower_source(&parsed).unwrap();

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
        });
        let truncating = unit_dividing(BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardZero),
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
        });
        let dividend = unit_dividing(BinOp::Rem {
            sign: RemSign::Dividend,
        });
        assert_ne!(divisor.fingerprint(), dividend.fingerprint());
    }

    #[test]
    fn a_declared_mode_survives_the_artifact() {
        for op in [
            BinOp::Div {
                mode: DivMode::Exact,
            },
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
            },
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardZero),
            },
            BinOp::Rem {
                sign: RemSign::Divisor,
            },
            BinOp::Rem {
                sign: RemSign::Dividend,
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
        let mut unit = unit_dividing(BinOp::Rem {
            sign: RemSign::Divisor,
        });
        unit.set_origin("python", &[Guarantee::IntegerOverflowReported]);

        let restored = Unit::from_json(&unit.to_json().unwrap()).expect("round trip");
        assert_eq!(
            restored.origin().map(|o| o.frontend.as_str()),
            Some("python")
        );
        assert_eq!(restored.requires(), [Guarantee::IntegerOverflowReported]);
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
        let unit = unit_dividing(BinOp::Add);
        assert!(unit.origin().is_none());
        assert!(unit.requires().is_empty());
        assert!(!unit.to_json().unwrap().contains("origin"));
    }
}
