//! Lowering takes a resolved behavior and sets every mode from it.
//!
//! Two properties carry this change, and they pull in opposite directions:
//!
//! * **The behavior decides what an operation means.** Every mode on every node comes from it, so
//!   the same source under two behaviors is two different programs — differing in exactly the
//!   modes the two behaviors differ on, and nowhere else.
//! * **The behavior decides nothing about what is *accepted*.** No program lowers under one
//!   behavior and fails under another, and no expression changes type. A behavior that could move
//!   acceptance would mean the same annotated source was two different programs in a second,
//!   worse sense: one of them would not type-check.
//!
//! The second is the one that is easy to break without noticing, which is why it is checked over
//! the whole fixture corpus rather than over an example.

use std::fs;
use std::path::{Path, PathBuf};

use compylr_core::{Axis, Behavior, BehaviorRequest, LanguagePair, resolve};
use compylr_frontend_python::frontend::parse_source;
use compylr_frontend_python::lower::lower_source_members;
use compylr_ir::{BinOp, Checked, DivMode, Expr, Function, IndexOrigin, Stmt, TextUnits, Ty};

/// The two languages this repository ships, and the names compylr recognises.
fn pair() -> (
    &'static compylr_ir::LanguageBehavior,
    &'static compylr_ir::LanguageBehavior,
) {
    (
        compylr_registry::frontends::lookup("python")
            .unwrap()
            .behavior(),
        compylr_registry::backends::lookup("rust")
            .unwrap()
            .behavior(),
    )
}

fn resolved(request: &BehaviorRequest) -> Behavior {
    let (source, target) = pair();
    let known: Vec<&str> = vec!["python", "rust", "typescript", "go", "cpp"];
    let pair = LanguagePair {
        source: "python",
        source_behavior: source,
        target: "rust",
        target_behavior: target,
        known: &known,
    };
    resolve(request, &pair, None).expect("the pair's own languages must resolve")
}

fn python() -> Behavior {
    resolved(&BehaviorRequest::inherit())
}

fn rust() -> Behavior {
    resolved(&BehaviorRequest::language("rust"))
}

fn lower(source: &str, behavior: Behavior) -> Vec<Function> {
    let parsed = parse_source(source).expect("fixture must parse");
    lower_source_members(&parsed, behavior)
        .unwrap_or_else(|e| panic!("should lower: {}", e.render(source)))
        .0
}

/// A source exercising every axis at once.
const EVERY_AXIS: &str = "\
def op(a: int, b: int, xs: list[int], s: str) -> int:
    total = a + b
    quotient = a // b
    remainder = a % b
    negated = -a
    element = xs[0]
    width = len(s)
    return total + quotient + remainder + negated + element + width
";

fn only<'a>(functions: &'a [Function], name: &str) -> &'a Function {
    functions
        .iter()
        .find(|f| f.name == name)
        .expect("the fixture defines it")
}

/// Every mode on every node comes from the behavior, not from a constant.
#[test]
fn every_declared_mode_matches_the_resolved_behavior() {
    for behavior in [python(), rust()] {
        let functions = lower(EVERY_AXIS, behavior);
        let body = &only(&functions, "op").body;

        let mut seen = Vec::new();
        for stmt in body {
            if let Stmt::Bind { name, value, .. } = stmt {
                seen.push((name.as_str(), value));
            }
        }

        for (name, value) in seen {
            match (name, value) {
                ("total", Expr::Binary { op, .. }) => {
                    assert_eq!(
                        *op,
                        BinOp::Add {
                            checked: behavior.arithmetic()
                        }
                    );
                }
                ("quotient", Expr::Binary { op, .. }) => {
                    assert_eq!(*op, behavior.integer_division());
                }
                ("remainder", Expr::Binary { op, .. }) => {
                    assert_eq!(*op, behavior.remainder());
                }
                ("negated", Expr::Neg { checked, .. }) => {
                    assert_eq!(*checked, behavior.arithmetic());
                }
                (
                    "element",
                    Expr::Subscript {
                        origin, checked, ..
                    },
                ) => {
                    assert_eq!(*origin, behavior.index_origin());
                    assert_eq!(*checked, behavior.index_checked());
                }
                ("width", Expr::Len { units, .. }) => {
                    assert_eq!(*units, behavior.text_units());
                }
                (name, other) => panic!("unexpected binding {name}: {other:?}"),
            }
        }
    }
}

/// Two behaviors differing on one axis produce units differing only in that axis's modes.
///
/// The strong form of "lowering is a pure function of the source and the behavior together". A
/// behavior that leaked into anything else — a type, a statement's shape, a name — would show up
/// here as a difference somewhere the axis does not govern.
#[test]
fn two_behaviors_differ_only_where_the_behaviors_differ() {
    let only_text_length = resolved(&BehaviorRequest::inherit().with(Axis::TextLength, "rust"));

    let baseline = lower(EVERY_AXIS, python());
    let changed = lower(EVERY_AXIS, only_text_length);

    assert_eq!(baseline.len(), changed.len());
    let (before, after) = (&only(&baseline, "op").body, &only(&changed, "op").body);
    assert_eq!(before.len(), after.len());

    for (before, after) in before.iter().zip(after) {
        match (before, after) {
            // The one statement the axis governs.
            (
                Stmt::Bind {
                    name,
                    value: Expr::Len { units: before, .. },
                    ..
                },
                Stmt::Bind {
                    value: Expr::Len { units: after, .. },
                    ..
                },
            ) if name == "width" => {
                assert_eq!(*before, TextUnits::CodePoints);
                assert_eq!(*after, TextUnits::Utf8Bytes);
            }
            // Everything else is untouched, including the other five axes' modes.
            (before, after) => assert_eq!(
                before, after,
                "a behavior naming one axis must not move anything else"
            ),
        }
    }
}

