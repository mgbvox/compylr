## Context

See proposal.md — Why. What makes this change possible at all is that numpy's buffer is contiguous
(or regularly strided) memory of a single element type, which is what Rust's array views already
expect. Nothing else crossing the boundary has that property.

Constraints:

* `add-borrowed-parameters` supplies the ownership axis. A view *is* a borrow; without a passing
  mode there is nothing to emit.
* The mutability fixpoint already decides whether a receiver or instance parameter is mutable. An
  array parameter is decided by the same analysis.
* Collections cross by value and may not be mutated. Arrays deliberately do not follow that rule,
  and the difference must be explained where a user meets it.
* `Ty` has one integer type and one float type, by deliberate choice.
* Emission is a pure function of the unit; anything about the caller's memory is bridge work.

## Goals / Non-Goals

**Goals:**
- An array parameter that is a view over the caller's buffer, never a copy, at any size.
- Writes through a mutable array parameter visible to the caller.
- Rank and storage in the type, so indexing and emission are decided statically.
- Memory safety at the boundary, including the aliasing case.

**Non-Goals:**
- Array creation, array returns, and storing arrays. Deferred, and refused with a diagnostic naming
  the deferred capability.
- Vectorized whole-array arithmetic and broadcasting. `a + b` over two arrays is a ufunc, not the
  scalar `+` the IR carries; conflating them would give one of the two the wrong meaning.
- Slicing as an expression. Slicing is already refused and stays refused; a slice produces a view,
  which is the escaping case.
- `float32` and `int32`. Reserved and diagnosed as planned.
- Structured dtypes, object arrays, masked arrays, and anything non-numeric.

## Decisions

### Rank is part of the type, and must be written down

Indexing, the shape tuple's length, and the emitted view type all depend on the rank. Discovering it
at runtime would mean the emitted signature could not be chosen, so it is declared.

`np.ndarray` alone carries neither storage nor rank, and `NDArray[np.float64]` carries storage only.
So the accepted spelling is a compylr-provided ranked alias. This is not a new principle: the
corpus already refuses a bare `list` annotation, because "an element type that is not written down
is not a type compylr can use." Rank is the same kind of missing fact.

*Alternative considered: dynamic rank everywhere.* A dynamically-ranked view compiles, but then
`a[i, j]` cannot be checked at lowering, the shape tuple has no length, and every index costs a
runtime rank check. The subset's premise is mandatory annotations; this is where that premise pays.

### Storage is a property of the buffer; reads widen

The IR has one integer type and one float type on purpose. Adding `float32` as a *scalar* type
would introduce widths into a model that deliberately has none, and every backend and every
operator rule would have to answer for them.

So storage describes the buffer only, and reading an element yields the model's `Int` or `Float` —
which is also exactly what CPython does when you read a numpy scalar into Python. The array type
gets to be new without the scalar model changing.

Restricting storage to `float64` and `int64` here follows from the same reasoning: with those two,
a read is a widening of nothing and a write needs no narrowing rule. Adding `float32` means
answering what writing an out-of-range value into a narrower cell means, which is a genuine
semantic question and not one this change must answer to eliminate the copy.

### Mutation is visible to the caller, and the contrast is documented at the diagnostic

This is the change's most surprising rule, because it contradicts the collections rule a user has
already learned. The justification is representational, not a preference: a `list[int]` parameter is
converted element by element into a Rust `Vec`, so a mutation could not be observed; an array
parameter is a *view onto the caller's memory*, so a mutation is observed by construction.

That is the same distinction the codebase already draws between a collection and an *instance* —
"a collection parameter crosses by value and may not be mutated; an instance is not converted at
all". Arrays join instances on the second side.

Because a rule that contradicts another rule is where users lose confidence, the *existing*
diagnostic on a mutated collection parameter is extended to name the contrast. CLAUDE.md's standard
applies: "a rule without its reason leaves the user no workaround."

### Strided views, not enforced contiguity

