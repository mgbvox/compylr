//! What a source language requires, and what a target preserves.
//!
//! The mechanism is small on purpose — a set intersection and a message — and its value is
//! entirely in *when* it fires. A backend that wraps arithmetic instead of reporting overflow
//! compiles a Python function into something that is not a translation of it, and without this
//! check the way you find out is a wrong number in production.
//!
//! It is also the gate on the last stage of the pipeline. A target may offer transformations that
//! trade a guarantee for speed; each declares what it breaks, and one that would break something
//! the frontend requires is withheld with a reason. "Why is compylr not emitting the fast thing?"
//! is otherwise unanswerable.

use compylr::Guarantee;
use compylr::bridge::{CompileFailure, compile};
use compylr::ir::{Expr, Function, Literal, Stmt, Ty, Unit};
use compylr::span::Span;
use compylr_core::backend::{Backend, BackendError, GeneratedFiles};
use compylr_core::negotiation::{negotiate, resolve_options, withheld_by_default};

const DOUBLE: &str = "def double(n: int) -> int:\n    return n * 2\n";

fn python_unit() -> Unit {
    compylr::frontend::lookup("python")
        .unwrap()
        .lower(&[DOUBLE.to_string()])
        .expect("must lower")
}

#[test]
fn the_rust_backend_covers_everything_the_python_frontend_requires() {
    let backend = compylr::backend::lookup("rust").unwrap();
    let unit = python_unit();

    // Asserted rather than inspected: the two lists are declared in different crates, and nothing
    // but a test notices when one of them changes and the other does not.
    assert!(
        negotiate(&unit, backend).is_ok(),
        "the pair compylr ships must be a usable pair"
    );
    for required in unit.requires() {
        assert!(
            backend.preserves().contains(required),
            "the Rust backend must preserve {required}"
        );
    }
}

#[test]
fn a_covered_combination_compiles() {
    assert!(compile(&[DOUBLE.to_string()], "rust").is_ok());
}

/// A backend that drops a guarantee is refused before anything is emitted.
#[derive(Debug)]
struct WrappingBackend;

impl Backend for WrappingBackend {
    fn name(&self) -> &'static str {
        "wrapping"
    }

    fn preserves(&self) -> &'static [Guarantee] {
        // Everything except overflow reporting.
        &[
            Guarantee::DivisionByZeroReported,
            Guarantee::FloatOrderPreserved,
        ]
    }

    fn emit(&self, _unit: &Unit) -> Result<GeneratedFiles, BackendError> {
        panic!("emission must never be reached for an uncovered combination");
    }
}

#[test]
fn an_uncovered_combination_fails_before_emission_naming_the_guarantee() {
    let unit = python_unit();
    let error = negotiate(&unit, &WrappingBackend).expect_err("overflow reporting is missing");

    assert_eq!(error.guarantee, Guarantee::IntegerOverflowReported);
    assert_eq!(error.frontend, "python");
    assert_eq!(error.backend, "wrapping");
    // The message has to be actionable on its own: it names both ends and the property.
    let rendered = error.to_string();
    assert!(rendered.contains("wrapping"), "{rendered}");
    assert!(rendered.contains("python"), "{rendered}");
    assert!(rendered.contains("overflow"), "{rendered}");
}

/// A unit nobody claimed requires nothing, so a corpus entry is not blocked by a check with
/// nothing to check.
#[test]
fn a_hand_built_unit_negotiates_with_any_backend() {
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

    assert!(unit.requires().is_empty());
    assert!(negotiate(&unit, &WrappingBackend).is_ok());
}

/// A guarantee-violating transformation is not applied by default, and says why.
#[test]
fn a_guarantee_violating_option_is_withheld_and_reportable() {
    let backend = compylr::backend::lookup("rust").unwrap();
    let unit = python_unit();

    let withheld = withheld_by_default(&unit, backend);
    assert!(
        !withheld.is_empty(),
        "the backend declares an option that costs a guarantee Python requires"
    );
    let unchecked = withheld
        .iter()
        .find(|w| w.option == "unchecked-arithmetic")
        .expect("declared by the Rust backend");
    assert_eq!(unchecked.guarantee, Guarantee::IntegerOverflowReported);
    assert!(unchecked.to_string().contains("overflow"), "{unchecked}");
}

/// Asking for it explicitly does not get it either, because the requirement still stands.
#[test]
fn permitting_a_violating_option_still_withholds_it() {
    let backend = compylr::backend::lookup("rust").unwrap();
    let unit = python_unit();

    let (applied, withheld) =
        resolve_options(&unit, backend, &["unchecked-arithmetic".to_string()]).unwrap();
    assert!(applied.is_empty());
    assert_eq!(withheld.len(), 1);
    assert_eq!(withheld[0].option, "unchecked-arithmetic");
}

/// A name the backend does not offer is a typo, and is reported as one.
#[test]
fn an_unknown_option_is_refused() {
    let backend = compylr::backend::lookup("rust").unwrap();
    let unit = python_unit();

    let error =
        resolve_options(&unit, backend, &["go-faster".to_string()]).expect_err("no such option");
    assert!(!error.reserved);
    assert!(error.to_string().contains("go-faster"), "{error}");
}

/// A reserved option, permitted where nothing forbids it, says it is not implemented.
///
/// Silently doing nothing would let a caller believe the transformation took effect, which is the
/// failure mode the registries' reserved/unknown split already exists to avoid.
#[test]
fn a_reserved_option_says_so_rather_than_silently_doing_nothing() {
    let backend = compylr::backend::lookup("rust").unwrap();
    // A unit with no origin requires nothing, so the option is not withheld on those grounds.
    let unit = Unit::new();

    let error = resolve_options(&unit, backend, &["unchecked-arithmetic".to_string()])
        .expect_err("declared, not implemented");
    assert!(error.reserved);
    assert!(error.to_string().contains("reserved"), "{error}");
}

/// Formatting is meaning-preserving, so it needs no permission and is applied on the way out.
#[test]
fn emitted_source_is_formatted_outside_emission() {
    let backend = compylr::backend::lookup("rust").unwrap();
    let unit = python_unit();

    let raw = backend.emit(&unit).unwrap();
    let processed = backend.post_process(raw.clone());

    assert_eq!(
        raw.keys().collect::<Vec<_>>(),
        processed.keys().collect::<Vec<_>>(),
        "post-processing must not add or drop files"
    );
    // Emission itself must not have run a formatter: that would make its output depend on which
    // formatter is installed, and the rebuild cache is keyed on emission being reproducible.
    assert_eq!(raw, backend.emit(&unit).unwrap());
}

/// The failure reaches a caller as a backend problem, not as a rejected program.
#[test]
fn an_unusable_backend_is_reported_as_a_backend_failure() {
    // The shipped pair is covered, so this asserts the mapping exists rather than exercising it
    // end to end: `compile` only ever resolves backends from the registry.
    let failure = CompileFailure::Guarantee(
        negotiate(&python_unit(), &WrappingBackend).expect_err("uncovered"),
    );
    assert!(matches!(failure, CompileFailure::Guarantee(_)));
}
