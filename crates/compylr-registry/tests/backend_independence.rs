//! A frontend's IR does not depend on the backend it was directed at.
//!
//! This is an invariant, not a quantity to minimize. A frontend is defined to be unaware of the
//! target, so a difference here is a target leak in the frontend — a defect with a location, not a
//! score to drive down. It is checked from `compylr-registry` because checking it needs a frontend
//! and two backends at once, and this is the one crate permitted to know them all.
//!
//! The check is not vacuous. `Frontend::lower` takes no backend, but it *does* take a
//! [`Behavior`](compylr_ir::Behavior), and that behavior is resolved from the `(source, target)`
//! pair — so a target could reach the IR through the negotiation even though it never reaches the
//! frontend directly. What this asserts is that it does not.

use compylr_core::{Behavior, BehaviorRequest, LanguagePair, Source, diff, resolve};
use compylr_ir::Unit;
use compylr_registry::{backends, frontends};

/// A program touching the operations whose modes the negotiation could plausibly move: overflow,
/// both divisions, remainder, indexing, and text length.
const PROGRAM: &str = "\
def measure(xs: list[int], text: str, a: int, b: int) -> int:
    total: int = a + b
    total = total * a
    total = total - b
    exact: float = a / b
    floored: int = a // b
    left: int = a % b
    first: int = xs[0]
    width: int = len(text)
    return total + floored + left + first + width
";

/// Lower `PROGRAM` as a build directed at `target` would.
fn lower_for(target: &str) -> Unit {
    let frontend = frontends::lookup("python").expect("python frontend");
    let backend = backends::lookup(target).expect("backend");

    let mut known: Vec<&str> = frontends::names();
    known.extend(backends::names());
    known.sort_unstable();
    known.dedup();

    let behavior: Behavior = resolve(
        &BehaviorRequest::default(),
        &LanguagePair {
            source: frontend.name(),
            source_behavior: frontend.behavior(),
            target: backend.name(),
            target_behavior: backend.behavior(),
            known: &known,
        },
        None,
    )
    .expect("the pair resolves");

    frontend
        .lower(&[Source::new(PROGRAM, behavior)])
        .expect("the program is in the accepted subset")
}

/// The same source, directed at two different targets, is the same program.
#[test]
fn one_frontend_lowers_the_same_ir_for_every_backend() {
    let for_rust = lower_for("rust");
    let for_go = lower_for("go");

    // Compared by the differ rather than by `==` so that a failure says which member and which
    // node leaked the target, instead of only that two large values are unequal.
    let found = diff::divergence(&for_rust, &for_go);
    assert!(
        found.is_zero(),
        "the python frontend leaked its target into the IR: {:?}",
        found.divergent().map(|m| m.notes()).collect::<Vec<_>>()
    );
}

/// The stronger form: not merely equivalent under the differ, but the same value.
///
/// The differ deliberately disregards the semantic modes, so it would report agreement between two
/// units that resolved `//` differently. That is the right answer for a *cross-language* score and
/// the wrong one here, where the whole question is whether the target moved anything at all.
#[test]
fn the_lowered_units_are_identical_not_merely_equivalent() {
    assert_eq!(
        lower_for("rust"),
        lower_for("go"),
        "the negotiated behavior differs by target, so the target reaches the IR"
    );
}

/// The fingerprint is what a rebuild keys off, so a target moving it would mean two targets
/// disagreeing about whether a cached build is current.
#[test]
fn the_fingerprint_does_not_depend_on_the_target() {
    assert_eq!(
        lower_for("rust").fingerprint(),
        lower_for("go").fingerprint()
    );
}
