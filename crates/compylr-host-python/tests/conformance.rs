//! A corpus of IR units every implemented backend must render.
//!
//! Authored as IR, not as Python. That is the whole point: a backend's job is to render the IR,
//! and a corpus written in one source language can only cover what that language happens to
//! produce. `frontends/python/fixtures/accepted/` is a good test of the Python *frontend* and a poor test
//! of a backend, because a tree Python cannot express is a tree the corpus would never contain —
//! and the modes Python cannot write were exactly where a backend could have been silently wrong.
//!
//! Backends are enumerated from the registry rather than listed here. A hand-maintained list is
//! how the fixture lists in this repo once drifted and hid a real defect; deriving it means a
//! backend added tomorrow is covered today.

use std::collections::BTreeMap;

use compylr_diagnostics::span::Span;
use compylr_ir::{
    Attribute, Axis, Behavior, BinOp, Checked, Class, DivMode, Expr, Function, IndexOrigin,
    Literal, Param, RemSign, Rounding, Stance, Stmt, TextUnits, Ty, Unit,
};

fn param(name: &str, ty: Ty) -> Param {
    Param {
        name: name.to_string(),
        ty,
    }
}

fn function(name: &str, params: Vec<Param>, ret: Ty, body: Vec<Stmt>) -> Function {
    Function {
        name: name.to_string(),
        params,
        ret,
        body,
        doc: None,
        span: Span::default(),
    }
}

fn int(value: i64) -> Expr {
    Expr::Literal(Literal::Int(value))
}

