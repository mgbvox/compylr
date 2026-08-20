//! The stage between lowering and emission: verification, then passes.
//!
//! Verification is the piece with immediate value, and it is worth being clear about why it looks
//! redundant. For Python it never fires: lowering enforces the same invariants while it works, so
//! every accepted fixture passes verification trivially. It exists for the *second* frontend,
//! which will not have re-derived them — and whose mistakes would otherwise arrive as a backend
//! complaining about Rust rather than as a diagnostic about the program.

use compylr::bridge::{CompileFailure, compile, compile_with};
use compylr::ir::{Expr, Function, Param, Stmt, Ty, Unit};
use compylr::span::Span;
use compylr_core::pass::{Optimization, PassConfig};
use compylr_core::verify::verify;

const DOUBLE: &str = "def double(n: int) -> int:\n    return n * 2\n";

#[test]
fn an_accepted_program_verifies() {
    let unit = compylr::frontend::lookup("python")
        .unwrap()
        .lower(&[DOUBLE.to_string()])
        .unwrap();
    assert!(verify(&unit).is_ok());
}

/// A tree a frontend could produce that no backend can render.
///
/// Built by hand because the Python frontend cannot emit it — which is exactly the point. The
/// check is here for a frontend that has not re-derived lowering's rules.
#[test]
fn a_unit_no_backend_could_render_is_rejected_before_emission() {
    let mut unit = Unit::new();
    unit.add_function(Function {
        name: "caller".to_string(),
        params: vec![Param {
            name: "n".to_string(),
            ty: Ty::Int,
        }],
        ret: Ty::Int,
        body: vec![Stmt::Return(Expr::Call {
            callee: "defined_nowhere".to_string(),
            args: vec![],
        })],
        doc: None,
        span: Span::default(),
    })
    .unwrap();

    let error = verify(&unit).expect_err("the callee exists in no unit");
    assert!(error.to_string().contains("defined_nowhere"), "{error}");
}

#[test]
fn the_default_pipeline_reports_what_ran() {
    let compiled = compile(&[DOUBLE.to_string()], "rust").expect("must compile");
    assert_eq!(compiled.passes, ["constant-folding"]);
}

/// Turning optimization off must not change what the program computes.
///
/// Compared on the *translated* file. The crate around it embeds the module name, which encodes
/// the pass configuration on purpose — two builds of one source under different settings are
/// different artifacts, and CPython cannot hold two under one name. `DOUBLE` has no constant to
/// fold, so the translation is byte-identical; where folding does apply only the result is
/// required to match, which is what the execution tests check.
#[test]
fn optimization_off_produces_the_same_program() {
    let optimized = compile_with(&[DOUBLE.to_string()], "rust", &PassConfig::default()).unwrap();
    let plain = compile_with(
        &[DOUBLE.to_string()],
        "rust",
        &PassConfig {
            optimization: Optimization::None,
        },
    )
    .unwrap();

    assert_eq!(
        optimized.target_sources["src/generated.rs"],
        plain.target_sources["src/generated.rs"]
    );
    assert!(plain.passes.is_empty());
    assert_eq!(optimized.passes, ["constant-folding"]);
}

/// The two builds are the same program and different artifacts, and the names say both.
#[test]
fn builds_under_different_configurations_load_under_different_names() {
    let optimized = compile_with(&[DOUBLE.to_string()], "rust", &PassConfig::default()).unwrap();
    let plain = compile_with(
        &[DOUBLE.to_string()],
        "rust",
        &PassConfig {
            optimization: Optimization::None,
        },
    )
    .unwrap();

    assert_eq!(optimized.fingerprint, plain.fingerprint, "the same program");
    assert_ne!(
        optimized.module_name, plain.module_name,
        "different artifacts, which must not collide in one process"
    );
}

/// Folding is visible in the artifact, which is the window a reader has into what ran.
#[test]
fn a_folded_constant_appears_in_the_ir_artifact() {
    let folding = "def answer() -> int:\n    return 6 * 7\n";
    let optimized = compile_with(&[folding.to_string()], "rust", &PassConfig::default()).unwrap();
    let plain = compile_with(
        &[folding.to_string()],
        "rust",
        &PassConfig {
            optimization: Optimization::None,
        },
    )
    .unwrap();

    assert!(
        !optimized.ir_artifact.contains("\"Mul\""),
        "the multiplication must be gone:\n{}",
        optimized.ir_artifact
    );
    assert!(
        plain.ir_artifact.contains("\"Mul\""),
        "without folding the multiplication must still be there"
    );
    // The fingerprint is taken before the pass, so the two are the same program.
    assert_eq!(optimized.fingerprint, plain.fingerprint);
}

/// The fingerprint is taken before optimization.
///
/// Otherwise enabling a pass would look to the rebuild cache exactly like the user editing their
/// code, and every project would rebuild on a compiler setting nobody changed in their source.
#[test]
fn the_fingerprint_does_not_move_with_the_pass_configuration() {
    let optimized = compile_with(&[DOUBLE.to_string()], "rust", &PassConfig::default()).unwrap();
    let plain = compile_with(
        &[DOUBLE.to_string()],
        "rust",
        &PassConfig {
            optimization: Optimization::None,
        },
    )
    .unwrap();
    assert_eq!(optimized.fingerprint, plain.fingerprint);
}

/// Configurations that produce different artifacts must be distinguishable in build state.
#[test]
fn each_configuration_has_its_own_key() {
    assert_ne!(
        Optimization::Default.key(),
        Optimization::None.key(),
        "build state records this key; two configurations sharing one would reuse the wrong build"
    );
}

/// A pair with no directed passes compiles with the agnostic set alone.
#[test]
fn a_pair_with_no_directed_passes_still_compiles() {
    assert!(compylr_registry::passes::for_pair("python", "rust").is_empty());
    assert!(compile(&[DOUBLE.to_string()], "rust").is_ok());
}

/// Verification failures reach the caller as a rejection, not as a backend complaint.
#[test]
fn a_verification_failure_is_reported_as_an_unsupported_program() {
    // A call to a function that exists nowhere: accepted by lowering, which defers cross-source
    // resolution, and caught when the whole unit is in hand.
    let failure = compile(
        &["def f(n: int) -> int:\n    return elsewhere(n)\n".to_string()],
        "rust",
    )
    .expect_err("the callee is in no source");

    match failure {
        CompileFailure::Unsupported { message, .. } => {
            assert!(message.contains("elsewhere"), "{message}");
        }
        other => panic!("expected an unsupported-program failure, got {other:?}"),
    }
}
