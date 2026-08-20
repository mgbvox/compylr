//! Classes in the IR: a second kind of unit member, and the first nominal type in the model.
//!
//! Two properties carry disproportionate weight. A unit containing no classes must fingerprint
//! exactly as it did before, or every cached build in every project invalidates on upgrade for no
//! reason. And `walk_calls` must reach into attribute objects, construction arguments, and method
//! receivers, or a call hidden in one escapes unit validation.

use compylr::ir::{Attribute, Class, Expr, Function, Param, Stmt, Ty, Unit};
use compylr::span::Span;
use compylr_frontend_python::PythonTypeName;

fn func(name: &str, params: Vec<Param>, ret: Ty, body: Vec<Stmt>) -> Function {
    Function {
        name: name.into(),
        params,
        ret,
        body,
        doc: None,
        span: Span::default(),
    }
}

fn param(name: &str, ty: Ty) -> Param {
    Param {
        name: name.into(),
        ty,
    }
}

/// A class with one integer attribute and whatever methods are given.
fn counter(name: &str, methods: Vec<Function>) -> Class {
    Class {
        name: name.into(),
        attributes: vec![Attribute {
            name: "count".into(),
            ty: Ty::Int,
        }],
        init: func(
            "__init__",
            Vec::new(),
            Ty::Unit,
            vec![Stmt::SetAttr {
                object: Expr::name("self"),
                name: "count".into(),
                ty: Ty::Int,
                value: Expr::int(0),
            }],
        ),
        methods: methods.into_iter().map(|m| (m.name.clone(), m)).collect(),
        doc: None,
        span: Span::default(),
    }
}

