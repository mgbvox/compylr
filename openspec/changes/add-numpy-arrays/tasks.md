## 0. Prerequisite

- [ ] 0.1 Confirm `add-borrowed-parameters` is complete and archived, and with it
      `add-typed-ir-expressions`. **Do not start before both.** A zero-copy array parameter is a
      borrowed parameter; without the passing mode there is nothing to emit.
- [ ] 0.2 Record the measured per-element boundary cost for a sequence argument, as the before-state
      the array path is compared against.

## 1. The type

- [ ] 1.1 Write the failing tests: an array type carries storage and rank; two ranks are unequal; an
      array may not key a mapping; an array is not trivially copyable; the type round-trips.
- [ ] 1.2 Add the array type to [`Ty`](../../../crates/compylr-ir/src/ir.rs#L103) carrying element
      storage and rank, naming no library dtype spelling.
- [ ] 1.3 Support `float64` and `int64` storage; reserve `float32` and `int32` as
      diagnosed-as-planned.
- [ ] 1.4 Advance [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) and extend
      [`Unit::fingerprint`](../../../crates/compylr-ir/src/ir.rs#L1299) over storage and rank.
- [ ] 1.5 Add the neutral rendering for the array type, and the Python spelling in
      [`spelling.rs`](../../../crates/compylr-frontend-python/src/spelling.rs#L16).
- [ ] 1.6 Confirm [`crate_boundaries.rs`](../../../crates/compylr-host-python/tests/crate_boundaries.rs)
      still holds — nothing named numpy may reach `compylr-ir`.

## 2. Annotations and lowering

- [ ] 2.1 Write the failing tests: a ranked annotation lowers; an unranked one names the ranked
      form; an unsupported storage reports as planned; a partial index is refused; an array return
      is refused; storing an array is refused.
- [ ] 2.2 Add the `Array1` and `Array2` annotation aliases to
      [`frontends/python/`](../../../frontends/python/), written so `ty` accepts them, and confirm
      `make py-types` passes on code using them.
- [ ] 2.3 Lower the ranked annotation to the array type in
      [`lower.rs`](../../../crates/compylr-frontend-python/src/lower.rs); refuse the unranked form
      naming the ranked spelling.
- [ ] 2.4 Type indexing from the rank: one index per rank, yielding a scalar of the storage's model
      type.
- [ ] 2.5 Apply the declared index origin and checking mode to array indexing, reusing the sequence
      path rather than a second copy of it.
- [ ] 2.6 Type shape as a tuple of integers of length equal to the rank; type length as the first
      extent, matching numpy.
- [ ] 2.7 Accept element assignment into an array parameter, and extend the existing
      mutated-sequence-parameter diagnostic — currently *"'values' is a parameter, and a collection
      parameter is a copy"* — to name the contrast with an array parameter.
- [ ] 2.8 Refuse whole-array arithmetic naming ufuncs as unsupported, so the diagnostic teaches the
      boundary.
- [ ] 2.9 Decide array parameter mutability with the existing fixpoint; add no second analysis.

## 3. Emission

- [ ] 3.1 Write the failing tests: a shared parameter emits a shared view; a mutable one emits a
      mutable view; no clone is emitted; an element write emits a place.
- [ ] 3.2 Emit array parameters in [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs) as
      strided views of the declared rank and storage, from the passing mode.
- [ ] 3.3 Emit element reads as direct indexed reads under the declared checking mode.
- [ ] 3.4 Emit element writes through `emit_place` so the write lands in the caller's buffer.
- [ ] 3.5 Confirm `place_root` follows an array element chain when deciding mutability.
- [ ] 3.6 Extend [`conformance.rs`](../../../crates/compylr-host-python/tests/conformance.rs) for
      array use in each legal position.
- [ ] 3.7 Confirm emission stays a pure function of the unit and byte-reproducible.

## 4. The boundary

- [ ] 4.1 Write the failing tests: binding does not copy; a write reaches the caller; a strided array
      binds without copying; a wrong storage or rank is refused before any code runs.
- [ ] 4.2 Bind a shared parameter as a read-only view and a mutable one as a writable view over the
      caller's buffer, in
      [`compylr-bridge-python-rust`](../../../crates/compylr-bridge-python-rust/src/lib.rs).
- [ ] 4.3 Support strided arrays as strided views; never make an array contiguous by copying.
- [ ] 4.4 Refuse a mismatched storage or rank at the boundary, naming what was expected.
- [ ] 4.5 Confirm no view is retained after the call returns.
- [ ] 4.6 Do not release the host runtime lock while a view is live, and add a comment saying why,
      since releasing it is the obvious future optimization.

## 5. Aliasing safety

- [ ] 5.1 Write the failing tests first: the same array passed for a mutable and another array
      parameter is refused; overlapping slices are refused; disjoint arrays proceed; all-shared
      parameters are accepted.
- [ ] 5.2 Compute each array argument's memory range at the boundary.
- [ ] 5.3 Refuse before running compiled code when a mutably bound parameter overlaps another array
      parameter, naming the overlap. This is undefined behavior rather than a wrong answer, so the
      test must exercise it under a release build where the miscompilation would appear.
- [ ] 5.4 Skip the check where it cannot matter — at most one array parameter, or every array
      parameter shared.
- [ ] 5.5 Confirm the check's cost is not paid by a single-array call.

## 6. Build pipeline

- [ ] 6.1 Write the failing tests: a program using arrays declares the dependencies; one that does
      not is unchanged; a missing numpy reports as a setup failure with its category.
- [ ] 6.2 Declare the pinned array and array-binding dependencies in the generated manifest, only for
      programs that use arrays.
- [ ] 6.3 Report a missing or incompatible numpy as a located setup failure naming numpy, carrying
      the machine-readable category.

## 7. Corpus

- [ ] 7.1 Move the proposal's worked example into
      [`accepted/`](../../../frontends/python/fixtures/accepted/) as `arrays.py` — `dot` reads and
      `scale` writes — and add a rank-two fixture, with member names unique across the whole
      accepted corpus.
- [ ] 7.2 Add drivers in [`drivers/`](../../../frontends/python/fixtures/drivers/) carrying no
      expected values, asserting on the caller's array after a mutating call rather than on a return
      value.
- [ ] 7.3 Add a fixture passing a non-contiguous array, so the strided path is exercised.
- [ ] 7.4 Compare floating-point answers within a stated tolerance, since reduction order may differ
      from numpy's pairwise summation.
- [ ] 7.5 Add rejected fixtures in [`rejected/`](../../../frontends/python/fixtures/rejected/):
      unranked annotation, unsupported storage, partial index, array return, array stored into an
      attribute, and whole-array arithmetic.
- [ ] 7.6 Add the boundary case asserting an overlapping call is refused.
- [ ] 7.7 Add the derived check that fails when a supported rank or storage has no fixture.

## 8. Demo coverage and measurement

- [ ] 8.1 Remove `.compylr/` and `demo/demo-python-rust/.compylr/` before measuring.
- [ ] 8.2 Add an array algorithm to
      [`demo/demo-python-rust/src/algorithms/`](../../../demo/demo-python-rust/src/algorithms/) — a
      new `Ty` trips
      [`demo_coverage.rs`](../../../crates/compylr-host-python/tests/demo_coverage.rs), and the demo
      is where the copy-elimination claim gets measured.
- [ ] 8.3 Measure call setup across increasing array sizes and confirm it does not grow with the
      element count.
- [ ] 8.4 Measure the same computation over a sequence parameter and an array parameter, and record
      the difference.
- [ ] 8.5 Regenerate the README benchmark tables with
      [`update_benchmarks.py`](../../../scripts/update_benchmarks.py); never hand-edit.

## 9. Documentation and checks

- [ ] 9.1 Regenerate the README subset matrix with
      [`update_subset.py`](../../../scripts/update_subset.py); confirm `--check` passes.
- [ ] 9.2 Update README prose: the array type, the ranked annotation, and that an array parameter is
      a view while a collection parameter is a copy.
- [ ] 9.3 Update [`CLAUDE.md`](../../../CLAUDE.md): the array type, the mutation contrast and why it
      differs, the aliasing check, the deferred creation and ufuncs, and numpy as a build-time
      requirement.
- [ ] 9.4 Update [`demo/README.md`](../../../demo/demo-python-rust/README.md) or the coverage claim
      if the added algorithm changes it.
- [ ] 9.5 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and
      `cargo test --workspace`.
- [ ] 9.6 Run `make check` and `make demo`.
