## Context

See `proposal.md` — Why. The facts that constrain the approach:

* `lower_expr` already returns `(Expr, Option<Ty>)`, and the `Option` is *undetermined*, not
  *error*. Undetermined arises only for a call the compilation cannot resolve; `collect_signatures`
  runs across every source in the compilation first, so a callee in another file of the same project
  is resolvable. The genuinely unresolvable case is a callee in a module the frontend never sees, and
  the subset already requires an annotation there.
* `Unit::validate` resolves calls and arity across the assembled unit, and `verify` already rejects
  what would produce target source that does not build. A type-consistency check belongs beside
  them, not in a new stage.
* `Literal::Float` stores an IEEE bit pattern so every IR type derives `Eq` and `Hash`. Whatever
  carries the type must not break that.
* `Function::fingerprint` hashes name, params, ret, and body, and excludes `span` and `doc`.
* `Expr::walk` is the single traversal, and `walk_calls` is built on it.
* `add-differential-fixture-testing` has landed, so every accepted fixture is driven and compared
  against CPython at two tiers. That corpus is what proves this change moves no answer.
* `add-python-backend` has landed, so `conformance.rs` exercises two backends and whatever that
  change recorded about the IR's neutrality is an input here.

## Goals / Non-Goals

**Goals**

* A type on every expression, constructed with its form so the two cannot disagree.
* Remove the three workarounds the absence forced, and measure each.
* Split `lower.rs` while the change is already touching all of it.
* Exactly one artifact-format bump for this whole line of work.

**Non-Goals**

* Borrowed parameters. Necessary but not sufficient; its own change, gated on
  `a_text_parameter_is_usable_in_every_position`.
* Numeric widths. `Ty::Int` stays 64-bit; widths are a separate question with an unmeasured payoff.
* Type *inference* changes. The frontend already computes these types; this change stops discarding
  them. If a type is wrong today, it is wrong today — visibly, after this.
* Changing any diagnostic, any accepted program, or any answer.

## Decisions

### D1. `Expr` becomes a form plus a type, not an optional annotation

**Decision.** `Expr` becomes a node holding an expression *form* and a `Ty`. The type is not
optional, not a side table, and not present on only some forms.

**Why, against each alternative:**

| Alternative | Why not |
| --- | --- |
| A side table keyed by expression id | Every expression needs an id, so the churn is the same *plus* an indirection — and the table can go stale, because a pass that rewrites an expression must remember to maintain it. A representation where the wrong thing is possible is the thing being removed. |
| A type on only the forms that need one | The backend still has to infer the rest, so the workarounds stay and the change buys a field. Whether a form "needs" a type is also a backend-specific judgement, which is exactly what should not be in the IR. |
| `Option<Ty>`, absent where undetermined | Every backend then needs a rule for absent, and there is no correct one. Undetermined is a *lowering* state, not a property of a finished program. |
| No IR change; the backend builds a type environment | Re-derives the frontend's inference in every backend — N copies free to disagree. `CLAUDE.md` already records why `returns_on_all_paths` is shared for exactly this reason: two implementations disagreeing means either rejecting a valid program or emitting code that does not compile. |

**Consequence for hand-built IR.** `conformance.rs`, `execution.rs`, and the IR unit tests build
trees by hand and would become unreadable if every node spelled its type. D2 answers that.

### D2. Constructors derive the type; a raw form is not constructible

**Decision.** Expressions are built through constructors that compute the type from the operands and
the declared modes — `Expr::binary(op, left, right)`, `Expr::subscript(base, index, origin, checked)`
and so on — with the raw form kept out of public reach.

**Why.** Two payoffs from one decision. Hand-built IR stays about as short as it is now, so the
corpus does not become a wall of type annotations. And a test cannot author a tree whose type
contradicts its form, so `conformance.rs` keeps testing backends rather than accidentally testing
whether a hand-written type was right.

**Where a constructor cannot decide,** it takes the type explicitly — `Expr::call` needs the
callee's return type, which the caller has and the expression does not.

### D3. The type is in the fingerprint and in the artifact

**Decision.** The type participates in `Function::fingerprint` and is serialized.

**Why.** Serializing it makes the artifact self-checking in the way the recomputed-fingerprint check
already intends: an artifact whose types were edited is refused rather than trusted. And the
fingerprint must cover it because two programs whose expressions differ in type are different
programs — even where the forms coincide.

**Why the redundancy is acceptable.** Types are derivable from forms plus declarations, so storing
them is redundant. Redundancy that is *checked* is a consistency check, not duplication: `verify`
recomputes and rejects a disagreement, so the stored type can never quietly diverge from the tree.

### D4. Consistency is checked in `verify`, not in lowering

