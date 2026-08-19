## Why

The compiler cannot express a base case. Every program it accepts runs straight through:

```python
@c.compyle
def nth_prime(n: int) -> int:
    if n == 1:          # 2:5: conditional statements are not supported
        return 2
    ...
```

That rules out essentially every algorithm worth compiling. Recursion already works — a function may
call itself and be typed from its own signature — but recursion without branching cannot terminate,
so the feature is unreachable in practice. Iteration is rejected outright, and so is `i = i + 1`,
which every loop counter needs.

This is the change that makes compylr able to compile a real function rather than an expression.

## What Changes

- Add **`if` / `elif` / `else`**.
- **BREAKING (internal)**: the "body must end in a return" check becomes real reachability
  analysis. It is currently structural — the last statement must be a `return` — which is correct
  only while nothing branches. With `if`, a function returns when *every* path returns.
- Add **`while`**, with **`break`** and **`continue`**. They are cheap once the loop exists and
  awkward to retrofit, because both affect whether a loop's body can be assumed to run.
- Add **`for x in ...`** over `range(...)` and over the supported collections. `range` becomes a
  second reserved builtin, on the same terms as `len`.
- Add **reassignment of locals**. `i = i + 1` is currently rejected. Rebinding **keeps the
  original type**: a name's type is fixed where it is first bound, and assigning a different type
  to it is an error rather than a re-declaration.
- **Iterating a `dict` or `set` yields no guaranteed order**, and the order varies between runs.
  The collections change accepted that for *returned* mappings; iteration makes it reachable from
  inside compiled code, so it is restated where a user will meet it.

Explicitly **not** in this change: mutation of any kind, `for ... else`, `match`, comprehensions,
generators, exceptions, and `with`.

## Capabilities

### New Capabilities

None — this widens three existing capabilities.

### Modified Capabilities

- `ir`: statement forms gain conditionals, loops, and loop control; expression forms gain a range.
- `ir-lowering`: the constructs move from rejected to supported; the missing-return rule becomes
  reachability analysis; reassignment gains a type rule; `range` is reserved.
- `rust-backend`: emission of branches, loops, loop control, and ranges, including a range whose
  step is negative — which Rust's `..` cannot express.

## Impact

- **Reachability is the subtle part.** `if cond: return 1` does not return on every path; adding
  `else: return 2` does. A `while` body cannot be assumed to run at all, so a function whose only
  `return` is inside a loop does not return. Getting this wrong means either rejecting valid
  programs or emitting Rust that fails to compile, and the second is worse because the diagnostic
  will be about Rust.
- **Reassignment forces a mutability decision in emission.** A rebound local must be `let mut`, and
  a local that is never rebound must not be, or generated code warns. Either the backend scans the
  body first or it allows the lint; design settles which.
- **`range` with a negative step has no Rust equivalent.** `for i in a..b` counts up only.
  `(a..b).rev()` and `step_by` do not compose into Python's three-argument `range`, so this needs a
  helper in the emitted compat module.
- **Code**: `src/ir.rs` (new `Stmt` variants, a range expression), `src/lower.rs` (the largest
  share: reachability, scope rules for rebinding, loop context for `break`/`continue`),
  `src/backend/rust.rs` and `runtime.rs`.
- **Ordering**: first of five. Everything else in this arc depends on it — the demo's three
  functions each need branching, and classes need method bodies that can branch.
