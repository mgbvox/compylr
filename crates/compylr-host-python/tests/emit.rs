//! Shape of the emitted Rust.
//!
//! These tests read the emitted *text*. That is enough for structural questions — does a `str`
//! parameter become a `String`, do parameters keep their order — but deliberately not enough for
//! semantic ones: a helper can look right and compute the wrong thing. Semantics are covered in
//! `tests/execution.rs`, which compiles and runs what is emitted.

use compylr_diagnostics::span::Span;
use compylr_frontend_python::frontend::parse_source;
use compylr_frontend_python::lower::lower_source;
use compylr_ir::{BinOp, DivMode, Expr, Function, Param, Stmt, Ty, Unit};
use compylr_registry::backends::lookup;

/// Lower source into a unit and return its translated functions.
fn emit(source: &str) -> String {
    functions_of(&unit_from(source))
}

/// The translated functions of a unit.
///
/// A lookup rather than a search: the backend emits a file per concern, so the file holding the
/// functions is simply asked for by name.
fn functions_of(unit: &Unit) -> String {
    lookup("rust")
        .unwrap()
        .emit(unit)
        .expect("must emit")
        .remove("src/generated.rs")
        .expect("a translated-code file must be emitted")
}

fn unit_from(source: &str) -> Unit {
    let parsed = parse_source(source).expect("fixture must parse");
    let functions =
        lower_source(&parsed).unwrap_or_else(|e| panic!("should lower: {}", e.render(source)));
    let mut unit = Unit::new();
    for function in functions {
        unit.add_function(function).unwrap();
    }
    unit
}

/// Identity, kept so call sites read the same as before the crate was split.
fn functions_only(emitted: &str) -> String {
    emitted.to_string()
}

#[test]
fn every_type_gets_its_rust_spelling() {
    let emitted = functions_only(&emit(
        "def types(i: int, f: float, b: bool, s: str) -> str:\n    return s\n",
    ));
    for spelling in ["i: i64", "f: f64", "b: bool", "s: String"] {
        assert!(
            emitted.contains(spelling),
            "expected `{spelling}` in:\n{emitted}"
        );
    }
    assert!(emitted.contains("Result<String, RuntimeError>"));
}

#[test]
fn a_unit_returning_function_yields_no_value() {
    let emitted = functions_only(&emit("def nothing(a: int) -> None:\n    pass\n"));
    assert!(
        emitted.contains("Result<(), RuntimeError>"),
        "a unit function must still be able to report a failure:\n{emitted}"
    );
    assert!(emitted.contains("Ok(())"));
}

#[test]
fn a_unit_returning_function_can_still_report_failure() {
    // The case that makes a bare return type wrong: the body can fail even though the function
    // has nothing to return.
    let emitted = functions_only(&emit("def nothing(a: int) -> None:\n    b = a // 0\n"));
    assert!(emitted.contains("Result<(), RuntimeError>"));
    assert!(
        emitted.contains("div_floor"),
        "the failing operation must go through the checked helper:\n{emitted}"
    );
}

#[test]
fn emission_leaves_the_ir_unchanged() {
    let unit = unit_from("def add(a: int, b: int) -> int:\n    return a + b\n");
    let before = unit.to_json().unwrap();
    let _ = lookup("rust").unwrap().emit(&unit).unwrap();
    assert_eq!(
        before,
        unit.to_json().unwrap(),
        "emitting must not mutate the tree it reads"
    );
}

#[test]
fn the_ir_module_names_no_rust_types() {
    // The mapping belongs to the backend. If a Rust spelling appears in the IR, a second
    // backend has already been made harder to write.
    let ir = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("the crate lives at <root>/crates/<name>")
            .join("crates/compylr-ir/src/ir.rs"),
    )
    .unwrap();
    let code: String = ir
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    for spelling in ["f64::to_bits", "i64::MIN"] {
        // These are legitimate: the IR stores an i64 and a float bit pattern. Their presence is
        // about storage, not about a target language's type names.
        let _ = spelling;
    }
    assert!(
        !code.contains("\"i64\"") && !code.contains("\"String\"") && !code.contains("\"f64\""),
        "the IR must not spell target types"
    );
}

