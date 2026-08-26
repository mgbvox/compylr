//! Class-valued free-function signatures and their borrow-only ownership rules.
//!
//! These tests enter through the registered frontend so cross-source class collection and located
//! diagnostics are exercised together. The pure lowering tests cover class bodies separately;
//! this file owns the whole-unit boundary policy added for Python-callable free functions.

use compylr_bridge_python_rust::bindings::emit_extension;
use compylr_core::{Frontend, Source};
use compylr_core::{backend::BackendError, bridge::BuildKey};
use compylr_diagnostics::span::Span;
use compylr_frontend_python::component::{PYTHON_BEHAVIOR, PythonFrontend};
use compylr_ir::{Behavior, Expr, Function, Literal, Param, Stmt, Ty, Unit};
use compylr_registry::backends::lookup;

fn source(text: &str) -> Source {
    Source::new(text, Behavior::of(&PYTHON_BEHAVIOR))
}

fn lower(texts: &[&str]) -> Result<compylr_ir::Unit, compylr_core::LoweringError> {
    let sources: Vec<Source> = texts.iter().map(|text| source(text)).collect();
    PythonFrontend.lower(&sources)
}

fn emit(text: &str) -> String {
    let unit = lower(&[text]).expect("source should lower");
    lookup("rust")
        .unwrap()
        .emit(&unit)
        .expect("unit should emit")
        .remove("src/generated.rs")
        .expect("the translated source should exist")
}

