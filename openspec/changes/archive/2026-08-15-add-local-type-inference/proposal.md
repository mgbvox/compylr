## Why

The current subset requires an explicit annotation on every local binding, with one narrow
exception for direct aliases (`b = a`). That makes ordinary Python read as ceremony:

```python
def f(n: int) -> float:
    label: str = "x"      # the annotation says nothing the literal did not
    count: int = 3
    ratio: float = 1.3    # ...and `float` is not even a supported type yet
```

Every annotation above restates something already written down. The previous change argued
that inferring literals would make the boundary arbitrary — but that argument cuts the other
way once the rule is *"infer whenever the answer is determined"* rather than *"infer only
aliases"*. An initializer built from literals and already-typed names has exactly one type,
and computing it is a lookup plus an operator table, not a general inference engine.

`float` is also simply missing from the type model, so `c = 1.3` cannot be written at all,
annotated or not.

## What Changes

- Add **`float`** to the type model as a 64-bit binary floating-point type, accepted as a
  parameter, return, and local annotation. This is the first new type since the model was
  established.
- Add **true division (`/`)**. It was rejected only because Python's `/` always yields a
  float and no float type existed; keeping it rejected now would be the arbitrary rule.
  `7 / 2` is `3.5`, distinct from `7 // 2` being `3`.
- Add **numeric promotion**: mixing integer and floating-point operands in one arithmetic
  expression yields a float, matching Python.
- Replace the alias-only inference rule with **expression type inference for local
  bindings**. A binding may omit its annotation whenever its initializer's type is
  determined: literals, name references, negation, arithmetic, and comparisons, composed to
  any depth. Aliases remain a special case of the general rule rather than a rule of their
  own.
- Keep requiring an annotation where the type is genuinely **not** determined — today that
  means any initializer containing a call, because a call's type comes from the callee's
  signature and lowering deliberately does not resolve callees (see Impact).
- **BREAKING**: introduce type checking of expressions. Programs that lower today but are
  ill-typed will now be rejected — `def f() -> int: return "x"` and `b: str = 1` currently
  pass lowering untouched and will become errors. This is the point of the change, but it is
  a real behavior change for anything already written against the old rules.
- Keep function **parameter and return annotations mandatory**. They are the boundary the
  eventual PyO3 bindings are generated from, so they must stay explicit and local to the
  function; inferring them would require whole-program analysis.

Explicitly **not** in this change: inferring a binding from a call's return type (see the
ordering problem in Impact), inferring parameter or return types, reassignment and
mutability, and any new type beyond `float`.

## Capabilities

### New Capabilities

None — this change extends two existing capabilities.

### Modified Capabilities

- `ir`: the type model gains a floating-point type; expression forms gain a float literal and
  a true-division operator; the operator-semantics requirement gains true division's
  always-float result, which most target languages spell as integer division between two
  integers.
- `ir-lowering`: the annotation requirement narrows to "required only where the type is not
  determined"; the alias-inference requirement generalises to expression inference; `float`
  moves from rejected to supported annotation; true division moves from rejected to
  supported operator; and new requirements cover operator type rules, numeric promotion, and
  type mismatch diagnostics.

## Impact

- **Code**: `src/ir.rs` (`Ty::Float`, `Literal::Float`, `BinOp::TrueDiv`), `src/lower.rs`
  (an expression type checker replacing the alias lookup), `src/error.rs` (the existing
  `TypeMismatch` kind gains much wider use). `src/frontend.rs` is untouched.
- **A float literal cannot be stored directly.** Every IR type derives `Eq` and `Hash`, which
  `f64` does not implement, and `Function::fingerprint` depends on `Hash`. The literal must
  hold something hashable; design.md settles on the IEEE-754 bit pattern, which also gives
  the right fingerprint behavior for source literals.
- **Call inference is blocked by an ordering decision, not by effort.** The previous change
  moved call resolution to `Unit::validate` so that lowering never depends on which function
  was decorated first. Typing `b = helper(a)` during lowering would need `helper`'s signature
  and would reintroduce exactly that order-dependence. Lifting it needs a two-pass design
  (collect signatures, then lower bodies) and belongs in its own change.
- **Tests and fixtures**: `python/fixtures/rejected/unsupported_type_float.py`,
  `true_division.py`, `unannotated_local.py`, and `unannotated_local_from_expression.py` all
  describe behavior this change reverses; they move to `accepted/` or are replaced by
  narrower cases. New rejected fixtures cover ill-typed programs.
- **Downstream**: a future Rust backend must map the float type to `f64` and must not emit
  native `/` for integer operands, since Python's `/` promotes to float while Rust's does not.
