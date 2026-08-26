## Context

As described in `proposal.md`, we are introducing a Go backend and a TypeScript frontend. To ensure the viability of our universal IR, we must ensure that different frontends emit structurally equivalent IR for equivalent source operations, while preserving source-language semantics in the operator modes.

## Goals / Non-Goals

**Goals:**
- Implement `compylr-frontend-typescript` to parse TS and emit `compylr` IR.
- Implement `compylr-backend-go` to consume IR and emit Go source.
- Implement an IR normalization pass that standardizes structural layout (e.g., ordering of independent locals, commutative operator ordering) without altering semantics.
- Implement a semantic IR diff tool that computes a divergence score `D` between two normalized IR units, ignoring semantic mode differences (e.g., integer overflow behavior).
- Enforce that `D(I_python, I_ts)` is minimized for the core feature set via the test suite.

**Non-Goals:**
- Perfectly zero divergence for complex multi-statement constructs where languages fundamentally disagree on control flow paradigms (e.g., `for...of` vs Python's snapshotting `for`). We want to *minimize* D, not break execution correctness trying to force it to 0.

## Decisions

### 1. The Normalization Pass
**Decision:** We will introduce a new `Canonicalize` pass in `compylr-passes` (or `compylr-ir` if passes aren't a separate crate yet). This pass will sort commutative operands (e.g., `b + a` -> `a + b` based on variable IDs), sort independent variable declarations, and normalize block structures.
**Alternatives Considered:** Diffing directly without normalization. This was rejected because trivial structural changes (like the order variables are declared) would artificially inflate the divergence score.

### 2. The Semantic Diff Metric (D)
**Decision:** `D` will be calculated by a recursive structural tree-walk over the normalized IR. It will compute a Levenshtein-like distance on node types and structural edges. Crucially, when comparing operator nodes (like `BinOp::Div`), the differ will *ignore* differences in the checking modes (e.g., `RoundingMode`) because those are required to preserve source language semantics.
**Alternatives Considered:** A simple string/JSON diff of the IR. Rejected because it would flag expected semantic mode differences as divergence, which contradicts the core design principles of `compylr`.

### 3. TypeScript Parsing
**Decision:** We will use `swc_ecma_parser` (from the SWC project) as the parser for `compylr-frontend-typescript`. It is the standard, high-performance Rust TS parser.
**Alternatives Considered:** Using tree-sitter. Rejected because SWC provides a more ergonomic, strongly-typed AST for Rust.

### 4. CI Integration
**Decision:** We will add a new test tier to `tests/conformance.rs` (or create `tests/divergence.rs`) that runs both frontends on equivalent `fixtures/accepted/` files, normalizes the IR, calculates `D`, and fails if `D > threshold`.

## Risks / Trade-offs

- **Risk:** The normalization pass might be too aggressive and mask real structural bugs. → *Mitigation:* Ensure normalization rules are strictly commutative and identity-preserving, validated by execution tests.
- **Risk:** Forcing identical IR structures might cause the TS frontend to adopt non-idiomatic TS behaviors. → *Mitigation:* We will rely on execution tests to prove that TS semantics are preserved. If identical IR breaks TS execution, we accept the higher divergence score `D` for that construct.
