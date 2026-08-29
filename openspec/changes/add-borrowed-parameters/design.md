## Context

See proposal.md — Why. The governing fact is that this was built once and reverted, and that **the
whole suite passed while it was broken**. Everything below is shaped by not repeating that.

Constraints:

* `add-typed-ir-expressions` must land first. Deciding whether an argument may be borrowed requires
  knowing an expression's type, which the backend deliberately does not know today.
* A fixpoint already decides `self`'s mutability and instance parameters' mutability, and it is
  deliberately one analysis because "two analyses would be free to disagree".
* The `borrowed_instance_*` rejected fixtures already state, for instances, how far a borrow
  reaches. Those rules are user-visible and stay exactly as they are.
* `a_text_parameter_is_usable_in_every_position` exists specifically as the gate for this work.

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
- Making collection arguments free at the boundary. Explicitly impossible — see below.
- Relaxing any `borrowed_instance_*` rule.

## Decisions

### Inference, never refusal

This is the decision that separates this change from the reverted one. When a parameter cannot be
borrowed, it is **owned, silently**. No diagnostic, no annotation, nothing for the user to fix —
because there is nothing wrong with their program.

The reverted attempt borrowed unconditionally and let the failure surface as emitted Rust that did
not compile. That is the worst available failure mode: it arrives as a complaint about generated
code rather than about the user's function, which CLAUDE.md flags repeatedly as the thing to avoid.

The consequence for testing is that a bug here is **silent and slow**, not loud and broken. A
parameter wrongly owned costs performance and nothing else. That is why the corpus asserts *modes*
and not only answers: an answers-only suite would pass with every parameter owned, which is
precisely the state the change exists to leave.

### Ownership is escape, not mutation

The reverted attempt's premise — a parameter never mutated may be borrowed — is false, and the four
shapes show why. `xs.append(who)` does not mutate `who`; it *keeps* it. `d[k] = who` keeps it.
`who in xs` and `who < "m"` need a representation the borrowed form does not provide.

So the analysis asks whether the value **outlives the call**, which is a different question with a
different answer. Framing it as escape also makes the instance rules and the text rules one rule:
CLAUDE.md's "A borrow reaches further than the parameter name" is exactly this, already written
down for instances.

### One fixpoint, extended

Ownership and mutability are decided together, in the analysis that already exists. Two analyses
could disagree about a parameter that is both mutated and forwarded, and the disagreement would
surface as a borrow-checker error about generated code — CLAUDE.md names that as the likeliest bug
class for the mutability fixpoint already.

The cost is a fixpoint that decides two things and is harder to read. That is accepted, and it is
the same trade `returns_on_all_paths` makes.

### A cross-source callee forces ownership

The decorator validates one function at a time, and a call to a function in another module stays
undetermined. A borrow cannot be proven safe against a signature this compilation cannot see, so
the parameter is owned. Conservative, silent, and consistent with how the subset already treats an
unseen callee.

### Collections are honest about what does not improve

A Python list is an array of object pointers, not a contiguous block of `T`. The boundary must walk
it and convert each element whatever the Rust signature says. **Borrowing a sequence parameter does
not make the boundary free**, and claiming otherwise would repeat the previous attempt's real
error: believing a change was free when it was not.

What borrowing *does* remove for collections is the internal clone when one compiled function hands
a collection to another. That is a real saving and it is the one claimed. The spec states both
halves so a later reader does not rediscover the limit as a surprise.

Numpy is the case where the boundary genuinely becomes free, and it is a separate change precisely
because its buffer *is* contiguous and C-allocated.

### Returns stay owned

A borrowed return would need lifetimes in the IR — a relationship between a return and a
parameter — which no other part of the model has. Returns stay owned, which is also what keeps
`borrowed_instance_return` refusing what it refuses today.

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
  is owned, which is always correct; refining it is a later optimization that changes no spec.
