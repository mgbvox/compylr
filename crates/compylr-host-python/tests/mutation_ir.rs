//! The IR's mutation and membership forms.
//!
//! Two properties are worth asserting directly. `walk_calls` must reach into a mutation's operands,
//! or a call hidden in `xs[f()] = g()` escapes unit validation and the backend emits a call to
//! something that does not exist. And `append` must not be validated as a function call, or a unit
//! containing one fails to resolve a callee named `append` that was never meant to exist.

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

fn call(name: &str) -> Expr {
    Expr::Call {
        callee: name.into(),
        args: Vec::new(),
    }
}

fn callees(function: &Function) -> Vec<String> {
    let mut found = Vec::new();
    function.walk_calls(&mut |name, _| found.push(name.to_string()));
    found.sort();
    found
}

fn name(n: &str) -> Expr {
    Expr::Name(n.into())
}

#[test]
fn element_assignment_carries_collection_index_and_value() {
    let f = func(
        "f",
        vec![Stmt::SetItem {
            collection: name("xs"),
            index: Expr::int(0),
            value: Expr::int(1),
        }],
    );
    let Stmt::SetItem {
        collection,
        index,
        value,
    } = &f.body[0]
    else {
        panic!("expected an element assignment");
    };
    assert_eq!(collection, &name("xs"));
    assert_eq!(index, &Expr::int(0));
    assert_eq!(value, &Expr::int(1));
}

#[test]
fn append_carries_the_sequence_and_the_value() {
    let f = func(
        "f",
        vec![Stmt::Append {
            sequence: name("xs"),
            value: Expr::int(1),
        }],
    );
    assert!(matches!(f.body[0], Stmt::Append { .. }));
}

#[test]
fn membership_carries_the_value_and_the_container() {
    let contains = Expr::Contains {
        value: Box::new(Expr::int(1)),
        container: Box::new(name("xs")),
    };
    let Expr::Contains { value, container } = &contains else {
        panic!("expected a membership test");
    };
    assert_eq!(**value, Expr::int(1));
    assert_eq!(**container, name("xs"));
}

#[test]
fn negated_membership_is_a_negation_rather_than_its_own_form() {
    // A `negated` flag on the membership form would make `not in` a second spelling of the same
    // node, and every consumer would have to remember to honour it. A negation composes instead.
    let negated = Expr::Not(Box::new(Expr::Contains {
        value: Box::new(Expr::int(1)),
        container: Box::new(name("xs")),
    }));
    let Expr::Not(inner) = &negated else {
        panic!("expected a negation");
    };
    assert!(matches!(**inner, Expr::Contains { .. }));
}

#[test]
fn calls_in_every_new_form_are_found() {
    let f = func(
        "f",
        vec![
            Stmt::SetItem {
                collection: call("collection"),
                index: call("index"),
                value: call("value"),
            },
            Stmt::Append {
                sequence: call("sequence"),
                value: call("appended"),
            },
            Stmt::Bind {
                name: "found".into(),
                ty: Ty::Bool,
                value: Expr::Not(Box::new(Expr::Contains {
                    value: Box::new(call("needle")),
                    container: Box::new(call("haystack")),
                })),
            },
        ],
    );
    assert_eq!(
        callees(&f),
        [
            "appended",
            "collection",
            "haystack",
            "index",
            "needle",
            "sequence",
            "value",
        ]
    );
}

#[test]
fn append_is_not_resolved_as_a_call() {
    // If `Append` were a call form, a unit containing one would fail to resolve a callee named
    // `append`, which is a function the user never wrote and could not add.
    let mut unit = Unit::new();
    unit.add_function(func(
        "f",
        vec![Stmt::Append {
            sequence: name("xs"),
            value: Expr::int(1),
        }],
    ))
    .unwrap();
    unit.validate()
        .expect("append is a form, not a call to resolve");
}

#[test]
fn the_new_forms_survive_the_artifact() {
    let mut unit = Unit::new();
    unit.add_function(func(
        "f",
        vec![
            Stmt::SetItem {
                collection: name("d"),
                index: Expr::string("k"),
                value: Expr::int(1),
            },
            Stmt::Append {
                sequence: name("xs"),
                value: Expr::int(2),
            },
            Stmt::Bind {
                name: "present".into(),
                ty: Ty::Bool,
                value: Expr::Not(Box::new(Expr::Contains {
                    value: Box::new(Expr::string("k")),
                    container: Box::new(name("d")),
                })),
            },
        ],
    ))
    .unwrap();

    let restored = Unit::from_json(&unit.to_json().expect("serialize")).expect("deserialize");
    assert_eq!(
        restored.functions().collect::<Vec<_>>(),
        unit.functions().collect::<Vec<_>>()
    );
    assert_eq!(restored.fingerprint(), unit.fingerprint());
}