fn binary(op: BinOp, left: Expr, right: Expr) -> Expr {
    Expr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn unit_of(functions: Vec<Function>, classes: Vec<Class>) -> Unit {
    let mut unit = Unit::new();
    for class in classes {
        unit.add_class(class).expect("corpus names must be unique");
    }
    for function in functions {
        unit.add_function(function)
            .expect("corpus names must be unique");
    }
    unit
}

/// Every scalar and collection type, in a signature and in a binding.
fn types() -> Unit {
    unit_of(
        vec![
            function(
                "scalars",
                vec![
                    param("i", Ty::Int),
                    param("f", Ty::Float),
                    param("b", Ty::Bool),
                    param("s", Ty::Str),
                ],
                Ty::Str,
                vec![Stmt::Return(Expr::name("s"))],
            ),
            function("nothing", vec![], Ty::Unit, vec![Stmt::ReturnUnit]),
            function(
                "collections",
                vec![
                    param("xs", Ty::List(Box::new(Ty::Int))),
                    param(
                        "d",
                        Ty::Dict(Box::new(Ty::Str), Box::new(Ty::List(Box::new(Ty::Int)))),
                    ),
                    param("st", Ty::Set(Box::new(Ty::Str))),
                    param("t", Ty::Tuple(vec![Ty::Int, Ty::Str])),
                ],
                Ty::Int,
                vec![Stmt::Return(Expr::Len {
                    value: Box::new(Expr::name("xs")),
                    units: TextUnits::CodePoints,
                })],
            ),
        ],
        vec![],
    )
}

/// Every arithmetic and comparison form, including the modes no Python program can produce.
fn operators() -> Unit {
    let mut body = vec![Stmt::Bind {
        name: "acc".to_string(),
        ty: Ty::Int,
        value: binary(
            BinOp::Add {
                checked: Checked::Reported,
            },
            Expr::name("a"),
            Expr::name("b"),
        ),
    }];
    for op in [
        BinOp::Sub {
            checked: Checked::Reported,
        },
        BinOp::Mul {
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
        body.push(Stmt::Assign {
            name: "acc".to_string(),
            ty: Ty::Int,
            value: binary(op, Expr::name("acc"), Expr::name("b")),
        });
    }
    body.push(Stmt::Return(Expr::name("acc")));

    let mut comparisons = Vec::new();
    for (index, op) in [
        BinOp::Eq,
        BinOp::NotEq,
        BinOp::Lt,
        BinOp::LtE,
        BinOp::Gt,
        BinOp::GtE,
    ]
    .into_iter()
    .enumerate()
    {
        comparisons.push(Stmt::Bind {
            name: format!("c{index}"),
            ty: Ty::Bool,
            value: binary(op, Expr::name("a"), Expr::name("b")),
        });
    }
    comparisons.push(Stmt::Return(Expr::Not(Box::new(Expr::name("c0")))));

    unit_of(
        vec![
            function(
                "arithmetic",
                vec![param("a", Ty::Int), param("b", Ty::Int)],
                Ty::Int,
                body,
            ),
            function(
                "comparisons",
                vec![param("a", Ty::Int), param("b", Ty::Int)],
                Ty::Bool,
                comparisons,
            ),
            function(
                "promotion",
                vec![param("a", Ty::Int), param("b", Ty::Int)],
                Ty::Float,
                vec![Stmt::Return(binary(
                    BinOp::Div {
                        mode: DivMode::Exact,
                        checked: Checked::Reported,
                    },
                    Expr::name("a").to_float(),
                    Expr::name("b").to_float(),
                ))],
            ),
            function(
                "negation",
                vec![param("a", Ty::Int)],
                Ty::Int,
                vec![Stmt::Return(Expr::Neg {
                    value: Box::new(Expr::name("a")),
                    checked: Checked::Reported,
                })],
            ),
            function(
                "literals",
                vec![],
                Ty::Float,
                vec![
                    Stmt::Bind {
                        name: "flag".to_string(),
                        ty: Ty::Bool,
                        value: Expr::Literal(Literal::Bool(true)),
                    },
                    Stmt::Bind {
                        name: "text".to_string(),
                        ty: Ty::Str,
                        value: Expr::Literal(Literal::Str("corpus".to_string())),
                    },
                    Stmt::Return(Expr::Literal(Literal::float(1.5))),
                ],
            ),
        ],
        vec![],
    )
}

/// Every collection literal, read, membership test, and mutation.
fn collections() -> Unit {
    unit_of(
        vec![
            function(
                "literals",
                vec![],
                Ty::Int,
                vec![
                    Stmt::Bind {
                        name: "xs".to_string(),
                        ty: Ty::List(Box::new(Ty::Int)),
                        value: Expr::ListLit(vec![int(1), int(2)]),
                    },
                    Stmt::Bind {
                        name: "d".to_string(),
                        ty: Ty::Dict(Box::new(Ty::Str), Box::new(Ty::Int)),
                        value: Expr::DictLit(vec![(
                            Expr::Literal(Literal::Str("k".to_string())),
                            int(1),
                        )]),
                    },
                    Stmt::Bind {
                        name: "st".to_string(),
                        ty: Ty::Set(Box::new(Ty::Int)),
                        value: Expr::SetLit(vec![int(3)]),
                    },
                    Stmt::Bind {
                        name: "t".to_string(),
                        ty: Ty::Tuple(vec![Ty::Int, Ty::Str]),
                        value: Expr::TupleLit(vec![
                            int(4),
                            Expr::Literal(Literal::Str("s".to_string())),
                        ]),
                    },
                    Stmt::Append {
                        sequence: Expr::name("xs"),
                        value: int(5),
                    },
                    Stmt::SetItem {
                        collection: Expr::name("d"),
                        index: Expr::Literal(Literal::Str("k".to_string())),
                        value: int(6),
                    },
                    Stmt::Bind {
                        name: "present".to_string(),
                        ty: Ty::Bool,
                        value: Expr::Contains {
                            value: Box::new(int(1)),
                            container: Box::new(Expr::name("xs")),
                        },
                    },
                    Stmt::Bind {
                        name: "first".to_string(),
                        ty: Ty::Int,
                        value: Expr::Subscript {
                            base: Box::new(Expr::name("xs")),
                            index: Box::new(int(0)),
                            origin: IndexOrigin::FromEitherEnd,
                            checked: Checked::Reported,
                        },
                    },
                    Stmt::Bind {
                        name: "second".to_string(),
                        ty: Ty::Int,
                        value: Expr::TupleIndex {
                            base: Box::new(Expr::name("t")),
                            position: 0,
                        },
                    },
                    Stmt::Return(binary(
                        BinOp::Add {
                            checked: Checked::Reported,
                        },
                        Expr::name("first"),
                        Expr::name("second"),
                    )),
                ],
            ),
            function(
                "iterate",
                vec![param("xs", Ty::List(Box::new(Ty::Int)))],
                Ty::Int,
                vec![
                    Stmt::Bind {
                        name: "total".to_string(),
                        ty: Ty::Int,
                        value: int(0),
                    },
                    Stmt::For {
                        name: "x".to_string(),
                        ty: Ty::Int,
                        iter: Expr::name("xs"),
                        body: vec![Stmt::Assign {
                            name: "total".to_string(),
                            ty: Ty::Int,
                            value: binary(
                                BinOp::Add {
                                    checked: Checked::Reported,
                                },
                                Expr::name("total"),
                                Expr::name("x"),
                            ),
                        }],
                    },
                    Stmt::Return(Expr::name("total")),
                ],
            ),
        ],
        vec![],
    )
}

/// Every control-flow form, and the counted-iteration expression.
fn control_flow() -> Unit {
    unit_of(
        vec![function(
            "shapes",
            vec![param("n", Ty::Int)],
            Ty::Int,
            vec![
                Stmt::Bind {
                    name: "total".to_string(),
                    ty: Ty::Int,
                    value: int(0),
                },
                Stmt::For {
                    name: "i".to_string(),
                    ty: Ty::Int,
                    iter: Expr::Range {
                        start: Box::new(int(0)),
                        stop: Box::new(Expr::name("n")),
                        step: Box::new(int(2)),
                    },
                    body: vec![
                        Stmt::If {
                            test: binary(BinOp::Lt, Expr::name("i"), int(0)),
                            then: vec![Stmt::Continue],
                            otherwise: vec![Stmt::Assign {
                                name: "total".to_string(),
                                ty: Ty::Int,
                                value: binary(
                                    BinOp::Add {
                                        checked: Checked::Reported,
                                    },
                                    Expr::name("total"),
                                    Expr::name("i"),
                                ),
                            }],
                        },
                        Stmt::If {
                            test: binary(BinOp::Gt, Expr::name("total"), int(100)),
                            then: vec![Stmt::Break],
                            otherwise: vec![],
                        },
                    ],
                },
                Stmt::While {
                    test: binary(BinOp::Lt, Expr::name("total"), int(0)),
                    body: vec![Stmt::Assign {
                        name: "total".to_string(),
                        ty: Ty::Int,
                        value: binary(
                            BinOp::Add {
                                checked: Checked::Reported,
                            },
                            Expr::name("total"),
                            int(1),
                        ),
                    }],
                },
                Stmt::Return(Expr::name("total")),
            ],
        )],
        vec![],
    )
}

/// A class: attributes, a constructor, a mutating method, and every form that reaches one.
fn classes() -> Unit {
    let init = function(
        "__init__",
        vec![param("start", Ty::Int)],
        Ty::Unit,
        vec![
            Stmt::SetAttr {
                object: Expr::name("self"),
                name: "count".to_string(),
                ty: Ty::Int,
                value: Expr::name("start"),
            },
            Stmt::SetAttr {
                object: Expr::name("self"),
                name: "seen".to_string(),
                ty: Ty::List(Box::new(Ty::Int)),
                value: Expr::ListLit(vec![]),
            },
        ],
    );

    let bump = function(
        "bump",
        vec![param("by", Ty::Int)],
        Ty::Unit,
        vec![
            Stmt::SetAttr {
                object: Expr::name("self"),
                name: "count".to_string(),
                ty: Ty::Int,
                value: binary(
                    BinOp::Add {
                        checked: Checked::Reported,
                    },
                    Expr::Attribute {
                        object: Box::new(Expr::name("self")),
                        name: "count".to_string(),
                    },
                    Expr::name("by"),
                ),
            },
            Stmt::Append {
                sequence: Expr::Attribute {
                    object: Box::new(Expr::name("self")),
                    name: "seen".to_string(),
                },
                value: Expr::name("by"),
            },
            Stmt::ReturnUnit,
        ],
    );

    let total = function(
        "total",
        vec![],
        Ty::Int,
        vec![Stmt::Return(Expr::Attribute {
            object: Box::new(Expr::name("self")),
            name: "count".to_string(),
        })],
    );

    let counter = Class {
        name: "Counter".to_string(),
        attributes: vec![
            Attribute {
                name: "count".to_string(),
                ty: Ty::Int,
            },
            Attribute {
                name: "seen".to_string(),
                ty: Ty::List(Box::new(Ty::Int)),
            },
        ],
        init,
        methods: BTreeMap::from([("bump".to_string(), bump), ("total".to_string(), total)]),
        doc: None,
        span: Span::default(),
    };

    unit_of(
        vec![function(
            "use_counter",
            vec![param("n", Ty::Int)],
            Ty::Int,
            vec![
                Stmt::Bind {
                    name: "c".to_string(),
                    ty: Ty::Instance("Counter".to_string()),
                    value: Expr::Construct {
                        class: "Counter".to_string(),
                        args: vec![int(0)],
                    },
                },
                // A method call whose value is discarded: the form a backend must emit as a
                // statement rather than as an expression.
                Stmt::Effect(Expr::MethodCall {
                    receiver: Box::new(Expr::name("c")),
                    class: Some("Counter".to_string()),
                    method: "bump".to_string(),
                    args: vec![Expr::name("n")],
                }),
                Stmt::Return(Expr::MethodCall {
                    receiver: Box::new(Expr::name("c")),
                    class: Some("Counter".to_string()),
                    method: "total".to_string(),
                    args: vec![],
                }),
            ],
        )],
        vec![counter],
    )
}

/// A call between two functions in one unit, and a documented function.
fn calls() -> Unit {
    unit_of(
        vec![
            function(
                "helper",
                vec![param("n", Ty::Int)],
                Ty::Int,
                vec![Stmt::Return(binary(
                    BinOp::Mul {
                        checked: Checked::Reported,
                    },
                    Expr::name("n"),
                    int(2),
                ))],
            ),
            Function {
                doc: Some("Documented, so a backend that carries docs is exercised.".to_string()),
                ..function(
                    "caller",
                    vec![param("n", Ty::Int)],
                    Ty::Int,
                    vec![Stmt::Return(Expr::Call {
                        callee: "helper".to_string(),
                        args: vec![Expr::name("n")],
                    })],
                )
            },
        ],
        vec![],
    )
}

/// Every statement form, in every position a backend renders separately.
///
/// The corpus above covers each form once, which is what let the constructor defect through:
/// `Stmt::ReturnUnit` was already exercised — in a *function* — while the bug was its behaviour in
/// a **constructor**, which `emit_constructor` renders through a bespoke path. Forms are not the
/// unit of coverage here; `(form, position)` pairs are.
fn positions() -> Unit {
    let discard = |name: &str| {
        Stmt::Effect(Expr::Call {
            callee: name.to_string(),
            args: vec![],
        })
    };

    // Every form legal at the top of a body that has a receiver, plus a loop holding the two that
    // are legal nowhere else.
    let body_with_receiver = |returning: Option<Expr>| {
        let mut body = vec![
            Stmt::SetAttr {
                object: Expr::name("self"),
                name: "count".to_string(),
                ty: Ty::Int,
                value: int(1),
            },
            Stmt::Bind {
                name: "local".to_string(),
                ty: Ty::Int,
                value: int(0),
            },
            Stmt::Assign {
                name: "local".to_string(),
                ty: Ty::Int,
                value: int(2),
            },
            Stmt::Bind {
                name: "xs".to_string(),
                ty: Ty::List(Box::new(Ty::Int)),
                value: Expr::ListLit(vec![int(1)]),
            },
            Stmt::Bind {
                name: "d".to_string(),
                ty: Ty::Dict(Box::new(Ty::Int), Box::new(Ty::Int)),
                value: Expr::DictLit(vec![]),
            },
            Stmt::Append {
                sequence: Expr::name("xs"),
                value: int(3),
            },
            Stmt::SetItem {
                collection: Expr::name("d"),
                index: int(1),
                value: int(1),
            },
            discard("effectful"),
            Stmt::If {
                test: binary(BinOp::Lt, Expr::name("local"), int(9)),
                then: vec![Stmt::Assign {
                    name: "local".to_string(),
                    ty: Ty::Int,
                    value: int(3),
                }],
                otherwise: vec![],
            },
            Stmt::While {
                test: binary(BinOp::Lt, Expr::name("local"), int(0)),
                body: vec![Stmt::Break],
            },
            // The loop body: every form again, including the two that are legal only here.
            Stmt::For {
                name: "i".to_string(),
                ty: Ty::Int,
                iter: Expr::Range {
                    start: Box::new(int(0)),
                    stop: Box::new(int(3)),
                    step: Box::new(int(1)),
                },
                body: vec![
                    Stmt::SetAttr {
                        object: Expr::name("self"),
                        name: "count".to_string(),
                        ty: Ty::Int,
                        value: Expr::name("i"),
                    },
                    Stmt::Bind {
                        name: "inner".to_string(),
                        ty: Ty::Int,
                        value: Expr::name("i"),
                    },
                    Stmt::Assign {
                        name: "local".to_string(),
                        ty: Ty::Int,
                        value: Expr::name("inner"),
                    },
                    Stmt::Append {
                        sequence: Expr::name("xs"),
                        value: Expr::name("i"),
                    },
                    Stmt::SetItem {
                        collection: Expr::name("d"),
                        index: Expr::name("i"),
                        value: int(0),
                    },
                    discard("effectful"),
                    Stmt::While {
                        test: binary(BinOp::Lt, Expr::name("local"), int(0)),
                        body: vec![Stmt::Break],
                    },
                    Stmt::For {
                        name: "j".to_string(),
                        ty: Ty::Int,
                        iter: Expr::Range {
                            start: Box::new(int(0)),
                            stop: Box::new(int(1)),
                            step: Box::new(int(1)),
                        },
                        body: vec![Stmt::Continue],
                    },
                    Stmt::If {
                        test: binary(BinOp::Gt, Expr::name("i"), int(1)),
                        then: vec![Stmt::Break],
                        otherwise: vec![Stmt::Continue],
                    },
                ],
            },
        ];
        // A constructor may not return at all, so `None` simply ends the body.
        if let Some(value) = returning {
            body.push(Stmt::Return(value));
        }
        body
    };

    // A free function: the same, minus the attribute assignment it has no receiver for.
    let free_body = {
        let mut body = body_with_receiver(Some(int(0)));
        body.retain(|stmt| !matches!(stmt, Stmt::SetAttr { .. }));
        if let Some(Stmt::For {
            body: loop_body, ..
        }) = body.iter_mut().find(|s| matches!(s, Stmt::For { .. }))
        {
            loop_body.retain(|stmt| !matches!(stmt, Stmt::SetAttr { .. }));
        }
        body
    };

    let class = Class {
        name: "Positions".to_string(),
        attributes: vec![Attribute {
            name: "count".to_string(),
            ty: Ty::Int,
        }],
        init: function("__init__", vec![], Ty::Unit, body_with_receiver(None)),
        methods: BTreeMap::from([
            // A mutating receiver: it assigns an attribute.
            (
                "mutate".to_string(),
                function("mutate", vec![], Ty::Unit, body_with_receiver(None)),
            ),
            // A shared receiver: it only reads.
            (
                "read".to_string(),
                function(
                    "read",
                    vec![],
                    Ty::Int,
                    vec![Stmt::Return(Expr::Attribute {
                        object: Box::new(Expr::name("self")),
                        name: "count".to_string(),
                    })],
                ),
            ),
        ]),
        doc: None,
        span: Span::default(),
    };

    // Returning from inside a loop, in both flavours. Kept out of the shared body above because a
    // constructor may not return at all, and that body is used for one.
    let returning_loop = |returned: Option<Expr>| Stmt::For {
        name: "k".to_string(),
        ty: Ty::Int,
        iter: Expr::Range {
            start: Box::new(int(0)),
            stop: Box::new(int(2)),
            step: Box::new(int(1)),
        },
        body: vec![Stmt::If {
            test: binary(BinOp::Gt, Expr::name("k"), int(0)),
            then: vec![match returned {
                Some(value) => Stmt::Return(value),
                None => Stmt::ReturnUnit,
            }],
            otherwise: vec![],
        }],
    };

    let mut free_body = free_body;
    free_body.insert(free_body.len() - 1, returning_loop(Some(int(7))));

    unit_of(
        vec![
            function(
                "effectful",
                vec![],
                Ty::Unit,
                vec![returning_loop(None), Stmt::ReturnUnit],
            ),
            function("everywhere", vec![], Ty::Int, free_body),
        ],
        vec![class],
    )
}

/// The corpus, by name.
/// One binding per mode-carrying form, under one language's stance.
///
/// Named by `tag` so the two stances can sit side by side in one body without colliding, which is
/// what lets a single entry cover both halves of every axis in every position.
fn stance_bindings(tag: &str, behavior: Behavior) -> Vec<Stmt> {
    let named = |what: &str| format!("{what}_{tag}");
    let int_bind = |what: &str, value: Expr| Stmt::Bind {
        name: named(what),
        ty: Ty::Int,
        value,
    };

    vec![
        // The overflow axis, on all four operations that carry it.
        int_bind(
            "sum",
            binary(
                BinOp::Add {
                    checked: behavior.arithmetic(),
                },
                Expr::name("a"),
                Expr::name("b"),
            ),
        ),
        int_bind(
            "diff",
            binary(
                BinOp::Sub {
                    checked: behavior.arithmetic(),
                },
                Expr::name("a"),
                Expr::name("b"),
            ),
        ),
        int_bind(
            "prod",
            binary(
                BinOp::Mul {
                    checked: behavior.arithmetic(),
                },
                Expr::name("a"),
                Expr::name("b"),
            ),
        ),
        int_bind(
            "neg",
            Expr::Neg {
                value: Box::new(Expr::name("a")),
                checked: behavior.arithmetic(),
            },
        ),
        int_bind(
            "quot",
            binary(
                behavior.integer_division(),
                Expr::name("a"),
                Expr::name("b"),
            ),
        ),
        int_bind(
            "rem",
            binary(behavior.remainder(), Expr::name("a"), Expr::name("b")),
        ),
        int_bind(
            "elem",
            Expr::Subscript {
                base: Box::new(Expr::name("xs")),
                index: Box::new(int(0)),
                origin: behavior.index_origin(),
                checked: behavior.index_checked(),
            },
        ),
        int_bind(
            "size",
            Expr::Len {
                value: Box::new(Expr::name("s")),
                units: behavior.text_units(),
            },
        ),
        // Exact division promotes both operands, so this one is a float.
        Stmt::Bind {
            name: named("exact"),
            ty: Ty::Float,
            value: binary(
                behavior.exact_division(),
                Expr::to_float(Expr::name("a")),
                Expr::to_float(Expr::name("b")),
            ),
        },
    ]
}

/// Both languages' stances on every axis, in every position a body can be.
///
/// The corpus was written entirely under the source language's stance, which made it a good test
/// of one half of every axis and no test at all of the other. A backend arm that handled only
/// `Reported` would have rendered every entry correctly and been wrong for every program that
/// asked for the target's meaning.
///
/// Scoped as design D15 states: the forms that *carry* a mode, in every position they are legal
/// in, under both stances of the axis governing each. The full cross product with every
/// statement form is neither necessary nor affordable — a form with no mode is unaffected by
/// behavior by construction, and a test asserting that would be asserting the absence of a field.
fn stances() -> Unit {
    let python = Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR);
    let rust = Behavior::of(
        compylr_registry::backends::lookup("rust")
            .unwrap()
            .behavior(),
    );

    // Both stances, then the same again inside a loop: four positions in total once the class
    // below repeats the pair in a constructor and a method.
    let both = |suffix: &str| {
        let mut body = stance_bindings(&format!("py{suffix}"), python);
        body.extend(stance_bindings(&format!("rs{suffix}"), rust));
        body
    };

    let with_loop = || {
        let mut body = both("");
        body.push(Stmt::For {
            name: "i".to_string(),
            ty: Ty::Int,
            iter: Expr::Range {
                start: Box::new(int(0)),
                stop: Box::new(int(2)),
                step: Box::new(int(1)),
            },
            body: both("_in_loop"),
        });
        body
    };

    let params = || {
        vec![
            param("a", Ty::Int),
            param("b", Ty::Int),
            param("xs", Ty::List(Box::new(Ty::Int))),
            param("s", Ty::Str),
        ]
    };

    let mut free = with_loop();
    free.push(Stmt::Return(int(0)));

    let mut init_body = with_loop();
    init_body.push(Stmt::SetAttr {
        object: Expr::name("self"),
        name: "seen".to_string(),
        ty: Ty::Int,
        value: int(0),
    });

    let mut method_body = with_loop();
    method_body.push(Stmt::Return(int(0)));

    let class = Class {
        name: "Stances".to_string(),
        attributes: vec![Attribute {
            name: "seen".to_string(),
            ty: Ty::Int,
        }],
        init: function("__init__", params(), Ty::Unit, init_body),
        methods: BTreeMap::from([(
            "measure".to_string(),
            function("measure", params(), Ty::Int, method_body),
        )]),
        doc: None,
        span: Span::default(),
    };

    unit_of(
        vec![function("both_stances", params(), Ty::Int, free)],
        vec![class],
    )
}

fn corpus() -> Vec<(&'static str, Unit)> {
    vec![
        ("types", types()),
        ("operators", operators()),
        ("collections", collections()),
        ("control-flow", control_flow()),
        ("classes", classes()),
        ("calls", calls()),
        ("positions", positions()),
        ("stances", stances()),
    ]
}

#[test]
fn every_corpus_entry_is_well_formed() {
    for (name, unit) in corpus() {
        compylr_core::verify::verify(&unit)
            .unwrap_or_else(|error| panic!("corpus entry '{name}' is malformed: {error}"));
    }
}

/// Every implemented backend must render every entry.
///
/// The backend list comes from the registry, never from a list here. Adding a target tomorrow
/// puts it under this test today, which is the only version of "the corpus covers every backend"
/// that stays true without somebody remembering.
#[test]
fn every_implemented_backend_renders_the_whole_corpus() {
    let backends = compylr_registry::backends::implemented_names();
    assert!(!backends.is_empty(), "at least one backend must compile");

    for backend_name in &backends {
        let backend =
            compylr_registry::backends::lookup(backend_name).expect("the registry listed it");
        for (name, unit) in corpus() {
            let files = backend.emit(&unit).unwrap_or_else(|error| {
                panic!("the '{backend_name}' backend cannot render corpus entry '{name}': {error}")
            });
            assert!(
                !files.is_empty(),
                "'{backend_name}' rendered '{name}' as no files at all"
            );
        }
    }
}

/// Rendering is not enough: the result has to build.
///
/// A backend can emit plausible text for a form it does not really support — that is exactly how
/// tuple indexing once shipped emitting a call to a helper that did not exist. Only handing the
/// output to the target's own compiler settles it.
#[test]
fn every_corpus_entry_compiles_for_the_rust_backend() {
    use std::process::Command;

    let backend = compylr_registry::backends::lookup("rust").expect("the shipped backend");
    for (name, unit) in corpus() {
        let files = backend.post_process(backend.emit(&unit).expect("must emit"));
        let dir =
            std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("conf_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        for (relative, contents) in &files {
            let path = dir.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).expect("scratch directory");
            std::fs::write(&path, contents).expect("write generated source");
        }

        let output = Command::new("rustc")
            .args(["--edition", "2024", "--crate-type", "lib", "-o"])
            .arg(dir.join("libcorpus.rlib"))
            .arg(dir.join("src/lib.rs"))
            .output()
            .expect("rustc ships with the toolchain running these tests");
        assert!(
            output.status.success(),
            "corpus entry '{name}' emitted Rust that does not compile:\n{}\n--- generated ---\n{}",
            String::from_utf8_lossy(&output.stderr),
            files["src/generated.rs"]
        );
    }
}

