//! The IR's control-flow forms, and the two properties that are easy to lose.
//!
//! `walk_calls` must descend into nested bodies, or unit validation misses a call inside a loop
//! inside a branch and the backend emits a call to something that does not exist. And every new
//! form must survive the artifact, or the IR stops being readable for exactly the programs this
//! change makes worth reading.

use compylr_diagnostics::span::Span;
use compylr_ir::{Expr, Function, Stmt, Ty, Unit};

fn func(name: &str, body: Vec<Stmt>) -> Function {
    Function {
        name: name.into(),
        params: Vec::new(),
        ret: Ty::Unit,
        body,
        doc: None,
        span: Span::default(),
    }
}

fn unit_of(functions: Vec<Function>) -> Unit {
    let mut unit = Unit::new();
    for function in functions {
        unit.add_function(function).unwrap();
    }
    unit
}

/// A call, for asserting that a walk reached it.
fn call(name: &str) -> Expr {
    Expr::Call {
        callee: name.into(),
        args: Vec::new(),
    }
}

/// Every callee reached by walking a function.
fn callees(function: &Function) -> Vec<String> {
    let mut found = Vec::new();
    function.walk_calls(&mut |name, _| found.push(name.to_string()));
    found.sort();
    found
}

#[test]
fn every_control_flow_form_is_representable() {
    let f = func(
        "f",
        vec![
            Stmt::If {
                test: Expr::bool(true),
                then: vec![Stmt::Break],
                otherwise: vec![Stmt::Continue],
            },
            Stmt::While {
                test: Expr::bool(false),
                body: vec![Stmt::ReturnUnit],
            },
            Stmt::For {
                name: "i".into(),
                ty: Ty::Int,
                iter: Expr::Range {
                    start: Box::new(Expr::int(0)),
                    stop: Box::new(Expr::int(3)),
                    step: Box::new(Expr::int(1)),
                },
                body: vec![Stmt::Assign {
                    ty: Ty::Int,
                    name: "x".into(),
                    value: Expr::int(1),
                }],
            },
        ],
    );
    assert_eq!(f.body.len(), 3);
}

