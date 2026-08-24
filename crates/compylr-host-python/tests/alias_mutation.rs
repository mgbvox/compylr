//! Mutating a local that aliases a parameter.
//!
//! `add-collection-mutation` rejected mutating a parameter, because collections cross the boundary
//! by value and the caller would not observe the change. One extra line defeated it: in Python
//! `copied = xs` binds a second name to the same object, so mutating either is observable — while
//! compylr's bind is a copy, so neither is. The rule was correct about one spelling of the hazard
//! and blind to the other.
//!
//! The transitive case is the one that matters most. A rule that stops at one binding is defeated
//! by writing two, and the failure is silent: the program compiles and diverges.

use compylr_diagnostics::error::LowerErrorKind;
use compylr_frontend_python::frontend::parse_source;
use compylr_frontend_python::lower::lower_source;

fn accepts(source: &str) {
    let parsed = parse_source(source).expect("fixture must parse");
    lower_source(&parsed, python_stance())
        .unwrap_or_else(|e| panic!("should lower: {}", e.render(source)));
}

fn reject(source: &str) -> LowerErrorKind {
    let parsed = parse_source(source).expect("fixture must parse");
    match lower_source(&parsed, python_stance()) {
        Ok(_) => panic!("should have been rejected but lowered:\n{source}"),
        Err(error) => error.kind(),
    }
}

fn message(source: &str) -> String {
    let parsed = parse_source(source).expect("fixture must parse");
    match lower_source(&parsed, python_stance()) {
        Ok(_) => panic!("should have been rejected but lowered:\n{source}"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn appending_to_an_alias_of_a_parameter_is_rejected() {
    assert_eq!(
        reject(
            "def f(xs: list[int]) -> list[int]:\n\
             \x20   copied = xs\n\
             \x20   copied.append(1)\n\
             \x20   return copied\n"
        ),
        LowerErrorKind::UnsupportedConstruct
    );
}

#[test]
fn assigning_into_an_alias_of_a_parameter_is_rejected() {
    assert_eq!(
        reject(
            "def f(d: dict[str, int]) -> dict[str, int]:\n\
             \x20   copied = d\n\
             \x20   copied[\"a\"] = 1\n\
             \x20   return copied\n"
        ),
        LowerErrorKind::UnsupportedConstruct
    );
}

#[test]
fn aliasing_is_transitive() {
    // A rule that stopped at one binding would be defeated by writing two, and nothing would
    // report it: the program compiles and quietly disagrees with its interpreted original.
    assert_eq!(
        reject(
            "def f(xs: list[int]) -> list[int]:\n\
             \x20   first = xs\n\
             \x20   second = first\n\
             \x20   second.append(1)\n\
             \x20   return second\n"
        ),
        LowerErrorKind::UnsupportedConstruct
    );
}

#[test]
fn an_annotated_alias_is_tracked_too() {
    // The annotation changes nothing about what the name denotes.
    assert_eq!(
        reject(
            "def f(xs: list[int]) -> list[int]:\n\
             \x20   copied: list[int] = xs\n\
             \x20   copied.append(1)\n\
             \x20   return copied\n"
        ),
        LowerErrorKind::UnsupportedConstruct
    );
}

#[test]
fn the_diagnostic_names_the_alias_and_its_origin() {
    // The refusal points at a local the user just wrote. Without naming the parameter it came
    // from, they have no reason to look at the signature, and the fix is not discoverable.
    let text = message(
        "def f(xs: list[int]) -> list[int]:\n\
         \x20   copied = xs\n\
         \x20   copied.append(1)\n\
         \x20   return copied\n",
    );
    assert!(
        text.contains("copied"),
        "diagnostic should name the local, got: {text}"
    );
    assert!(
        text.contains("xs"),
        "diagnostic should name the parameter it came from, got: {text}"
    );
    assert!(
        text.contains("copy"),
        "diagnostic should still explain the copy, got: {text}"
    );
}

#[test]
fn a_fresh_collection_filled_from_a_parameter_may_be_mutated() {
    // The workaround the diagnostic recommends. It must actually work, or the advice is empty.
    accepts(
        "def f(xs: list[int]) -> list[int]:\n\
         \x20   out: list[int] = []\n\
         \x20   for x in xs:\n\
         \x20       out.append(x)\n\
         \x20   out.append(0)\n\
         \x20   return out\n",
    );
}

#[test]
fn a_local_rebound_away_from_a_parameter_may_be_mutated() {
    // Rebinding to a fresh collection is exactly the workaround, so a rule that remembered the
    // name had *ever* aliased a parameter would refuse the thing it recommends.
    accepts(
        "def f(xs: list[int]) -> list[int]:\n\
         \x20   working = xs\n\
         \x20   working = []\n\
         \x20   working.append(1)\n\
         \x20   return working\n",
    );
}

#[test]
fn reading_through_an_alias_is_unaffected() {
    accepts(
        "def f(xs: list[int], d: dict[str, int]) -> int:\n\
         \x20   items = xs\n\
         \x20   mapping = d\n\
         \x20   return items[0] + mapping[\"a\"] + len(items)\n",
    );
}

#[test]
fn a_local_from_a_fresh_value_is_unaffected() {
    accepts(
        "def build(n: int) -> list[int]:\n\
         \x20   made: list[int] = [n]\n\
         \x20   also = made\n\
         \x20   also.append(n)\n\
         \x20   return also\n",
    );
}

#[test]
fn aliasing_a_scalar_parameter_is_unrestricted() {
    // A scalar has no mutation to observe, so nothing about it should be tracked -- a user who
    // writes `total = count` must never see a word about aliasing.
    accepts(
        "def f(count: int, label: str) -> int:\n\
         \x20   total = count\n\
         \x20   total = total + 1\n\
         \x20   name = label\n\
         \x20   return total + len(name)\n",
    );
}

#[test]
fn a_local_bound_from_a_call_is_unaffected() {
    // A call returns a fresh value, whatever it was built from.
    accepts(
        "def source(xs: list[int]) -> list[int]:\n\
         \x20   out: list[int] = []\n\
         \x20   return out\n\
         \n\
         def f(xs: list[int]) -> list[int]:\n\
         \x20   made = source(xs)\n\
         \x20   made.append(1)\n\
         \x20   return made\n",
    );
}

/// Python's own stance, which is what an unconfigured compilation resolves to.
///
/// Read from the frontend's declaration rather than rebuilt here, so these tests lower under the
/// same bundle the pipeline uses.
fn python_stance() -> compylr_ir::Behavior {
    compylr_ir::Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR)
}
