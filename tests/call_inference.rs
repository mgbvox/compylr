//! Typing calls from collected signatures, and the order-independence that has to survive it.
//!
//! Before this, lowering resolved nothing, so results were order-independent by construction. Now
//! lowering resolves calls within a source, and the property holds only because of an invariant of
//! the signature pass — so it is asserted directly rather than assumed.
//!
//! The other half of this file is about what lowering still does *not* resolve. A decorated
//! function is validated on its own, so a callee in another module is invisible; rejecting it here
//! would make acceptance depend on decoration order, which is exactly what unit validation exists
//! to prevent.

use compylr::error::LowerErrorKind;
use compylr::frontend::parse_source;
use compylr::ir::{Expr, Function, Stmt, Ty, Unit};
use compylr::lower::{collect_signatures, lower_source};

fn lower(source: &str) -> Vec<Function> {
    let parsed = parse_source(source).expect("fixture must parse");
    lower_source(&parsed).unwrap_or_else(|e| panic!("should lower: {}", e.render(source)))
}

fn reject(source: &str) -> LowerErrorKind {
    let parsed = parse_source(source).expect("fixture must parse");
    match lower_source(&parsed) {
        Ok(_) => panic!("should have been rejected but lowered:\n{source}"),
        Err(error) => error.kind(),
    }
}

/// The type a binding named `name` was given.
fn binding_ty(function: &Function, name: &str) -> Ty {
    for stmt in &function.body {
        if let Stmt::Bind {
            name: bound, ty, ..
        } = stmt
            && bound == name
        {
            return ty.clone();
        }
    }
    panic!("no binding named {name} in {:?}", function.body);
}

const DOUBLE: &str = "def double(n: int) -> int:\n    return n * 2\n\n";

mod signature_collection {
    use super::*;

    #[test]
    fn every_function_contributes_a_signature() {
        let parsed = parse_source(concat!(
            "def a(x: int) -> int:\n    return x\n\n",
            "def b(x: float, y: str) -> bool:\n    return True\n",
        ))
        .unwrap();
        let sigs = collect_signatures(&parsed, &Default::default());

        assert_eq!(sigs["a"].params, vec![Ty::Int]);
        assert_eq!(sigs["a"].ret, Ty::Int);
        assert_eq!(sigs["b"].params, vec![Ty::Float, Ty::Str]);
        assert_eq!(sigs["b"].ret, Ty::Bool);
    }

    #[test]
    fn collection_reads_annotations_only() {
        // The body here would not lower, but the signature is still readable. That is what makes
        // the pass safe to run first: it can never depend on inference.
        let parsed =
            parse_source("def a(x: int) -> int:\n    while x:\n        pass\n    return x\n")
                .unwrap();
        assert_eq!(
            collect_signatures(&parsed, &Default::default())["a"].ret,
            Ty::Int
        );
    }

    #[test]
    fn a_function_without_annotations_contributes_nothing() {
        // It cannot be typed, and `lower_function` reports it properly in source order.
        let parsed = parse_source("def a(x):\n    return x\n").unwrap();
        assert!(collect_signatures(&parsed, &Default::default()).is_empty());
    }
}

mod call_typing {
    use super::*;

    #[test]
    fn a_call_initializer_is_inferred() {
        let source = format!("{DOUBLE}def f(n: int) -> int:\n    b = double(n)\n    return b\n");
        let functions = lower(&source);
        let f = functions.iter().find(|f| f.name == "f").unwrap();
        assert_eq!(binding_ty(f, "b"), Ty::Int);
    }

    #[test]
    fn a_call_nested_in_an_expression_is_inferred() {
        let source =
            format!("{DOUBLE}def f(n: int) -> int:\n    b = double(n) + 1\n    return b\n");
        let f = lower(&source).into_iter().find(|f| f.name == "f").unwrap();
        assert_eq!(binding_ty(&f, "b"), Ty::Int);
    }

