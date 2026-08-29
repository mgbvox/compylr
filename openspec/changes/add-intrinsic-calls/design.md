## Context

See proposal.md — Why. The constraints that shape the approach are all existing invariants:

* [`Expr::Call`](../../../crates/compylr-ir/src/ir.rs#L605) resolves against the unit.
  [`Expr::Len`](../../../crates/compylr-ir/src/ir.rs#L575) and
  [`Expr::Range`](../../../crates/compylr-ir/src/ir.rs#L596) are separate forms *because* of that,
  and the IR's own documentation states the rule.
* `compylr-ir` may not depend on anything. [`CLAUDE.md`](../../../CLAUDE.md): "If you find yourself
  wanting to add a dependency to `compylr-diagnostics` or `compylr-ir`, that is the signal to stop."
* Concrete spellings belong to a backend; how a construct is spelled back to the programmer belongs
  to the frontend. [`python_name`](../../../crates/compylr-frontend-python/src/spelling.rs#L16)
  lives in `compylr-frontend-python::spelling` for this reason.
* Fallible operations already declare whether the program defines their failure, via
  [`Checked`](../../../crates/compylr-ir/src/ir.rs#L268).
* [`conformance.rs`](../../../crates/compylr-host-python/tests/conformance.rs) checks coverage over
  `(form, position)` pairs, because "a statement's emission depends on where it is, not only on what
  it is."

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
- `from X import y`. Deliberately refused here — see decision 5.
- Go emission. Reserved through the existing not-implemented path.
- User-defined imports, relative imports, or importing another compiled source. Cross-source calls
  already work through unit assembly and are a different mechanism entirely.
- Making `**` work. It stays rejected.

## Decisions

### 1. An intrinsic is a new `Expr` form, not a call with a dotted name

**Decision.** Add a variant to [`Expr`](../../../crates/compylr-ir/src/ir.rs#L441) carrying the
module, the operation, its arguments, and a checking mode:

```rust
// before — a call is the only way to name an operation, and it resolves against the unit
Call { callee: String, args: Vec<Expr> },
// after — an intrinsic names a registry entry and cannot be shadowed by the unit
Intrinsic {
    module: String,
    operation: String,
    args: Vec<Expr>,
    checked: Checked,
},
```

**Why.** `Expr::Len` and `Expr::Range` are already separate forms for exactly this reason, and the
IR states it: a call resolved against the unit would make an intrinsic's meaning depend on what else
was compiled. Reusing `Call` with `callee: "math.sqrt"` would also make
[`Unit::validate`](../../../crates/compylr-ir/src/ir.rs#L1384) reject the program, since no function
of that name exists.

**Alternatives considered.** *A dotted `callee` plus a resolution rule in the verifier.* The string
becomes a namespace with no type behind it, every backend has to re-split it, and a user function
genuinely named `math.sqrt` is unspellable-but-not-impossible in a future frontend. *A `Module`
value in [`Ty`](../../../crates/compylr-ir/src/ir.rs#L103).* Covered in decision 4 — there is
nothing for a backend to render.

#### The IR, in both faces

The definition delta is above. The value, for the worked example's `root`, is the JSON
`--emit ir` writes. The surrounding envelope below is real output from the tip of this branch; the
`Intrinsic` node is `expected`, since the form does not exist yet:

```json
{
  "version": 5,
  "fingerprint": "…",
  "functions": [
    {
      "name": "root",
      "params": [{ "name": "x", "ty": "Float" }],
      "ret": "Float",
      "body": [
        {
          "Return": {
            "Intrinsic": {
              "module": "math",
              "operation": "sqrt",
              "args": [{ "Name": "x" }],
              "checked": "Reported"
            }
          }
        }
      ]
    }
  ],
  "origin": { "frontend": "python", "requires": ["IntegerOverflowReported", "FloatOrderPreserved"] }
}
```

The five questions an IR change raises:

- **Neutrality.** `module` and `operation` are strings the registry defines, not Python's. `math` is
  a namespace name that Go, C++, and TypeScript all have an analogue for, and the registry entry —
  not the frontend — is what fixes the signature. Nothing in the form names a source or target
  language, so [`crate_boundaries.rs`](../../../crates/compylr-host-python/tests/crate_boundaries.rs)
  is unaffected.
- **Mode or form?** A distinct **form**. An intrinsic differs from a call in *shape* — it resolves
  against a registry rather than the unit — not in the semantics of one operation. This is the same
  call `Expr::Range` made against `Expr::Call`. Getting it backwards, and making it a mode on
  `Call`, is the recurring mistake this section exists to catch.
- **Format version.** [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) moves from 4 to
  5. Every cached build is invalidated once; see the Migration Plan.
- **Fingerprint.** [`Unit::fingerprint`](../../../crates/compylr-ir/src/ir.rs#L1299) must cover
  `module`, `operation`, and `checked`. All three change the program's meaning, so all three are on
  the covered side of the pre-pass line — two units differing only in `checked` must fingerprint
  differently, or turning a domain check on would reuse the wrong cached build.
- **Coverage.** A new `Expr` form trips
  [`demo_coverage.rs`](../../../crates/compylr-host-python/tests/demo_coverage.rs), which reads the
  IR's enum definitions and fails when a form appears that the demo's tables do not list. This
  change pays that with an algorithm in the demo that uses `math`, scheduled in tasks — not by
  narrowing the claim in the demo README.

### 2. The registry lives in `compylr-ir`

**Decision.** A signature table beside the type model, keyed by module and operation:

```rust
pub struct IntrinsicSignature {
    pub params: &'static [Ty],
    pub ret: Ty,
    pub fallible: bool,
}
pub fn lookup(module: &str, operation: &str) -> Option<&'static IntrinsicSignature>;
```

**Why.** Both the frontend (to type-check a call) and every backend (to know what it is emitting)
must agree on an operation's signature. A registry either could not reach would be duplicated, and
two copies of a signature table is exactly the "two implementations disagreeing" failure that
[`returns_on_all_paths`](../../../crates/compylr-ir/src/ir.rs#L912) is shared to avoid.

**Alternatives considered.** *`compylr-registry`.* It is the natural-sounding home and is wrong.
That crate names every frontend and backend, so `compylr-ir` depending on it would let the IR reach
a Rust backend — the edge `crate_boundaries.rs` exists to forbid. The name is a coincidence.
*Per-backend tables only, with no shared signatures.* Then the frontend cannot type-check, and
`math.sqrt("four")` becomes a rustc error about `&str` having no `sqrt` method — the exact failure
mode the located-diagnostic rule exists to prevent.

### 3. Fallibility reuses `Checked` rather than adding a behavior axis

**Decision.** The mode already on every fallible operation, resolved from the behavior in force:

```rust
// the same enum that already answers for overflow, division, and indexing
Intrinsic { /* ... */ checked: Checked },
```

**Why.** `math.sqrt(-1)` raises in Python and yields NaN in Rust and Go. That is a genuine
divergence, but it is the same *shape* as the divergences `Checked` already covers, and the IR
already requires that fallible operations declare whether the program defines their failure.

**Alternatives considered.** *A seventh behavior axis.* Adding one means a field on
[`LanguageBehavior`](../../../crates/compylr-ir/src/behavior.rs#L179), a
[`Stance`](../../../crates/compylr-ir/src/behavior.rs#L141) variant, an arm in `stance`, and a
declared answer from every frontend and backend. That cost buys a distinction nobody has asked for:
no user wants Python's `sqrt` domain rule with Rust's `log` domain rule. One mode per operation,
resolved from the behavior already in force, is the smaller and more honest model.

### 4. A module is a namespace, not a value

**Decision.** An imported name is usable only left of a dot; every other position is a diagnostic:

```python
import math

x = math.sqrt(2.0)    # the only legal shape
m = math              # error: a module is not a value
```

**Why.** Making a module a value would require a module type in `Ty`, which every backend would then
have to render — and there is nothing to render: no target has a runtime value corresponding to
Python's module object that means the same thing. The diagnostic matters as much as the rule.
`m = math` reports *a module is not a value* rather than an unknown-name or unsupported-type error,
because the second reads as a compiler bug when the name is plainly right there.

**Alternatives considered.** *A first-class module value.* Postponed indefinitely; nothing in the
subset can consume one.

### 5. `from math import sqrt` is refused, for now

**Decision.** The import form is rejected with a diagnostic naming the supported spelling:

```python
from math import sqrt    # error: only `import math` and `import math as m` are supported
```

**Why.** After `from math import sqrt`, a bare `sqrt(x)` in a body is textually identical to a call
to a user function named `sqrt`. Resolving that means choosing a precedence rule — and the rule has
to hold when the user's `sqrt` is defined in *another* decorated source that this validation cannot
see, which is exactly the case `CLAUDE.md` already carves out as undetermined. Refusing costs users
an import spelling. Accepting costs a precedence rule that would have to be right before anyone
could rely on it.

**Alternatives considered.** *Accept it and prefer the user's function.* Then adding a decorator
elsewhere in the project silently changes what `sqrt` means here — the "meaning depends on what else
was compiled" failure the intrinsic form exists to prevent, reintroduced through the import.

### 6. `math.pow` is not `**`

**Decision.** No lowering path connects them; `**` stays rejected.

```python
2 ** 10           # error: unchanged — rejected/exponentiation.py
math.pow(2, 10)   # 1024.0, a float, in Python and in Rust alike
```

**Why.** Python's `**` is integer-preserving: `2 ** 10` is `1024`, an `int`. `math.pow(2, 10)` is
`1024.0`, a float, in Python as much as in Rust. They are different operations, and mapping `**`
onto `math.pow` would silently change an integer program's result type.

**Alternatives considered.** *Lower `**` to `math.pow` and cast back for integer operands.* The cast
is wrong past 2^53, which is where an integer program most needs it to be right.

### 7. Constants are intrinsics with no arguments

**Decision.** `math.pi` is the same form with an empty argument list:

```rust
Intrinsic { module: "math", operation: "pi", args: vec![], checked: Checked::Unchecked }
```

**Why.** `math.pi` and `math.sqrt(x)` differ only in arity. A separate constant form would be a
second thing for every backend to match on and would need its own resolution path, for no gain.
`math.inf` and `math.nan` are how a non-finite float enters a program at all — Python source cannot
spell one as a literal, which the lowering spec already records.

**Alternatives considered.** *A `Literal::Float` folded at lowering.* It would work for `pi` and `e`
and not for `inf` and `nan`, and a backend that spells `std::f64::consts::PI` reads better than one
that spells sixteen digits.

### 8. Go is reserved at the (module, backend) pair, not at the backend

**Decision.** The refusal attaches to the pair, so `--backend go` keeps working for programs that
use no module. This is a sequencing-and-naming decision with no type behind it: it reuses the
existing three-way answer one level down.

**Why.** `--backend go` works today for programs that do not use a module. It must keep working. The
message says the *mapping* is planned — distinct from `BackendError::NotImplemented`, which would
wrongly claim Go itself is unimplemented, and from `Unknown`, which would wrongly claim Go is not a
backend. Change 2 and change 3 inherit the behavior without restating it.

**Alternatives considered.** *Reserve the Go backend outright while intrinsics exist.* It would
regress every Go program that compiles today, to describe a gap in one table.

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
a listed operation has no fixture. This mirrors the defect `CLAUDE.md` records, where hardcoded
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
