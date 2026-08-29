## Context

See proposal.md — Why. The constraints that shape the approach are all existing invariants:

* `Expr::Call` resolves against the unit. `Expr::Len` and `Expr::Range` are separate forms
  *because* of that, and the IR's own documentation states the rule.
* `compylr-ir` may not depend on anything. CLAUDE.md: "If you find yourself wanting to add a
  dependency to `compylr-diagnostics` or `compylr-ir`, that is the signal to stop."
* Concrete spellings belong to a backend; how a construct is spelled back to the programmer belongs
  to the frontend. `Ty::python_name` lives in `compylr-frontend-python::spelling` for this reason.
* Fallible operations already declare whether the program defines their failure, via `Checked`.
* `tests/conformance.rs` checks coverage over `(form, position)` pairs, because "a statement's
  emission depends on where it is, not only on what it is."

## Goals / Non-Goals

**Goals:**
- One namespace-and-intrinsic mechanism that `print`, `logging`, and `numpy` can each extend with a
  table entry rather than a new concept.
- An intrinsic whose meaning is fixed by the registry and cannot be altered by the unit.
- Type errors against an intrinsic that are located diagnostics, not complaints about generated
  Rust.
- `math` supported end to end, proven by a driven fixture against CPython.

**Non-Goals:**
- Any module other than `math`. The mechanism is the deliverable; breadth is a later table entry.
- `from X import y`. Deliberately refused here — see Decisions.
- Go emission. Reserved through the existing not-implemented path.
- User-defined imports, relative imports, or importing another compiled source. Cross-source calls
  already work through unit assembly and are a different mechanism entirely.
- Making `**` work. It stays rejected.

## Decisions

### The registry lives in `compylr-ir`

Both the frontend (to type-check a call) and every backend (to know what it is emitting) must agree
on an operation's signature. A registry either could not reach would be duplicated, and two copies
of a signature table is exactly the "two implementations disagreeing" failure that
`returns_on_all_paths` is shared to avoid.

*Alternative considered: `compylr-registry`.* It is the natural-sounding home and is wrong. That
crate names every frontend and backend, so `compylr-ir` depending on it would let the IR reach a
Rust backend — the edge `tests/crate_boundaries.rs` exists to forbid. The name is a coincidence.

*Alternative considered: per-backend tables only, with no shared signatures.* Then the frontend
cannot type-check, and `math.sqrt("four")` becomes a rustc error about `&str` having no `sqrt`
method. That is the exact failure mode the located-diagnostic rule exists to prevent.

### Fallibility reuses `Checked` rather than adding a behavior axis

`math.sqrt(-1)` raises in Python and yields NaN in Rust and Go. That is a genuine divergence, but
it is the same *shape* as the divergences `Checked` already covers, and the IR already requires
that fallible operations declare whether the program defines their failure.

*Alternative considered: a seventh behavior axis.* Adding one means a field on `LanguageBehavior`,
a `Stance` variant, an arm in `LanguageBehavior::stance`, and a declared answer from every frontend
and backend. That cost buys a distinction nobody has asked for: no user wants Python's `sqrt`
domain rule with Rust's `log` domain rule. One mode per operation, resolved from the behavior
already in force, is the smaller and more honest model.

### A module is a namespace, not a value

`import math` introduces a name usable only left of a dot. Making a module a value would require a
module type in `Ty`, which every backend would then have to render, and there is nothing to render:
no target has a runtime value corresponding to Python's module object that means the same thing.

The diagnostic matters as much as the rule. `m = math` reports *a module is not a value* rather
than an unknown-name or unsupported-type error, because the second reads as a compiler bug when the
name is plainly right there.

### `from math import sqrt` is refused, for now

After `from math import sqrt`, a bare `sqrt(x)` in a body is textually identical to a call to a
user function named `sqrt`. Resolving that means choosing a precedence rule — and the rule has to
hold when the user's `sqrt` is defined in *another* decorated source that this validation cannot
see, which is exactly the case CLAUDE.md already carves out as undetermined.

Refusing costs users an import spelling. Accepting costs a precedence rule that would have to be
right before anyone could rely on it. The diagnostic names the supported form, so the workaround is
one line and obvious.

*Alternative considered: accept it and prefer the user's function.* Then adding a decorator
elsewhere in the project silently changes what `sqrt` means here. That is the "meaning depends on
what else was compiled" failure the intrinsic form exists to prevent, reintroduced through the
import.

### `math.pow` is not `**`

Python's `**` is integer-preserving: `2 ** 10` is `1024`, an `int`. `math.pow(2, 10)` is
`1024.0`, a float, in Python as much as in Rust. They are different operations, and mapping `**`
onto `math.pow` would silently change an integer program's result type.

### Constants are intrinsics with no arguments

`math.pi` and `math.sqrt(x)` differ only in arity. A separate constant form would be a second thing
for every backend to match on and would need its own resolution path, for no gain. `math.inf` and
`math.nan` are how a non-finite float enters a program at all — Python source cannot spell one as a
literal, which the lowering spec already records.

### Go is reserved at the mapping, not at the backend

`--backend go` works today for programs that do not use a module. It must keep working. So the
refusal attaches to the *(module, backend)* pair, and its message says the mapping is planned —
distinct from `BackendError::NotImplemented`, which would wrongly claim Go itself is unimplemented,
and from `Unknown`, which would wrongly claim Go is not a backend.

This is the same three-way-answer reasoning the backend registry already uses, applied one level
down. It is also why change 2 and change 3 inherit the behavior without restating it.

## Risks / Trade-offs

**The artifact version collides with `add-typed-ir-expressions`** (0/43 tasks, not started), which
also claims the next number → Whichever lands first takes it and the second rebases. The failure
mode if both ship the same number is a cache that deserializes into a subtly wrong unit rather than
being refused, so this is checked in review of the second change, not left to chance.

**Transcendental results may differ in the last bit between CPython and Rust** → Fixture comparison
uses a stated tolerance, and non-finite results are compared by classification. Exact equality here
would produce a suite that fails on a different machine, which reads as a compiler bug and is not
one.

**A registry entry can claim support nothing proved** → The corpus requirement fails the suite when
a listed operation has no fixture. This mirrors the defect CLAUDE.md records, where hardcoded
fixture lists drifted and hid a tuple-indexing failure; the coverage check is derived, not a list.

**`math.pow` overflowing to infinity is not a domain failure** → It is an ordinary float result and
stays unchecked. Treating it as fallible would make every power operation return a result type, for
a value IEEE-754 defines.

**Scope creep into "the whole standard library"** → The registry makes adding a module cheap, which
is the point and also the temptation. The corpus requirement is the brake: a module costs a fixture
exercising every operation plus a driver, so breadth is paid for in tests rather than in table
entries.

## Migration Plan

The artifact version advances, so every existing `.compylr` cache is refused once and rebuilt.
`_state_is_current` already compares the recorded compylr version, so a user upgrading rebuilds
automatically and the only visible effect is a slower first run. No user action, no migration code:
the only thing a pre-change artifact could mean is a program with no intrinsics, and a reader
asserting that would be more code than the one rebuild it saves — the same reasoning the version-3
reader was dropped under.

Rollback is removing the change; artifacts written by it are refused by the earlier version through
the same version check, in the same way.

## Open Questions

- Whether `math.pow` with an integer exponent should emit `powi` rather than `powf` when the
  exponent is a literal. A pure emission-quality question: same answer either way, decided by
  measurement after the mechanism exists, and it changes no spec and no task.
