## 1. Frontend Resolution and Located Boundary Diagnostics

- [ ] 1.1 Write diagnostic and lowering tests for same-source and cross-source direct class-valued
  free-function parameters/returns, both source orders, a misspelled class name, known unsupported
  built-ins, nested class-valued containers at multiple depths, and explicit method boundary
  positions; assert error category and exact source location before changing lowering.
- [ ] 1.2 Add a dedicated machine-readable unresolved-class-annotation category and teach
  single-source validation to use it only for bare names that could resolve from another source,
  preserving immediate errors for known unsupported or malformed annotations.
- [ ] 1.3 Resolve direct class annotations from the complete cross-source class table, then add a
  boundary-shape validator that accepts only direct top-level free-function instances and rejects
  nested or out-of-scope instance positions at their annotation spans before backend emission.
- [ ] 1.4 Run the focused diagnostics, frontend, lowering, unit-verifier, and host-Python tests and
  commit the frontend checkpoint.

## 2. Decorator Deferral Without Silent Acceptance

- [ ] 2.1 Write Python manager tests showing that a class-annotated function may be marked before or
  after its class, a final typo fails at whole-project build with its location, `complex` and
  malformed annotations still fail while marking, and nested instance containers fail before any
  build pipeline writes Rust.
- [ ] 2.2 Replace the manager's single deferred code with an explicit immutable set containing only
  undetermined bindings and unresolved class annotations, leaving every other validation error
  immediate and preserving the no-interpreted-fallback behavior.
- [ ] 2.3 Run the focused Python manager, programmatic compilation, and host exception-category tests
  and commit the decorator checkpoint.

## 3. Borrowed Instance ABI in Generated Rust

- [ ] 3.1 Write backend tests first for a read-only instance parameter, direct attribute mutation,
  nested place mutation, a transitively mutating method call, generated-to-generated calls, and
  parameter rebinding; assert both compilable emitted form and executed state semantics.
- [ ] 3.2 Introduce one shared instance-parameter access analysis that distinguishes shared reads,
  object mutation, and local rebinding while reusing the existing place-root and transitive method
  mutation logic.
- [ ] 3.3 Emit direct instance parameters as `&T` or `&mut T` from that analysis and adapt generated
  call arguments to the callee's borrowed ABI; reject any rebinding/mutation combination that cannot
  preserve Python semantics with a located frontend diagnostic instead of cloning or mutating the
  caller incorrectly.
- [ ] 3.4 Run backend unit, emission-quality, conformance, generated-crate compile, and execution
  tests and commit the borrowed-ABI checkpoint.

## 4. Stable Python Wrapper Conversion

- [ ] 4.1 Write bridge generation tests first for deterministic class-name-to-wrapper lookup,
  shared `PyRef`, mutable `PyRefMut`, class-valued result wrapping, missing wrapper-map entries, and
  unchanged scalar/collection signatures.
- [ ] 4.2 Build the stable wrapper map once from the unit's deterministic class order and route
  free-function boundary parameter, call-argument, and return spelling through pair-specific
  helpers rather than the backend's generic owned `rust_ty` spelling.
- [ ] 4.3 Pass `PyRef`/`PyRefMut` inner values to generated functions by shared/mutable borrow and
  wrap successful owned instance results in the mapped `#[pyclass]` wrapper before returning them;
  keep existing runtime-error-to-Python-exception conversion unchanged.
- [ ] 4.4 Add extension-level tests showing `read(t: Tally)` observes current state, a free function
  that mutates directly or only through a mutating method changes the same Python object, and
  `build(...) -> Tally` returns the exposed class with independent persistent state across calls.
- [ ] 4.5 Run the focused bridge, build-pipeline, PyO3 boundary, and end-to-end tests and commit the
  wrapper-conversion checkpoint.

## 5. Differential Corpus and Documentation

- [ ] 5.1 On top of the differential-fixture change, verify or add only the accepted
  `class_valued_signatures.py` fixture and its shared driver, extending them as needed to exercise
  direct reads, persistent mutation, and returned instances without copying unrelated harness code.
- [ ] 5.2 Remove `BOUNDARY_EXCLUDED`, `test_the_exclusion_stays_one_fixture_wide`, and the fixture
  header's exclusion note so the boundary tier derives and exercises the complete accepted corpus.
- [ ] 5.3 Remove the temporary class-signature narrowing from the differential change's `notes.md`
  while preserving its unrelated measurements and findings.
- [ ] 5.4 Update README prose for direct class-valued free-function signatures and the explicit
  nested-boundary limitation, regenerate the subset matrix with `scripts/update_subset.py`, and run
  its `--check` mode rather than editing generated tables by hand.
- [ ] 5.5 Run both differential tiers and their fixture/driver completeness checks, confirm the
  class-valued fixture agrees with CPython through the real bridge, and commit the corpus and
  documentation checkpoint.

## 6. Full Verification

- [ ] 6.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` with the vendored Ruff submodule initialized.
- [ ] 6.2 With the project venv active and the host extension rebuilt, run Python tests with
  `pytest-cov` enforcing at least 80% coverage, `ruff check python/ scripts/`,
  `ruff format --check python/ scripts/`, and `ty check python/compylr`, never linting fixtures.
- [ ] 6.3 With the venv deactivated, run workspace `cargo llvm-cov` excluding vendored code and
  `main.rs`, and confirm the changed non-trivial paths remain covered.
- [ ] 6.4 Clear generated compylr caches required after emission changes, run `make demo`, and verify
  compiled and interpreted answers still agree with no unexplained performance regression.
- [ ] 6.5 Run `make check`, review the final diff for generated-file drift or accidental project
  changes, and commit the verified implementation checkpoint.