/// The same source under the same behavior lowers identically, twice.
#[test]
fn lowering_is_a_pure_function_of_the_source_and_the_behavior() {
    assert_eq!(lower(EVERY_AXIS, python()), lower(EVERY_AXIS, python()));
    assert_eq!(lower(EVERY_AXIS, rust()), lower(EVERY_AXIS, rust()));
    assert_ne!(
        lower(EVERY_AXIS, python()),
        lower(EVERY_AXIS, rust()),
        "the two stances disagree on every axis, so the units must differ"
    );
}

fn fixtures(kind: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate sits two levels below the repository root")
        .join("frontends/python/fixtures")
        .join(kind);
    let mut paths: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "py"))
        .collect();
    // Read from the directory rather than listed, for the reason `fixtures.rs` records: a list
    // drifts, and the drift hides exactly the fixture that would have failed.
    paths.sort();
    assert!(!paths.is_empty(), "{} must hold fixtures", root.display());
    paths
}

/// Behavior selects meaning, never acceptance.
#[test]
fn every_accepted_fixture_lowers_under_every_behavior() {
    for path in fixtures("accepted") {
        let source = fs::read_to_string(&path).expect("fixture must be readable");
        let parsed = parse_source(&source).expect("an accepted fixture must parse");
        for (name, behavior) in [("python", python()), ("rust", rust())] {
            assert!(
                lower_source_members(&parsed, behavior).is_ok(),
                "{} lowers under the default behavior but not under {name}'s",
                path.display()
            );
        }
    }
}

/// And a rejected fixture is rejected the same way under each.
///
/// Compared on the diagnostic **code**, not the message: prose is presentation, and a behavior
/// that changed only the wording would be a change nobody should have to defend. A behavior that
/// changed the *category* would mean the subset itself had moved.
#[test]
fn every_rejected_fixture_is_rejected_identically_under_every_behavior() {
    for path in fixtures("rejected") {
        let source = fs::read_to_string(&path).expect("fixture must be readable");
        let Ok(parsed) = parse_source(&source) else {
            // A fixture that does not parse is rejected before lowering, so no behavior reaches it.
            continue;
        };

        let under_python = lower_source_members(&parsed, python());
        let under_rust = lower_source_members(&parsed, rust());

        match (under_python, under_rust) {
            (Err(python), Err(rust)) => assert_eq!(
                python.kind().code(),
                rust.kind().code(),
                "{} is rejected for different reasons under two behaviors",
                path.display()
            ),
            (python, rust) => panic!(
                "{} must be rejected under both behaviors; python: {:?}, rust: {:?}",
                path.display(),
                python.map(|_| "accepted"),
                rust.map(|_| "accepted"),
            ),
        }
    }
}

/// `/` yields a float under every behavior. Design D10.
///
/// The load-bearing case for "behavior never moves acceptance". If `true_div="rust"` meant Rust's
/// `/`, then `def f(a: int, b: int) -> float: return a / b` would type-check under one behavior
/// and fail under another — and the annotations are the one thing this subset insists on. What
/// the axis selects is what happens when the divisor is zero, not what type the result has.
#[test]
fn exact_division_is_typed_float_under_every_behavior() {
    const SOURCE: &str = "def f(a: int, b: int) -> float:\n    return a / b\n";

    for behavior in [python(), rust()] {
        let functions = lower(SOURCE, behavior);
        let function = only(&functions, "f");
        assert_eq!(function.ret, Ty::Float);

        match &function.body[0] {
            Stmt::Return(Expr::Binary { op, left, right }) => {
                assert!(
                    matches!(
                        op,
                        BinOp::Div {
                            mode: DivMode::Exact,
                            ..
                        }
                    ),
                    "the mode must stay exact; got {op:?}"
                );
                // The promotion is still inserted, under both.
                assert!(matches!(**left, Expr::ToFloat(_)));
                assert!(matches!(**right, Expr::ToFloat(_)));
            }
            other => panic!("unexpected body: {other:?}"),
        }
    }
}

/// A negative index is not rejected statically, even where it is out of range. Design D10.
///
/// The index is a runtime value. Refusing a literal `-1` would refuse only the cases that are
/// visible while leaving `xs[i]` with a negative `i` to fail at runtime — and a rule that catches
/// the easy half is worse than no rule, because it reads as a guarantee.
#[test]
fn a_negative_index_lowers_under_a_behavior_that_calls_it_out_of_range() {
    const SOURCE: &str = "def last(xs: list[int]) -> int:\n    return xs[-1]\n";

    let functions = lower(SOURCE, rust());
    match &only(&functions, "last").body[0] {
        Stmt::Return(Expr::Subscript {
            origin, checked, ..
        }) => {
            assert_eq!(*origin, IndexOrigin::FromStart);
            assert_eq!(*checked, Checked::Unchecked);
        }
        other => panic!("unexpected body: {other:?}"),
    }
}