/// A hand-built unit has no source language, so it must not be blocked by a guarantee check.
#[test]
fn the_corpus_needs_no_source_language() {
    for (name, unit) in corpus() {
        assert!(
            unit.origin().is_none(),
            "corpus entry '{name}' must not claim a frontend"
        );
        for backend_name in compylr_registry::backends::implemented_names() {
            let backend = compylr_registry::backends::lookup(&backend_name).unwrap();
            assert!(compylr_core::negotiation::negotiate(&unit, backend).is_ok());
        }
    }
}

/// Every IR node form must appear somewhere in the corpus.
///
/// Read off the IR's own source rather than from a list, so a variant added tomorrow fails this
/// test until it is covered. The alternative is a corpus that quietly stops keeping pace with the
/// model it exists to exercise — which is how the fixture lists in this repo drifted before.
#[test]
fn the_corpus_covers_every_ir_node_form() {
    let serialized: String = corpus()
        .iter()
        .map(|(_, unit)| unit.to_json().expect("corpus entries serialize"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut missing = Vec::new();
    for variant in ir_variants() {
        // Serde tags externally, so a variant carrying data is a JSON *key* — `"Int":` — and one
        // carrying none is a bare string — `"Int"`. Distinguishing the two matters: `Ty::Int` and
        // `Literal::Int` share a name, and matching on the name alone would let either stand in
        // for the other. The quoting also keeps `Return` from matching `ReturnUnit`.
        let token = if variant.carries_data {
            format!("\"{}\":", variant.name)
        } else {
            format!("\"{}\"", variant.name)
        };
        if !serialized.contains(&token) {
            missing.push(format!("{}::{}", variant.enum_name, variant.name));
        }
    }
    assert!(
        missing.is_empty(),
        "these IR forms are not exercised by any corpus entry: {}",
        missing.join(", ")
    );
}

/// One variant of an IR enum, as declared in the source.
struct IrVariant {
    enum_name: String,
    name: String,
    /// Whether the variant carries a payload, which decides how serde writes it.
    carries_data: bool,
}

/// Where in a unit a statement appears.
///
/// These are the positions a backend renders through *different code*, which is the only
/// distinction that matters for coverage. Methods differ from free functions in their receiver,
/// constructors are rendered by a bespoke path entirely, and a loop body goes through the loop
/// emitters. Two methods differing only in receiver mutability share a statement path, so they are
/// one position here and are covered separately by the corpus holding both kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Position {
    FreeFunction,
    Constructor,
    Method,
    Loop,
}

impl Position {
    fn name(self) -> &'static str {
        match self {
            Self::FreeFunction => "a free function's body",
            Self::Constructor => "a constructor's body",
            Self::Method => "a method's body",
            Self::Loop => "a loop body",
        }
    }

    /// Whether a statement form can legally appear here.
    ///
    /// Coverage is only required where the combination is legal; demanding the impossible would
    /// make the check unsatisfiable rather than informative.
    fn admits(self, form: &str) -> bool {
        match (self, form) {
            // Loop control needs an enclosing loop.
            (Self::Loop, _) => true,
            (_, "Break" | "Continue") => false,
            // A free function has no receiver to assign an attribute of.
            (Self::FreeFunction, "SetAttr") => false,
            // A constructor produces the instance it builds, so it may not return at all.
            (Self::Constructor, "Return" | "ReturnUnit") => false,
            _ => true,
        }
    }
}

