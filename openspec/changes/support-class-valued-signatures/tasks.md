## 1. Frontend Resolution and Located Boundary Diagnostics

- [x] 1.1 Write diagnostic and lowering tests for same-source and cross-source direct class-valued
  free-function parameters/returns, both source orders, a misspelled class name, known unsupported
  built-ins, nested class-valued containers at multiple depths, and explicit method boundary
  positions; assert error category and exact source location before changing lowering.
- [x] 1.2 Write failing lowering tests for every direct-instance-parameter use class before
  implementation: attribute reads, direct and method mutation, compatible forwarding, direct and
  aliased return, local/collection/attribute storage, rebinding, incompatible consumption, newly
  constructed returns, and owned results returned from another function; assert that each escape
  is located at the consuming use and target emission is never reached.
- [x] 1.3 Add dedicated machine-readable unresolved-class-annotation and
  borrowed-instance-escape categories; use the former only for bare names that could resolve from
  another source, and report the latter once the class is known without adding it to the manager's
  deferred set.
- [x] 1.4 Resolve direct class annotations from the complete cross-source class table, then add a
  boundary-shape validator that accepts only direct top-level free-function instances and rejects
  nested or out-of-scope instance positions at their annotation spans before backend emission.
- [x] 1.5 Add an exhaustive pre-emission ownership-use validator that permits reads, mutation, and
  compatible borrowed forwarding; rejects return, alias/storage, rebinding, and owned consumption;
  and accepts only constructor- or owned-call-derived class results as owned returns.
- [x] 1.6 Run the focused diagnostics, frontend, lowering, unit-verifier, and host-Python tests and
  commit the frontend checkpoint.

## 2. Decorator Deferral Without Silent Acceptance

- [x] 2.1 Write Python manager tests showing that a class-annotated function may be marked before or
  after its class, a final typo fails at whole-project build with its location, `complex` and
  malformed annotations still fail while marking, and nested instance containers or borrowed
  ownership escapes fail at whole-project validation before any build pipeline writes Rust.
- [x] 2.2 Replace the manager's single deferred code with an explicit immutable set containing only
  undetermined bindings and unresolved class annotations, leaving every other validation error
  immediate and preserving the no-interpreted-fallback behavior.
- [x] 2.3 Run the focused Python manager, programmatic compilation, and host exception-category tests
  and commit the decorator checkpoint.

## 3. Borrowed Instance ABI in Generated Rust

- [ ] 3.1 Write backend tests first for a read-only instance parameter, direct attribute mutation,
  nested place mutation, a transitively mutating method call, shared and mutable forwarding across
  forward/chained/mutually recursive generated calls, and newly owned returns; assert compilable
  emitted form, executed state semantics, and the absence of an inner-instance clone.
- [ ] 3.2 Introduce one whole-unit instance-parameter access analysis that seeds shared/directly
  mutable modes, reuses the existing place-root and transitive method-mutation logic, and propagates
  mutable requirements across free-function call edges to a fixpoint.
- [ ] 3.3 Emit direct instance parameters as `&T` or `&mut T` from that analysis and adapt generated
  call arguments to the callee's borrowed ABI; rely on validated non-escaping input and never clone
  a borrowed parameter into an owned result or storage location.
- [ ] 3.4 Run backend unit, emission-quality, conformance, generated-crate compile, and execution
  tests and commit the borrowed-ABI checkpoint.

## 4. Stable Python Wrapper Conversion

- [ ] 4.1 Write bridge generation tests first for deterministic class-name-to-wrapper lookup,
  shared `PyRef`, mutable `PyRefMut`, compatible forwarding, newly owned class-valued result
  wrapping, missing wrapper-map entries, no borrowed-inner clone, and unchanged scalar/collection
  signatures.
- [ ] 4.2 Build the stable wrapper map once from the unit's deterministic class order and route
  free-function boundary parameter, call-argument, and return spelling through pair-specific
  helpers rather than the backend's generic owned `rust_ty` spelling.
- [ ] 4.3 Pass `PyRef`/`PyRefMut` inner values to generated functions by shared/mutable borrow and
  wrap successful newly owned instance results in the mapped `#[pyclass]` wrapper before returning
  them; keep existing runtime-error-to-Python-exception conversion unchanged and provide no clone
  fallback for invalid ownership escapes.
- [ ] 4.4 Add extension-level tests showing `read(t: Tally)` observes current state, a free function
  that mutates directly, through a mutating method, or through compatible forwarding changes the
  same Python object, and `build(...) -> Tally` returns a newly owned exposed class with independent
  persistent state across calls.
- [ ] 4.5 Run the focused bridge, build-pipeline, PyO3 boundary, and end-to-end tests and commit the
  wrapper-conversion checkpoint.

## 5. Differential Corpus and Documentation

- [ ] 5.1 On top of the differential-fixture change, verify or add only the accepted
  `class_valued_signatures.py` fixture and its shared driver, extending them as needed to exercise
  direct reads, persistent mutation, compatible forwarding, and newly owned returned instances
  without copying unrelated harness code.
- [ ] 5.2 Add rejected-corpus fixtures for direct and aliased borrowed returns, collection/attribute
  storage, rebinding, and other owned consumption, each asserting the stable located
  borrowed-instance-escape diagnostic; do not add allowances to the inverted rejection guard.
- [ ] 5.3 Remove `BOUNDARY_EXCLUDED`, `test_the_exclusion_stays_one_fixture_wide`, and the fixture
  header's exclusion note so the boundary tier derives and exercises the complete accepted corpus.
- [ ] 5.4 Remove the temporary class-signature narrowing from the differential change's `notes.md`
  while preserving its unrelated measurements and findings.
- [ ] 5.5 Update README prose for borrow-only direct parameters, newly owned class-valued returns,
  the borrowed-escape diagnostic, and the explicit nested-boundary limitation; regenerate the
  subset matrix with `scripts/update_subset.py`, and run its `--check` mode rather than editing
  generated tables by hand.
- [ ] 5.6 Run both differential tiers, the inverted rejection guard, and fixture/driver completeness
  checks; confirm the accepted class-valued fixture agrees with CPython through the real bridge and
  every ownership escape fails before emission, then commit the corpus and documentation checkpoint.

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
