## Context

See proposal.md — Why, for motivation, and the delta specs for the behavior being contracted.

Four facts about the current code shape everything below.

**The axes are already modelled; nothing chooses between them.** `DivMode`, `RemSign`,
`IndexOrigin`, and `TextUnits` are already on the nodes, and every one is set from a constant at
the top of `crates/compylr-frontend-python/src/lower.rs`. Five of six axes are therefore a matter
of replacing a constant with a lookup. The sixth — overflow — has no mode at all, because
`PyAdd`/`PyNum` unconditionally check, and it is the one that changes the IR's shape.

**The backend deliberately cannot see expression types.** `rust.rs` says so at the top: the IR
does not annotate expressions, and re-deriving types in the backend would mean a second type
checker. That is why `+` emits as `PyAdd::py_add(&(a), &(b))?` and lets Rust's trait resolution
pick between `i64` and `String`. Any plan to emit a bare `+` has to say what happens where the
type is unknown.

**Guarantees are declared statically per language.** `PythonFrontend::requires()` returns a
`&'static [Guarantee]`, and `Unit::set_origin` copies it onto the unit. Nothing today can express
"this program requires less than Python usually does".

**`Guarantee` lives in `compylr-ir`, not in `compylr-core`.** Its module doc says why: a `Unit`
records what it requires, and the IR cannot depend on the crate that consumes it. Behavior has the
same shape and takes the same placement.

## Goals / Non-Goals

**Goals:**

- Every mode on every node comes from one resolved behavior, so the IR remains fully
  self-describing and no component infers a meaning from which frontend ran.
- Resolution costs one declaration per language, not one entry per pair.
- The default path is byte-identical to the output **as of whenever this change lands** — provable
  by diffing emitted source, not by argument. Note the moving baseline:
  `improve-generated-code-performance` changes emission for semantics-preserving reasons (an
  in-place string append, a borrowed loop variable, a moved rather than cloned return), so "today's
  output" is not a fixed target. The snapshot this diff compares against must be taken from the
  tree at the point the default path is frozen, after whichever of the two changes lands first —
  not from a snapshot captured when this design was written.
- Where a node declares Rust's meaning, the generated source reads like Rust a person would have
  written, and the reader of `.compylr/` can see that it does.
- A behavior mistake is reported by the decorator that contains it.

**Non-Goals:**

- No third value on any axis. The domain is the two languages in the compilation, and a value like
  `"checked"` or `"fast"` would be a meaning belonging to neither.
- No behavior on the *bridge*. How values cross the boundary — collections by value, instances by
  reference — is a property of the pair and not something an axis selects. Worth stating what that
  costs, since it is the single largest performance lever and this change deliberately declines it:
  a collection parameter is converted element by element on every call, ~4 ns per element for
  `list[int]` and ~42 ns for `list[str]`, so `binary_search` over 2000 elements does O(n) boundary
  work for an O(log n) algorithm and loses to the interpreter by 16x. That is real and it is not an
  axis — it belongs to `improve-generated-code-performance`.
- No per-expression or per-block behavior. The member is the unit of choice; a `with` block that
  changed arithmetic mid-function would make a line's meaning depend on where it sits.
- No change to the accepted subset, and no new diagnostics beyond those an invalid behavior needs.
- Not an implementation of `unchecked-arithmetic`. This change makes the option *permittable* for
  the right unit; whether the Rust backend ever implements it stays a separate question.

## Decisions

### D1. Behavior resolves before lowering and is applied *by* lowering

A resolved behavior is computed once, from the request and the two languages' declarations, and
handed to the frontend. Lowering sets every node's modes from it. The backend is unchanged in
principle: it still matches on modes and never on which frontend ran.

*Alternative considered: a switch on the backend — emit native operators when the caller asks.*
Rejected, and this is the load-bearing rejection. It would make the IR no longer say what the
program means: two units that are structurally identical would compile to programs that disagree
on `-7 // 2`, with the difference held outside the tree. Every consequence follows from that. The
fingerprint would stop distinguishing two builds that compute different answers, so the rebuild
cache would serve the wrong artifact. Constant folding would fold `-7 // 2` to `-4` for a program
the backend was about to emit as `-3`. The written IR under `.compylr/` would no longer describe
what was built. And a second backend would have to re-derive the same switch.

*Alternative considered: a `SemanticsProfile` on the unit, with nodes staying bare.* Rejected for
the reason the archived `modularize-language-pipeline` change already rejected it: it recreates
the problem one level up, every pass grows a profile switch, and mixed behavior within a project
becomes unrepresentable.