Requiring C-contiguity would be simpler to emit and would silently copy exactly the arrays people
slice — `a[::2]`, a column of a matrix, a transposed view. Since the whole point is not copying,
supporting strided views is the requirement rather than an optimization. Rust's array views are
strided already, so this costs a view type, not an algorithm.

### Aliasing is checked at the boundary, and it is a safety requirement

`f(a, a)` with one parameter mutably bound produces two Rust references to one buffer, one mutable.
That is **undefined behavior**, not a wrong answer — the compiler is entitled to assume it cannot
happen, and the symptom would be a miscompilation that appears under optimization and not under
debug.

Nothing in the type system catches it, because both arguments are well-typed. So the bridge
compares the arguments' memory ranges before running compiled code and raises on overlap. The check
runs only when it can matter — more than one array parameter, at least one mutable — so the common
single-array call pays nothing.

*Alternative considered: copy on overlap.* Silently copying would make a call that looks zero-copy
occasionally not be, with no way for the user to know which. Refusing is louder and honest.

*Alternative considered: trust the user.* The failure mode is undefined behavior in generated code,
which is the single worst outcome this repository's design principles are organized against.

### Arrays are parameters only, in this change

Returning an array requires creating one, which requires array creation operations, an owned array
type crossing back to the host, and a decision about who owns the allocation. That is a coherent
follow-on change.

Not having it is less limiting than it sounds, because in-place output through a parameter is
numpy's own idiom — `out=` exists throughout numpy's own API — and reductions to scalars return
normally. The diagnostic names the deferred capability so a user knows it is coming rather than
absent by design.

### Partial indexing is refused

`a[i]` on a rank-two array yields a row *view* in numpy. A view is a borrow that would outlive the
expression, which is the escaping case `add-borrowed-parameters` establishes. `a[i, j]` is required
instead. This also removes a trap: in numpy `a[i][j]` and `a[i, j]` differ in cost, and refusing the
first means nobody writes the slow one by accident.

## Risks / Trade-offs

**Undefined behavior through aliasing** → The boundary check, specified as a requirement and
exercised by a corpus case. This is the risk that justifies the check's cost.

**Holding a view across a GIL release would be unsound** → Compiled code does not release the host
runtime's lock while a view is live. Nothing in the generated code releases it at all today, so this
is a constraint to preserve rather than one to add — and it is worth stating, because releasing it
is the obvious future optimization for a long-running array kernel.

**A user expects vectorized arithmetic and gets scalars** → `a + b` over two arrays is refused
rather than silently meaning something else. The refusal names ufuncs as unsupported, so the
diagnostic teaches the boundary instead of implying arrays are broken.

**Two mutation rules to learn** → Mitigated by extending the existing collection diagnostic to name
the contrast, so a user meets the explanation at the moment they hit the rule.

**numpy becomes a build-time requirement** → Only for programs that use arrays, and reported as a
located setup failure naming numpy, joining `cargo` and `maturin` on the list of things compiling
needs at runtime.

**Reduction order may differ from numpy's** → numpy uses pairwise summation for `sum`; a naive loop
does not, and the last bits differ. Fixtures compare floating-point answers within a stated
tolerance, and the difference is a property of the arithmetic rather than a defect.

**The artifact version collides with the other in-flight changes** → This change is last in the
chain, so it takes the number after `add-borrowed-parameters`.

## Migration Plan

The artifact version advances; caches are refused once and rebuilt automatically off the recorded
compylr version. No existing program changes meaning: the array type is additive, and every existing
type keeps its rules.

The generated crate gains two pinned dependencies, but only for programs that use arrays, so
projects that do not are unaffected including in build time.

Rollback is removing the change. Programs using arrays stop compiling with an unsupported-annotation
diagnostic; no program that compiled without arrays is affected.

## Open Questions

- Whether `len` on a rank-two array should be the first extent, matching numpy, or refused as
  ambiguous. The spec takes numpy's meaning, matching what the interpreted program returns; if the
  differential corpus shows users reaching for it expecting the total element count, refusing it is
  a diagnostic change and not a spec change.