/// Every `(statement form, position)` pair the corpus exercises.
fn covered_pairs() -> BTreeMap<String, Vec<Position>> {
    fn walk(stmts: &[Stmt], position: Position, seen: &mut BTreeMap<String, Vec<Position>>) {
        for stmt in stmts {
            let form = statement_form(stmt);
            let positions = seen.entry(form).or_default();
            if !positions.contains(&position) {
                positions.push(position);
            }
            match stmt {
                Stmt::If {
                    then, otherwise, ..
                } => {
                    walk(then, position, seen);
                    walk(otherwise, position, seen);
                }
                Stmt::While { body, .. } | Stmt::For { body, .. } => {
                    walk(body, Position::Loop, seen);
                }
                _ => {}
            }
        }
    }

    let mut seen = BTreeMap::new();
    for (_, unit) in corpus() {
        for function in unit.functions() {
            walk(&function.body, Position::FreeFunction, &mut seen);
        }
        for class in unit.classes() {
            walk(&class.init.body, Position::Constructor, &mut seen);
            for method in class.methods.values() {
                walk(&method.body, Position::Method, &mut seen);
            }
        }
    }
    seen
}

fn statement_form(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Return(_) => "Return",
        Stmt::ReturnUnit => "ReturnUnit",
        Stmt::Bind { .. } => "Bind",
        Stmt::Assign { .. } => "Assign",
        Stmt::Effect(_) => "Effect",
        Stmt::SetAttr { .. } => "SetAttr",
        Stmt::SetItem { .. } => "SetItem",
        Stmt::Append { .. } => "Append",
        Stmt::If { .. } => "If",
        Stmt::While { .. } => "While",
        Stmt::For { .. } => "For",
        Stmt::Break => "Break",
        Stmt::Continue => "Continue",
    }
    .to_string()
}

