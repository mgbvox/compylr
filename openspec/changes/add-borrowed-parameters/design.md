## Context

See proposal.md — Why. The governing fact is that this was built once and reverted, and that **the
whole suite passed while it was broken**. Everything below is shaped by not repeating that.

Constraints:

* `add-typed-ir-expressions` must land first. Deciding whether an argument may be borrowed requires
  knowing an expression's type, which the backend deliberately does not know today.
* A fixpoint already decides `self`'s mutability and instance parameters' mutability, and it is
  deliberately one analysis because "two analyses would be free to disagree".
* The [`borrowed_instance_*`](../../../frontends/python/fixtures/rejected/) rejected fixtures
  already state, for instances, how far a borrow reaches. Those rules are user-visible and stay
  exactly as they are.
* [`a_text_parameter_is_usable_in_every_position`](../../../crates/compylr-host-python/tests/execution.rs#L2728)
  exists specifically as the gate for this work.

## Goals / Non-Goals

**Goals:**
- Borrowing decided per parameter from the body, defaulting to owned.
- The four documented shapes provably owned, each with its own case.
- A measurable fall in text-argument conversion cost.
- No change to which programs are accepted, and no new diagnostic.

**Non-Goals:**
- User-visible ownership syntax. There is none and there will not be one here.
- Borrowing a *return* value. Returns stay owned; a borrowed return needs lifetimes in the IR,
  which is a much larger claim.
- Making collection arguments free at the boundary. Explicitly impossible — see decision 5.
- Relaxing any `borrowed_instance_*` rule.

## Decisions

### 1. A parameter carries a passing mode, defaulting to owned

**Decision.** Add a mode to [`Param`](../../../crates/compylr-ir/src/ir.rs#L103), with owned as the
`Default`:

```rust
// before — every parameter crosses by value
pub struct Param { pub name: String, pub ty: Ty }
// after — how it crosses is a decided property of the parameter
pub struct Param { pub name: String, pub ty: Ty, pub passing: Passing }

#[derive(Default)]
pub enum Passing { #[default] Owned, Shared, Mutable }
```

**Why.** Making owned the `Default` is the safety property, not a formality: every path that
constructs a `Param` without deciding gets the mode that is always correct. The reverted attempt had
borrowing as the default, which is why its failure mode was emitted Rust that did not compile.

**Alternatives considered.** *A separate side table keyed by parameter.* It can go stale against the
`Param` it describes, and nothing in the type system would say so.

#### The IR, in both faces

The definition delta is above. The value, for the worked example, as the JSON `--emit ir` writes.
The envelope and the `params` shape are real output from the tip of this branch; the `passing` field
is `expected`:

```json
{
  "version": 5,
  "functions": [
    {
      "name": "is_long",
      "params": [{ "name": "word", "ty": "Text", "passing": "Shared" }],
      "ret": "Bool"
    },
    {
      "name": "roster",
      "params": [{ "name": "who", "ty": "Text", "passing": "Owned" }],
      "ret": "Int"
    }
  ],
  "origin": { "frontend": "python", "requires": ["IntegerOverflowReported", "FloatOrderPreserved"] }
}
```

The five questions:

- **Neutrality.** `Passing` names an ownership relationship, not Rust's. Go's pointer-versus-value
  and C++'s reference-versus-value are the same distinction; `&` appears only in
  [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs). A backend with no borrowing
  concept emits every mode as a value and is still correct, because owned is a *weakening* of
  borrowed rather than a different meaning.
- **Mode or form?** A **mode**. How a parameter crosses is a property of the same operation — a
  call — not a differently shaped one. Making it a form would mean two `Param` types and two arms
  everywhere one exists today.
- **Format version.** [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) advances.
  Ordering with `add-typed-ir-expressions` is fixed: that change lands first and takes the earlier
  number.
- **Fingerprint.** [`Unit::fingerprint`](../../../crates/compylr-ir/src/ir.rs#L1299) must cover
  `passing`. It is derived from the body rather than written by the user, so it cannot change
  without something else in the fingerprint changing too — but covering it is what keeps the
  artifact self-checking against its own contents, and it costs nothing.
- **Coverage.** No new `Expr` or `Stmt` form, so
  [`demo_coverage.rs`](../../../crates/compylr-host-python/tests/demo_coverage.rs) is not tripped
  and the demo owes no new algorithm. The demo does owe a *measurement*, which is decision 6.

### 2. Inference, never refusal

**Decision.** A parameter that cannot be borrowed is owned, silently:

```python
def roster(who: str) -> int:    # `who` is appended, so it is Owned
    names = ["ada"]             # no diagnostic: nothing is wrong with this program
    names.append(who)
    return len(names)
```

**Why.** This is the decision that separates this change from the reverted one. The reverted attempt
borrowed unconditionally and let the failure surface as emitted Rust that did not compile — the
worst available failure mode, because it arrives as a complaint about generated code rather than
about the user's function.

The consequence for testing is that a bug here is **silent and slow**, not loud and broken. A
parameter wrongly owned costs performance and nothing else. That is why the corpus asserts *modes*
and not only answers: an answers-only suite would pass with every parameter owned, which is
precisely the state the change exists to leave.

**Alternatives considered.** *Diagnose a parameter that could have been borrowed but was not.* It
would report on correct programs, and the report would be about compiler internals.

### 3. Ownership is escape, not mutation

**Decision.** The analysis asks whether the value outlives the call:

```python
xs.append(who)   # not a mutation of `who` — but `who` is KEPT, so: Owned
d[k] = who       # kept: Owned
who in xs        # needs the owned representation: Owned
who < "m"        # needs the owned representation: Owned
len(who)         # read and discarded: Shared
```

**Why.** The reverted attempt's premise — a parameter never mutated may be borrowed — is false, and
the four shapes show why. Framing it as escape also makes the instance rules and the text rules one
rule: `CLAUDE.md`'s "A borrow reaches further than the parameter name" is exactly this, already
written down for instances.

**Alternatives considered.** *Test for mutation and special-case the four shapes.* That is the
reverted change plus four patches, and the fifth shape nobody thought of behaves the way the first
four did.

### 4. One fixpoint, extended

**Decision.** Ownership and mutability are decided together, in the analysis that already exists —
a sequencing decision about where code goes, so it carries no new type.

**Why.** Two analyses could disagree about a parameter that is both mutated and forwarded, and the
disagreement would surface as a borrow-checker error about generated code — `CLAUDE.md` names that
as the likeliest bug class for the mutability fixpoint already. The cost is a fixpoint that decides
two things and is harder to read. That is accepted, and it is the same trade
[`returns_on_all_paths`](../../../crates/compylr-ir/src/ir.rs#L912) makes.

**Alternatives considered.** *A second pass after the mutability fixpoint.* Cheaper to read and free
to disagree, which is the failure the shared fixpoint exists to prevent.

### 5. Collections are honest about what does not improve

**Decision.** The saving claimed for a collection parameter is the internal clone, not the boundary:

```text
boundary:  Python list -> Vec<T>   still element by element, borrowed or not
internal:  f(xs) -> g(xs)          the clone between them goes away
```

**Why.** A Python list is an array of object pointers, not a contiguous block of `T`. The boundary
must walk it and convert each element whatever the Rust signature says. Claiming otherwise would
repeat the previous attempt's real error: believing a change was free when it was not. Numpy is the
case where the boundary genuinely becomes free, and it is a separate change precisely because its
buffer *is* contiguous and C-allocated.

**Alternatives considered.** *Claim the boundary saving and measure later.* That is how the reverted
attempt got as far as it did.

### 6. A cross-source callee forces ownership

**Decision.** An unseen callee's signature cannot be proven against, so the parameter is owned.

**Why.** The decorator validates one function at a time, and a call to a function in another module
stays undetermined. Conservative, silent, and consistent with how the subset already treats an
unseen callee.

**Alternatives considered.** *Assume the callee borrows and fix it at link time.* There is no link
time; the crate is generated once from what this compilation can see.

### 7. Returns stay owned

**Decision.** No mode on a return; a return is always owned.

**Why.** A borrowed return would need lifetimes in the IR — a relationship between a return and a
parameter — which no other part of the model has. Returns stay owned, which is also what keeps
[`borrowed_instance_return.py`](../../../frontends/python/fixtures/rejected/borrowed_instance_return.py)
refusing what it refuses today.

**Alternatives considered.** *Lifetimes in the IR.* A much larger claim, and one that would have to
be neutral across targets with no borrow checker.

## Risks / Trade-offs

**A wrong decision is silent** → The defining risk, and the reason for the corpus requirement that
modes are asserted directly. An answers-only suite cannot distinguish this change working from this
change doing nothing.

**Regression to the reverted state** → `a_text_parameter_is_usable_in_every_position` runs
unchanged as the gate, and each of the four shapes gets a case asserting an owned mode. The gate
test is not modified by this change under any circumstances; if it needs modifying, the design is
wrong.

**The fixpoint may not terminate** → It must, over a finite lattice of three modes with monotone
transitions: a parameter only ever moves toward owned, never back. Mutual recursion is covered by
an explicit case, since the subset supports it.

**Interaction with the mutability fixpoint could change a receiver's mutability** → Asserted not
to: the corpus checks that every existing receiver mutability conclusion is unchanged.

**The artifact version collides with the other in-flight changes** → Same coordination as the
others; ordering is `add-typed-ir-expressions`, then this, so the numbers are taken in that order.

**Measured improvement may be smaller than hoped** → Possible, and the spec requires measurement
rather than assertion. If the text saving does not materialise, that is a finding worth having
before numpy depends on the mechanism, which is an argument for this ordering rather than against
the change.

## Migration Plan

The artifact version advances; caches are refused once and rebuilt automatically off the recorded
compylr version. No source changes for any user, no diagnostics, no behavior change — the only
observable effects are a slower first run and faster text arguments after it.

Rollback is removing the change. Because ownership is the default and a borrow is the optimization,
reverting returns every parameter to owned and every program keeps working — which is the property
the reverted attempt did not have, since it had made borrowing the default.

## Open Questions

- Whether a borrowed parameter forwarded to a *method* on a borrowed instance can stay borrowed in
  every case, or only where the receiver's mutability is already settled. The conservative answer
  is owned, which is always correct; refining it is a later optimization that changes no spec, no
  task, and no answer.
