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
    for spelling in ["i: i64", "f: f64", "b: bool", "s: &str"] {
        assert!(
            emitted.contains(spelling),
            "expected `{spelling}` in:\n{emitted}"
        );
    }
    assert!(emitted.contains("Result<String, RuntimeError>"));
}

#[test]
fn a_reassigned_text_parameter_remains_owned() {
    let emitted = emit("def replace(s: str) -> str:\n    s = \"new\"\n    return s\n");
    assert!(emitted.contains("mut s: String"), "{emitted}");
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

/// An accumulator that reads itself updates in place rather than building a fresh value.
///
/// The shape `x = x + y` is what makes string accumulation quadratic: building a new value per
/// iteration copies everything accumulated so far, while CPython resizes in place when the target
/// holds the only reference. The previous emission was therefore asymptotically *worse* than the
/// interpreter it replaces.
///
/// The choice stays type-directed. The backend does not know an expression's type and must not
/// learn it here, so every accumulator emits the same call and the trait's implementations differ
/// per type — exactly as the ordinary addition already does.
mod in_place_accumulation {
    use super::*;

    #[test]
    fn a_string_accumulator_appends_in_place() {
        let emitted = emit(
            "def concat(a: str, b: str) -> str:\n    out = a\n    out = out + b\n    return out\n",
        );
        assert!(
            emitted.contains("py_add_assign"),
            "a str accumulator must update in place:\n{emitted}"
        );
    }

    #[test]
    fn a_numeric_accumulator_uses_the_same_in_place_call() {
        // Same emitted call, different implementation. The backend cannot see the type, so the
        // alternative would be a second type checker in the emitter.
        let emitted = emit(
            "def total(a: int, b: int) -> int:\n    out = a\n    out = out + b\n    return out\n",
        );
        assert!(
            emitted.contains("py_add_assign"),
            "the choice must be type-directed, not made in the emitter:\n{emitted}"
        );
    }

    #[test]
    fn the_mirrored_form_is_left_alone() {
        // `x = y + x` is the one that looks like it should work. Appending `x` onto `y` in place
        // would produce `y + x` written into `x` only if the append target were `y` — for text it
        // silently yields the wrong string, so the rule pins the name to the LEFT operand.
        let emitted = emit(
            "def prefixed(a: str, b: str) -> str:\n    out = a\n    out = b + out\n    return out\n",
        );
        assert!(
            !emitted.contains("py_add_assign"),
            "the mirrored form must use the ordinary emission:\n{emitted}"
        );
        assert!(emitted.contains("py_str_add"), "{emitted}");
    }

    #[test]
    fn a_name_read_on_both_sides_is_left_alone() {
        // `x = x + x` reads the value it would be modifying. The ordinary emission builds the sum
        // from two unmodified reads, which is what Python means.
        let emitted =
            emit("def doubled(a: str) -> str:\n    out = a\n    out = out + out\n    return out\n");
        assert!(
            !emitted.contains("py_add_assign"),
            "a name appearing on the right too must not update in place:\n{emitted}"
        );
    }

    #[test]
    fn a_name_read_deeper_in_the_right_operand_is_left_alone() {
        // The name does not have to be the whole right operand to be read by it.
        let emitted = emit(
            "def grow(a: str, b: str) -> str:\n    out = a\n    out = out + (out + b)\n    return out\n",
        );
        assert!(
            !emitted.contains("py_add_assign"),
            "a nested read of the name must not update in place:\n{emitted}"
        );
    }

    #[test]
    fn only_addition_updates_in_place() {
        // Subtraction has no in-place form here; the rule is about the one operator whose
        // rebuild-per-step is quadratic.
        let emitted = emit(
            "def less(a: int, b: int) -> int:\n    out = a\n    out = out - b\n    return out\n",
        );
        assert!(!emitted.contains("py_add_assign"), "{emitted}");
        assert!(emitted.contains("py_sub"), "{emitted}");
    }

    #[test]
    fn an_assignment_to_a_different_name_is_left_alone() {
        let emitted = emit(
            "def other(a: str, b: str) -> str:\n    out = a\n    tail = out + b\n    return tail\n",
        );
        assert!(
            !emitted.contains("py_add_assign"),
            "only the assigned name may be accumulated into:\n{emitted}"
        );
    }
}

/// A chain of additions accumulates in place too.
///
/// `a + b + c` parses as `(a + b) + c`, so a three-operand accumulation presents its left operand
/// as a `Binary` rather than as the name. Handling only the two-operand pair looked complete and
/// left the demo's `joined` — whose hot line is `out = out + separator + word` — rebuilding the
/// whole accumulated string on every iteration. The pair rule fired on that function exactly
/// once, on the line that runs once.
mod accumulation_over_a_chain {
    use super::*;

    #[test]
    fn a_three_operand_chain_appends_twice() {
        let emitted = emit(concat!(
            "def joined(words: list[str], sep: str) -> str:\n",
            "    out = \"\"\n",
            "    for word in words:\n",
            "        out = out + sep + word\n",
            "    return out\n",
        ));
        assert_eq!(
            emitted.matches("py_add_assign").count(),
            2,
            "each operand in the chain is one append:\n{emitted}"
        );
        assert!(
            !emitted.contains("PyAdd::py_add"),
            "nothing in this function should still rebuild:\n{emitted}"
        );
    }

    #[test]
    fn a_chain_that_does_not_start_at_the_name_is_left_alone() {
        // `out = sep + out + word` has the name in the middle of the spine, so the leftmost
        // operand is `sep`. Appending would silently reorder the text.
        let emitted = emit(concat!(
            "def wrapped(words: list[str], sep: str) -> str:\n",
            "    out = \"\"\n",
            "    for word in words:\n",
            "        out = sep + out + word\n",
            "    return out\n",
        ));
        assert!(
            !emitted.contains("py_add_assign"),
            "the name must be the leftmost operand:\n{emitted}"
        );
    }

    #[test]
    fn a_chain_reading_the_name_again_is_left_alone() {
        let emitted = emit(concat!(
            "def twice(words: list[str]) -> str:\n",
            "    out = \"\"\n",
            "    for word in words:\n",
            "        out = out + word + out\n",
            "    return out\n",
        ));
        assert!(
            !emitted.contains("py_add_assign"),
            "an operand reading the target must decline the rewrite:\n{emitted}"
        );
    }

    #[test]
    fn a_chain_mixing_in_another_operator_is_left_alone() {
        // `n = n + a - b` parses as `((n + a) - b)`, whose outermost operator is subtraction.
        let emitted = emit(concat!(
            "def drift(a: int, b: int) -> int:\n",
            "    n = 0\n",
            "    n = n + a - b\n",
            "    return n\n",
        ));
        assert!(!emitted.contains("py_add_assign"), "{emitted}");
    }

    #[test]
    fn a_long_chain_appends_once_per_operand() {
        let emitted = emit(concat!(
            "def four(a: str, b: str, c: str, d: str) -> str:\n",
            "    out = a\n",
            "    out = out + b + c + d\n",
            "    return out\n",
        ));
        assert_eq!(emitted.matches("py_add_assign").count(), 3, "{emitted}");
    }
}

/// A local returned in tail position is moved rather than deep-copied.
///
/// The function is ending and the original is about to be dropped, so the copy has no reader. The
/// restriction to tail position is load-bearing rather than cautious: a `return` nested inside a
/// loop over the same name would move out of a value the loop borrows. Tail position is the last
/// statement at the top level of the body and therefore cannot sit inside any loop, which makes
/// the move safe by construction rather than safe if an analysis is right.
mod moved_returns {
    use super::*;

    #[test]
    fn a_returned_collection_is_not_copied() {
        let emitted = emit(concat!(
            "def build(n: int) -> list[int]:\n",
            "    out: list[int] = []\n",
            "    i = 0\n",
            "    while i < n:\n",
            "        out.append(i)\n",
            "        i = i + 1\n",
            "    return out\n",
        ));
        assert!(emitted.contains("Ok(out)"), "{emitted}");
        assert!(
            !emitted.contains("out.clone()"),
            "the value is about to be dropped; the copy has no reader:\n{emitted}"
        );
    }

    #[test]
    fn a_returned_parameter_is_also_moved() {
        let emitted = emit("def identity(xs: list[int]) -> list[int]:\n    return xs\n");
        assert!(emitted.contains("Ok(xs)"), "{emitted}");
        assert!(!emitted.contains("xs.clone()"), "{emitted}");
    }

    #[test]
    fn a_return_outside_tail_position_is_unchanged() {
        // `return early` sits inside an `if`, so it is not the last statement at the top level.
        // Only the trailing `return rest` may move.
        let emitted = emit(concat!(
            "def pick(early: list[int], rest: list[int], flag: bool) -> list[int]:\n",
            "    if flag:\n",
            "        return early\n",
            "    return rest\n",
        ));
        assert!(
            emitted.contains("return Ok(early.clone())"),
            "a non-tail return keeps the existing emission:\n{emitted}"
        );
        assert!(emitted.contains("Ok(rest)"), "{emitted}");
        assert!(!emitted.contains("rest.clone()"), "{emitted}");
    }

    #[test]
    fn a_returned_text_value_is_moved_too() {
        let emitted = emit("def echo(s: str) -> str:\n    return s\n");
        assert!(emitted.contains("echo(s: &str)"), "{emitted}");
        assert!(emitted.contains("Ok(py_str_owned(&(s)))"), "{emitted}");
    }

    #[test]
    fn a_returned_mapping_is_moved() {
        let emitted =
            emit("def pass_through(d: dict[str, int]) -> dict[str, int]:\n    return d\n");
        assert!(emitted.contains("Ok(d)"), "{emitted}");
        assert!(!emitted.contains("d.clone()"), "{emitted}");
    }

    #[test]
    fn a_returned_expression_is_untouched() {
        // Only a bare name is a move. An expression already builds a fresh value.
        let emitted = emit("def total(a: int, b: int) -> int:\n    return a + b\n");
        assert!(emitted.contains("PyAdd::py_add"), "{emitted}");
    }
}

/// A loop variable the body only reads is bound by reference.
///
/// For a collection of owned values, copying each element is an allocation and a copy per element
/// per loop. Whether the body assigns to the loop variable is already computed — it is what
/// decides whether the binding is `mut` — so the same answer decides this and there is no second
/// analysis to disagree with the first.
mod borrowed_loop_variables {
    use super::*;

    #[test]
    fn a_read_only_loop_variable_over_text_is_borrowed() {
        let emitted = emit(concat!(
            "def total_length(words: list[str]) -> int:\n",
            "    total = 0\n",
            "    for word in words:\n",
            "        total = total + len(word)\n",
            "    return total\n",
        ));
        assert!(
            emitted.contains("py_iter_borrowed"),
            "a read-only loop variable must not be copied:\n{emitted}"
        );
        assert!(emitted.contains("let word: &String"), "{emitted}");
    }

    #[test]
    fn a_loop_variable_the_body_assigns_is_still_owned() {
        // Assigning to the loop variable needs a value of its own, and must not affect what is
        // being iterated.
        let emitted = emit(concat!(
            "def shout(words: list[str], suffix: str) -> int:\n",
            "    n = 0\n",
            "    for word in words:\n",
            "        word = word + suffix\n",
            "        n = n + len(word)\n",
            "    return n\n",
        ));
        assert!(
            emitted.contains("PyIterate::py_iter(") && !emitted.contains("py_iter_borrowed"),
            "an assigned loop variable needs its own value:\n{emitted}"
        );
        assert!(emitted.contains("let mut word: String"), "{emitted}");
    }

    #[test]
    fn a_scalar_loop_variable_is_still_owned() {
        // An `i64` is consumed by value wherever it is read, so binding one behind a reference
        // would be a type error in the body rather than a copy avoided — and there is no copy
        // worth avoiding.
        let emitted = emit(concat!(
            "def total(values: list[int]) -> int:\n",
            "    sum = 0\n",
            "    for v in values:\n",
            "        sum = sum + v\n",
            "    return sum\n",
        ));
        assert!(!emitted.contains("py_iter_borrowed"), "{emitted}");
        assert!(emitted.contains("let v: i64"), "{emitted}");
    }

    #[test]
    fn a_read_only_loop_over_a_collection_of_collections_is_borrowed() {
        let emitted = concat!(
            "def widest(rows: list[list[int]]) -> int:\n",
            "    best = 0\n",
            "    for row in rows:\n",
            "        if len(row) > best:\n",
            "            best = len(row)\n",
            "    return best\n",
        );
        let emitted = emit(emitted);
        assert!(emitted.contains("py_iter_borrowed"), "{emitted}");
        assert!(emitted.contains("let row: &Vec<i64>"), "{emitted}");
    }

    #[test]
    fn a_mapping_key_loop_is_borrowed_when_only_read() {
        let emitted = emit(concat!(
            "def key_length(d: dict[str, int]) -> int:\n",
            "    total = 0\n",
            "    for k in d:\n",
            "        total = total + len(k)\n",
            "    return total\n",
        ));
        assert!(emitted.contains("py_iter_borrowed"), "{emitted}");
        assert!(emitted.contains("let k: &String"), "{emitted}");
    }
}

/// A compared loop variable keeps its copy.
///
/// Every other position a loop variable reaches is a function argument, which is a coercion site,
/// so `&&String` becomes `&String` on its own. A comparison is not: `a < b` picks a `PartialOrd`
/// implementation before any coercion is considered, and there is no reference depth that is
/// right for both an owned local and a borrowed loop variable at once.
///
/// The demo found this, not the fixture suite — `text.most_common` breaks ties with `word < best`
/// — which is why CLAUDE.md says to run `make demo` when emission changes.
#[test]
fn a_compared_loop_variable_is_not_borrowed() {
    let emitted = emit(concat!(
        "def smallest(words: list[str], start: str) -> str:\n",
        "    best = start\n",
        "    for word in words:\n",
        "        if word < best:\n",
        "            best = word\n",
        "    return best\n",
    ));
    assert!(
        !emitted.contains("py_iter_borrowed"),
        "a compared loop variable must keep its own value:\n{emitted}"
    );
    assert!(emitted.contains("let word: String"), "{emitted}");
}

/// `d[k] = d[k] + v` looks up the key once.
///
/// It was a read followed by a write — two hashes on a statement whose whole purpose is to touch
/// one slot. Counting occurrences is the most common thing anyone does with a mapping, and it
/// paid for that once per element.
///
/// The fixtures build their container locally rather than taking one as a parameter: a collection
/// parameter is a copy, so the subset rejects mutating it or an alias of it.
mod fused_indexed_accumulation {
    use super::*;

    /// A function that fills a list, then does `body` to it.
    fn over_a_list(body: &str) -> String {
        emit(&format!(
            "def run(n: int) -> list[int]:\n\
             \x20   xs: list[int] = []\n\
             \x20   i = 0\n\
             \x20   while i < n:\n\
             \x20       xs.append(i)\n\
             \x20       i = i + 1\n\
             {body}\
             \x20   return xs\n"
        ))
    }

    /// Whether the emitted source *calls* the separate read, rather than merely importing it.
    fn reads_separately(emitted: &str) -> bool {
        emitted.contains("py_subscript(")
    }

    #[test]
    fn a_mapping_increment_is_one_lookup() {
        let emitted = emit(concat!(
            "def tally(words: list[str]) -> dict[str, int]:\n",
            "    counts: dict[str, int] = {}\n",
            "    for word in words:\n",
            "        if word in counts:\n",
            "            counts[word] = counts[word] + 1\n",
            "        else:\n",
            "            counts[word] = 1\n",
            "    return counts\n",
        ));
        assert!(emitted.contains("py_add_assign_at"), "{emitted}");
        assert!(
            !reads_separately(&emitted),
            "the fused form performs no separate read:\n{emitted}"
        );
    }

    #[test]
    fn a_sequence_increment_is_one_lookup_too() {
        // The emitter cannot tell a mapping from a sequence, so the choice is type-directed and
        // both containers have to work.
        let emitted = over_a_list("    xs[0] = xs[0] + 1\n");
        assert!(emitted.contains("py_add_assign_at"), "{emitted}");
        assert!(!reads_separately(&emitted), "{emitted}");
    }

    #[test]
    fn the_mirrored_form_is_left_alone() {
        // `xs[0] = 1 + xs[0]` puts the read on the right. For text that is the other string.
        let emitted = over_a_list("    xs[0] = 1 + xs[0]\n");
        assert!(!emitted.contains("py_add_assign_at"), "{emitted}");
        assert!(reads_separately(&emitted), "{emitted}");
    }

    #[test]
    fn a_different_index_is_left_alone() {
        // Reading one slot and writing another is two operations, not one.
        let emitted = over_a_list("    xs[0] = xs[1] + 1\n");
        assert!(!emitted.contains("py_add_assign_at"), "{emitted}");
    }

    #[test]
    fn an_operand_touching_the_collection_is_left_alone() {
        // The fused form holds the collection mutably, so a right operand that reads it would
        // ask for a shared borrow inside a mutable one.
        let emitted = over_a_list("    xs[0] = xs[0] + xs[1]\n");
        assert!(!emitted.contains("py_add_assign_at"), "{emitted}");
    }

    #[test]
    fn a_plain_assignment_is_unchanged() {
        let emitted = over_a_list("    xs[0] = 1\n");
        assert!(!emitted.contains("py_add_assign_at"), "{emitted}");
        assert!(emitted.contains("PySetItem::py_set"), "{emitted}");
    }

    #[test]
    fn subtraction_is_left_alone() {
        let emitted = over_a_list("    xs[0] = xs[0] - 1\n");
        assert!(!emitted.contains("py_add_assign_at"), "{emitted}");
    }
}

/// A collection built once per iteration starts with enough room for that loop.
mod known_collection_capacity {
    use super::*;

    #[test]
    fn a_list_built_from_a_collection_uses_its_length() {
        let emitted = emit(concat!(
            "def copy(values: list[int]) -> list[int]:\n",
            "    out: list[int] = []\n",
            "    for value in values:\n",
            "        out.append(value)\n",
            "    return out\n",
        ));
        assert!(
            emitted.contains(
                "Vec::with_capacity(PyLen::py_len(&(values), TextUnits::CodePoints) as usize)"
            ),
            "{emitted}"
        );
    }

    #[test]
    fn a_mapping_built_from_a_collection_uses_its_length() {
        let emitted = emit(concat!(
            "def index(words: list[str]) -> dict[str, int]:\n",
            "    positions: dict[str, int] = {}\n",
            "    i = 0\n",
            "    for word in words:\n",
            "        positions[word] = i\n",
            "        i = i + 1\n",
            "    return positions\n",
        ));
        assert!(
            emitted.contains("FastMap::with_capacity_and_hasher("),
            "{emitted}"
        );
        assert!(
            emitted.contains("PyLen::py_len(&(words), TextUnits::CodePoints) as usize"),
            "{emitted}"
        );
    }

    #[test]
    fn a_list_built_by_a_simple_range_uses_its_trip_count() {
        let emitted = emit(concat!(
            "def zeros(size: int) -> list[int]:\n",
            "    out: list[int] = []\n",
            "    for _i in range(size):\n",
            "        out.append(0)\n",
            "    return out\n",
        ));
        assert!(
            emitted.contains("Vec::with_capacity(usize::try_from(size).unwrap_or(0))"),
            "{emitted}"
        );
    }

    #[test]
    fn an_iterable_call_is_not_evaluated_early_or_twice() {
        let emitted = emit(concat!(
            "def source(values: list[int]) -> list[int]:\n",
            "    return values\n",
            "\n",
            "def copy(values: list[int]) -> list[int]:\n",
            "    out: list[int] = []\n",
            "    for value in source(values):\n",
            "        out.append(value)\n",
            "    return out\n",
        ));
        assert!(!emitted.contains("Vec::with_capacity"), "{emitted}");
    }
}