### D2. One new mode, `Checked`, on the operations that can fail

`Checked::{Reported, Unchecked}` goes on `BinOp::Add`, `Sub`, `Mul`, `Div`, `Rem`, on `Expr::Neg`,
and on `Expr::Subscript`. It composes with the modes already there rather than replacing them:
`Div { mode: Integer(TowardZero), checked: Unchecked }` is Rust's `/`, and
`Div { mode: Integer(TowardNegInf), checked: Unchecked }` is a flooring division whose zero
divisor is undefined — a real combination, reachable from `Behavior(floor_div="python",
true_div="rust")`, and one the backend must still emit a flooring helper for.

*Alternative considered: separate enums per operation — `OverflowMode`, `ZeroDivisorMode`,
`BoundsMode`.* Rejected: three enums with identical shape and identical meaning, differing only in
which failure they describe, and every backend would match on all three the same way.

*Alternative considered: a single unit-level "checked" flag.* Rejected — it is the D1 mistake
again, and it forecloses `Behavior(overflow="rust", index="python")`, which is the combination a
careful user most plausibly wants.

### D3. `Unchecked` is a statement about the program, not about the target

The IR may not contain a mode meaning "whatever the target does" — that would make a unit's
meaning depend on who reads it, which is the property the IR exists to have. `Unchecked` instead
says the *program* does not define the result. That is a fact about the program, true of the unit
regardless of backend, and it happens to license a backend to emit its native operator.

This is what makes Rust's debug-panics / release-wraps split expressible at all. A mode named
`Wrapping` would be a lie in a debug build; `Unchecked` is true in both.

### D4. Axes are named neutrally in core; the user-facing flag names belong to the host

`compylr-ir` defines the six axes with neutral identifiers — `integer_overflow`,
`integer_division`, `exact_division`, `remainder`, `sequence_index`, `text_length`. The Python
package exposes them as `overflow`, `floor_div`, `true_div`, `modulo`, `index`, `text_len`, which
are Python's names for Python's operators.

That split is the same one `Ty::python_name` and `BinOp::python_symbol` already made: how a
construct is spelled back to the programmer belongs to the frontend that read it. A TypeScript
host would name the same axes after `/`, `%`, and `.length`, and would resolve against the same
neutral identifiers underneath.

### D5. Each language declares a stance; resolution picks per axis

`Frontend` and `Backend` each gain `fn behavior(&self) -> &'static LanguageBehavior` — a complete
bundle, one mode per axis, describing that language only. Resolution takes the request, the two
bundles, and the two names, and returns a `Behavior` with exactly one stance per axis.

`LanguageBehavior` lives in `compylr-ir` beside `Guarantee`, for the reason recorded there: a unit
holds the modes, and the IR cannot depend on the crate that consumes it. Resolution itself lives
in `compylr-core`, which is where two components already meet.

Validation happens during resolution and produces a three-way answer matching the registries':
unknown language, known-but-not-in-this-pair, unknown axis.

### D6. Native emission is chosen from the node's modes and the expected type

`emit_binop` already receives an `expected: &Ty` and derives operand types from it. Where
`expected` is `Ty::Int` or `Ty::Float` **and** the node's modes are exactly Rust's own, emit the
bare operator: `((a) + (b))`. Where `expected` is `Ty::Unit` — which happens for arithmetic under
a comparison, since comparison operands say nothing about the result type — emit through a new
infallible trait, `NativeAdd`/`NativeNum`, whose `i64` implementations are `self + rhs` and which
return a value rather than a `Result`.

This is the one place the plan does not fully deliver "no adapters", and the reason is D6's
premise: the backend must not re-derive types. The shim is a dispatch, not a check — it inlines to
one instruction, and it is what makes `a + b > c` compile for both integers and strings without a
type checker in `rust.rs`.

*Alternative considered: annotate expressions with their types in the IR, so the backend always
knows.* Rejected as far larger than this change: it touches every `Expr`, every fingerprint, and
the artifact format again, and it is a change worth making on its own merits rather than as a
side effect of a behavior flag.

*Alternative considered: emit the shim everywhere, never a bare operator.* Rejected — it is
correct and it is invisible. Part of what the user is buying is generated source they can read and
recognise; `.compylr/` full of `NativeAdd::add` calls would deliver the speed and not the claim.

### D7. Generated signatures stay uniformly `Result<T, RuntimeError>`

