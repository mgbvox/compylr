//! Host bridges resolve by *pair*.
//!
//! This is where compylr stops resembling LLVM. LLVM's frontends and backends compose N + M
//! because it emits object code and never calls back into the source language. compylr's whole
//! purpose is that the source language calls the result, and a calling convention is a
//! negotiation between two runtimes — who owns the memory, how errors signal, how strings encode.
//! Neither side can decide it alone, so neither side can own the code.
//!
//! The cost is therefore N x M, and these tests exist to keep it *visible*: a pair with a backend
//! but no bridge is a specific, reportable answer rather than a missing method or a wrong guess.

use compylr::bridge_registry;
use compylr::ir::{Expr, Function, Literal, Param, Stmt, Ty, Unit};
use compylr::span::Span;
use compylr_core::bridge::BuildKey;
use compylr_core::pass::Optimization;

fn key_for(unit: &Unit) -> BuildKey {
    BuildKey {
        fingerprint: unit.fingerprint(),
        target: "rust".to_string(),
        passes: Optimization::Default.key(),
    }
}

fn unit_with_one_function() -> Unit {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "answer".to_string(),
        params: vec![Param {
            name: "n".to_string(),
            ty: Ty::Int,
        }],
        ret: Ty::Int,
        body: vec![Stmt::Return(Expr::Literal(Literal::Int(42)))],
        doc: None,
        span: Span::default(),
    })
    .expect("a fresh unit accepts a function");
    unit
}

#[test]
fn the_python_rust_pair_is_bridged() {
    let bridge = bridge_registry::lookup("python", "rust").expect("this pair must be bridged");
    assert_eq!(bridge.source(), "python");
    assert_eq!(bridge.target(), "rust");
}

#[test]
fn a_bridged_pair_produces_a_loadable_artifact() {
    let bridge = bridge_registry::lookup("python", "rust").unwrap();
    let unit = unit_with_one_function();
    let artifact = bridge
        .emit(&unit, &key_for(&unit))
        .expect("the unit is inside the supported subset");

    assert!(
        artifact.files.contains_key("src/bindings.rs"),
        "a bridge exists to produce the boundary layer"
    );
    assert!(!artifact.manifest.is_empty());
    assert!(!artifact.loaded_as.is_empty());
}

/// Generating a target and calling into it are separate abilities.
///
/// The honest answer for a pair compylr can generate but not call back from is not "unknown
/// target" — the target is known and implemented. Saying so would send someone looking for a
/// backend that is already there.
#[test]
fn an_unbridged_pair_names_both_languages() {
    let error = bridge_registry::lookup("python", "go").expect_err("go has no Python bridge");
    assert!(error.is_unbridged(), "{error}");

    let rendered = error.to_string();
    assert!(rendered.contains("python"), "{rendered}");
    assert!(rendered.contains("go"), "{rendered}");
}

/// A caller must be able to tell "no bridge" from "no backend" without reading prose.
#[test]
fn an_unbridged_pair_is_distinguishable_from_an_unimplemented_target() {
    let unbridged = bridge_registry::lookup("python", "go").expect_err("no bridge");
    let unimplemented = compylr::backend::lookup("go").expect_err("no backend either");

    assert!(unbridged.is_unbridged());
    assert!(unimplemented.is_not_implemented());
    // Different types entirely, which is the strongest form the distinction can take: a caller
    // cannot accidentally treat one as the other.
    assert!(!unimplemented.is_unknown());
}

/// Adding a backend must not make an unrelated source language claim it can call it.
#[test]
fn a_bridge_is_not_assumed_from_either_side() {
    for (source, target) in [("typescript", "rust"), ("python", "cpp"), ("go", "go")] {
        let error = bridge_registry::lookup(source, target)
            .expect_err("only the (python, rust) pair is bridged today");
        assert!(error.is_unbridged(), "{source} -> {target}: {error}");
    }
}

#[test]
fn every_registered_pair_reports_its_own_endpoints() {
    let pairs = bridge_registry::pairs();
    assert!(!pairs.is_empty());
    for (source, target) in &pairs {
        let bridge = bridge_registry::lookup(source, target).expect("a listed pair must resolve");
        assert_eq!(&bridge.source(), source);
        assert_eq!(&bridge.target(), target);
    }
}

/// The bridge reads the IR and nothing else.
///
/// A unit that has been through the artifact and back holds exactly what the IR models. If the
/// binding layer differed, the bridge would be reading something the IR does not carry — and
/// whatever that is would not survive to a second frontend.
#[test]
fn a_binding_layer_is_the_same_from_a_deserialized_unit() {
    let bridge = bridge_registry::lookup("python", "rust").unwrap();
    let unit = unit_with_one_function();

    let key = key_for(&unit);
    let from_memory = bridge.emit(&unit, &key).unwrap();
    let round_tripped = Unit::from_json(&unit.to_json().unwrap()).unwrap();
    let from_artifact = bridge.emit(&round_tripped, &key).unwrap();

    assert_eq!(from_memory.files, from_artifact.files);
    assert_eq!(from_memory.manifest, from_artifact.manifest);
    assert_eq!(from_memory.loaded_as, from_artifact.loaded_as);
}
