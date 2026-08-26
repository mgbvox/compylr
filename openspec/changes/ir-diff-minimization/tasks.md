## 1. IR Canonicalization Pass

- [ ] 1.1 Scaffold the `Canonicalize` pass within the `compylr-passes` (or relevant IR module) infrastructure.
- [ ] 1.2 Implement block structure normalization (ordering of independent blocks/variables where semantically neutral).
- [ ] 1.3 Implement operand sorting for commutative operations (e.g. `+`, `*`) using deterministic sorting (e.g. by variable ID or node hash).
- [ ] 1.4 Write unit tests asserting that structurally different but semantically identical Python IR nodes are canonicalized to the exact same shape.

## 2. Semantic IR Diff Checker

- [ ] 2.1 Implement the core structural diffing algorithm (`semantic_diff`) that takes two IR units and computes a structural distance score `D`.
- [ ] 2.2 Configure `semantic_diff` to explicitly ignore differences in semantic mode parameters (like `RoundingMode` or `OverflowMode`).
- [ ] 2.3 Write tests validating that identical IR with different checking modes yields `D == 0`.
- [ ] 2.4 Expose the diff checker via the `compylr-cli` or test utilities for programmatic use.

## 3. TypeScript Frontend

- [ ] 3.1 Scaffold `compylr-frontend-typescript` crate and add `swc_ecma_parser` dependency.
- [ ] 3.2 Implement parsing of TypeScript source text into the SWC AST.
- [ ] 3.3 Implement the lowering from SWC AST to `compylr` IR, ensuring semantic modes correctly reflect TS behavior (e.g., f64 math, array bounds checking modes).
- [ ] 3.4 Wire the TS frontend into the `compylr-registry`.

## 4. Go Backend

- [ ] 4.1 Scaffold `compylr-backend-go` crate.
- [ ] 4.2 Implement the `Backend` trait for Go, translating `compylr` IR modes accurately to Go code (handling overflows, arrays, etc.).
- [ ] 4.3 Implement Go source emission/formatting logic.
- [ ] 4.4 Wire the Go backend into the `compylr-registry`.

## 5. Fixture Assertions & CI

- [ ] 5.1 Create a shared `fixtures/accepted/` sub-corpus or metadata format that links identical operations across Python and TypeScript.
- [ ] 5.2 Add a new conformance test tier that asserts `D(canonicalize(I_python), canonicalize(I_ts))` is below the allowed threshold for paired fixtures.
- [ ] 5.3 Validate that `cargo test` successfully enforces IR alignment without breaking execution correctness.
