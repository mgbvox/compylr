## Why

The subset cannot name anything it did not itself compile. `lower.rs:582` rejects every import —
*"imports are not supported; only function definitions may appear at top level"* — so `import math`
fails on line 1, and there is no second line worth writing. That is not a missing table of
functions; it is a missing *kind of name*.

The IR has no form for one either. `Expr::Call` carries a bare `callee: String` that is resolved
against the unit during validation, which is precisely why `len` and `range` are **distinct IR
forms** rather than calls. The IR says so itself:

> A distinct node rather than a call: a call is resolved against the unit during validation, so
> leaving `len` as one would make its meaning depend on whether someone had decorated a function of
> that name.

That reasoning generalizes exactly. If `math.sqrt` lowered to `Call { callee: "math.sqrt" }`, its
meaning would depend on whether a user had decorated a function called `math.sqrt` — and
`Unit::validate` would reject the program for calling a function that exists nowhere. Every
construct the subset shares with the outside world therefore needs a form that resolves against a
**registry** rather than against the unit.

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
  arguments, and — where the operation can fail — a `Checked` mode. It resolves against a registry,
  never against the unit, so its meaning cannot depend on what else was compiled. `Expr::Call` is
  untouched and still means "a function in this unit".

- **An intrinsic registry, keyed by module and operation.** It records each operation's signature
  (parameter types and result type) so that lowering type-checks a call to it exactly as it checks
  a call to a user function, and so an arity or type error is a located diagnostic rather than a
  complaint about generated Rust. A backend supplies the *spelling*; the registry supplies the
  *meaning*. This mirrors the existing frontend/backend split precisely: `Ty::python_name` lives in
  the frontend, `i64` lives in the backend, and neither lives in the IR.

- **`math` is the proving module**, supported end to end: `sqrt`, `floor`, `ceil`, `fabs`, `exp`,
  `log`, `log2`, `log10`, `pow`, `sin`, `cos`, `tan`, `atan2`, `hypot`, `isnan`, `isinf`,
  `isfinite`, `trunc`, and the constants `pi`, `e`, `tau`, `inf`, `nan`.

- **A domain failure declares whether the program defines it**, reusing `Checked` rather than adding
  a behavior axis. `math.sqrt(-1)` raises `ValueError` in Python and returns `NaN` in Rust and Go —
  a real divergence, but the same *shape* as the divergences `Checked` already covers, and the IR
  already requires that "fallible operations declare whether the program defines their failure".
  A new axis would mean a new field on `LanguageBehavior` and a new stance for every language; the
  existing mode says what needs saying.

- **`math.pow` is not `**`.** Exponentiation stays rejected (`rejected/exponentiation.py` is
  unchanged). `math.pow` always yields a float and is a different operation from Python's `**`,
  which is integer-preserving — folding them together would make one of the two silently wrong.

- **Rust emits; Go is reserved.** The Rust backend maps every listed operation onto `f64` inherent
  methods and `std::f64::consts`. The Go backend has no table and refuses an intrinsic through the
  existing reserved-target path, so `--backend go` on a program using `math` fails saying the
  mapping is planned — distinct from Go being an unknown backend, which it is not.

- **An unsupported module is a located diagnostic that names what is supported.** `import json`
  reports that `json` is not supported yet and lists the modules that are. Adding a module later is
  a registry entry, a backend table entry, a fixture, and a driver — no new machinery.

- **BREAKING (artifact format).** The IR gains a form, so the artifact version advances and every
  existing `.compylr` cache rebuilds once, automatically, off the recorded compylr version.

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
- `crates/compylr-ir/src/ir.rs` — the intrinsic form, the artifact version, the fingerprint.
- `crates/compylr-frontend-python/src/lower.rs` — import handling at `:582`, namespace binding,
  attribute access against a module, and the registry type-check.
- `crates/compylr-frontend-python/src/spelling.rs` — how a module and operation are quoted back.
- `crates/compylr-backend-rust/src/rust.rs` — the `math` emission table.
- `crates/compylr-backend-golang/src/` — refusal through the reserved path.
- `crates/compylr-core/src/verify.rs` — an intrinsic resolves against the registry.
- `frontends/python/fixtures/accepted/math_module.py` and its driver; new rejected fixtures.
- `README.md` (subset matrix is generated; prose half is not), `CLAUDE.md`.

**New**
- The intrinsic registry. It sits in `compylr-ir` beside the type model, because both the frontend
  and every backend must agree on an operation's signature, and a crate either of them could not
  reach would have to be duplicated.

**Unaffected**
- Every existing accepted fixture, every diagnostic already emitted, and every answer produced.
- `Expr::Call` and `Unit::validate`'s treatment of it.

**Costs**
- One rebuild per existing project, handled by the recorded compiler version.
- The artifact version collides with `add-typed-ir-expressions`, which also claims the next number.
  Whichever lands first takes it; the second rebases onto the number after. Worth knowing before
  both are in flight, because the symptom is a cache that deserializes into a subtly wrong unit.