    #[test]
    fn a_forward_reference_is_inferred() {
        // The caller is defined first, so this only works because signatures are collected before
        // any body is lowered.
        let source = concat!(
            "def caller(n: int) -> int:\n    b = callee(n)\n    return b\n\n",
            "def callee(n: int) -> int:\n    return n\n",
        );
        let f = lower(source)
            .into_iter()
            .find(|f| f.name == "caller")
            .unwrap();
        assert_eq!(binding_ty(&f, "b"), Ty::Int);
    }

    #[test]
    fn a_self_recursive_function_types() {
        let source = concat!(
            "def countdown(n: int) -> int:\n",
            "    b = countdown(n)\n",
            "    return b\n",
        );
        let f = lower(source).into_iter().next().unwrap();
        assert_eq!(binding_ty(&f, "b"), Ty::Int);
    }

    #[test]
    fn a_declared_annotation_still_wins_via_promotion() {
        let source =
            format!("{DOUBLE}def f(n: int) -> float:\n    b: float = double(n)\n    return b\n");
        let f = lower(&source).into_iter().find(|f| f.name == "f").unwrap();
        assert_eq!(binding_ty(&f, "b"), Ty::Float);
    }

    #[test]
    fn an_argument_is_promoted_to_the_declared_type() {
        let source = concat!(
            "def scale(x: float) -> float:\n    return x * 2.0\n\n",
            "def f(n: int) -> float:\n    return scale(n)\n",
        );
        let f = lower(source).into_iter().find(|f| f.name == "f").unwrap();
        match &f.body[0] {
            Stmt::Return(Expr::Call { args, .. }) => assert!(
                matches!(args[0], Expr::ToFloat(_)),
                "the integer argument must carry an explicit conversion, got {:?}",
                args[0]
            ),
            other => panic!("unexpected body: {other:?}"),
        }
    }

    #[test]
    fn wrong_arity_is_rejected() {
        let source = format!("{DOUBLE}def f(n: int) -> int:\n    return double(n, n)\n");
        assert_eq!(reject(&source), LowerErrorKind::ArityMismatch);
    }

    #[test]
    fn a_wrong_argument_type_is_rejected() {
        let source = format!("{DOUBLE}def f(s: str) -> int:\n    return double(s)\n");
        assert_eq!(reject(&source), LowerErrorKind::TypeMismatch);
    }

    #[test]
    fn a_narrowing_argument_is_rejected() {
        let source = concat!(
            "def whole(n: int) -> int:\n    return n\n\n",
            "def f(x: float) -> int:\n    return whole(x)\n",
        );
        assert_eq!(reject(source), LowerErrorKind::TypeMismatch);
    }
}

mod order_independence {
    use super::*;

    /// Lower the same two functions in both definition orders.
    fn both_orders(first: &str, second: &str) -> (Vec<Function>, Vec<Function>) {
        let forward = lower(&format!("{first}\n{second}"));
        let backward = lower(&format!("{second}\n{first}"));
        (forward, backward)
    }

    #[test]
    fn definition_order_does_not_change_the_ir() {
        // The property this whole design exists to protect. It used to hold because lowering
        // resolved nothing; now it holds because of an invariant of the signature pass, so it has
        // to be asserted rather than assumed.
        let (mut forward, mut backward) = both_orders(
            "def alpha(n: int) -> int:\n    b = beta(n)\n    return b\n",
            "def beta(n: int) -> int:\n    b = alpha(n)\n    return b\n",
        );
        forward.sort_by(|a, b| a.name.cmp(&b.name));
        backward.sort_by(|a, b| a.name.cmp(&b.name));

        for (a, b) in forward.iter().zip(&backward) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.params, b.params);
            assert_eq!(a.ret, b.ret);
            assert_eq!(a.body, b.body, "bodies differ for {}", a.name);
        }
    }

    #[test]
    fn definition_order_does_not_change_the_unit_fingerprint() {
        let (forward, backward) = both_orders(
            "def alpha(n: int) -> int:\n    b = beta(n)\n    return b\n",
            "def beta(n: int) -> int:\n    b = alpha(n)\n    return b\n",
        );
        let unit = |functions: Vec<Function>| {
            let mut unit = Unit::new();
            for function in functions {
                unit.add_function(function).unwrap();
            }
            unit
        };
        assert_eq!(unit(forward).fingerprint(), unit(backward).fingerprint());
    }
}

