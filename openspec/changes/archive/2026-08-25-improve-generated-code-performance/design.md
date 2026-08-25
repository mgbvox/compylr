## Context

See `proposal.md` — Why, for the measurements, and the delta specs for what is being contracted.

Four facts about the current code shape everything below.

**Everything here is invisible to the test suite.** Each defect produces correct answers, which is
why none was caught. `demo/.compylr/crate/src/generated.rs` is committed precisely so the emitted
Rust can be read, and reading it is how all seven were found. The instrument for this change is the
benchmark, not `cargo test`.

**The benchmark cannot currently resolve most of what this change does.** It reports best-of-five
batches with no spread, and `sorting.merge_sort` returns anywhere from 160us to 277us from
byte-identical builds. Several improvements here are worth 10–20%, which is inside that. The
harness is a prerequisite, not a deliverable to schedule last.

**The rebuild key is the IR fingerprint, and nothing here touches the IR.** So no measurement in
this change is valid without `rm -rf .compylr demo/.compylr` first. CLAUDE.md records that this has
already cost real time once.

**The backend deliberately cannot see expression types.** `rust.rs` says so at the top. Every rule
below either works from information the backend already has (the expected type, whether a name is
assigned) or dispatches through a trait and lets Rust choose — none introduces a second type
checker.

## Goals / Non-Goals

**Goals:**

- Every workload in the demo is at least as fast compiled as interpreted, or the reason it is not
  is written down and is the boundary.
- Every claim is a measured before-and-after against a stated noise floor, and a rejected candidate
  keeps its measurement so it is not re-proposed on intuition.
- No answer changes anywhere. The change is invisible except in timings.
- The cheap items are separable and land first, so a 4.1x fix is not gated on a design question.

**Non-Goals:**

- No user-facing setting. If someone must ask for it, it changes meaning and belongs in
  `add-behavior-profiles`.
- No `unchecked-arithmetic`. Removing the overflow check changes what a program means.
- No change to the accepted subset and no new diagnostics.
- No rewriting of the user's algorithm. `text.joined` stays quadratic in the number of words; this
  change removes the constant factor of two on top of it, which is ours.
- Not a general optimization framework. `ir-optimization` already owns IR-level passes; these are
  emission and runtime rules, deliberately not passes, because they depend on target facts the IR
  must not carry.

## Decisions

### D1. The harness comes first, and it is a prerequisite rather than a phase

Ordering everything else after the harness is not tidiness. Item 1 is worth 10–25%, and the harness
cannot currently distinguish 25% from nothing on several workloads. Measuring before fixing the
instrument produces numbers that feel like evidence and are not.

`add-behavior-profiles` has the same prerequisite for the same reason, and its task list now says
so. Whichever change lands the harness fix satisfies it for both; the other verifies and moves on.

*Alternative considered: measure with an external tool and leave the demo alone.* Rejected — the
demo is what the project points users at, and a claim measured somewhere users cannot reproduce is
not much of a claim.

### D2. Semantics-preserving is a structural property, not a promise

Every rule in the delta specs is stated with the condition that makes it safe, and each condition
is checkable from what the backend already knows:

- in-place accumulation requires the name to be the *left* operand, so nothing reads a value that
  has already changed;
- borrowed iteration requires the body not to assign the loop variable, which is already computed
  to decide whether the binding is mutable;
- the tail-position move requires the return to be the last statement at the top level, which
  cannot be inside a loop and therefore cannot fight a borrow.

That last one is the pattern for all three: prefer a condition that makes the transformation safe
*by construction* over one that requires an analysis to be right.

*Alternative considered: move a returned local wherever it is returned, using a borrow analysis.*
Rejected for now — it covers strictly more sites, and all 25 sites in the demo are in tail
position, so the analysis buys nothing measurable and adds a way to be wrong.

### D3. The hasher is a target option, never a behavior axis

A hasher changes no answer. Iteration order over a mapping is already unguaranteed and already
varies between runs — CLAUDE.md says never to assert on it — so changing the hasher cannot break a
program that was not already broken.

That is exactly what makes it *not* a behavior axis. `add-behavior-profiles` is built on axes where
two languages disagree about meaning; a hasher is a place where nothing disagrees about meaning and
one choice is faster. Putting it on that dial would make the dial mean two different things.

The related defect is worth fixing regardless of which hasher wins: the runtime's implementations
are written against the two-parameter container form, which pins the standard hasher across ten
impls. Until they are generic, the decision cannot be *expressed*, let alone made.

### D4. Container literals must stop depending on the default hasher

The direct consequence of D3, and the one that turns a runtime change into an emitter change: the
convenient array-to-container conversion exists only for the standard hasher. Once generated
containers use another, dict and set literals must be built through the general construction path
instead. This was discovered by the compile error, not by reading, and is recorded so the next
person meets it in the plan rather than in the build.