#[test]
fn a_conditional_may_have_no_alternative() {
    let f = func(
        "f",
        vec![Stmt::If {
            test: Expr::bool(true),
            then: vec![Stmt::ReturnUnit],
            otherwise: Vec::new(),
        }],
    );
    match &f.body[0] {
        Stmt::If { otherwise, .. } => assert!(otherwise.is_empty()),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn bodies_nest_to_any_depth() {
    let inner = Stmt::While {
        test: Expr::bool(true),
        body: vec![Stmt::Break],
    };
    let middle = Stmt::If {
        test: Expr::bool(true),
        then: vec![inner],
        otherwise: Vec::new(),
    };
    let outer = Stmt::While {
        test: Expr::bool(true),
        body: vec![middle],
    };
    assert!(matches!(func("f", vec![outer]).body[0], Stmt::While { .. }));
}

mod ranges {
    use super::*;

    #[test]
    fn a_range_carries_all_three_components() {
        // Present even when the source omitted them, so a backend never has to know Python's
        // defaulting rules.
        let range = Expr::Range {
            start: Box::new(Expr::int(0)),
            stop: Box::new(Expr::int(5)),
            step: Box::new(Expr::int(1)),
        };
        match range {
            Expr::Range { start, stop, step } => {
                assert_eq!(*start, Expr::int(0));
                assert_eq!(*stop, Expr::int(5));
                assert_eq!(*step, Expr::int(1));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn a_range_is_not_a_call() {
        // Validation must not try to resolve `range` against the unit, or its meaning would
        // depend on what else happened to be compiled.
        let f = func(
            "f",
            vec![Stmt::For {
                name: "i".into(),
                ty: Ty::Int,
                iter: Expr::Range {
                    start: Box::new(Expr::int(0)),
                    stop: Box::new(Expr::int(3)),
                    step: Box::new(Expr::int(1)),
                },
                body: Vec::new(),
            }],
        );
        assert!(callees(&f).is_empty());
        unit_of(vec![f])
            .validate()
            .expect("a range resolves nothing");
    }
}

mod walking {
    use super::*;

    #[test]
    fn calls_in_a_branch_are_found() {
        let f = func(
            "f",
            vec![Stmt::If {
                test: call("test"),
                then: vec![Stmt::Return(call("taken"))],
                otherwise: vec![Stmt::Return(call("other"))],
            }],
        );
        assert_eq!(callees(&f), ["other", "taken", "test"]);
    }

    #[test]
    fn calls_in_a_while_are_found() {
        let f = func(
            "f",
            vec![Stmt::While {
                test: call("test"),
                body: vec![Stmt::Return(call("inside"))],
            }],
        );
        assert_eq!(callees(&f), ["inside", "test"]);
    }

    #[test]
    fn calls_in_a_for_are_found() {
        let f = func(
            "f",
            vec![Stmt::For {
                name: "i".into(),
                ty: Ty::Int,
                iter: call("iterable"),
                body: vec![Stmt::Return(call("inside"))],
            }],
        );
        assert_eq!(callees(&f), ["inside", "iterable"]);
    }

    #[test]
    fn calls_in_range_components_are_found() {
        let f = func(
            "f",
            vec![Stmt::For {
                name: "i".into(),
                ty: Ty::Int,
                iter: Expr::Range {
                    start: Box::new(call("from")),
                    stop: Box::new(call("to")),
                    step: Box::new(call("by")),
                },
                body: Vec::new(),
            }],
        );
        assert_eq!(callees(&f), ["by", "from", "to"]);
    }

    #[test]
    fn a_call_nested_three_deep_is_found() {
        // The case that matters: missing it would let unit validation pass and the backend emit a
        // call to a function that does not exist.
        let f = func(
            "f",
            vec![Stmt::While {
                test: Expr::bool(true),
                body: vec![Stmt::If {
                    test: Expr::bool(true),
                    then: vec![Stmt::For {
                        name: "i".into(),
                        ty: Ty::Int,
                        iter: Expr::Range {
                            start: Box::new(Expr::int(0)),
                            stop: Box::new(Expr::int(1)),
                            step: Box::new(Expr::int(1)),
                        },
                        body: vec![Stmt::Return(call("buried"))],
                    }],
                    otherwise: Vec::new(),
                }],
            }],
        );
        assert_eq!(callees(&f), ["buried"]);
    }

    #[test]
    fn an_unresolved_nested_call_fails_validation() {
        let f = func(
            "f",
            vec![Stmt::While {
                test: Expr::bool(true),
                body: vec![Stmt::Return(call("nowhere"))],
            }],
        );
        let error = unit_of(vec![f]).validate().expect_err("must not resolve");
        assert!(error.message().contains("nowhere"));
    }

    #[test]
    fn an_assignment_value_is_walked() {
        let f = func(
            "f",
            vec![Stmt::Assign {
                ty: Ty::Int,
                name: "x".into(),
                value: call("source"),
            }],
        );
        assert_eq!(callees(&f), ["source"]);
    }
}

mod artifact {
    use super::*;

    fn round_trip(unit: &Unit) -> Unit {
        Unit::from_json(&unit.to_json().unwrap()).unwrap()
    }

    #[test]
    fn every_form_survives_a_round_trip() {
        let unit = unit_of(vec![func(
            "everything",
            vec![
                Stmt::If {
                    test: Expr::bool(true),
                    then: vec![Stmt::Break],
                    otherwise: vec![Stmt::Continue],
                },
                Stmt::While {
                    test: Expr::bool(false),
                    body: vec![Stmt::Assign {
                        ty: Ty::Int,
                        name: "x".into(),
                        value: Expr::int(1),
                    }],
                },
                Stmt::For {
                    name: "i".into(),
                    ty: Ty::Int,
                    iter: Expr::Range {
                        start: Box::new(Expr::int(0)),
                        stop: Box::new(Expr::int(3)),
                        step: Box::new(Expr::int(-1)),
                    },
                    body: vec![Stmt::ReturnUnit],
                },
            ],
        )]);
        assert_eq!(
            unit.get("everything").unwrap().body,
            round_trip(&unit).get("everything").unwrap().body
        );
    }

    #[test]
    fn nesting_survives() {
        let unit = unit_of(vec![func(
            "nested",
            vec![Stmt::While {
                test: Expr::bool(true),
                body: vec![Stmt::If {
                    test: Expr::bool(true),
                    then: vec![Stmt::While {
                        test: Expr::bool(false),
                        body: vec![Stmt::Continue],
                    }],
                    otherwise: Vec::new(),
                }],
            }],
        )]);
        assert_eq!(
            unit.get("nested").unwrap().body,
            round_trip(&unit).get("nested").unwrap().body
        );
    }

    #[test]
    fn the_artifact_names_no_target_syntax() {
        let unit = unit_of(vec![func(
            "f",
            vec![Stmt::While {
                test: Expr::bool(true),
                body: vec![Stmt::Break],
            }],
        )]);
        let json = unit.to_json().unwrap();
        for spelling in ["loop {", "if !", "i64", "usize"] {
            assert!(!json.contains(spelling), "artifact leaks `{spelling}`");
        }
    }

    #[test]
    fn the_fingerprint_survives() {
        let unit = unit_of(vec![func(
            "f",
            vec![Stmt::For {
                name: "i".into(),
                ty: Ty::Int,
                iter: Expr::Range {
                    start: Box::new(Expr::int(0)),
                    stop: Box::new(Expr::int(3)),
                    step: Box::new(Expr::int(1)),
                },
                body: Vec::new(),
            }],
        )]);
        assert_eq!(unit.fingerprint(), round_trip(&unit).fingerprint());
    }
}
