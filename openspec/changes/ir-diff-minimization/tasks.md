## 1. IR Canonicalization Pass

- [ ] 1.1 Scaffold the `Canonicalize` pass within `compylr-core/src/pass.rs`.
- [ ] 1.2 Implement block structure normalization (ordering of independent blocks/variables where semantically neutral).
- [ ] 1.3 Implement operand sorting for commutative operations (e.g. `+`, `*`) using deterministic sorting (e.g. by variable ID or node hash).
- [ ] 1.4 Write unit tests asserting that structurally different but semantically identical Python IR nodes are canonicalized to the exact same shape.

## 2. Semantic IR Diff Checker

- [ ] 2.1 Implement the core structural diffing algorithm (`semantic_diff`) that takes two IR units and computes a structural distance score `D`.
- [ ] 2.2 Configure `semantic_diff` to explicitly ignore differences in semantic mode parameters (like `RoundingMode` or `OverflowMode`).
- [ ] 2.3 Write tests validating that identical IR with different checking modes yields `D == 0`.
- [ ] 2.4 Expose the diff checker via the `compylr-cli` or test utilities for programmatic use.

## 3. Fixture Assertions & CI

- [ ] 3.1 Create a shared `fixtures/accepted/` sub-corpus or metadata format that links identical operations across Python and TypeScript.
- [ ] 3.2 Add a new conformance test tier that asserts `D(canonicalize(I_python), canonicalize(I_ts))` is below the allowed threshold for paired fixtures.
- [ ] 3.3 Validate that `cargo test` successfully enforces IR alignment without breaking execution correctness.
