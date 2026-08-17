## Context

See proposal.md — Why. What the current code gives and withholds:

* `lower_body` walks a flat `&[PyStmt]`, and `lower_function` ends with a structural check that the
  last statement is a `Stmt::Return`. That check is where reachability has to grow.
* Names live in a flat `Scope: HashMap<String, Ty>`, and `ensure_unbound` refuses to rebind. There
  is no block scoping, because there have been no blocks.
* `len` established the pattern for a reserved builtin lowered to its own IR node; `range` follows
  it exactly.
* The backend already emits everything through helpers rather than native operators wherever Python
  and Rust disagree. Ranges are the next instance of that.

## Goals / Non-Goals

**Goals:**

* Reachability that rejects what Rust would reject, so a valid-looking program never fails with a
  rustc error instead of a diagnostic.
* Loop semantics that match Python where the two languages differ, in the same explicit way `//`
  and `%` already do.

**Non-Goals:**

* Mutation of anything but a local name. Collections stay read-only until the next change.
* `for ... else`, `match`, comprehensions, generators, exceptions, `with`.
* Narrowing a type inside a branch. `if isinstance(...)` is not in the subset and this does not
  begin it.

## Decisions

### D1. Reachability is a function over statements, not a CFG

```
returns(stmts)  = any stmt in stmts returns
returns(Return) = true
returns(If{then, Some(else)}) = returns(then) && returns(else)
returns(If{then, None})       = false
returns(While | For)          = false
returns(_)                    = false
```

A loop never counts. Its body may run zero times, and proving otherwise means evaluating the test,
which lowering does not do. `while True:` would be provable and is not worth a special case that
only one spelling benefits from.

*Alternative considered:* a real control-flow graph. Rejected — with no `goto`, no exceptions, and
no `match`, the recursive definition above is the CFG, written in the shape of the tree it walks.

### D2. Branches are scopes for names

A name bound inside a branch is not visible after it. The alternative is to admit names whose
existence depends on a runtime test, and then either reject reads of them anyway or emit Rust that
does not compile.

This is stricter than Python, which leaks a branch's names into the enclosing function and fails at
runtime if the branch did not run. Rejecting at compile time is the whole point of the subset.

`Scope` therefore becomes a stack of maps: push on entering a block, pop on leaving. Lookup walks
outward; binding writes to the innermost frame; **reassignment writes to the frame that owns the
name**, which is what makes `i = i + 1` inside a loop update the counter declared outside it rather
than shadowing it.

### D3. Reassignment keeps the type, and mutability is decided by a pre-pass

`ensure_unbound` becomes: if the name is unbound, bind it; if bound, check the value's type against
the existing one, with promotion. An annotation on a rebinding is an error — `i: int = 1` after
`i = 0` is a re-declaration, and accepting it would raise the question of whether the annotation
may differ.

Emission needs to know whether a local is ever reassigned, and that is not local information: the
`let` comes before the assignment that makes it mutable. The backend therefore scans a function's
body for assignment targets before emitting it, and marks those bindings `mut`.

*Alternative considered:* emit every binding `let mut` and allow `unused_mut`. Rejected — the
allow-list in generated code is already load-bearing, and each entry makes a real warning easier to
miss. A scan is a dozen lines.

### D4. Ranges are emitted as a loop, not as `..`

Python's `range(a, b, c)` has no Rust equivalent: `..` counts up by one, `step_by` takes an
unsigned step, and `rev()` does not compose with either for a computed step. The compat module
gains a helper producing the values, and `for i in range(...)` emits a loop driven by it:

```rust
let mut i = start;
while (step > 0 && i < stop) || (step < 0 && i > stop) { ...; i += step; }
```

A zero step is checked before the loop and returns `RuntimeError::ZeroStep`, because otherwise the
condition never changes and the program hangs — the one failure mode worse than a wrong answer,
since it produces no output at all to diagnose from.

### D5. Iteration borrows

`for x in xs` emits iteration over a borrow with each element cloned out, matching how subscripting
already behaves. Consuming the collection would mean a name could be iterated once and not read
again, which is not how Python behaves and would surface as a borrow-checker error rather than a
diagnostic.

Mapping iteration yields keys, matching Python. The order is whatever the map gives, which is the
divergence the collections change already accepted — restated in the spec because iteration is
where a user will actually meet it.

### D6. `break` and `continue` need a loop context during lowering

Lowering carries a depth counter, incremented when entering a loop body. Zero means `break` is a
diagnostic rather than something the backend discovers. Conditionals do not reset it, so a `break`
inside an `if` inside a loop is fine, which is the common case.

## Risks / Trade-offs

* **Reachability rejects a program a human reads as fine** → `if c: return 1` with no `else` is the
  case people will hit. The diagnostic should say a path produces no value, not merely "missing
  return", so the fix is obvious.
* **Block scoping is stricter than Python** → A name bound in a branch and read after it is
  rejected, where Python would sometimes work. Deliberate, and the diagnostic should say the
  binding may not have happened rather than that the name is unknown.
* **The mutability scan can disagree with emission** → If the scan misses a target, generated code
  fails to compile. It is the same walk the emitter does, so the fix is to derive both from one
  traversal rather than writing the walk twice.
* **A hanging program is the worst failure** → Hence the zero-step check. Nothing else in the
  subset can fail to terminate except an unbounded `while`, which is the user's own doing.

## Migration Plan

Nothing to migrate: the change only accepts programs that were previously rejected. No fingerprint
moves for a program that does not use control flow, so caches stay valid.
