//! Where a node declares Rust's own meaning, the backend emits Rust's own operator.
//!
//! These assert the emitted **text**, which the repository normally forbids — assert on values
//! after mutation, never on emitted source. The exception is narrow and this is it: the emitted
//! *form* is the property being bought. A user who asked for the target's arithmetic is buying
//! generated source they can read and recognise, and a `.compylr/` full of `NativeAdd::native_add`
//! would deliver the speed and not the claim. A test that only checked the answer would pass on
//! an implementation that delivered neither.
//!
//! Every unit here is hand-built, with no frontend and no Python involved. That is deliberate: it
//! is the only way to reach a mode combination the Python frontend cannot currently produce, and
//! it proves the backend decides from the node rather than from who produced it.

use compylr_diagnostics::span::Span;
use compylr_ir::{
    BinOp, Checked, DivMode, Expr, Function, IndexOrigin, Literal, Param, RemSign, Rounding, Stmt,
    TextUnits, Ty, Unit,
};

/// The translated functions, **without** the import preamble.
///
/// The preamble names every helper whether the unit uses it or not, so a test asserting that some
/// helper is absent would be reading the import list and always failing. Two of these tests did
/// exactly that on their first run.
fn emit(unit: &Unit) -> String {
    let backend = compylr_registry::backends::lookup("rust").unwrap();
    let file = backend
        .emit(unit)
        .expect("a hand-built unit must render")
        .remove("src/generated.rs")
        .expect("the translated functions land here");

    let (_, after_compat) = file
        .split_once("use crate::compat::{")
        .expect("every generated file imports the helpers");
    let (_, body) = after_compat
        .split_once("};")
        .expect("the import block is closed");
    body.to_string()
}

/// `def op(a, b) -> ret: return a <op> b`, with the operand and result types given.
fn binary_unit(op: BinOp, ty: Ty) -> Unit {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "op".to_string(),
        params: vec![
            Param {
                name: "a".to_string(),
                ty: ty.clone(),
            },
            Param {
                name: "b".to_string(),
                ty: ty.clone(),
            },
        ],
        ret: ty,
        body: vec![Stmt::Return(Expr::binary(
            op,
            Expr::name("a"),
            Expr::name("b"),
        ))],
        doc: None,
        span: Span::default(),
    })
    .unwrap();
    unit
}

/// The same operation under a comparison, where the expected type says nothing about the operands.
fn compared_unit(op: BinOp, ty: Ty) -> Unit {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "op".to_string(),
        params: vec![
            Param {
                name: "a".to_string(),
                ty: ty.clone(),
            },
            Param {
                name: "b".to_string(),
                ty,
            },
        ],
        ret: Ty::Bool,
        body: vec![Stmt::Return(Expr::binary(
            BinOp::Lt,
            Expr::binary(op, Expr::name("a"), Expr::name("b")),
            Expr::int(0),
        ))],
        doc: None,
        span: Span::default(),
    })
    .unwrap();
    unit
}

fn unchecked_add() -> BinOp {
    BinOp::Add {
        checked: Checked::Unchecked,
    }
}

#[test]
fn an_unchecked_add_with_a_known_type_emits_a_bare_operator() {
    let rendered = emit(&binary_unit(unchecked_add(), Ty::Int));

    assert!(
        rendered.contains("((a) + (b))"),
        "expected Rust's own `+`; got:\n{rendered}"
    );
    assert!(
        !rendered.contains("PyAdd"),
        "a node that declared no checking must not reach the reporting helper:\n{rendered}"
    );
    assert!(
        !rendered.contains("NativeAdd"),
        "the type is known here, so the dispatch is not needed and would not read like Rust:\n\
         {rendered}"
    );
}

#[test]
fn a_reported_add_still_emits_the_helper_unchanged() {
    let rendered = emit(&binary_unit(
        BinOp::Add {
            checked: Checked::Reported,
        },
        Ty::Int,
    ));

    assert!(
        rendered.contains("PyAdd::py_add"),
        "a node that asked for overflow reporting must still get it:\n{rendered}"
    );
}

/// Where the expected type is unknown the dispatch is used, and it is infallible.
///
/// A comparison's operands say nothing about the result type, so the backend cannot know whether
/// it holds integers or strings — and Rust's `+` on two owned `String`s does not compile. The
/// shim exists to let Rust's trait resolution answer, which is a dispatch and not a check.
#[test]
fn an_unchecked_add_under_a_comparison_dispatches_infallibly() {
    let rendered = emit(&compared_unit(unchecked_add(), Ty::Int));

    assert!(
        rendered.contains("NativeAdd::native_add"),
        "an unknown expected type must reach the dispatch:\n{rendered}"
    );
    assert!(
        !rendered.contains("native_add(&(a), &(b))?"),
        "the dispatch returns a value, so the call site carries no `?`:\n{rendered}"
    );
}

