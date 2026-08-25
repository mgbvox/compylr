## Context

See `proposal.md` for motivation. The IR and pure Rust backend already represent `Ty::Instance`
and class-valued free-function signatures. The failure is at two seams around that representation:

- decorator validation lowers one captured member without the project class table, so an otherwise
  valid class annotation is currently indistinguishable from an unsupported annotation;
- the Python↔Rust bridge uses the backend's owned `rust_ty` spelling for every boundary position,
  even though a Python-visible class is a generated wrapper holding that inner Rust struct.

The wrapper is deliberately outside `generated.rs`: target translation remains Python-free, while
the pair-specific bridge owns Python conversion. Existing instance persistence relies on methods
borrowing `wrapper.inner`; a free-function parameter must preserve the same invariant. The existing
frontend already collects class names and signatures across sources during complete-unit assembly,
and the backend already analyzes assignments, nested mutation places, and transitively mutating
methods. The design should extend those mechanisms rather than duplicate them.

The differential fixture and its driver were added on the intended parent change, not on this
branch's `main` base. Their required shapes and temporary exclusion are authoritative in
`HANDOFF.md`; implementation should stack on or otherwise incorporate that parent change rather
than copy unrelated work into this proposal.

## Goals / Non-Goals

**Goals:**

- Keep the pure generated Rust class as the internal representation and the stable generated
  `#[pyclass]` wrapper as the only Python-visible representation.
- Make borrowing mode a property of a direct instance parameter's actual use, including
  transitive mutating method calls.
- Preserve located diagnostics through single-member validation and complete-unit resolution.
- Ensure the accepted differential corpus reaches this shape through both translation and the
  real Python call boundary.

**Non-Goals:**

- Converting instances nested inside lists, mappings, sets, tuples, or deeper combinations.
- General reference inference for text or collection parameters; those continue to cross by value.
- Changing the IR's `Ty::Instance` representation or introducing Python concepts into the Rust
  backend.
- Expanding direct instance conversion to explicit method or constructor parameters/returns beyond
  the existing implicit `self` receiver in this first slice.
- Guaranteeing Python aliasing patterns that require two simultaneous mutable Rust borrows of the
  same object; this change must not silently clone such an object, and any unsupported aliasing
  shape should fail explicitly rather than corrupt state.

## Decisions

### D1. Defer a dedicated unresolved-annotation category, then resolve against the complete class table

Add a machine-readable lowering category for a bare annotation that may name a class outside the
current captured source. Single-member validation reports that category and the manager adds only
it to the existing narrow deferral set. Complete-unit compilation first gathers all class names,
then lowers signatures with that table; a name still absent is reported at its original annotation
span.

Known language spellings remain decidable immediately. `complex`, unparameterized collections,
wrong generic arity, `None` in parameter position, and malformed expressions continue to use their
existing non-deferred diagnostics. This keeps a typo like `Taly` potentially resolvable while a
known unsupported built-in does not become a delayed error.

Alternative: defer every `unsupported_type`. Rejected because it would move useful decorator-time
errors to first build and let unrelated unsupported annotations masquerade as cross-source names.

Alternative: ask the manager to inspect Python globals. Rejected because compilation is defined by
the marked project sources, not by whatever objects happen to be imported into the live module, and
precompilation must behave the same way.

### D2. Classify boundary annotations after type resolution and before backend emission

Once an annotation resolves to a `Ty`, validate its boundary shape at the frontend/lowering seam:
a top-level free-function parameter or return may be exactly `Ty::Instance`; any container that
contains an instance recursively, and direct explicit instance positions outside this slice, are
rejected at the annotation span. Keep this check separate from generic type construction so nested
instance types remain representable internally and the policy stays about the exported boundary,
not about the language-neutral IR.

Alternative: let the generated bridge reject `Ty::List(Instance(...))`. Rejected because the bridge
works from IR and is too late to guarantee the source-located diagnostic required by the contract;
it would also allow pure Rust emission before discovering that the Python pair cannot convert the
type.

### D3. Give direct instance parameters a borrowed Rust ABI

Introduce one backend query for a direct instance parameter's access mode. It distinguishes a
shared read from mutation rooted at that parameter, including attribute assignment, nested place
mutation, and calls to methods classified mutable by the existing fixpoint. The generated function
signature uses `&T` for shared access and `&mut T` for mutable access, while scalar and collection
parameters keep their existing owned ABI. Generated call emission consults the callee signature and
the same access query so references are propagated consistently through free-function calls.