#[test]
fn parameters_keep_their_source_order() {
    let emitted = functions_only(&emit(
        "def ordered(first: int, second: float, third: bool) -> int:\n    return first\n",
    ));
    let signature = emitted
        .lines()
        .find(|line| line.contains("fn ordered"))
        .expect("function must be emitted");
    let first = signature.find("first").unwrap();
    let second = signature.find("second").unwrap();
    let third = signature.find("third").unwrap();
    assert!(first < second && second < third, "got: {signature}");
}

#[test]
fn every_function_in_the_unit_is_emitted_in_deterministic_order() {
    let emitted = functions_only(&emit(concat!(
        "def gamma(a: int) -> int:\n    return a\n\n",
        "def alpha(a: int) -> int:\n    return a\n\n",
        "def beta(a: int) -> int:\n    return a\n",
    )));
    let alpha = emitted.find("fn alpha").expect("alpha missing");
    let beta = emitted.find("fn beta").expect("beta missing");
    let gamma = emitted.find("fn gamma").expect("gamma missing");
    assert!(
        alpha < beta && beta < gamma,
        "functions must follow the unit's name order, not source order"
    );
}

#[test]
fn a_local_binding_states_its_type() {
    let emitted = functions_only(&emit(
        "def bind(a: int) -> int:\n    b = a + 1\n    return b\n",
    ));
    assert!(
        emitted.contains("let b: i64 ="),
        "a binding must state its type rather than leaving it to inference:\n{emitted}"
    );
}

#[test]
fn literals_of_every_type_are_emitted() {
    let emitted = functions_only(&emit(concat!(
        "def literals() -> str:\n",
        "    i = 42\n",
        "    f = 1.5\n",
        "    b = True\n",
        "    s = \"hello\"\n",
        "    return s\n",
    )));
    assert!(emitted.contains("42i64"), "integer literal:\n{emitted}");
    assert!(emitted.contains("1.5f64"), "float literal:\n{emitted}");
    assert!(emitted.contains("true"), "bool literal:\n{emitted}");
    assert!(
        emitted.contains("String::from(\"hello\")"),
        "string literal:\n{emitted}"
    );
}

#[test]
fn a_string_literal_needing_escapes_denotes_the_same_characters() {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "tricky".into(),
        params: Vec::new(),
        ret: Ty::Str,
        body: vec![Stmt::Return(Expr::string("a\"b\\c\nd\te"))],
        doc: None,
        span: Span::default(),
    })
    .unwrap();
    let emitted = functions_of(&unit);

    assert!(
        emitted.contains(r#"String::from("a\"b\\c\nd\te")"#),
        "escaping must denote exactly the original characters:\n{}",
        functions_only(&emitted)
    );
}

#[test]
fn negation_and_promotion_are_emitted() {
    let emitted = functions_only(&emit(
        "def mix(a: int, f: float) -> float:\n    return -a + f\n",
    ));
    assert!(emitted.contains("py_neg"), "negation:\n{emitted}");
    assert!(emitted.contains("as f64"), "promotion node:\n{emitted}");
}

#[test]
fn a_call_to_another_function_in_the_unit_is_emitted() {
    let emitted = functions_only(&emit(concat!(
        "def helper(a: int) -> int:\n    return a * 2\n\n",
        "def caller(a: int) -> int:\n    return helper(a)\n",
    )));
    assert!(
        emitted.contains("helper(a)?"),
        "a call must propagate the callee's failure:\n{emitted}"
    );
}

#[test]
fn nesting_is_preserved_regardless_of_rust_precedence() {
    // Python groups `a + b * c` as `a + (b * c)` and `(a + b) * c` differently. Emission must
    // carry that grouping itself rather than relying on Rust's precedence table agreeing, so the
    // two must produce different output.
    let natural = functions_only(&emit(
        "def nested(a: int, b: int, c: int) -> int:\n    return a + b * c\n",
    ));
    let forced = functions_only(&emit(
        "def nested(a: int, b: int, c: int) -> int:\n    return (a + b) * c\n",
    ));
    assert_ne!(
        natural, forced,
        "differently grouped sources must not emit the same Rust"
    );

    // In the natural grouping the addition is outermost; in the forced one the multiplication is.
    assert!(natural.find("py_add").unwrap() < natural.find("py_mul").unwrap());
    assert!(forced.find("py_mul").unwrap() < forced.find("py_add").unwrap());
}