#[test]
fn unchecked_truncating_division_emits_rusts_own_division() {
    let rendered = emit(&binary_unit(
        BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardZero),
            checked: Checked::Unchecked,
        },
        Ty::Int,
    ));

    assert!(
        rendered.contains("((a) / (b))"),
        "truncating and unchecked is exactly what Rust's `/` means:\n{rendered}"
    );
    assert!(!rendered.contains("div_trunc"), "{rendered}");
}

#[test]
fn unchecked_remainder_taking_the_dividends_sign_emits_rusts_own_remainder() {
    let rendered = emit(&binary_unit(
        BinOp::Rem {
            sign: RemSign::Dividend,
            checked: Checked::Unchecked,
        },
        Ty::Int,
    ));

    assert!(
        rendered.contains("((a) % (b))"),
        "the sign of the dividend and unchecked is Rust's `%`:\n{rendered}"
    );
    assert!(!rendered.contains("rem_trunc"), "{rendered}");
}

/// **The combination most likely to be got wrong.**
///
/// A flooring division whose zero divisor the program declined to define is reachable from
/// `Behavior(floor_div="python", overflow="rust")` and is a perfectly ordinary request. Rust's `/`
/// does not floor: emitting a bare `/` here would silently produce `-3` where the program says
/// `-4`, on exactly the inputs — a negative dividend — that nobody writes a test for by accident.
#[test]
fn an_unchecked_flooring_division_still_emits_the_flooring_helper() {
    let rendered = emit(&binary_unit(
        BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardNegInf),
            checked: Checked::Unchecked,
        },
        Ty::Int,
    ));

    assert!(
        rendered.contains("div_floor"),
        "Rust's `/` does not floor, so the correcting helper must stay:\n{rendered}"
    );
    assert!(
        !rendered.contains("((a) / (b))"),
        "emitting a bare `/` here is a wrong answer, not a slow one:\n{rendered}"
    );
}

/// The same for a remainder taking the divisor's sign.
#[test]
fn an_unchecked_remainder_taking_the_divisors_sign_still_corrects() {
    let rendered = emit(&binary_unit(
        BinOp::Rem {
            sign: RemSign::Divisor,
            checked: Checked::Unchecked,
        },
        Ty::Int,
    ));

    assert!(rendered.contains("rem_floor"), "{rendered}");
    assert!(!rendered.contains("((a) % (b))"), "{rendered}");
}

#[test]
fn an_unchecked_negation_emits_rusts_own_minus() {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "op".to_string(),
        params: vec![Param {
            name: "a".to_string(),
            ty: Ty::Int,
        }],
        ret: Ty::Int,
        body: vec![Stmt::Return(Expr::Neg {
            value: Box::new(Expr::name("a")),
            checked: Checked::Unchecked,
        })],
        doc: None,
        span: Span::default(),
    })
    .unwrap();

    let rendered = emit(&unit);
    assert!(rendered.contains("(-(a))"), "{rendered}");
    assert!(!rendered.contains("py_neg"), "{rendered}");
}

fn subscript_unit(origin: IndexOrigin, checked: Checked) -> Unit {
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
            checked,
        })],
        doc: None,
        span: Span::default(),
    })
    .unwrap();
    unit
}

#[test]
fn an_unchecked_read_from_the_start_emits_native_indexing() {
    let rendered = emit(&subscript_unit(IndexOrigin::FromStart, Checked::Unchecked));

    assert!(
        rendered.contains("[i as usize]"),
        "expected Rust's own indexing:\n{rendered}"
    );
    assert!(
        !rendered.contains("py_subscript"),
        "the bounds resolution is what the mode removes:\n{rendered}"
    );
}

/// Counting from either end still needs the helper, whatever the checking mode says.
///
/// The two axes are independent: `index="python"` with the overflow axis on Rust's side is a real
/// request, and Rust's indexing has no notion of counting backwards. Only the *check* is optional.
#[test]
fn an_unchecked_read_from_either_end_still_resolves_the_offset() {
    let rendered = emit(&subscript_unit(
        IndexOrigin::FromEitherEnd,
        Checked::Unchecked,
    ));

    assert!(
        rendered.contains("py_subscript"),
        "a negative index still has to be resolved against the length:\n{rendered}"
    );
}

#[test]
fn a_reported_read_still_reports() {
    let rendered = emit(&subscript_unit(IndexOrigin::FromStart, Checked::Reported));
    assert!(rendered.contains("py_subscript"), "{rendered}");
}