/// Every statement form must be exercised in every position it is legal in.
///
/// Forms alone are not enough, and the corpus proved it: `Stmt::ReturnUnit` was covered — in a
/// function — while the defect was its behaviour in a **constructor**, which is rendered by its
/// own emitter. Checking pairs rather than forms is what turns "the corpus is complete" from a
/// claim about the IR into a claim about the backend's code paths.
///
/// Running this check for the first time found four defects, three of them reachable from ordinary
/// Python: an attribute assigned below the top level of a constructor, a local reassigned in one,
/// a `continue` in a counted loop that skipped the cursor increment and hung, and a constructor
/// that returned early.
#[test]
fn the_corpus_covers_every_statement_form_in_every_position_it_is_legal_in() {
    let covered = covered_pairs();
    let positions = [
        Position::FreeFunction,
        Position::Constructor,
        Position::Method,
        Position::Loop,
    ];

    let forms: Vec<String> = ir_variants()
        .into_iter()
        .filter(|variant| variant.enum_name == "Stmt")
        .map(|variant| variant.name)
        .collect();
    assert!(forms.len() >= 13, "the statement scan has probably broken");

    let mut missing = Vec::new();
    for form in &forms {
        for position in positions {
            if !position.admits(form) {
                continue;
            }
            let seen = covered.get(form).is_some_and(|at| at.contains(&position));
            if !seen {
                missing.push(format!("Stmt::{form} in {}", position.name()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "these (form, position) pairs are not exercised by any corpus entry:\n  {}",
        missing.join("\n  ")
    );
}

/// Which axis governs a node, and which language's stance it declares.
///
/// `None` for a node no axis governs, and — deliberately — for a mode *combination* that is
/// neither language's. `Div { mode: Integer(TowardNegInf), checked: Unchecked }` is a real and
/// reachable node, and it is not Python's stance and not Rust's; counting it as either would
/// report coverage the corpus does not have.
fn declared_stance(
    expr: &Expr,
    python: &Behavior,
    rust: &Behavior,
) -> Option<(Axis, &'static str)> {
    let axis = match expr {
        Expr::Neg { .. } => Axis::IntegerOverflow,
        Expr::Len { .. } => Axis::TextLength,
        Expr::Subscript { .. } => Axis::SequenceIndex,
        Expr::Binary { op, .. } => match op {
            BinOp::Add { .. } | BinOp::Sub { .. } | BinOp::Mul { .. } => Axis::IntegerOverflow,
            BinOp::Div {
                mode: DivMode::Exact,
                ..
            } => Axis::ExactDivision,
            BinOp::Div { .. } => Axis::IntegerDivision,
            BinOp::Rem { .. } => Axis::Remainder,
            _ => return None,
        },
        _ => return None,
    };

    // Compared against the two declarations rather than against literals, so this test keeps
    // asking the same question if a language ever restates its stance.
    let declared = node_stance(expr, axis)?;
    if declared == python.stance(axis) {
        Some((axis, "python"))
    } else if declared == rust.stance(axis) {
        Some((axis, "rust"))
    } else {
        None
    }
}

/// The stance a node declares, as a value comparable against a language's.
fn node_stance(expr: &Expr, axis: Axis) -> Option<Stance> {
    Some(match (expr, axis) {
        (Expr::Neg { checked, .. }, _) => Stance::IntegerOverflow(*checked),
        (Expr::Len { units, .. }, _) => Stance::TextLength(*units),
        (
            Expr::Subscript {
                origin, checked, ..
            },
            _,
        ) => Stance::SequenceIndex(compylr_ir::SequenceIndex {
            origin: *origin,
            checked: *checked,
        }),
        (
            Expr::Binary {
                op: BinOp::Add { checked } | BinOp::Sub { checked } | BinOp::Mul { checked },
                ..
            },
            _,
        ) => Stance::IntegerOverflow(*checked),
        (
            Expr::Binary {
                op:
                    BinOp::Div {
                        mode: DivMode::Exact,
                        checked,
                    },
                ..
            },
            _,
        ) => Stance::ExactDivision(*checked),
        (
            Expr::Binary {
                op:
                    BinOp::Div {
                        mode: DivMode::Integer(rounding),
                        checked,
                    },
                ..
            },
            _,
        ) => Stance::IntegerDivision(compylr_ir::IntegerDivision {
            rounding: *rounding,
            checked: *checked,
        }),
        (
            Expr::Binary {
                op: BinOp::Rem { sign, checked },
                ..
            },
            _,
        ) => Stance::Remainder(compylr_ir::Remainder {
            sign: *sign,
            checked: *checked,
        }),
        _ => return None,
    })
}

/// Every form carrying a mode must be exercised under **both** stances of its axis, in every
/// position a body can be.
///
/// The third dimension design D15 asks for, scoped as it says. Before this, the corpus was
/// written entirely under the source language's stance — so a backend arm handling only
/// `Reported` would have rendered every entry correctly while being wrong for every program that
/// asked for the target's meaning. No test written in Python could have caught that either, since
/// the only frontend reports everything.
#[test]
fn the_corpus_covers_both_stances_of_every_axis_in_every_position() {
    let python = Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR);
    let rust = Behavior::of(
        compylr_registry::backends::lookup("rust")
            .unwrap()
            .behavior(),
    );

    let mut seen: BTreeMap<(Axis, &'static str), Vec<Position>> = BTreeMap::new();
    let mut record = |expr: &Expr, position: Position| {
        if let Some(key) = declared_stance(expr, &python, &rust) {
            let positions = seen.entry(key).or_default();
            if !positions.contains(&position) {
                positions.push(position);
            }
        }
    };

    fn walk(stmts: &[Stmt], position: Position, record: &mut impl FnMut(&Expr, Position)) {
        for stmt in stmts {
            let mut visit = |expr: &Expr| expr.walk(&mut |node| record(node, position));
            match stmt {
                Stmt::Return(expr) | Stmt::Effect(expr) => visit(expr),
                Stmt::Bind { value, .. } | Stmt::Assign { value, .. } => visit(value),
                Stmt::SetAttr { object, value, .. } => {
                    visit(object);
                    visit(value);
                }
                Stmt::SetItem {
                    collection,
                    index,
                    value,
                } => {
                    visit(collection);
                    visit(index);
                    visit(value);
                }
                Stmt::Append { sequence, value } => {
                    visit(sequence);
                    visit(value);
                }
                Stmt::If {
                    test,
                    then,
                    otherwise,
                } => {
                    visit(test);
                    walk(then, position, record);
                    walk(otherwise, position, record);
                }
                Stmt::While { test, body } => {
                    visit(test);
                    walk(body, Position::Loop, record);
                }
                Stmt::For { iter, body, .. } => {
                    visit(iter);
                    walk(body, Position::Loop, record);
                }
                Stmt::ReturnUnit | Stmt::Break | Stmt::Continue => {}
            }
        }
    }

    for (_, unit) in corpus() {
        for function in unit.functions() {
            walk(&function.body, Position::FreeFunction, &mut record);
        }
        for class in unit.classes() {
            walk(&class.init.body, Position::Constructor, &mut record);
            for method in class.methods.values() {
                walk(&method.body, Position::Method, &mut record);
            }
        }
    }

    let positions = [
        Position::FreeFunction,
        Position::Constructor,
        Position::Method,
        Position::Loop,
    ];
    let mut missing = Vec::new();
    for axis in Axis::ALL {
        for language in ["python", "rust"] {
            for position in positions {
                let covered = seen
                    .get(&(axis, language))
                    .is_some_and(|at| at.contains(&position));
                if !covered {
                    missing.push(format!(
                        "{} under {language}'s stance, in {}",
                        axis.code(),
                        position.name()
                    ));
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "these (axis, stance, position) triples are not exercised by any corpus entry:\n  {}",
        missing.join("\n  ")
    );
}

/// Both receiver kinds must be rendered, since they differ in signature rather than in body.
#[test]
fn the_corpus_covers_both_method_receivers() {
    let mut shared = 0;
    let mut mutable = 0;
    for (_, unit) in corpus() {
        let accesses = compylr_backend_rust::instance_parameter_accesses(&unit);
        for class in unit.classes() {
            let mutating = compylr_backend_rust::rust::mutating_methods(class, &accesses);
            for name in class.methods.keys() {
                if mutating.contains(name) {
                    mutable += 1;
                } else {
                    shared += 1;
                }
            }
        }
    }
    assert!(mutable > 0, "no method takes a mutable receiver");
    assert!(shared > 0, "no method takes a shared receiver");
}

/// The variants of the IR's four central enums, read from its source.
fn ir_variants() -> Vec<IrVariant> {
    let source = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("the crate lives at <root>/crates/<name>")
            .join("crates/compylr-ir/src/ir.rs"),
    )
    .expect("the IR source must be readable");

    let mut found = Vec::new();
    let mut current: Option<String> = None;
    for line in source.lines() {
        if let Some(rest) = line.strip_prefix("pub enum ") {
            let name = rest.trim_end_matches(" {").trim();
            current = ["Ty", "Expr", "Stmt", "Literal"]
                .contains(&name)
                .then(|| name.to_string());
            continue;
        }
        if line == "}" {
            current = None;
            continue;
        }
        let Some(enum_name) = &current else { continue };
        // Variants are indented exactly four spaces and start with a capital.
        let Some(rest) = line.strip_prefix("    ") else {
            continue;
        };
        if rest.starts_with(' ') || !rest.starts_with(|c: char| c.is_ascii_uppercase()) {
            continue;
        }
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            continue;
        }
        let after = rest[name.len()..].trim_start();
        found.push(IrVariant {
            enum_name: enum_name.clone(),
            name,
            carries_data: after.starts_with('(') || after.starts_with('{'),
        });
    }

    assert!(
        found.len() > 30,
        "the variant scan found only {} forms, so it has probably broken",
        found.len()
    );
    found
}

/// The pieces a caller reaches that nothing else exercises.
///
/// `pairs()` on each registry answers "what can compylr do?", which is a question a CLI or a
/// diagnostic asks and no other test does.
#[test]
fn every_registry_can_enumerate_itself() {
    let backends = compylr_registry::backends::names();
    let frontends = compylr_registry::frontends::names();
    let bridges = compylr_registry::bridges::pairs();
    let directed = compylr_registry::passes::pairs();

    assert!(backends.contains(&"rust"));
    assert!(frontends.contains(&"python"));
    assert!(bridges.contains(&("python".to_string(), "rust".to_string())));
    // Empty, and asserted as such: a directed pass registered without anyone noticing would
    // change what runs for a pair, and this is the only place that would say so.
    assert!(
        directed.is_empty(),
        "a pair-directed pass appeared without a test: {directed:?}"
    );

    // Every bridged pair must name languages both registries know, or the bridge is unreachable.
    for (source, target) in &bridges {
        assert!(frontends.contains(&source.as_str()), "{source}");
        assert!(backends.contains(&target.as_str()), "{target}");
    }
}
