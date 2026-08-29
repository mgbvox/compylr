## Why

The subset cannot name anything it did not itself compile. [`lower.rs`](../../../crates/compylr-frontend-python/src/lower.rs#L585)
rejects every import — *"imports are not supported; only function definitions may appear at top
level"* — so `import math` fails on line 1, and there is no second line worth writing. That is not
a missing table of functions; it is a missing *kind of name*.

The IR has no form for one either. [`Expr::Call`](../../../crates/compylr-ir/src/ir.rs#L605) carries
a bare `callee: String` that is resolved against the unit during validation, which is precisely why
[`Expr::Len`](../../../crates/compylr-ir/src/ir.rs#L575) and [`Expr::Range`](../../../crates/compylr-ir/src/ir.rs#L596)
are **distinct IR forms** rather than calls. The IR says so itself:

> A distinct form rather than a call, for the reason [`Expr::Len`] is: a call is resolved against
> the unit, so leaving it as one would make its meaning depend on what else was compiled.

That reasoning generalizes exactly. If `math.sqrt` lowered to `Call { callee: "math.sqrt" }`, its
meaning would depend on whether a user had decorated a function called `math.sqrt` — and
[`Unit::validate`](../../../crates/compylr-ir/src/ir.rs#L1384) would reject the program for calling
a function that exists nowhere. Every construct the subset shares with the outside world therefore
needs a form that resolves against a **registry** rather than against the unit.

Today the cost is paid by users. A function needing a square root either reimplements Newton's
method inside the subset or stays interpreted, and the second is the common answer — which means
the compiler's reach is bounded by what a programmer is willing to rewrite rather than by what it
can translate.

**Why this change first.** `print`, `logging`, and `numpy` are each blocked on the same two missing
pieces, and each would otherwise invent its own. Building the namespace model and the intrinsic form
once, with `math` as the proving module, is what keeps three later changes from being three
incompatible answers to one question.

## What Changes

- **Imports enter the subset, and bind a namespace rather than a value.** `import math` and
  `import math as m` make a *module name* available; they do not introduce a value that can be
  bound, passed, returned, or stored. A module name used anywhere except as the receiver of an
  attribute access is a located diagnostic. `from math import sqrt` is **rejected** in this change,
  with a diagnostic naming the supported form — a bare `sqrt` in the body is indistinguishable from
  a user's own function at the point of use, and resolving that ambiguity is a naming decision this
  change does not need to make.

- **A new IR form: a namespaced intrinsic operation.** It carries a module, an operation, its
  arguments, and — where the operation can fail — a [`Checked`](../../../crates/compylr-ir/src/ir.rs#L268)
  mode. It resolves against a registry, never against the unit, so its meaning cannot depend on what
  else was compiled. `Expr::Call` is untouched and still means "a function in this unit".

- **An intrinsic registry, keyed by module and operation.** It records each operation's signature
  (parameter types and result type) so that lowering type-checks a call to it exactly as it checks
  a call to a user function, and so an arity or type error is a located diagnostic rather than a
  complaint about generated Rust. A backend supplies the *spelling*; the registry supplies the
  *meaning*. This mirrors the existing frontend/backend split precisely:
  [`python_name`](../../../crates/compylr-frontend-python/src/spelling.rs#L16) lives in the
  frontend, `i64` lives in the backend, and neither lives in the IR.

- **`math` is the proving module**, supported end to end: `sqrt`, `floor`, `ceil`, `fabs`, `exp`,
  `log`, `log2`, `log10`, `pow`, `sin`, `cos`, `tan`, `atan2`, `hypot`, `isnan`, `isinf`,
  `isfinite`, `trunc`, and the constants `pi`, `e`, `tau`, `inf`, `nan`.

- **A domain failure declares whether the program defines it**, reusing `Checked` rather than adding
  a behavior axis. `math.sqrt(-1)` raises `ValueError` in Python and returns `NaN` in Rust and Go —
  a real divergence, but the same *shape* as the divergences `Checked` already covers, and the IR
  already requires that fallible operations declare whether the program defines their failure. A new
  axis would mean a new field on [`LanguageBehavior`](../../../crates/compylr-ir/src/behavior.rs#L179)
  and a new [`Stance`](../../../crates/compylr-ir/src/behavior.rs#L141) for every language; the
  existing mode says what needs saying.

- **`math.pow` is not `**`.** Exponentiation stays rejected
  ([`exponentiation.py`](../../../frontends/python/fixtures/rejected/exponentiation.py) is
  unchanged). `math.pow` always yields a float and is a different operation from Python's `**`,
  which is integer-preserving — folding them together would make one of the two silently wrong.

- **Rust emits; Go is reserved.** [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs) maps
  every listed operation onto `f64` inherent methods and `std::f64::consts`.
  [`golang.rs`](../../../crates/compylr-backend-golang/src/golang.rs) has no table and refuses an
  intrinsic through the existing reserved-target path, so `--backend go` on a program using `math`
  fails saying the mapping is planned — distinct from Go being an unknown backend, which it is not.

- **An unsupported module is a located diagnostic that names what is supported.** `import json`
  reports that `json` is not supported yet and lists the modules that are. Adding a module later is
  a registry entry, a backend table entry, a fixture, and a driver — no new machinery.

- **BREAKING (artifact format).** The IR gains a form, so
  [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) advances from 4 to 5 and every
  existing `.compylr` cache rebuilds once, automatically, off the recorded compylr version.

## Worked Example

The program below is written as an accepted fixture, so the tasks phase moves it into
[`frontends/python/fixtures/accepted/`](../../../frontends/python/fixtures/accepted/) rather than
inventing one. It reaches every part of this change: an import, a fallible operation, a pure one, a
constant, and an intrinsic whose result type is not its argument's.

**Input** — `math_module.py`:

```python
import math


def root(x: float) -> float:
    return math.sqrt(x)


def circle_cells(radius: float) -> int:
    return math.floor(math.pi * radius * radius)
```

**Today** — the first line is as far as it gets. Verified against the CLI at the tip of this branch:

```text
$ cargo run -p compylr-cli -- math_module.py
error: 1:1: imports are not supported; only function definitions may appear at top level
```

Deleting the import does not help: `math.sqrt` is then an attribute access on an unknown name, and
there is no form in the IR that could hold the result.

**After** — the module lowers, and each intrinsic reaches the generated Rust as a native operation.
The `Result` wrapper is already how every fallible operation is emitted today, so a reported domain
failure needs no new shape at the boundary:

```rust
// expected — the mechanism does not exist yet
pub fn root(x: f64) -> Result<f64, RuntimeError> {
    if x < 0.0 { return Err(RuntimeError::domain("math.sqrt")); }
    Ok(x.sqrt())
}
pub fn circle_cells(radius: f64) -> Result<i64, RuntimeError> {
    Ok((std::f64::consts::PI * radius * radius).floor() as i64)
}
```

For contrast, this is what the same arithmetic emits today, verified by
`cargo run -p compylr-cli -- --emit rust` on the tip of this branch:

```rust
pub fn hypotenuse(a: f64, b: f64) -> Result<f64, RuntimeError> {
    Ok(PyAdd::py_add(
        &(PyNum::py_mul(&(a), &(a))?),
        &(PyNum::py_mul(&(b), &(b))?),
    )?)
}
```

**At the boundary** — `math.pi` is a constant, `math.floor` returns an `int`, and a domain failure
is reported rather than silently becoming `NaN`:

```pycon
>>> import math_module
>>> math_module.root(4.0)
2.0
>>> math_module.circle_cells(2.0)
12
>>> math_module.root(-1.0)
ValueError: math domain error
```

These answers are `# expected` in the sense the schema means: the mechanism does not exist yet, so
they are what CPython answers for the same source — which is exactly what the fixture's driver
asserts against, since a driver carries no expected values of its own. `circle_cells(2.0)` is `12`
because `math.pi * 4.0` is `12.566...`, and `math.sqrt(-1.0)` is where the `Checked` mode becomes
observable: `Reported` raises as above, `Unchecked` would answer `nan`.

## Capabilities

### New Capabilities
- `intrinsics`: what an intrinsic operation is, how a module namespace resolves, how the registry
  carries signatures and fallibility, and what an unsupported module or operation reports.

### Modified Capabilities
- `ir`: a namespaced intrinsic expression form, distinct from a call; the artifact version advances
  and the fingerprint covers the added information.
- `ir-lowering`: imports are accepted and bind a namespace; a module name is not a value; an
  intrinsic call is type-checked against the registry rather than against the unit.
- `rust-backend`: intrinsic operations emit target-native operations, and a checked domain failure
  emits a recoverable error rather than a panic.
- `fixture-corpus`: `math` is exercised by an accepted fixture with a driver, and the rejected
  corpus grows the shapes this change refuses.

## Impact

**Modified**
- [`ir.rs`](../../../crates/compylr-ir/src/ir.rs) — the intrinsic form, the artifact version, the
  fingerprint.
- [`lower.rs`](../../../crates/compylr-frontend-python/src/lower.rs#L585) — import handling,
  namespace binding, attribute access against a module, and the registry type-check.
- [`spelling.rs`](../../../crates/compylr-frontend-python/src/spelling.rs) — how a module and
  operation are quoted back.
- [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs) — the `math` emission table.
- [`golang.rs`](../../../crates/compylr-backend-golang/src/golang.rs) — refusal through the reserved
  path.
- [`verify.rs`](../../../crates/compylr-core/src/verify.rs) — an intrinsic resolves against the
  registry.
- [`accepted/`](../../../frontends/python/fixtures/accepted/) gains `math_module.py` and
  [`drivers/`](../../../frontends/python/fixtures/drivers/) its driver; new rejected fixtures.
- [`README.md`](../../../README.md) (the subset matrix is generated; the prose half is not) and
  [`CLAUDE.md`](../../../CLAUDE.md).

**New**
- The intrinsic registry. It sits in `compylr-ir` beside the type model, because both the frontend
  and every backend must agree on an operation's signature, and a crate either of them could not
  reach would have to be duplicated. See design.md — decision 1 for why not `compylr-registry`.

**Unaffected**
- Every existing accepted fixture, every diagnostic already emitted, and every answer produced.
- `Expr::Call` and `Unit::validate`'s treatment of it.

**Costs**
- One rebuild per existing project, handled by the recorded compiler version.
- The artifact version collides with `add-typed-ir-expressions`, which also claims the next number.
  Whichever lands first takes it; the second rebases onto the number after. Worth knowing before
  both are in flight, because the symptom is a cache that deserializes into a subtly wrong unit.