#[test]
fn arithmetic_inside_a_comparison_inside_a_call_argument() {
    let emitted = functions_only(&emit(concat!(
        "def sink(flag: bool) -> int:\n    return 1\n\n",
        "def nested(a: int, b: int, c: int) -> int:\n    return sink(a + b * c > 10)\n",
    )));
    // The multiplication must be bracketed inside the addition, which is inside the comparison,
    // which is inside the call.
    assert!(emitted.contains("sink("), "call:\n{emitted}");
    assert!(emitted.contains("py_mul"), "multiplication:\n{emitted}");
    assert!(emitted.contains("py_add"), "addition:\n{emitted}");
    assert!(emitted.contains(" > "), "comparison:\n{emitted}");

    let add = emitted.find("py_add").unwrap();
    let mul = emitted.find("py_mul").unwrap();
    assert!(
        add < mul,
        "the addition must enclose the multiplication, matching the IR's grouping:\n{emitted}"
    );
}

#[test]
fn exact_division_emits_a_plain_division_because_lowering_already_promoted() {
    // The backend must not re-derive promotion. Lowering hands it
    // `Div{Exact}(ToFloat(a), ToFloat(b))`, so both operands are already `f64`.
    let unit = unit_from("def ratio(a: int, b: int) -> float:\n    return a / b\n");
    let function = unit.get("ratio").unwrap();
    match &function.body[0] {
        Stmt::Return(Expr::Binary { op, left, right }) => {
            assert_eq!(
                *op,
                BinOp::Div {
                    mode: DivMode::Exact
                }
            );
            assert!(
                matches!(**left, Expr::ToFloat(_)) && matches!(**right, Expr::ToFloat(_)),
                "lowering is expected to have wrapped both operands"
            );
        }
        other => panic!("unexpected body: {other:?}"),
    }

    let emitted = functions_only(&functions_of(&unit));
    assert!(emitted.contains("div_exact"), "{emitted}");
    assert_eq!(
        emitted.matches("as f64").count(),
        2,
        "exactly the two promotions lowering inserted, and no extras added by the backend:\n{emitted}"
    );
}

#[test]
fn rust_keywords_are_escaped() {
    // `match`, `type`, and `move` are ordinary Python identifiers.
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "match".into(),
        params: vec![Param {
            name: "type".into(),
            ty: Ty::Int,
        }],
        ret: Ty::Int,
        body: vec![Stmt::Return(Expr::name("type"))],
        doc: None,
        span: Span::default(),
    })
    .unwrap();
    let emitted = functions_only(&functions_of(&unit));
    assert!(
        emitted.contains("r#match") && emitted.contains("r#type"),
        "Python names that collide with Rust keywords must be escaped:\n{emitted}"
    );
}

#[test]
fn emission_is_byte_identical_across_runs_and_addition_orders() {
    let source = concat!(
        "def alpha(a: int) -> int:\n    return a + 1\n\n",
        "def beta(b: int) -> int:\n    return b * 2\n\n",
        "def gamma(c: int) -> int:\n    return c - 3\n",
    );
    let parsed = parse_source(source).unwrap();
    let functions = lower_source(&parsed).unwrap();
    let backend = lookup("rust").unwrap();

    let build = |reverse: bool| {
        let mut functions = functions.clone();
        if reverse {
            functions.reverse();
        }
        let mut unit = Unit::new();
        for function in functions {
            unit.add_function(function).unwrap();
        }
        backend.emit(&unit).unwrap()
    };

    let forward = build(false);
    assert_eq!(forward, build(false), "repeated emission must be identical");
    assert_eq!(
        forward,
        build(true),
        "addition order must not change output"
    );
}

#[test]
fn a_function_that_cannot_return_is_reported_rather_than_emitted_broken() {
    // Lowering should never produce this, but a backend that emitted it anyway would fail with a
    // confusing rustc error instead of naming the problem.
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "broken".into(),
        params: Vec::new(),
        ret: Ty::Int,
        body: vec![Stmt::Bind {
            name: "x".into(),
            ty: Ty::Int,
            value: Expr::int(1),
        }],
        doc: None,
        span: Span::default(),
    })
    .unwrap();

    let error = lookup("rust").unwrap().emit(&unit).unwrap_err();
    assert!(error.to_string().contains("broken"), "got: {error}");
}