**Decision.** The type-consistency rules live in `compylr-core/src/verify.rs`, run over the assembled
unit, and are frontend-independent.

**Why there.** Lowering already guarantees consistency for the units it builds; the units that need
checking are the ones built by hand, by a pass, by a future frontend, or read from an artifact.
`verify` is already the stage that rejects what would emit code that does not build, and already
tests that its verdict does not depend on the producing frontend.

**Scope of the check.** Result types follow from operands and modes; arguments match parameters;
returns match the declared return type; a name reads at the type it was bound at. Deliberately not a
full type checker — it checks the invariant the backend relies on, and nothing more.

### D5. `lower.rs` splits along the seams that already exist

**Decision.** Split into `scope.rs` (binding, frames, departed names), `signatures.rs`
(`collect_class_names`, `collect_signatures`, `collect_class_signatures`), `annotations.rs`
(annotation lowering and type-expression rules), `stmt.rs`, and `expr.rs` (expression lowering with
inference, which stay together for the reason `lower_expr` already states). `lower.rs` keeps the
entry points.

**Why now rather than separately.** Threading a type through every expression edits every one of
those regions. Splitting first means one mechanical commit and then a readable diff; splitting later
means a second pass over the same code. Splitting never means the file that has to grow for every
new construct is the one nobody wants to open.

**Why not further.** Inference does not become its own module. It is fused into `lower_expr` on
purpose — shape and type produced together so they cannot disagree — and separating them would
recreate the exact defect class this change is closing.

### D6. Each removed workaround is measured, not assumed

**Decision.** The three sites named in the proposal are removed one at a time, and each is measured
on the demo before and after.

**Why.** `improve-generated-code-performance` established the standard: every claimed win in that
change was applied by hand and rebuilt, so the numbers were what the change bought rather than what
it ought to buy. `CLAUDE.md` also records that the demo found a quadratic clone, an O(n) clone per
nested read, and a full recompile per marked member — all invisible to every correctness test. A
change that removes copies and claims nothing about cost is a change nobody can evaluate.

**Remember `rm -rf .compylr demo/.compylr` first.** The rebuild key is the IR fingerprint plus the
compiler version, and the version does not move during development here.

### D7. The migration is the existing one, and nothing is added for it

**Decision.** No reader for format 4 is kept. An artifact below version 5 is refused, and build state
already records the compiler version, so an upgraded install rebuilds once.

**Why.** This is precisely what versions 2, 3, and 4 did, and `ir.rs` already records that no earlier
reader is kept. Adding a migration path would be adding the first one, for a format whose only
consumer is a cache that can be regenerated in seconds.

## Risks / Trade-offs

**The change is enormous and touches everything** → It is, and the sequencing is the mitigation:
`add-differential-fixture-testing` gives every accepted fixture an oracle before this starts, so
"no answer moved" is checked mechanically rather than reviewed. The tasks also stage it — the split
first as a pure move, then the field, then the workarounds one at a time, each commit green.

**A wrong type becomes a compile error in generated code** → Today a wrong inference is invisible
because nothing reads it. After this it produces target source that does not build, which reads as a
complaint about Rust rather than about the user's program. Mitigation: that is exactly what D4's
`verify` check is for — it turns the class of failure into a located diagnostic before emission, and
its scenarios are written to cover each way the backend could be misled.

**Constructors hide a type the reader wanted to see** → A derived type is a type nobody wrote down,
so a reader of `conformance.rs` cannot see what a node claims. Mitigation: the constructors are
total and tested per form, and the corpus's job is coverage rather than documentation. Where an
entry exists *because* of a type, it uses the explicit constructor.

**The split makes the diff unreviewable** → Mitigation: the split is its own commit containing no
other edit, so `cargo test --workspace` before and after is a clean comparison and the diff is
`git log --follow`-able.

**Scope creep into borrowed parameters** → The temptation will be strong, because the enabling fact
finally exists. It is a non-goal above and `CLAUDE.md` records the four shapes that break; the guard
is that `a_text_parameter_is_usable_in_every_position` must keep passing unchanged, and any change
to it belongs to the other change.

## Migration Plan

One rebuild per project, automatic. `_state_is_current` compares the recorded compylr version, so an
upgraded install rebuilds rather than reusing a version-4 artifact. Nothing for a user to do beyond
knowing the first run after upgrading is slow — the same note versions 2, 3, and 4 carried.

Within this repository, `rm -rf .compylr demo/.compylr` before any measurement, per D6.

Rollback is reverting the change; artifacts written at version 5 are refused by the earlier compiler,
which is the correct behavior and costs a rebuild.

## Open Questions

* **Whether `verify`'s type check should report the first inconsistency or all of them.** Lowering
  reports the first violation in source order, and matching that is the starting answer. Changing it
  later alters no spec scenario and no task.
