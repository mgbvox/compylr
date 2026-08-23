//! What a unit requires preserved is a property of the program, not of its language.
//!
//! `Origin.requires` used to be a copy of `Frontend::requires()`, so every Python unit required
//! exactly the same three things whatever it contained. That made `unchecked-arithmetic` a name
//! with nothing behind it: the Rust backend declared the option, and no unit could ever be
//! eligible for it, because every unit required overflow reported whether or not it performed any
//! arithmetic at all.
//!
//! Deriving the list by walking the unit is what makes the option coherent. It also means the
//! negotiation reads the *program*: a transformation refused for one Python function may be
//! permitted for its neighbour, and neither of them is a special case.

use compylr_core::negotiation::{resolve_options, withheld_by_default};
use compylr_core::{Behavior, BehaviorRequest, Guarantee, LanguagePair, Source, resolve};
use compylr_diagnostics::span::Span;
use compylr_ir::{Expr, Function, Literal, Stmt, Ty, Unit};

fn behavior(request: &BehaviorRequest) -> Behavior {
    let frontend = compylr_registry::frontends::lookup("python").unwrap();
    let backend = compylr_registry::backends::lookup("rust").unwrap();
    let known: Vec<&str> = vec!["python", "rust"];
    let pair = LanguagePair {
        source: frontend.name(),
        source_behavior: frontend.behavior(),
        target: backend.name(),
        target_behavior: backend.behavior(),
        known: &known,
    };
    resolve(request, &pair, None).expect("both languages of the pair resolve")
}

fn python() -> Behavior {
    behavior(&BehaviorRequest::inherit())
}

fn rust() -> Behavior {
    behavior(&BehaviorRequest::language("rust"))
}

fn unit_of(source: &str, behavior: Behavior) -> Unit {
    compylr_registry::frontends::lookup("python")
        .unwrap()
        .lower(&[Source::new(source, behavior)])
        .expect("must lower")
}

/// Arithmetic, a division, and a remainder — every guarantee an axis can waive.
const ARITHMETIC: &str = "\
def op(a: int, b: int) -> int:
    return a + b * a - b // a + a % b
";

#[test]
fn a_unit_under_the_source_languages_stance_requires_what_it_always_did() {
    let unit = unit_of(ARITHMETIC, python());

    for guarantee in [
        Guarantee::IntegerOverflowReported,
        Guarantee::DivisionByZeroReported,
        Guarantee::FloatOrderPreserved,
    ] {
        assert!(
            unit.requires().contains(&guarantee),
            "a program under Python's stance must still require {guarantee}"
        );
    }
}

/// The claim the whole change rests on: a program can require less than its language.
#[test]
fn a_unit_whose_arithmetic_is_unchecked_does_not_require_overflow_reported() {
    let unit = unit_of(ARITHMETIC, rust());

    assert!(
        !unit
            .requires()
            .contains(&Guarantee::IntegerOverflowReported),
        "nothing in this program asked for overflow to be reported: {:?}",
        unit.requires()
    );
    assert!(
        !unit.requires().contains(&Guarantee::DivisionByZeroReported),
        "nor for a zero divisor to be: {:?}",
        unit.requires()
    );
}

/// Float ordering is never waived, because it is not an axis.
///
/// Reassociation is a transformation a *backend* might apply rather than an operation a
/// programmer wrote, so there is nothing on a node to look for and nothing for a user to ask for.
#[test]
fn every_unit_requires_float_ordering_whatever_its_behavior() {
    for behavior in [python(), rust()] {
        let unit = unit_of(ARITHMETIC, behavior);
        assert!(
            unit.requires().contains(&Guarantee::FloatOrderPreserved),
            "float ordering has no axis and must survive every behavior"
        );
    }
}

/// A program that performs no fallible operation requires nothing beyond float ordering.
///
/// Worth pinning separately from the unchecked case: it is the difference between "derived from
/// the unit" and "derived from the behavior". A function that only returns a constant asks for
/// nothing, whichever stance compiled it.
#[test]
fn a_program_with_no_fallible_operation_requires_only_float_ordering() {
    for behavior in [python(), rust()] {
        let unit = unit_of("def answer() -> int:\n    return 42\n", behavior);
        assert_eq!(unit.requires(), [Guarantee::FloatOrderPreserved]);
    }
}

/// A hand-built unit still requires nothing at all.
///
/// The conformance corpus is built this way, with no frontend and no behavior. Deriving
/// requirements must not turn a corpus entry into something a backend can refuse — which is what
/// mapping a *behavior* onto requirements would have done, since a hand-built unit has none.
#[test]
fn a_hand_built_unit_requires_nothing() {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "answer".to_string(),
        params: vec![],
        ret: Ty::Int,
        body: vec![Stmt::Return(Expr::Literal(Literal::Int(42)))],
        doc: None,
        span: Span::default(),
    })
    .unwrap();

    assert!(unit.origin().is_none());
    assert!(unit.requires().is_empty());
}

/// Two programs in one language may require different things.
#[test]
fn requirements_follow_the_program_not_the_language() {
    let reported = unit_of(ARITHMETIC, python());
    let unchecked = unit_of(ARITHMETIC, rust());

    assert_ne!(
        reported.requires(),
        unchecked.requires(),
        "the same source under two behaviors must be able to require different things"
    );
    assert_eq!(
        reported.origin().map(|o| o.frontend.as_str()),
        unchecked.origin().map(|o| o.frontend.as_str()),
        "and they are still the same language"
    );
}

/// The option that had nothing behind it now has something.
///
/// `unchecked-arithmetic` breaks overflow and division-by-zero reporting. A unit that waived both
/// is no longer refused it on those grounds — which is the difference between a declared option
/// and a real one.
#[test]
fn a_waived_guarantee_stops_the_option_being_withheld() {
    let backend = compylr_registry::backends::lookup("rust").unwrap();

    let reported = unit_of(ARITHMETIC, python());
    let withheld = withheld_by_default(&reported, backend);
    assert!(
        withheld.iter().any(|w| w.option == "unchecked-arithmetic"),
        "a program that asked for overflow reporting must still be refused the option"
    );

    let unchecked = unit_of(ARITHMETIC, rust());
    let withheld = withheld_by_default(&unchecked, backend);
    assert!(
        !withheld.iter().any(|w| w.option == "unchecked-arithmetic"),
        "a program that waived both guarantees must not be refused on their account: {withheld:?}"
    );
}

/// And permitting it explicitly now gets past the guarantee check.
///
/// It still fails — the Rust backend declares the option and does not implement it — but it fails
/// saying *that*, which is the three-way honesty the registries already use. Before, the refusal
/// was always about the guarantee and the reserved-name case was unreachable for any real unit.
#[test]
fn permitting_the_option_for_a_waived_unit_reaches_the_reserved_answer() {
    let backend = compylr_registry::backends::lookup("rust").unwrap();
    let unchecked = unit_of(ARITHMETIC, rust());

    let error = resolve_options(&unchecked, backend, &["unchecked-arithmetic".to_string()])
        .expect_err("declared, not implemented");
    assert!(
        error.reserved,
        "the refusal must now be about the option being unimplemented, not about a guarantee"
    );

    // The same request against a unit that did not waive them is withheld instead, and never
    // reaches the reserved answer.
    let reported = unit_of(ARITHMETIC, python());
    let (applied, withheld) =
        resolve_options(&reported, backend, &["unchecked-arithmetic".to_string()])
            .expect("withholding is not an error");
    assert!(applied.is_empty());
    assert_eq!(withheld.len(), 1);
}