### D5. Borrowed loop variables need the runtime to accept borrowed values

Also discovered by compile error. The runtime's traits are implemented on owned types, so a
borrowed loop variable does not satisfy them and the emitter change alone does not compile. The fix
is blanket implementations over references, delegating to the owned implementation.

The emitter change is a few lines. The runtime change is the work, and it is what makes the emitter
change legal rather than aspirational.

### D6. The boundary is staged last, and may not land here at all

Item 6 is the largest measured effect in the change and the only one with real design risk. A
borrowed text parameter puts a lifetime on generated signatures, and generated signatures are
something the repository has deliberately kept uniform.

So it is staged behind everything else, and the plan admits it may be split out. The parts of it
that carry no risk — documenting the per-element cost, and adding a conversion-dominated workload
so the cost is visible — are separated from the part that changes signatures, and land regardless.

*Alternative considered: keep collections Rust-side across calls, so a pipeline converts once.*
That is a larger and probably better answer, and it is a change of its own: it needs a user-visible
type, a lifetime story, and a decision about what happens when Python mutates the original. Named
here so it is not mistaken for something this change forecloses.

**Outcome: it did not land, and the risk was not the one this decision predicted.** The lifetime
was fine; generated signatures stayed uniform and results stayed owned `String` values. What broke
was the *element* type. The premise the item rested on — a text parameter is never mutated, so
borrowing it is always legal — is true and insufficient: a parameter can also be **stored**, and
storing needs ownership. `xs.append(who)`, `d[k] = who`, `who < "m"`, and `who in xs` each emitted
Rust that does not compile, and the whole suite passed anyway.

Deciding it correctly needs the backend to know an expression's type, which it deliberately does
not — every type-dependent choice dispatches through a trait so Rust selects the impl, and that is
what lets one emitter serve types it cannot see. Recovering the information, or proving safety with
a closed whitelist of positions that tolerate a borrow, is a change of its own.

What the attempt did establish is where the remaining text cost actually lives, and it is not the
parameter: a **mapping key** allocates an owned `String` per element even when the loop variable it
comes from is already borrowed. That is worth about 10us on `text.word_count`, it is independent of
parameter passing, and it is the better-targeted follow-up.

### D7. Rejected candidates keep their measurements

`-C target-cpu=native` was tested and rejected: no row moved outside noise, and it would make a
copied `.compylr/` directory fault on a machine with a different instruction set. Recorded in the
spec rather than only in a commit message, because it is exactly the kind of thing that gets
re-proposed every six months on the grounds that it is obviously free.

## Risks / Trade-offs

**A performance guard is a flaky test waiting to happen.** A threshold tight enough to catch a
regression is tight enough to fire on a loaded machine. → The guard is stated against the noise
floor rather than against an absolute figure, and D1 makes the noise floor a real measured
quantity rather than a guess. If it still proves flaky, the honest fallback is to run it outside
the default suite rather than to loosen it until it catches nothing.

**Link-time optimization makes the first build slower.** Roughly 7s to 10s on the demo's crate. →
Paid once per fingerprint change and recovered on every call thereafter. Worth re-measuring on a
substantially larger project before treating the figure as general.

**The in-place accumulation rule is a pattern match, and pattern matches over-fire.** A rule that
matched `x = y + x` would produce wrong answers for text. → The specs pin the name to the left
operand, and conformance must cover the mirrored form specifically, since it is the one that looks
like it should work.

**Blanket implementations over references can make inference worse.** Adding them may make some
existing generated code ambiguous. → It is a compile error, not a wrong answer, and the emit-quality
fixtures compile every accepted fixture. This is the failure mode this repository handles best.

**Six of seven items are separable; the seventh is not.** There is a temptation to do the boundary
"while we're in here". → D6 stages it, and the proposal says it may be split. The cheap items
should be shipped and measured before anyone opens the bridge.

**Measuring the wrong build.** The standing hazard: the fingerprint does not move, so nothing
rebuilds. → Every measurement task names `rm -rf .compylr demo/.compylr` explicitly rather than
assuming it is remembered.

## Migration Plan

There is nothing for a user to do, and nothing for them to notice except that their code is faster.
No setting is added, no API changes, and no IR changes — so no cache is invalidated for correctness
and no rebuild is forced.

That last point cuts both ways and is the migration's only real hazard: because the fingerprint
does not move, an existing project **will keep running its previously built artifact** after
upgrading, and will not pick up any of this until something else triggers a rebuild. If that is
judged unacceptable, the existing mechanism is the recorded compylr version in the build state,
which `_state_is_current` already compares — this change would then rely on the version moving,
which it does on any release.

Rollback is reverting the emission rules; nothing here is persisted, negotiated, or recorded in an
artifact that outlives a build.