fn binding_layer(unit: &Unit) -> Result<String, BackendError> {
    let key = BuildKey {
        fingerprint: unit.fingerprint(),
        target: "rust".to_string(),
        passes: "default".to_string(),
    };
    emit_extension(unit, &key).map(|mut files| {
        files
            .remove("src/bindings.rs")
            .expect("the extension must include bindings")
    })
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

#[test]
fn instance_parameter_signatures_follow_shared_and_mutable_use() {
    let text = format!(
        "{TALLY}\ndef read(tally: Tally) -> int:\n    return tally.value\n\n\
         def mutate(tally: Tally) -> int:\n    tally.value = tally.value + 1\n    return tally.value\n"
    );
    let emitted = emit(&text);
    assert!(emitted.contains("pub fn read(tally: &Tally)"), "{emitted}");
    assert!(
        emitted.contains("pub fn mutate(tally: &mut Tally)"),
        "{emitted}"
    );
}

#[test]
fn mutable_instance_access_propagates_through_methods_and_free_calls() {
    let text = format!(
        "{TALLY}\ndef inner(tally: Tally) -> int:\n    tally.bump(1)\n    return tally.value\n\n\
         def outer(tally: Tally) -> int:\n    return inner(tally)\n"
    );
    let emitted = emit(&text);
    assert!(
        emitted.contains("pub fn inner(tally: &mut Tally)"),
        "{emitted}"
    );
    assert!(
        emitted.contains("pub fn outer(tally: &mut Tally)"),
        "{emitted}"
    );
    assert!(emitted.contains("inner(tally)?"), "{emitted}");
    assert!(
        !emitted.contains("inner(tally.clone())"),
        "a forwarded instance must not be cloned:\n{emitted}"
    );
}

#[test]
fn mutable_instance_access_reaches_a_fixpoint_across_mutual_recursion() {
    let text = format!(
        "{TALLY}\ndef first(tally: Tally, n: int) -> int:\n\
         \x20   if n == 0:\n        return tally.value\n\
         \x20   return second(tally, n - 1)\n\n\
         def second(tally: Tally, n: int) -> int:\n\
         \x20   if n == 0:\n        tally.bump(1)\n        return tally.value\n\
         \x20   return first(tally, n - 1)\n"
    );
    let emitted = emit(&text);
    assert!(
        emitted.contains("pub fn first(tally: &mut Tally"),
        "{emitted}"
    );
    assert!(
        emitted.contains("pub fn second(tally: &mut Tally"),
        "{emitted}"
    );
}

#[test]
fn bridge_borrows_stable_wrappers_and_wraps_owned_results() {
    let text = format!(
        "{TALLY}\ndef read(tally: Tally) -> int:\n    return tally.value\n\n\
         def mutate(tally: Tally) -> int:\n    tally.bump(1)\n    return tally.value\n\n\
         def forward(tally: Tally) -> int:\n    return mutate(tally)\n\n\
         def build(start: int) -> Tally:\n    return Tally(start)\n"
    );
    let unit = lower(&[&text]).expect("source should lower");
    let bindings = binding_layer(&unit).expect("bridge should support direct instances");

    assert!(
        bindings.contains("tally: PyRef<'_, __compylr_class_0>"),
        "{bindings}"
    );
    assert!(
        bindings.contains("mut tally: PyRefMut<'_, __compylr_class_0>"),
        "{bindings}"
    );
    assert!(
        bindings.contains("generated::read(&tally.inner)"),
        "{bindings}"
    );
    assert!(
        bindings.contains("generated::mutate(&mut tally.inner)"),
        "{bindings}"
    );
    assert!(
        bindings.contains("generated::forward(&mut tally.inner)"),
        "{bindings}"
    );
    assert!(
        bindings.contains("-> PyResult<__compylr_class_0>"),
        "{bindings}"
    );
    assert!(
        bindings.contains(".map(|inner| __compylr_class_0 { inner })"),
        "{bindings}"
    );
    assert!(
        !bindings.contains("tally.inner.clone()"),
        "the bridge must borrow the inner struct rather than cloning it:\n{bindings}"
    );
}

#[test]
fn wrapper_lookup_and_binding_output_are_deterministic() {
    let functions = "def read(tally: Tally) -> int:\n    return tally.value\n";
    let first = lower(&[TALLY, functions]).expect("source should lower");
    let second = lower(&[functions, TALLY]).expect("source should lower");
    assert_eq!(
        binding_layer(&first).unwrap(),
        binding_layer(&second).unwrap()
    );
}

#[test]
fn scalar_bridge_signatures_are_unchanged() {
    let unit = lower(&["def scalar(value: int) -> int:\n    return value\n"])
        .expect("source should lower");
    let bindings = binding_layer(&unit).unwrap();
    assert!(
        bindings.contains("fn __compylr_export_0(value: i64) -> PyResult<i64>"),
        "{bindings}"
    );
    assert!(bindings.contains("generated::scalar(value)"), "{bindings}");
}

#[test]
fn a_missing_wrapper_map_entry_is_a_bridge_error() {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "missing".to_string(),
        params: vec![Param {
            name: "value".to_string(),
            ty: Ty::Instance("Missing".to_string()),
        }],
        ret: Ty::Int,
        body: vec![Stmt::Return(Expr::Literal(Literal::Int(1)))],
        doc: None,
        span: Span::default(),
    })
    .unwrap();

    let error = binding_layer(&unit).expect_err("an instance needs an exposed wrapper");
    let BackendError::Unsupported { detail } = error else {
        panic!("expected an unsupported bridge shape")
    };
    assert!(detail.contains("Missing"), "{detail}");
    assert!(detail.contains("wrapper"), "{detail}");
}

/// A borrow reaches further than the parameter name.
///
/// `holder.item` is an instance the caller still owns through `holder`, so handing it back as a
/// value gives them a detached copy of it: mutating the result would be lost, and CPython would
/// have returned the very object `holder` holds. The emitted Rust compiles either way, which is
/// exactly why this has to be a diagnostic rather than something the backend discovers.
#[test]
fn an_instance_reached_through_a_borrow_cannot_be_consumed_as_a_value() {
    let holder = "class Holder:\n\
                  \x20   def __init__(self) -> None:\n\
                  \x20       self.item: Tally = Tally(0)\n\
                  \x20       self.items: list[Tally] = []\n";
    let cases = [
        (
            "def steal(holder: Holder) -> Tally:\n    return holder.item\n",
            2,
            12,
        ),
        (
            "def steal_indexed(holder: Holder) -> Tally:\n    return holder.items[0]\n",
            2,
            12,
        ),
        (
            "def stash(holder: Holder) -> int:\n    item: Tally = holder.item\n    return item.value\n",
            2,
            19,
        ),
    ];
    for (function, relative_line, column) in cases {
        let text = format!("{TALLY}\n{holder}\n{function}");
        let error =
            lower(&[&text]).expect_err("an instance reached through a borrow stays borrowed");
        assert_eq!(error.code(), Some("borrowed_instance_escape"), "{function}");
        assert_eq!(
            (error.line(), error.column()),
            (12 + relative_line, column),
            "{function}"
        );
    }
}