/// The signature does not move with the behavior. Design D7.
///
/// A function whose every operation is unchecked keeps the fallible signature it would have had
/// under the default. The existing reason holds unchanged: a signature that became infallible
/// depending on the body's contents would move on an unrelated edit, and behavior only adds a
/// second way for it to move. What the body gets is the win — no `?`, one `Ok` at the boundary.
#[test]
fn an_all_unchecked_function_keeps_its_fallible_signature_and_needs_no_propagation() {
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
        body: vec![Stmt::Return(Expr::binary(
            unchecked_add(),
            Expr::binary(
                BinOp::Mul {
                    checked: Checked::Unchecked,
                },
                Expr::name("a"),
                Expr::name("b"),
            ),
            Expr::Neg {
                value: Box::new(Expr::name("b")),
                checked: Checked::Unchecked,
            },
        ))],
        doc: None,
        span: Span::default(),
    })
    .unwrap();

    let rendered = emit(&unit);
    assert!(
        rendered.contains("-> Result<i64, RuntimeError>"),
        "the signature must not move with the behavior:\n{rendered}"
    );

    // The body proper carries no propagation. Checked against the function's own lines rather
    // than the whole file, so the import preamble cannot mask a `?` that is really there.
    let body: String = rendered
        .lines()
        .skip_while(|line| !line.contains("pub fn op"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !body.contains('?'),
        "an all-unchecked body needs no error path:\n{body}"
    );
    assert!(
        body.contains("Ok("),
        "and is wrapped once at the boundary:\n{body}"
    );
}

/// UTF-8 length is left as the dispatch, and that is a decision rather than an omission.
///
/// `PyLen` already selects by operand type, and for a *collection* a length is a count of
/// elements under every reading. A bare `.len()` would be right for a string declaring UTF-8
/// bytes and wrong for a list — and the backend cannot tell which it has without re-deriving the
/// type, which is the one thing it must not do. The dispatch inlines away; guessing would not.
#[test]
fn length_stays_a_dispatch_under_every_declared_unit() {
    for units in [TextUnits::CodePoints, TextUnits::Utf8Bytes] {
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

        let rendered = emit(&unit);
        assert!(rendered.contains("PyLen::py_len"), "{units:?}: {rendered}");
        assert!(
            rendered.contains(match units {
                TextUnits::CodePoints => "CodePoints",
                TextUnits::Utf8Bytes => "Utf8Bytes",
                TextUnits::Utf16Units => "Utf16Units",
            }),
            "the declared units must reach the emitted call:\n{rendered}"
        );
    }
}

/// The decision reads the node, not the frontend.
///
/// Every unit above is hand-built and carries no origin, so nothing the backend consulted could
/// have told it a source language. Stated once, explicitly, because it is the property that makes
/// a second frontend possible.
#[test]
fn the_decision_is_made_without_any_recorded_frontend() {
    let unit = binary_unit(
        BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardZero),
            checked: Checked::Unchecked,
        },
        Ty::Int,
    );
    assert!(unit.origin().is_none(), "the fixture must claim no origin");
    assert!(emit(&unit).contains("((a) / (b))"));
}

/// String concatenation still works where the overflow axis took the target's side.
///
/// The overflow axis governs *integer* arithmetic. A user who waived it did not ask for their
/// string handling to change, and without an implementation of the dispatch for `String` this
/// program — which they wrote correctly — would fail to compile.
#[test]
fn strings_still_concatenate_under_unchecked_arithmetic() {
    let rendered = emit(&compared_unit(unchecked_add(), Ty::Str));
    assert!(rendered.contains("NativeAdd::native_add"), "{rendered}");

    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "join".to_string(),
        params: vec![
            Param {
                name: "a".to_string(),
                ty: Ty::Str,
            },
            Param {
                name: "b".to_string(),
                ty: Ty::Str,
            },
        ],
        ret: Ty::Str,
        body: vec![Stmt::Return(Expr::binary(
            unchecked_add(),
            Expr::name("a"),
            Expr::name("b"),
        ))],
        doc: None,
        span: Span::default(),
    })
    .unwrap();

    // A bare `+` would not compile for two owned strings, so the dispatch is what has to appear
    // even though the expected type is known here.
    let rendered = emit(&unit);
    assert!(
        rendered.contains("NativeAdd::native_add"),
        "a known *string* type must still dispatch, since Rust's `+` on two `String`s does not \
         compile:\n{rendered}"
    );
}

/// A literal is still a literal, so the emitted bare operator is not hiding a fold.
#[test]
fn a_bare_operator_is_emitted_over_literals_too() {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "op".to_string(),
        params: vec![],
        ret: Ty::Int,
        body: vec![Stmt::Return(Expr::binary(
            unchecked_add(),
            Expr::Literal(Literal::Int(2)),
            Expr::Literal(Literal::Int(3)),
        ))],
        doc: None,
        span: Span::default(),
    })
    .unwrap();

    let rendered = emit(&unit);
    assert!(rendered.contains("(2i64) + (3i64)"), "{rendered}");
}