Even when every operation in a body is unchecked. The existing reason holds unchanged: a signature
that becomes fallible or infallible depending on the body's contents moves on an unrelated edit.
Behavior only adds a *second* way for it to move.

The body still gets the win — no `?`, no error path, one `Ok(...)` at the boundary — and the
bridge stays a single call shape. The empty error branch costs nothing after inlining.

### D8. What a unit requires is derived from the unit

`Origin.requires` stops being a copy of `Frontend::requires()` and becomes the union of what the
lowered functions' modes ask for: a `Reported` arithmetic node contributes
`IntegerOverflowReported`, a `Reported` division or remainder contributes
`DivisionByZeroReported`. `FloatOrderPreserved` is contributed unconditionally, because
reassociation is a transformation a backend might apply rather than an operation the programmer
wrote — there is no axis for it and no way to ask for it.

Deriving it by walking the unit, rather than by mapping the resolved behavior, is deliberate: a
hand-built unit in the conformance corpus has no behavior and would otherwise require nothing at
all, and a unit assembled from members under different behaviors has no single behavior to map.

`Frontend::requires()` stays, redefined as what the language requires under its own stance. It is
what the negotiation's error message names, and what a caller asking "what does Python need?"
should get.

### D9. Behavior may be mixed within a project; backend still may not

`Manager.ensure_built` refuses members marked for different backends because a project compiles to
one artifact. Behavior is different in kind and is allowed: it rides on nodes, so two functions
with different meanings coexist in one unit, and a call between them is an ordinary call.

The consequence is an API change: `_core.compile_unit` takes `(source, behavior)` pairs rather
than a list of sources. `validate_source` is deliberately *not* changed — see D10 — so the
decorator's immediate validation is unaffected.

### D10. Behavior changes meaning, never acceptance

Nothing about which programs lower successfully depends on the behavior. In particular `/` keeps
its float result type under every behavior: `true_div="rust"` selects what happens when the
divisor is zero (IEEE `inf` rather than a reported error), not what type the expression has.

*Alternative considered: `true_div="rust"` meaning Rust's `/`, so `7 / 2` is `3`.* Rejected. It
would make `def f(a: int, b: int) -> float: return a / b` fail to type-check under one behavior
and pass under another, so the same annotated source would be two different programs — and the
annotations are the one thing this subset insists on. A user who wants truncation writes `//`.

The consequence worth stating: `xs[-1]` under `index="rust"` is *not* rejected at lowering. The
index is a runtime value, and refusing a literal `-1` would refuse only the visible cases while
leaving `xs[i]` with a negative `i` to fail at runtime — a rule that catches the easy half is
worse than no rule, because it reads as a guarantee.

### D11. Folding must read the checking mode

`compylr-core::folding` folds `7 // -2` correctly today because it reads the rounding mode off the
node. It must now also read `Checked`: folding an `Unchecked` overflow into a reported error would
manufacture a failure the program declined to define, and it is exactly the kind of defect that
shows up as one wrong constant in generated source. An `Unchecked` operation whose fold would
overflow or divide by zero is left unfolded.

### D12. Artifact format version 4, with no migration

The serialized shape changes, so the version advances and every existing `.compylr` cache is
refused once and rebuilt. No reader for version 3 is kept: the only thing a v3 artifact could mean
is "everything reported", and writing a migration to assert that would be more code than the
rebuild it saves.

### D13. The Python surface

`Behavior` is a frozen dataclass with six `str | None` fields; `None` means inherit. `Settings`
gains `behavior: str | Behavior`, normalised to a `Behavior` on construction, and validated in
`__post_init__` through a new `_core.check_behavior(frontend, backend, mapping)` — the same shape
as the existing `check_backend`, and the same reason: a bad value is reported by the decorator
that named it. `Settings.override` gains per-field inheritance for behavior, so
`@c.compyle(behavior=Behavior(overflow="rust"))` merges into the manager's behavior rather than
replacing it.

A bare string is normalised to `Behavior` with every field set, which makes "`behavior='rust'`
equals every flag `'rust'`" true by construction rather than by two code paths agreeing.

### D14. Threading the profile through lowering

`Ctx<'a>` gains the behavior, but `lower_expr` takes `Names<'_>` rather than `Ctx`, and it is
`lower_expr` that builds the operator nodes. Rather than add a second parameter at every call
site, `Names` is wrapped in a `Copy` carrier that holds both — a mechanical change across roughly
forty sites, with no logic moving.

