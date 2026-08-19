//! Lowering branches, loops, and reassignment.
//!
//! Three rules here are stricter than Python, and each is deliberate. A test must be a boolean,
//! because a subset that demands annotations everywhere should not then guess that an integer
//! means a condition. A name bound inside a branch does not survive it, because the alternative is
//! admitting names whose existence depends on a runtime test. And a name's type is fixed where it
//! was first bound, because a name that changes type is one a reader has to simulate the program
//! to follow.
//!
//! The reachability tests are the load-bearing ones: they decide which programs are accepted at
//! all, and getting them wrong in the permissive direction means generated code that does not
//! compile — a complaint about Rust rather than about the function the user wrote.

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

/// The type a `Bind` or `For` gave to `name`, searching nested bodies.
fn bound_ty(stmts: &[Stmt], name: &str) -> Option<Ty> {
    for stmt in stmts {
        let found = match stmt {
            Stmt::Bind { name: n, ty, .. } | Stmt::For { name: n, ty, .. } if n == name => {
                Some(ty.clone())
            }
            Stmt::If {
                then, otherwise, ..
            } => bound_ty(then, name).or_else(|| bound_ty(otherwise, name)),
            Stmt::While { body, .. } => bound_ty(body, name),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
        // A `for` binds its own name and may also bind others in its body.
        if let Stmt::For { body, .. } = stmt
            && let Some(ty) = bound_ty(body, name)
        {
            return Some(ty);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Reachability
// ---------------------------------------------------------------------------

#[test]
fn a_conditional_returning_on_both_branches_is_accepted() {
    let f = only(
        "def f(a: int) -> int:\n\
         \x20   if a > 0:\n\
         \x20       return 1\n\
         \x20   else:\n\
         \x20       return 2\n",
    );
    assert!(matches!(f.body.as_slice(), [Stmt::If { .. }]));
}

#[test]
fn a_conditional_with_no_alternative_does_not_return() {
    assert_eq!(
        reject(
            "def f(a: int) -> int:\n\
             \x20   if a > 0:\n\
             \x20       return 1\n"
        ),
        LowerErrorKind::MissingReturn
    );
}

#[test]
fn one_branch_returning_is_not_enough() {
    assert_eq!(
        reject(
            "def f(a: int) -> int:\n\
             \x20   if a > 0:\n\
             \x20       return 1\n\
             \x20   else:\n\
             \x20       b = 2\n"
        ),
        LowerErrorKind::MissingReturn
    );
}

#[test]
fn a_return_after_a_conditional_covers_it() {
    let f = only(
        "def f(a: int) -> int:\n\
         \x20   if a > 0:\n\
         \x20       return 1\n\
         \x20   return 2\n",
    );
    assert_eq!(f.body.len(), 2);
}

#[test]
fn a_loop_is_not_assumed_to_run() {
    // The body may execute zero times, and proving otherwise means evaluating the test.
    assert_eq!(
        reject(
            "def f(a: int) -> int:\n\
             \x20   while a > 0:\n\
             \x20       return 1\n"
        ),
        LowerErrorKind::MissingReturn
    );
}

#[test]
fn a_for_loop_is_not_assumed_to_run_either() {
    assert_eq!(
        reject(
            "def f(n: int) -> int:\n\
             \x20   for i in range(n):\n\
             \x20       return i\n"
        ),
        LowerErrorKind::MissingReturn
    );
}

#[test]
fn nested_conditionals_are_analysed_through() {
    let f = only(
        "def f(a: int) -> int:\n\
         \x20   if a > 0:\n\
         \x20       if a > 10:\n\
         \x20           return 2\n\
         \x20       else:\n\
         \x20           return 1\n\
         \x20   else:\n\
         \x20       return 0\n",
    );
    assert_eq!(f.body.len(), 1);
}

#[test]
fn a_unit_returning_function_needs_no_return() {
    let f = only(
        "def f(a: int) -> None:\n\
         \x20   if a > 0:\n\
         \x20       b = 1\n",
    );
    assert_eq!(f.ret, Ty::Unit);
}

#[test]
fn the_diagnostic_says_a_path_produces_no_value() {
    // "Missing return" alone sends the reader looking at the end of the function, which is not
    // where the problem is when one branch of a conditional is the gap.
    let text = message(
        "def f(a: int) -> int:\n\
         \x20   if a > 0:\n\
         \x20       return 1\n",
    );
    assert!(
        text.contains("path"),
        "diagnostic should point at the path that produces no value, got: {text}"
    );
}

// ---------------------------------------------------------------------------
// Conditionals
// ---------------------------------------------------------------------------

#[test]
fn an_alternative_lowers() {
    let f = only(
        "def f(a: int) -> int:\n\
         \x20   if a > 0:\n\
         \x20       return 1\n\
         \x20   else:\n\
         \x20       return 2\n",
    );
    let Stmt::If {
        then, otherwise, ..
    } = &f.body[0]
    else {
        panic!("expected a conditional");
    };
    assert_eq!(then.len(), 1);
    assert_eq!(otherwise.len(), 1);
}

#[test]
fn elif_lowers_as_a_nested_conditional() {
    let f = only(
        "def f(a: int) -> int:\n\
         \x20   if a > 10:\n\
         \x20       return 2\n\
         \x20   elif a > 0:\n\
         \x20       return 1\n\
         \x20   else:\n\
         \x20       return 0\n",
    );
    let Stmt::If { otherwise, .. } = &f.body[0] else {
        panic!("expected a conditional");
    };
    assert!(
        matches!(otherwise.as_slice(), [Stmt::If { .. }]),
        "elif should nest inside the alternative, got {otherwise:?}"
    );
}

#[test]
fn a_non_boolean_test_is_rejected() {
    let text = message(
        "def f(a: int) -> int:\n\
         \x20   if a:\n\
         \x20       return 1\n\
         \x20   return 0\n",
    );
    assert!(
        text.contains("bool"),
        "diagnostic should say a test must be a boolean, got: {text}"
    );
}

#[test]
fn a_name_bound_in_a_branch_is_not_visible_after_it() {
    let text = message(
        "def f(a: int) -> int:\n\
         \x20   if a > 0:\n\
         \x20       b = 1\n\
         \x20   return b\n",
    );
    assert!(
        text.contains("may not"),
        "diagnostic should say the binding may not have happened, got: {text}"
    );
}

// ---------------------------------------------------------------------------
// Loops
// ---------------------------------------------------------------------------

#[test]
fn a_while_loop_lowers() {
    let f = only(
        "def f(a: int, b: int) -> int:\n\
         \x20   while a < b:\n\
         \x20       a = a + 1\n\
         \x20   return a\n",
    );
    assert!(matches!(f.body[0], Stmt::While { .. }));
}

#[test]
fn a_non_boolean_while_test_is_rejected() {
    let text = message(
        "def f(n: int) -> int:\n\
         \x20   while n:\n\
         \x20       n = n - 1\n\
         \x20   return n\n",
    );
    assert!(
        text.contains("bool"),
        "diagnostic should say a test must be a boolean, got: {text}"
    );
}

#[test]
fn iterating_a_range_binds_an_integer() {
    let f = only(
        "def f(n: int) -> int:\n\
         \x20   total = 0\n\
         \x20   for i in range(n):\n\
         \x20       total = total + i\n\
         \x20   return total\n",
    );
    assert_eq!(bound_ty(&f.body, "i"), Some(Ty::Int));
}

#[test]
fn iterating_a_sequence_binds_its_element_type() {
    let f = only(
        "def f(xs: list[str]) -> int:\n\
         \x20   n = 0\n\
         \x20   for x in xs:\n\
         \x20       n = n + len(x)\n\
         \x20   return n\n",
    );
    assert_eq!(bound_ty(&f.body, "x"), Some(Ty::Str));
}

#[test]
fn iterating_a_mapping_binds_its_key_type() {
    // Python iterates a dict's keys, so anything else here would be a silent divergence.
    let f = only(
        "def f(d: dict[str, int]) -> int:\n\
         \x20   n = 0\n\
         \x20   for k in d:\n\
         \x20       n = n + len(k)\n\
         \x20   return n\n",
    );
    assert_eq!(bound_ty(&f.body, "k"), Some(Ty::Str));
}

#[test]
fn iterating_a_set_binds_its_element_type() {
    let f = only(
        "def f(s: set[int]) -> int:\n\
         \x20   n = 0\n\
         \x20   for v in s:\n\
         \x20       n = n + v\n\
         \x20   return n\n",
    );
    assert_eq!(bound_ty(&f.body, "v"), Some(Ty::Int));
}

#[test]
fn iterating_a_scalar_is_rejected() {
    let text = message(
        "def f(n: int) -> int:\n\
         \x20   for x in n:\n\
         \x20       n = n - 1\n\
         \x20   return n\n",
    );
    assert!(
        text.contains("int"),
        "diagnostic should report the type that cannot be iterated, got: {text}"
    );
}

#[test]
fn the_loop_variable_does_not_escape() {
    assert_eq!(
        reject(
            "def f(n: int) -> int:\n\
             \x20   for i in range(n):\n\
             \x20       n = n - 1\n\
             \x20   return i\n"
        ),
        LowerErrorKind::Unresolved
    );
}

#[test]
fn loop_control_inside_a_loop_lowers() {
    let f = only(
        "def f(n: int) -> int:\n\
         \x20   for i in range(n):\n\
         \x20       continue\n\
         \x20   while n > 0:\n\
         \x20       break\n\
         \x20   return n\n",
    );
    assert_eq!(f.body.len(), 3);
}

#[test]
fn loop_control_outside_a_loop_is_rejected() {
    let text = message(
        "def f(n: int) -> int:\n\
         \x20   break\n\
         \x20   return n\n",
    );
    assert!(
        text.contains("loop"),
        "diagnostic should say it is not inside a loop, got: {text}"
    );
}

#[test]
fn loop_control_reaches_the_nearest_enclosing_loop() {
    // A conditional does not reset the loop context, which is the common shape.
    let f = only(
        "def f(n: int) -> int:\n\
         \x20   while n > 0:\n\
         \x20       if n > 5:\n\
         \x20           break\n\
         \x20       n = n - 1\n\
         \x20   return n\n",
    );
    assert_eq!(f.body.len(), 2);
}

#[test]
fn a_for_else_clause_is_rejected() {
    // Python's `for`/`else` runs the alternative when the loop was not broken out of. Rare enough
    // that most readers misremember what it does, which is reason enough to keep it out.
    assert_eq!(
        reject(
            "def f(n: int) -> int:\n\
             \x20   for i in range(n):\n\
             \x20       n = n - 1\n\
             \x20   else:\n\
             \x20       n = 0\n\
             \x20   return n\n"
        ),
        LowerErrorKind::UnsupportedConstruct
    );
}

// ---------------------------------------------------------------------------
// Reassignment
// ---------------------------------------------------------------------------

#[test]
fn reassignment_lowers_and_keeps_the_type() {
    let f = only(
        "def f() -> int:\n\
         \x20   i = 0\n\
         \x20   i = i + 1\n\
         \x20   return i\n",
    );
    assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
    assert!(matches!(f.body[1], Stmt::Assign { ty: Ty::Int, .. }));
}

#[test]
fn a_different_type_is_rejected() {
    let text = message(
        "def f() -> int:\n\
         \x20   i = 0\n\
         \x20   i = \"x\"\n\
         \x20   return i\n",
    );
    assert!(
        text.contains("int") && text.contains("str"),
        "diagnostic should report both types, got: {text}"
    );
}

#[test]
fn promotion_applies_to_a_reassignment() {
    let f = only(
        "def f() -> float:\n\
         \x20   x: float = 1.0\n\
         \x20   x = 2\n\
         \x20   return x\n",
    );
    let Stmt::Assign { value, .. } = &f.body[1] else {
        panic!("expected an assignment");
    };
    assert!(
        matches!(value, Expr::ToFloat(_)),
        "the integer should carry an explicit conversion, got {value:?}"
    );
}

#[test]
fn an_annotation_on_a_rebinding_is_rejected() {
    // Accepting it would raise the question of whether the second annotation may differ.
    assert_eq!(
        reject(
            "def f() -> int:\n\
             \x20   i = 0\n\
             \x20   i: int = 1\n\
             \x20   return i\n"
        ),
        LowerErrorKind::Reassignment
    );
}

#[test]
fn a_parameter_may_be_reassigned() {
    let f = only(
        "def f(n: int) -> int:\n\
         \x20   n = n + 1\n\
         \x20   return n\n",
    );
    assert!(matches!(f.body[0], Stmt::Assign { ty: Ty::Int, .. }));
}

#[test]
fn reassignment_inside_a_loop_updates_the_outer_counter() {
    // The point of the scope stack: the assignment must find the frame that owns `i` rather than
    // introduce a new binding shadowing it, or the loop never terminates.
    let f = only(
        "def f(n: int) -> int:\n\
         \x20   i = 0\n\
         \x20   while i < n:\n\
         \x20       i = i + 1\n\
         \x20   return i\n",
    );
    let Stmt::While { body, .. } = &f.body[1] else {
        panic!("expected a loop");
    };
    assert!(
        matches!(body.as_slice(), [Stmt::Assign { .. }]),
        "the loop body should assign, not bind, got {body:?}"
    );
}

// ---------------------------------------------------------------------------
// range
// ---------------------------------------------------------------------------

/// The range a function's first `for` iterates.
fn range_of(f: &Function) -> (Expr, Expr, Expr) {
    let Some(Stmt::For {
        iter: Expr::Range { start, stop, step },
        ..
    }) = f.body.iter().find(|s| matches!(s, Stmt::For { .. }))
    else {
        panic!("expected a for over a range");
    };
    ((**start).clone(), (**stop).clone(), (**step).clone())
}

#[test]
fn one_argument_fills_in_start_and_step() {
    let f = only(
        "def f(n: int) -> None:\n\
         \x20   for i in range(n):\n\
         \x20       pass\n",
    );
    let (start, stop, step) = range_of(&f);
    assert_eq!(start, Expr::int(0));
    assert_eq!(stop, Expr::Name("n".into()));
    assert_eq!(step, Expr::int(1));
}

#[test]
fn two_and_three_arguments_are_carried_as_written() {
    let two = only(
        "def f(a: int, b: int) -> None:\n\
         \x20   for i in range(a, b):\n\
         \x20       pass\n",
    );
    let (start, stop, step) = range_of(&two);
    assert_eq!(start, Expr::Name("a".into()));
    assert_eq!(stop, Expr::Name("b".into()));
    assert_eq!(step, Expr::int(1));

    let three = only(
        "def f(a: int, b: int, c: int) -> None:\n\
         \x20   for i in range(a, b, c):\n\
         \x20       pass\n",
    );
    let (start, stop, step) = range_of(&three);
    assert_eq!(start, Expr::Name("a".into()));
    assert_eq!(stop, Expr::Name("b".into()));
    assert_eq!(step, Expr::Name("c".into()));
}

#[test]
fn a_non_integer_range_argument_is_rejected() {
    let text = message(
        "def f(x: str) -> None:\n\
         \x20   for i in range(x):\n\
         \x20       pass\n",
    );
    assert!(
        text.contains("str"),
        "diagnostic should report the type, got: {text}"
    );
}

#[test]
fn wrong_range_arity_is_rejected() {
    assert_eq!(
        reject(
            "def f() -> None:\n\
             \x20   for i in range():\n\
             \x20       pass\n"
        ),
        LowerErrorKind::ArityMismatch
    );
    assert_eq!(
        reject(
            "def f(a: int) -> None:\n\
             \x20   for i in range(a, a, a, a):\n\
             \x20       pass\n"
        ),
        LowerErrorKind::ArityMismatch
    );
}

#[test]
fn a_user_function_named_range_is_rejected() {
    let text = message("def range(n: int) -> int:\n    return n\n");
    assert!(
        text.contains("reserved"),
        "diagnostic should say range is reserved, got: {text}"
    );
}

#[test]
fn a_range_outside_a_loop_is_rejected() {
    // A range is only meaningful as something to iterate; there is no range value in the subset.
    assert_eq!(
        reject(
            "def f(n: int) -> None:\n\
             \x20   r = range(n)\n"
        ),
        LowerErrorKind::UnsupportedConstruct
    );
}

// ---------------------------------------------------------------------------
// Scope
// ---------------------------------------------------------------------------

#[test]
fn a_flat_body_is_unaffected_by_the_scope_stack() {
    // The stack must degenerate to what a single frame did before, or every existing program
    // changes meaning for no reason.
    let f = only(
        "def f(a: int) -> int:\n\
         \x20   b = a\n\
         \x20   c = b + 1\n\
         \x20   return c\n",
    );
    assert_eq!(bound_ty(&f.body, "c"), Some(Ty::Int));
}

#[test]
fn an_inner_block_reads_an_outer_name() {
    let f = only(
        "def f(a: int) -> int:\n\
         \x20   base = a + 1\n\
         \x20   if a > 0:\n\
         \x20       return base\n\
         \x20   return 0\n",
    );
    assert_eq!(f.body.len(), 3);
}