This query must distinguish mutating the object from rebinding the local parameter name; the latter
is a Python-local operation and must not overwrite the caller's object. If the current emitter
cannot preserve rebinding while holding a borrow, lowering should reject that precise combination
with a located diagnostic rather than translate it as mutation or fall back to cloning. Tests must
pin the chosen supported cases before emission changes.

Alternative: keep the owned ABI and clone `wrapper.inner`. Rejected because mutations would apply
to a disposable copy and violate the existing persistence promise while still returning plausible
answers for read-only cases.

Alternative: make every instance parameter mutable. Rejected because it needlessly takes exclusive
Python borrows, creates avoidable alias conflicts, and discards information the backend already has.

### D4. Centralize pair-specific boundary spelling around a stable class-to-wrapper map

Before emitting any binding, derive a deterministic map from each IR class name to the positional
wrapper identifier already used to emit and register classes. Replace free-function use of generic
`rust_ty` with boundary helpers that handle parameters, arguments, and returns together:

- a shared direct instance parameter is `PyRef<'_, Wrapper>` and calls generated code with
  `&parameter.inner`;
- a mutable direct instance parameter is `PyRefMut<'_, Wrapper>` and calls generated code with
  `&mut parameter.inner`;
- a direct instance result maps the successful inner value into `Wrapper { inner }` before
  producing the Python result;
- non-instance values retain their current spelling and error mapping.

The map is constructed once and passed to class and function binding emission so no loop can invent
a second positional identity. Missing map entries are bridge errors during emission, never invalid
Rust text.

Alternative: annotate the inner generated struct with `#[pyclass]`. Rejected because it mixes the
Python host into pure target translation and prevents the same generated Rust from serving another
host bridge.

### D5. Make state persistence and corpus coverage the acceptance evidence

Add lowering tests for same-source, cross-source, unknown, known-unsupported, nested, and marking
order cases. Add backend/bridge tests for the emitted borrowed signatures and stable wrapper map,
but make execution assertions decisive: a mutating free function must change the original Python
object, and a returned object must be the module's exposed class with usable persistent state.

On the differential parent, extend `class_valued_signatures.py` and its driver if needed to cover a
mutating parameter as well as `build` and `read`; then remove `BOUNDARY_EXCLUDED`, its one-fixture
guard, the fixture header's exclusion note, and the temporary narrowing in the differential change's
notes. If implementation begins before that parent is integrated, add the equivalent fixture and
driver as part of this change, but do not copy unrelated differential harness code.

## Risks / Trade-offs

- [Borrow mode and emitted call sites drift] → Centralize the access query and cover both direct
  Python calls and generated-to-generated calls with compile-and-run tests.
- [A mutation is missed through a method call] → Reuse the existing transitive method-mutation
  fixpoint and add a free-function regression where the body only calls a mutating method.
- [An unknown annotation is deferred forever] → Complete-unit compilation never suppresses the
  diagnostic; it preserves the original span and fails before backend emission.
- [Nested instance conversion leaks to rustc] → Recursively validate every exported boundary
  type after resolution and assert that backend emission is not reached on rejection.
- [Two arguments alias one mutable Python object] → Do not clone to make the borrow checker quiet;
  surface an explicit Python borrow/type error or a located compile-time restriction for shapes the
  safe borrowed ABI cannot represent, and document the tested boundary.
- [The proposal branch lacks the differential fixture] → Treat `HANDOFF.md` as evidence in this
  branch and integrate only the fixture/driver and exclusion cleanup when stacked on its parent.

## Migration Plan

1. Land or stack the differential-fixture change so its accepted fixture, driver, and temporary
   exclusion are present.
2. Add frontend diagnostics and boundary-shape validation, then update the manager deferral policy.
3. Change the backend ABI and call emission, then generate pair-specific wrapper parameter/result
   conversions.
4. Enable the complete differential boundary corpus and remove every temporary narrowing named in
   `HANDOFF.md`.
5. Run the full Rust, Python, generated-extension, documentation, and demo checks. No cache or data
   migration is required; reverting the code and restoring the guarded fixture exclusion is a safe
   rollback while the parent change remains under review.
