## Context

See `proposal.md` — Why. The relevant existing state:

- `Ty` is `{ Int, Bool, Str, Unit }` and every IR type derives `Debug, Clone, PartialEq, Eq,
  Hash`. `Function::fingerprint` hashes the structure, and `Unit::fingerprint` combines member
  fingerprints order-independently.
- `lower_expr` currently performs **no** typing. It validates shape (supported operators,
  resolvable names) and builds IR. The only type reasoning anywhere is a scope lookup for the
  alias rule and a declared-vs-aliased comparison.
- `Scope` is `HashMap<String, Ty>`, populated from parameters and extended per binding.
- Call targets are deliberately unresolved during lowering; `Unit::validate` resolves them
  once the unit is assembled, so lowering never depends on decoration order.

This change turns `lower_expr` into a lowering pass *and* a small type checker. That is the
central shift: the previous slice could lower without understanding meaning, and this one
cannot.

## Goals / Non-Goals

**Goals:**

- One function that returns both the lowered expression and its type, so shape and type are
  never computed from different traversals and cannot disagree.
- A type result that can say "not determined" without being an error, since calls are legal
  in expressions but untypeable here.
- Float support that does not break `Eq`/`Hash` and therefore does not break fingerprinting.
- Promotion recorded in the IR, not left implicit for a backend to rediscover.

**Non-Goals:**

- No unification, type variables, or constraint solving. Every rule is a direct table lookup
  on operand types.
- No inference for parameters or returns — those stay declared (see proposal).
- No two-pass signature collection. That is what call inference would need, and it is out of
  scope.
- No new numeric types (no `complex`, no sized ints).

## Decisions

### `lower_expr` returns `(Expr, TyResult)`

A single traversal produces the IR node and its type together. The alternative — lower first,
then run a separate `type_of(&Expr)` pass — would traverse twice and, worse, would need its
own copy of the scope and its own error spans, giving two places where a rule could drift.

`TyResult` is `Option<Ty>`: `Some(ty)` when determined, `None` when the expression contains a
call. `None` is not an error. It propagates outward — any expression with a call anywhere
inside it is undetermined — and the *binding* decides what to do: infer if `Some`, demand an
annotation if `None`, and skip the declared-vs-actual check if `None`.

*Alternative considered:* an explicit `Ty::Unknown` variant. Rejected: it would make every
backend `match` on a state that must never reach codegen, exactly the unrepresentable-state
creep the previous design warned about. `Option` confines the uncertainty to lowering.

### Float literals store IEEE-754 bits

`Literal::Float(u64)` holding `f64::to_bits()`, with a `Literal::float(f64)` constructor and
an `as_f64()` accessor. `f64` implements neither `Eq` nor `Hash`, so storing it directly would
force hand-written impls on `Literal`, `Expr`, `Stmt`, and `Function`, or drop fingerprinting.

Bit-pattern storage is also the *right* comparison for this use. Two source literals should
contribute the same fingerprint exactly when they denote the same value, and `to_bits` gives
that. The usual objections do not apply here: NaN cannot be written as a Python literal (it
requires a call), and `0.0` vs `-0.0` are genuinely different literals whose distinction is
worth preserving in a rebuild key.

*Alternative considered:* store the source text (`"1.3"`). Rejected: `1.3` and `1.30` would
fingerprint differently despite being the same value, and every backend would have to reparse.

### Promotion is an explicit IR node

An integer operand appearing where the expression's type is float is wrapped in an explicit
widening node (`Expr::ToFloat`). The IR already refuses to make backends re-derive semantics —
that is why operators carry Python meanings — and promotion is the same kind of trap: a
backend seeing `Binary { Add, Literal(Int(1)), Name("x") }` with a float `x` would have to
redo the type analysis to know a conversion is needed.

Making it explicit means a backend can emit operands positionally and be correct.

*Alternative considered:* store the result type on each `Binary` node and let backends compare
operand types against it. Rejected: it puts the inference back on every backend, and each one
would have to agree with lowering's rules independently.

### Booleans are not numbers

Python's `bool` subclasses `int`, so `True + 1 == 2` is legal Python that this change
rejects. Accepting it would force every backend to decide how a boolean widens to its numeric
type, and would make `a + b` with two booleans mean integer addition — surprising in the
target languages compylr emits. The subset is strict by design; this is a place where matching
Python exactly costs more than it is worth. Recorded here because it *is* a divergence from
Python, not an oversight.

### `int` where `float` is declared is accepted; the reverse is not

`c: float = 1` is fine — promotion already defines widening, and rejecting it would be
gratuitous. `n: int = 1.5` is rejected, because narrowing loses information silently. This
matches the asymmetry Python programmers already expect from static typing.

### Type checking extends to returns

Once expressions have types, checking `return <expr>` against the declared return type is
nearly free and catches a whole class of real errors. It also newly rejects programs that
lower today (`def f() -> int: return "x"`), which is why the proposal marks this **BREAKING**.

Returns whose type is undetermined (containing a call) are not checked, consistent with
bindings.

## Risks / Trade-offs

- **This rejects code that currently compiles** → Intended, and the only affected programs are
  ill-typed ones. The existing fixtures that encode the old rules are updated as part of the
  change rather than left to fail mysteriously.
- **`Option<Ty>` spreading through the checker is easy to get wrong** → Every combining rule
  must propagate `None` rather than treating it as a mismatch. Tests cover a call nested
  inside arithmetic, which is the case where a naive implementation would wrongly report a
  type error instead of demanding an annotation.
- **String `+` widens the operator's meaning** → `+` now means two things depending on operand
  types. This is Python's own overload, and rejecting it would remove behavior that lowers
  today, so it is kept. Backends must switch on operand type for `+`.
- **Promotion nodes change fingerprints** → A function whose body gains a widening node
  fingerprints differently than the same source did before this change, so the first build
  after upgrading rebuilds everything. Acceptable one-time cost; the alternative is a
  fingerprint that does not reflect the IR.
- **Float equality in fingerprints is bitwise** → Deliberate, and documented above.

## Migration Plan

No data or deployment migration. Four rejected fixtures encode behavior this change reverses
(`unsupported_type_float.py`, `true_division.py`, `unannotated_local.py`,
`unannotated_local_from_expression.py`); they move to `accepted/` or are replaced by cases
that are still genuinely rejected. The rejection table in `tests/fixtures.rs` and its
fixture-count guard are updated in the same step, so a stale table fails the build rather than
silently skipping a rule.

Rollback is `git revert`; nothing outside the repo depends on the crate.

## Open Questions

- Should the float type's fingerprint contribution be normalised so that `0.0` and `-0.0`
  agree? Deferred safely: it changes only rebuild granularity, not correctness, and can be
  revisited with the rebuild machinery that consumes the key.
- Should string comparison (`<` between two strings) be permitted? It is currently allowed by
  the comparison rule as written. If lexicographic ordering turns out to differ meaningfully
  across target languages, it can be narrowed later without affecting anything else here.
