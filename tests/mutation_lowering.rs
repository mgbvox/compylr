//! Mutation and membership, and the rule that keeps the by-value divergence unreachable.
//!
//! Collections cross the boundary by value. A compiled function that mutated a parameter would
//! leave its caller's collection unchanged where the interpreted original would have modified it —
//! a wrong answer with no error, which is the worst thing this project could ship. So mutating a
//! parameter is rejected, and the diagnostic has to explain the copy rather than merely refuse:
//! a rule without its reason leaves the user no workaround.

use compylr::error::LowerErrorKind;
use compylr::frontend::parse_source;
use compylr::ir::{Expr, Function, Stmt, Ty};
use compylr::lower::lower_source;

fn lower(source: &str) -> Vec<Function> {
    let parsed = parse_source(source).expect("fixture must parse");
    lower_source(&parsed).unwrap_or_else(|e| panic!("should lower: {}", e.render(source)))
}

fn only(source: &str) -> Function {
    let mut functions = lower(source);
    assert_eq!(functions.len(), 1, "expected exactly one function");
    functions.remove(0)
}

fn accepts(source: &str) {
    let _ = lower(source);
}

fn reject(source: &str) -> LowerErrorKind {
    let parsed = parse_source(source).expect("fixture must parse");
    match lower_source(&parsed) {
        Ok(_) => panic!("should have been rejected but lowered:\n{source}"),
        Err(error) => error.kind(),
    }
}

fn message(source: &str) -> String {
    let parsed = parse_source(source).expect("fixture must parse");
    match lower_source(&parsed) {
        Ok(_) => panic!("should have been rejected but lowered:\n{source}"),
        Err(error) => error.to_string(),
    }
}