fn call(name: &str) -> Expr {
    Expr::Call {
        callee: name.into(),
        args: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Units hold classes
// ---------------------------------------------------------------------------

#[test]
fn a_class_can_be_added_to_a_unit() {
    let mut unit = Unit::new();
    unit.add_class(counter("Counter", Vec::new())).unwrap();
    assert_eq!(unit.classes().count(), 1);
    assert!(unit.class("Counter").is_some());
}

#[test]
fn a_name_colliding_with_a_function_is_refused_in_both_directions() {
    // They compile into one file and one module, so a collision would be a Rust collision. Caught
    // in the unit, it is a diagnostic instead.
    let mut unit = Unit::new();
    unit.add_function(func(
        "Thing",
        Vec::new(),
        Ty::Int,
        vec![Stmt::Return(Expr::int(1))],
    ))
    .unwrap();
    assert!(
        unit.add_class(counter("Thing", Vec::new())).is_err(),
        "a class must not take a function's name"
    );

    let mut other = Unit::new();
    other.add_class(counter("Thing", Vec::new())).unwrap();
    assert!(
        other
            .add_function(func(
                "Thing",
                Vec::new(),
                Ty::Int,
                vec![Stmt::Return(Expr::int(1))]
            ))
            .is_err(),
        "a function must not take a class's name"
    );
}

#[test]
fn a_duplicate_class_is_refused() {
    let mut unit = Unit::new();
    unit.add_class(counter("Counter", Vec::new())).unwrap();
    assert!(unit.add_class(counter("Counter", Vec::new())).is_err());
}

#[test]
fn a_unit_with_no_classes_fingerprints_exactly_as_before() {
    // Load-bearing: if an empty class map contributed to the hash, every cached build would
    // invalidate on upgrade and nobody would know why.
    let mut unit = Unit::new();
    unit.add_function(func(
        "f",
        vec![param("a", Ty::Int)],
        Ty::Int,
        vec![Stmt::Return(Expr::name("a"))],
    ))
    .unwrap();
    // The value pinned here was produced before classes existed. It must not move.
    assert_eq!(
        format!("{:016x}", unit.fingerprint()),
        FUNCTION_ONLY_FINGERPRINT,
        "adding classes to the model must not move a class-free unit's fingerprint"
    );
}

/// Fingerprint of a one-function unit, recorded before classes were added to the model.
const FUNCTION_ONLY_FINGERPRINT: &str = "42920571d4bba791";

#[test]
fn adding_a_class_moves_the_fingerprint() {
    let mut without = Unit::new();
    without
        .add_function(func(
            "f",
            Vec::new(),
            Ty::Int,
            vec![Stmt::Return(Expr::int(1))],
        ))
        .unwrap();
    let mut with = Unit::new();
    with.add_function(func(
        "f",
        Vec::new(),
        Ty::Int,
        vec![Stmt::Return(Expr::int(1))],
    ))
    .unwrap();
    with.add_class(counter("Counter", Vec::new())).unwrap();
    assert_ne!(without.fingerprint(), with.fingerprint());
}

#[test]
fn changing_a_method_body_moves_the_fingerprint() {
    let reads = func(
        "get",
        Vec::new(),
        Ty::Int,
        vec![Stmt::Return(Expr::Attribute {
            object: Box::new(Expr::name("self")),
            name: "count".into(),
        })],
    );
    let constant = func("get", Vec::new(), Ty::Int, vec![Stmt::Return(Expr::int(0))]);

    let mut a = Unit::new();
    a.add_class(counter("Counter", vec![reads])).unwrap();
    let mut b = Unit::new();
    b.add_class(counter("Counter", vec![constant])).unwrap();
    assert_ne!(a.fingerprint(), b.fingerprint());
}

#[test]
fn class_order_is_content_determined() {
    let build = |first: &str, second: &str| {
        let mut unit = Unit::new();
        unit.add_class(counter(first, Vec::new())).unwrap();
        unit.add_class(counter(second, Vec::new())).unwrap();
        unit
    };
    assert_eq!(
        build("A", "B").fingerprint(),
        build("B", "A").fingerprint(),
        "decoration order must not invalidate a cached build"
    );
    assert_eq!(
        build("A", "B")
            .classes()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>(),
        build("B", "A")
            .classes()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Instance types
// ---------------------------------------------------------------------------

#[test]
fn two_classes_are_distinct_types() {
    // The model's one nominal type: equality is by name, not by shape. Two classes with identical
    // attributes are different types, which is what a user means by writing two classes.
    assert_ne!(
        Ty::Instance("A".into()),
        Ty::Instance("B".into()),
        "instance types compare by name"
    );
    assert_eq!(Ty::Instance("A".into()), Ty::Instance("A".into()));
}

#[test]
fn an_instance_nests_in_collections() {
    let nested = Ty::List(Box::new(Ty::Instance("Counter".into())));
    assert_eq!(nested.python_name(), "list[Counter]");
}

#[test]
fn an_instance_cannot_be_a_key_or_element() {
    // No defined hash or ordering, and no reason to invent one.
    assert!(!Ty::Instance("Counter".into()).can_key());
}

#[test]
fn an_instance_is_not_trivially_copyable() {
    // So the existing clone-where-consumed rule applies to it unchanged.
    assert!(!Ty::Instance("Counter".into()).is_trivially_copyable());
}

#[test]
fn an_instance_is_not_numeric() {
    assert!(!Ty::Instance("Counter".into()).is_numeric());
}

// ---------------------------------------------------------------------------
// The new forms
// ---------------------------------------------------------------------------

#[test]
fn construction_is_distinct_from_a_call() {
    // Leaving it a call would mean unit validation resolving it against functions, and the type
    // rules differ enough -- arguments check against `__init__`, the result is an instance -- that
    // one form for both would make each path carry the other's cases.
    let constructed = Expr::Construct {
        class: "Counter".into(),
        args: Vec::new(),
    };
    assert!(!matches!(constructed, Expr::Call { .. }));
}

#[test]
fn attribute_read_and_assignment_are_representable() {
    let read = Expr::Attribute {
        object: Box::new(Expr::name("self")),
        name: "count".into(),
    };
    let Expr::Attribute { object, name } = &read else {
        panic!("expected an attribute read");
    };
    assert_eq!(**object, Expr::name("self"));
    assert_eq!(name, "count");

    let written = Stmt::SetAttr {
        object: Expr::name("self"),
        name: "count".into(),
        ty: Ty::Int,
        value: Expr::int(1),
    };
    assert!(matches!(written, Stmt::SetAttr { .. }));
}

#[test]
fn calls_in_the_new_forms_are_found() {
    let f = func(
        "f",
        Vec::new(),
        Ty::Unit,
        vec![
            Stmt::SetAttr {
                object: call("object"),
                name: "x".into(),
                ty: Ty::Int,
                value: call("assigned"),
            },
            Stmt::Bind {
                name: "read".into(),
                ty: Ty::Int,
                value: Expr::Attribute {
                    object: Box::new(call("source")),
                    name: "x".into(),
                },
            },
            Stmt::Bind {
                name: "made".into(),
                ty: Ty::Instance("Counter".into()),
                value: Expr::Construct {
                    class: "Counter".into(),
                    args: vec![call("argument")],
                },
            },
            Stmt::Bind {
                name: "called".into(),
                ty: Ty::Int,
                value: Expr::MethodCall {
                    receiver: Box::new(call("receiver")),
                    class: Some("Counter".into()),
                    method: "get".into(),
                    args: vec![call("method_argument")],
                },
            },
        ],
    );
    let mut found = Vec::new();
    f.walk_calls(&mut |name, _| found.push(name.to_string()));
    found.sort();
    assert_eq!(
        found,
        [
            "argument",
            "assigned",
            "method_argument",
            "object",
            "receiver",
            "source",
        ],
        "a call hidden in any of these must still reach unit validation"
    );
}

#[test]
fn a_method_is_not_reported_as_a_free_call() {
    // It resolves against the class, not the unit. Reporting it would make validation demand a
    // free function of that name, which the user never wrote.
    let f = func(
        "f",
        Vec::new(),
        Ty::Unit,
        vec![Stmt::Bind {
            name: "n".into(),
            ty: Ty::Int,
            value: Expr::MethodCall {
                receiver: Box::new(Expr::name("self")),
                class: Some("C".into()),
                method: "get".into(),
                args: Vec::new(),
            },
        }],
    );
    let mut found = Vec::new();
    f.walk_calls(&mut |name, _| found.push(name.to_string()));
    assert!(found.is_empty(), "found {found:?}");
}

#[test]
fn a_unit_with_classes_survives_the_artifact() {
    let mut unit = Unit::new();
    unit.add_class(counter(
        "Counter",
        vec![func(
            "bump",
            vec![param("by", Ty::Int)],
            Ty::Unit,
            vec![Stmt::SetAttr {
                object: Expr::name("self"),
                name: "count".into(),
                ty: Ty::Int,
                value: Expr::name("by"),
            }],
        )],
    ))
    .unwrap();
    unit.add_function(func(
        "make",
        Vec::new(),
        Ty::Instance("Counter".into()),
        vec![Stmt::Return(Expr::Construct {
            class: "Counter".into(),
            args: Vec::new(),
        })],
    ))
    .unwrap();

    let restored = Unit::from_json(&unit.to_json().expect("serialize")).expect("deserialize");
    assert_eq!(
        restored.classes().collect::<Vec<_>>(),
        unit.classes().collect::<Vec<_>>()
    );
    assert_eq!(restored.fingerprint(), unit.fingerprint());
}