mod deferred_resolution {
    use super::*;

    #[test]
    fn a_callee_in_another_source_leaves_the_type_undetermined() {
        // This is what the decorator relies on: it validates one function at a time, so a callee
        // in another module is invisible. Rejecting here would make acceptance depend on which
        // function happened to be decorated first.
        assert_eq!(
            reject("def f(n: int) -> int:\n    b = elsewhere(n)\n    return b\n"),
            LowerErrorKind::UndeterminedBinding,
            "an unseen callee must leave the binding undetermined -- a category the caller can \
             defer once it sees every source -- rather than failing outright"
        );
    }

    #[test]
    fn an_annotated_binding_from_an_unseen_callee_lowers() {
        let functions = lower("def f(n: int) -> int:\n    b: int = elsewhere(n)\n    return b\n");
        assert_eq!(binding_ty(&functions[0], "b"), Ty::Int);
    }

    #[test]
    fn a_returned_call_to_an_unseen_callee_lowers() {
        // The single-function case the decorator produces.
        let functions = lower("def f(n: int) -> int:\n    return elsewhere(n)\n");
        assert_eq!(functions.len(), 1);
    }

    #[test]
    fn unit_validation_still_catches_a_callee_that_exists_nowhere() {
        let mut unit = Unit::new();
        for function in lower("def f(n: int) -> int:\n    return elsewhere(n)\n") {
            unit.add_function(function).unwrap();
        }
        let error = unit.validate().expect_err("must not resolve");
        assert_eq!(error.kind(), LowerErrorKind::Unresolved);
        assert!(error.message().contains("elsewhere"));
    }

    #[test]
    fn a_cross_source_call_resolves_once_the_unit_is_assembled() {
        let mut unit = Unit::new();
        for function in lower("def caller(n: int) -> int:\n    return callee(n)\n") {
            unit.add_function(function).unwrap();
        }
        for function in lower("def callee(n: int) -> int:\n    return n\n") {
            unit.add_function(function).unwrap();
        }
        unit.validate().expect("both sources together must resolve");
    }
}

mod missing_return {
    use super::*;

    #[test]
    fn a_body_of_only_pass_is_rejected() {
        assert_eq!(
            reject("def f() -> int:\n    pass\n"),
            LowerErrorKind::MissingReturn
        );
    }

    #[test]
    fn a_body_ending_in_a_binding_is_rejected() {
        assert_eq!(
            reject("def f(n: int) -> int:\n    b = n + 1\n"),
            LowerErrorKind::MissingReturn
        );
    }

    #[test]
    fn the_diagnostic_names_the_function_and_is_not_a_type_mismatch() {
        let parsed = parse_source("def compute() -> int:\n    pass\n").unwrap();
        let error = lower_source(&parsed).expect_err("must fail");
        assert_eq!(error.kind(), LowerErrorKind::MissingReturn);
        assert!(error.message().contains("compute"), "{}", error.message());
        assert_ne!(
            error.kind(),
            LowerErrorKind::TypeMismatch,
            "nothing disagrees about types here; the value is simply absent"
        );
    }

    #[test]
    fn a_unit_returning_function_needs_no_return() {
        assert_eq!(lower("def f() -> None:\n    pass\n").len(), 1);
    }

    #[test]
    fn a_unit_returning_function_ending_in_a_binding_is_fine() {
        assert_eq!(lower("def f(n: int) -> None:\n    b = n + 1\n").len(), 1);
    }

    #[test]
    fn a_function_that_does_return_is_unaffected() {
        assert_eq!(lower("def f(n: int) -> int:\n    return n\n").len(), 1);
    }

    #[test]
    fn a_documented_function_with_only_a_docstring_is_rejected_for_a_value_return() {
        // The docstring is stripped, leaving nothing — which must not look like a valid body.
        assert_eq!(
            reject("def f() -> int:\n    \"\"\"Docs.\"\"\"\n"),
            LowerErrorKind::MissingReturn
        );
    }
}