### D15. Conformance scope is the failing forms, not the cross product

`tests/conformance.rs` checks `(form, position)` pairs. Behavior adds a third dimension, and the
full cross product is neither necessary nor affordable. The honest scope: every form that carries
a mode, in every position it is legal in, under both stances of the axis that governs it. Forms
with no mode are unaffected by behavior by construction, and a test asserting that would be
asserting the absence of a field.

## Risks / Trade-offs

**`index="rust"` silently reinterprets every negative index.** `xs[-1]` stops being the last
element and becomes a panic, with no diagnostic — by D10, deliberately. → It is opt-in and never
implied by anything smaller; the README's axis table states it in the same row as the flag; and
the demo is required to say what its Rust-behavior build gives up.

**Overflow has two answers depending on the build profile.** Native `+` panics under
`overflow-checks` and wraps without them. compylr builds generated crates with `--release`, whose
default is to wrap, but the crate under `.compylr/` is a real crate that someone may build in
debug and get a different program. → Documented as what "Rust's own operator" means. The reversal,
if it proves confusing, is to pin `overflow-checks = false` in the generated manifest's release
profile, which makes the answer profile-independent at the cost of no longer being literally
Rust's default.

  **Correction, from measurement:** that reversal is not a one-line edit today, because there is no
  line to edit. `cargo_manifest` in `crates/compylr-bridge-python-rust/src/bindings.rs` emits
  `[workspace]`, `[package]`, `[lib]` and `[dependencies]` and stops — the generated crate has **no
  `[profile.release]` section at all**. So the mitigation is a section to create, not a setting to
  flip. `improve-generated-code-performance` adds that section for unrelated reasons (`lto`,
  `codegen-units`); if it lands first this becomes the one-line edit described above, and if it
  does not, this change must create the section itself. Either way the dependency is explicit
  rather than discovered while implementing.

**Mixed behavior is one more thing that can differ between two functions that look alike.** Two
adjacent functions can compute different answers for `-7 // 2`, and nothing in the Python source
of the second one says so. → The setting is on the decorator, one line above the function; and the
IR written to `.compylr/` shows the resolved modes per function, which is where a confused user
should be pointed.

**The `Checked` mode is easy to add and easy to forget to read.** A backend that matches on
`BinOp::Add { .. }` and ignores the mode compiles fine and is silently wrong. → The Rust backend's
match is written to bind the mode rather than wildcard it, and the conformance corpus carries both
stances of every axis, so a backend that ignores one fails rather than passing quietly.

**The IR shape churns for the third time.** Every cache invalidates again. → It is one rebuild,
`_state_is_current` already handles the version check, and the alternative is doing it later with
more users.

**The benchmark cannot currently resolve a behavior difference.** `sorting.merge_sort` returned
160, 202, 235, 256, 264 and 277 us across runs of binaries that were in some cases byte-identical
— best-of-five-batches does not stabilise an allocation-heavy recursive workload, and the harness
reports a single best with no spread. A behavior delta smaller than roughly 30% would therefore be
indistinguishable from the harness itself, which makes "measured rather than asserted" unachievable
as the demo spec currently requires it. → The demo capability now requires the benchmark to report
spread and to name its noise floor, and task 11.4 makes that a prerequisite of the behavior
comparison rather than something discovered when the comparison reads oddly.
`improve-generated-code-performance` carries the same prerequisite, for the same reason; whichever
lands first satisfies it for both.

**Folding is the likeliest silent defect.** A fold that ignores `Checked` produces one wrong
constant in otherwise-correct output — no crash, no diagnostic. → D11 names it, and the fold tests
must cover an overflowing `Unchecked` constant expression specifically.

## Migration Plan

There is nothing for a user to do. `behavior` is optional everywhere and defaults to the source
language, so an unchanged project compiles to unchanged output; the format version bump forces one
rebuild on first run after upgrading, which `_state_is_current` already triggers on the recorded
compylr version.

Rollback is removing the setting: with no `behavior` anywhere, the resolved behavior is Python's
stance on every axis and the emitted source is what it is today. That equivalence is worth
asserting as a test rather than trusting — diff emitted source for every accepted fixture against
a pre-change snapshot, once, during implementation.

During development in this repository, note the standing hazard from CLAUDE.md: the rebuild key is
the IR fingerprint, and editing the *backend* does not invalidate a cached build. Changing
emission for native operators means `rm -rf .compylr demo/.compylr` before measuring anything.
