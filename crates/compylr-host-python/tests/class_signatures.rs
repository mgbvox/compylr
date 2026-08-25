//! Class-valued free-function signatures and their borrow-only ownership rules.
//!
//! These tests enter through the registered frontend so cross-source class collection and located
//! diagnostics are exercised together. The pure lowering tests cover class bodies separately;
//! this file owns the whole-unit boundary policy added for Python-callable free functions.

use compylr_core::{Frontend, Source};
use compylr_frontend_python::component::{PYTHON_BEHAVIOR, PythonFrontend};
use compylr_ir::{Behavior, Ty};

fn source(text: &str) -> Source {
    Source::new(text, Behavior::of(&PYTHON_BEHAVIOR))
}

fn lower(texts: &[&str]) -> Result<compylr_ir::Unit, compylr_core::LoweringError> {
    let sources: Vec<Source> = texts.iter().map(|text| source(text)).collect();
    PythonFrontend.lower(&sources)
}

const TALLY: &str = "class Tally:\n\
    \x20   def __init__(self, start: int) -> None:\n\
    \x20       self.value: int = start\n\
    \n\
    \x20   def bump(self, amount: int) -> None:\n\
    \x20       self.value = self.value + amount\n";

#[test]
fn direct_class_annotations_resolve_in_either_same_source_order() {
    for text in [
        format!(
            "{TALLY}\ndef read(tally: Tally) -> int:\n    return tally.value\n\n\
             def build(start: int) -> Tally:\n    return Tally(start)\n"
        ),
        format!(
            "def read(tally: Tally) -> int:\n    return tally.value\n\n\
             def build(start: int) -> Tally:\n    return Tally(start)\n\n{TALLY}"
        ),
    ] {
        let unit = lower(&[&text]).expect("direct class annotations should resolve");
        let read = unit
            .functions()
            .find(|function| function.name == "read")
            .unwrap();
        let build = unit
            .functions()
            .find(|function| function.name == "build")
            .unwrap();
        assert_eq!(read.params[0].ty, Ty::Instance("Tally".into()));
        assert_eq!(build.ret, Ty::Instance("Tally".into()));
    }
}

#[test]
fn direct_class_annotations_resolve_across_sources_in_either_order() {
    let functions = "def read(tally: Tally) -> int:\n    return tally.value\n\n\
                     def build(start: int) -> Tally:\n    return Tally(start)\n";
    for texts in [[functions, TALLY], [TALLY, functions]] {
        lower(&texts).expect("the complete class table should be independent of source order");
    }
}

#[test]
fn an_unknown_bare_class_annotation_has_its_own_located_category() {
    let error = lower(&["def typo(value: Taly) -> int:\n    return 1\n"])
        .expect_err("the complete unit defines no Taly");
    assert_eq!(error.code(), Some("unresolved_class_annotation"));
    assert_eq!((error.line(), error.column()), (1, 17));
}

#[test]
fn a_known_unsupported_builtin_is_not_reclassified_as_a_class() {
    let error = lower(&["def bad(value: complex) -> int:\n    return 1\n"])
        .expect_err("complex remains outside the subset");
    assert_eq!(error.code(), Some("unsupported_type"));
    assert_eq!((error.line(), error.column()), (1, 16));
}

#[test]
fn nested_instance_boundary_types_are_rejected_at_the_annotation() {
    for (annotation, column) in [("list[Tally]", 19), ("dict[str, list[Tally]]", 19)] {
        let text = format!("{TALLY}\ndef nested(value: {annotation}) -> int:\n    return 1\n");
        let error = lower(&[&text]).expect_err("nested instance conversion is not supported");
        assert_eq!(error.code(), Some("unsupported_type"));
        assert_eq!((error.line(), error.column()), (8, column));
    }
}

#[test]
fn explicit_instance_positions_on_methods_are_rejected_at_the_annotation() {
    let text = format!(
        "{TALLY}\nclass Reader:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.seen: int = 0\n\
         \n\
         \x20   def take(self, other: Tally) -> Tally:\n\
         \x20       return other\n"
    );
    let error = lower(&[&text]).expect_err("explicit method instance boundaries are out of scope");
    assert_eq!(error.code(), Some("unsupported_type"));
    assert_eq!((error.line(), error.column()), (12, 27));
}

#[test]
fn borrowed_instance_reads_mutation_and_forwarding_are_accepted() {
    let text = format!(
        "{TALLY}\ndef read(tally: Tally) -> int:\n    return tally.value\n\n\
         def mutate(tally: Tally) -> int:\n    tally.value = tally.value + 1\n    tally.bump(1)\n    return tally.value\n\n\
         def forward(tally: Tally) -> int:\n    return mutate(tally)\n"
    );
    lower(&[&text]).expect("borrow-compatible uses should lower");
}

#[test]
fn borrowed_instances_cannot_be_stored_in_attributes_or_local_collections() {
    let holder = "class Holder:\n\
                  \x20   def __init__(self) -> None:\n\
                  \x20       self.item: Tally = Tally(0)\n";
    for function in [
        "def store(holder: Holder, value: Tally) -> None:\n    holder.item = value\n",
        "def collect(value: Tally) -> None:\n    values: list[Tally] = []\n    values.append(value)\n",
    ] {
        let text = format!("{TALLY}\n{holder}\n{function}");
        let error = lower(&[&text]).expect_err("a borrowed instance may not enter owned storage");
        assert_eq!(error.code(), Some("borrowed_instance_escape"));
    }
}

#[test]
fn borrowed_instances_cannot_be_consumed_through_a_nested_owned_argument() {
    let caller = "def caller(value: Tally) -> int:\n    return consume([value])\n";
    let consumer = "def consume(values: list[Tally]) -> int:\n    return 1\n";
    let error = lower(&[caller, consumer, TALLY])
        .expect_err("placing the borrow in an owned call argument is an escape");
    assert_eq!(error.code(), Some("borrowed_instance_escape"));
    assert_eq!((error.line(), error.column()), (2, 21));
}

#[test]
fn returning_a_borrowed_instance_is_a_located_escape() {
    let text = format!("{TALLY}\ndef identity(value: Tally) -> Tally:\n    return value\n");
    let error = lower(&[&text]).expect_err("a borrow cannot become an owned return");
    assert_eq!(error.code(), Some("borrowed_instance_escape"));
    assert_eq!((error.line(), error.column()), (9, 12));
}

#[test]
fn storing_or_rebinding_a_borrowed_instance_is_a_located_escape() {
    let cases = [
        (
            "def alias(value: Tally) -> int:\n    same = value\n    return same.value\n",
            2,
            12,
        ),
        (
            "def collect(value: Tally) -> int:\n    values: list[Tally] = [value]\n    return 1\n",
            2,
            28,
        ),
        (
            "def replace(value: Tally) -> int:\n    value = Tally(1)\n    return value.value\n",
            2,
            5,
        ),
    ];
    for (function, relative_line, column) in cases {
        let text = format!("{TALLY}\n{function}");
        let error = lower(&[&text]).expect_err("borrow-only parameters cannot enter storage");
        assert_eq!(error.code(), Some("borrowed_instance_escape"));
        assert_eq!((error.line(), error.column()), (7 + relative_line, column));
    }
}

#[test]
fn newly_owned_instance_results_may_be_returned_directly_or_from_a_call() {
    let text = format!(
        "{TALLY}\ndef build(start: int) -> Tally:\n    return Tally(start)\n\n\
         def rebuild(start: int) -> Tally:\n    return build(start)\n"
    );
    lower(&[&text]).expect("constructors and owned-producing calls return owned instances");
}