/// Reading such an instance, and passing it on by borrow, both stay legal.
#[test]
fn an_instance_reached_through_a_borrow_may_still_be_read_and_forwarded() {
    let holder = "class Holder:\n\
                  \x20   def __init__(self) -> None:\n\
                  \x20       self.item: Tally = Tally(0)\n";
    let text = format!(
        "{TALLY}\n{holder}\n\
         def read(holder: Holder) -> int:\n    return holder.item.value\n\n\
         def forward(holder: Holder) -> int:\n    return observe(holder.item)\n\n\
         def observe(tally: Tally) -> int:\n    return tally.value\n"
    );
    lower(&[&text]).expect("a borrowed inner instance may be read and forwarded by borrow");
}

/// A method that hands `self` to a mutating free function mutates through that call.
///
/// The receiver has to be derived from where `self` *goes*, not only from what the method body
/// writes to. A shared receiver here is a borrow-checker error about generated code rather than a
/// diagnostic about the user's program, which is the failure mode this whole analysis exists to
/// avoid.
#[test]
fn a_method_forwarding_self_to_a_mutating_free_function_takes_a_mutable_receiver() {
    let text = "class Counter:\n\
                \x20   def __init__(self) -> None:\n\
                \x20       self.value: int = 0\n\
                \n\
                \x20   def delegate(self) -> int:\n\
                \x20       return raise_it(self)\n\
                \n\
                def raise_it(counter: Counter) -> int:\n\
                \x20   counter.value = counter.value + 1\n\
                \x20   return counter.value\n";
    let emitted = emit(text);
    assert!(
        emitted.contains("pub fn raise_it(counter: &mut Counter)"),
        "{emitted}"
    );
    assert!(emitted.contains("pub fn delegate(&mut self)"), "{emitted}");
}

/// The same edge, one hop longer, and in both directions.
#[test]
fn receiver_mutability_and_parameter_access_reach_a_joint_fixpoint() {
    let text = "class Counter:\n\
                \x20   def __init__(self) -> None:\n\
                \x20       self.value: int = 0\n\
                \n\
                \x20   def raise_directly(self) -> int:\n\
                \x20       self.value = self.value + 1\n\
                \x20       return self.value\n\
                \n\
                \x20   def delegate(self) -> int:\n\
                \x20       return through(self)\n\
                \n\
                def through(counter: Counter) -> int:\n\
                \x20   return counter.raise_directly()\n\
                \n\
                def outer(counter: Counter) -> int:\n\
                \x20   return counter.delegate()\n";
    let emitted = emit(text);
    assert!(
        emitted.contains("pub fn through(counter: &mut Counter)"),
        "{emitted}"
    );
    assert!(
        emitted.contains("pub fn outer(counter: &mut Counter)"),
        "{emitted}"
    );
    assert!(emitted.contains("pub fn delegate(&mut self)"), "{emitted}");
}

/// A method forwarding `self` to a function that only reads keeps a shared receiver.
#[test]
fn a_method_forwarding_self_to_a_reading_free_function_keeps_a_shared_receiver() {
    let text = "class Counter:\n\
                \x20   def __init__(self) -> None:\n\
                \x20       self.value: int = 0\n\
                \n\
                \x20   def delegate(self) -> int:\n\
                \x20       return observe(self)\n\
                \n\
                def observe(counter: Counter) -> int:\n\
                \x20   return counter.value\n";
    let emitted = emit(text);
    assert!(
        emitted.contains("pub fn observe(counter: &Counter)"),
        "{emitted}"
    );
    assert!(emitted.contains("pub fn delegate(&self)"), "{emitted}");
}