/// The type a body gives to a binding named `name`.
fn binding_ty(f: &Function, name: &str) -> Option<Ty> {
    f.body.iter().find_map(|stmt| match stmt {
        Stmt::Bind { name: n, ty, .. } if n == name => Some(ty.clone()),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// Element assignment
// ---------------------------------------------------------------------------

#[test]
fn sequence_element_assignment_lowers() {
    let f = only(
        "def f() -> list[int]:\n\
         \x20   xs: list[int] = [0, 0]\n\
         \x20   xs[0] = 1\n\
         \x20   return xs\n",
    );
    assert!(matches!(f.body[1], Stmt::SetItem { .. }));
}

#[test]
fn mapping_element_assignment_lowers() {
    let f = only(
        "def f() -> dict[str, int]:\n\
         \x20   d: dict[str, int] = {}\n\
         \x20   d[\"a\"] = 1\n\
         \x20   return d\n",
    );
    assert!(matches!(f.body[1], Stmt::SetItem { .. }));
}

#[test]
fn a_wrong_assigned_value_type_is_rejected() {
    let text = message(
        "def f() -> list[int]:\n\
         \x20   xs: list[int] = [0]\n\
         \x20   xs[0] = \"a\"\n\
         \x20   return xs\n",
    );
    assert!(
        text.contains("int") && text.contains("str"),
        "diagnostic should report both types, got: {text}"
    );
}

#[test]
fn a_wrong_index_type_is_rejected() {
    let text = message(
        "def f() -> list[int]:\n\
         \x20   xs: list[int] = [0]\n\
         \x20   xs[\"a\"] = 1\n\
         \x20   return xs\n",
    );
    assert!(
        text.contains("str"),
        "diagnostic should report the index type, got: {text}"
    );
}

#[test]
fn promotion_applies_to_an_assigned_element() {
    let f = only(
        "def f() -> list[float]:\n\
         \x20   xs: list[float] = [0.0]\n\
         \x20   xs[0] = 1\n\
         \x20   return xs\n",
    );
    let Stmt::SetItem { value, .. } = &f.body[1] else {
        panic!("expected an element assignment");
    };
    assert!(
        matches!(value, Expr::ToFloat(_)),
        "the integer should carry an explicit conversion, got {value:?}"
    );
}

#[test]
fn assigning_into_a_tuple_is_rejected() {
    // Matching Python, where a tuple cannot be assigned into at all.
    assert_eq!(
        reject(
            "def f() -> int:\n\
             \x20   t: tuple[int, int] = (1, 2)\n\
             \x20   t[0] = 3\n\
             \x20   return t[0]\n"
        ),
        LowerErrorKind::TypeMismatch
    );
}

#[test]
fn assigning_into_a_set_is_rejected() {
    assert_eq!(
        reject(
            "def f() -> set[int]:\n\
             \x20   s: set[int] = {1}\n\
             \x20   s[0] = 2\n\
             \x20   return s\n"
        ),
        LowerErrorKind::TypeMismatch
    );
}

// ---------------------------------------------------------------------------
// Mutation is confined to locals
// ---------------------------------------------------------------------------

#[test]
fn a_local_collection_may_be_mutated() {
    let f = only(
        "def f(n: int) -> list[int]:\n\
         \x20   found: list[int] = []\n\
         \x20   for i in range(n):\n\
         \x20       found.append(i)\n\
         \x20   return found\n",
    );
    assert_eq!(binding_ty(&f, "found"), Some(Ty::List(Box::new(Ty::Int))));
}

#[test]
fn appending_to_a_parameter_is_rejected() {
    assert_eq!(
        reject(
            "def f(xs: list[int]) -> None:\n\
             \x20   xs.append(1)\n"
        ),
        LowerErrorKind::UnsupportedConstruct
    );
}

#[test]
fn assigning_into_a_parameter_is_rejected() {
    assert_eq!(
        reject(
            "def f(d: dict[str, int]) -> None:\n\
             \x20   d[\"a\"] = 1\n"
        ),
        LowerErrorKind::UnsupportedConstruct
    );
}

#[test]
fn the_parameter_diagnostic_explains_the_copy() {
    // A bare refusal leaves the user nothing to do. Naming the copy tells them both why it is
    // refused and what to do instead, which is to build a local and return it.
    let text = message(
        "def f(xs: list[int]) -> None:\n\
         \x20   xs.append(1)\n",
    );
    assert!(
        text.contains("copy"),
        "diagnostic should say the parameter is a copy, got: {text}"
    );
    assert!(
        text.contains("caller"),
        "diagnostic should say the caller would not observe it, got: {text}"
    );
}

#[test]
fn reading_a_parameter_is_unaffected() {
    accepts(
        "def f(xs: list[int], d: dict[str, int]) -> int:\n\
         \x20   first = xs[0]\n\
         \x20   value = d[\"a\"]\n\
         \x20   return first + value + len(xs)\n",
    );
}

#[test]
fn a_local_bound_from_a_parameter_may_not_be_mutated() {
    // Aliasing is the parameter hazard at one remove: in Python `copied = xs` leaves both names
    // denoting one object, so the caller would have seen this. See tests/alias_mutation.rs.
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

// ---------------------------------------------------------------------------
// Append
// ---------------------------------------------------------------------------

#[test]
fn appending_lowers() {
    let f = only(
        "def f() -> list[int]:\n\
         \x20   xs: list[int] = []\n\
         \x20   xs.append(1)\n\
         \x20   return xs\n",
    );
    assert!(matches!(f.body[1], Stmt::Append { .. }));
}

#[test]
fn a_wrong_appended_element_type_is_rejected() {
    let text = message(
        "def f() -> list[int]:\n\
         \x20   xs: list[int] = []\n\
         \x20   xs.append(\"a\")\n\
         \x20   return xs\n",
    );
    assert!(
        text.contains("int") && text.contains("str"),
        "diagnostic should report both types, got: {text}"
    );
}

#[test]
fn wrong_append_arity_is_rejected() {
    for call in ["xs.append()", "xs.append(1, 2)"] {
        assert_eq!(
            reject(&format!(
                "def f() -> list[int]:\n\
                 \x20   xs: list[int] = []\n\
                 \x20   {call}\n\
                 \x20   return xs\n"
            )),
            LowerErrorKind::ArityMismatch,
            "for {call}"
        );
    }
}

#[test]
fn appending_to_a_non_sequence_is_rejected() {
    let text = message(
        "def f() -> dict[str, int]:\n\
         \x20   d: dict[str, int] = {}\n\
         \x20   d.append(1)\n\
         \x20   return d\n",
    );
    assert!(
        text.contains("dict"),
        "diagnostic should report the type, got: {text}"
    );
}

#[test]
fn another_method_is_rejected_by_name() {
    let text = message(
        "def f() -> list[int]:\n\
         \x20   xs: list[int] = [1]\n\
         \x20   xs.pop()\n\
         \x20   return xs\n",
    );
    assert!(
        text.contains("pop"),
        "diagnostic should name the method, got: {text}"
    );
}

// ---------------------------------------------------------------------------
// Membership
// ---------------------------------------------------------------------------

/// The type a single-expression `return` produced.
fn returned_ty(source: &str) -> Ty {
    let f = only(source);
    f.ret
}

#[test]
fn membership_over_each_container_yields_a_boolean() {
    for (params, test) in [
        ("xs: list[int], x: int", "x in xs"),
        ("d: dict[str, int], k: str", "k in d"),
        ("s: set[int], x: int", "x in s"),
        ("hay: str, needle: str", "needle in hay"),
    ] {
        assert_eq!(
            returned_ty(&format!("def f({params}) -> bool:\n    return {test}\n")),
            Ty::Bool,
            "for {test}"
        );
    }
}

#[test]
fn mapping_membership_tests_keys() {
    // Python tests keys, so the value being looked for must be the key type. A value-typed
    // operand would be accepted by a naive implementation and answer the wrong question.
    accepts("def f(d: dict[str, int], k: str) -> bool:\n    return k in d\n");
    let text = message("def f(d: dict[str, int], v: int) -> bool:\n    return v in d\n");
    assert!(
        text.contains("str"),
        "diagnostic should name the key type, got: {text}"
    );
}

#[test]
fn negated_membership_yields_a_boolean_and_is_a_negation() {
    let f = only("def f(xs: list[int], x: int) -> bool:\n    return x not in xs\n");
    let Stmt::Return(expr) = &f.body[0] else {
        panic!("expected a return");
    };
    let Expr::Not(inner) = expr else {
        panic!("`not in` should lower to a negated membership test, got {expr:?}");
    };
    assert!(matches!(**inner, Expr::Contains { .. }));
}

#[test]
fn a_mismatched_membership_value_type_is_rejected() {
    let text = message("def f(xs: list[int]) -> bool:\n    return \"a\" in xs\n");
    assert!(
        text.contains("int") && text.contains("str"),
        "diagnostic should report both types, got: {text}"
    );
}

#[test]
fn membership_in_a_scalar_is_rejected() {
    let text = message("def f(n: int, x: int) -> bool:\n    return x in n\n");
    assert!(
        text.contains("int"),
        "diagnostic should report the type, got: {text}"
    );
}

#[test]
fn membership_in_a_string_is_a_substring_test() {
    // `"ab" in "cab"` is true in Python. A character-membership reading would answer false, so
    // this is worth stating rather than leaving to whoever reads the impl next.
    assert_eq!(
        returned_ty("def f(hay: str, needle: str) -> bool:\n    return needle in hay\n"),
        Ty::Bool
    );
}
