## Context

As described in `proposal.md`, we need to track and minimize the divergence between our existing Python and TypeScript frontends to ensure the viability of our universal IR. We must ensure that different frontends emit structurally equivalent IR for equivalent source operations, while preserving source-language semantics in the operator modes.

## Goals / Non-Goals

**Goals:**
- Implement an IR normalization pass that standardizes structural layout (e.g., ordering of independent locals, commutative operator ordering) without altering semantics.
- Implement a semantic IR diff tool that computes a divergence score `D` between two normalized IR units, ignoring semantic mode differences (e.g., integer overflow behavior).
- Enforce that `D(I_python, I_ts)` is minimized for the core feature set via the test suite.

**Non-Goals:**
- Perfectly zero divergence for complex multi-statement constructs where languages fundamentally disagree on control flow paradigms (e.g., `for...of` vs Python's snapshotting `for`). We want to *minimize* D, not break execution correctness trying to force it to 0.

## Decisions

### 1. The Normalization Pass
**Decision:** We will introduce a new `Canonicalize` pass in `compylr-core/src/pass.rs`. This pass will sort commutative operands (e.g., `b + a` -> `a + b` based on variable IDs), sort independent variable declarations, and normalize block structures.
**Alternatives Considered:** Diffing directly without normalization. This was rejected because trivial structural changes (like the order variables are declared) would artificially inflate the divergence score.

### 2. The Semantic Diff Metric (D)
**Decision:** `D` will be calculated by a recursive structural tree-walk over the normalized IR. It will compute a Levenshtein-like distance on node types and structural edges. Crucially, when comparing operator nodes (like `BinOp::Div`), the differ will *ignore* differences in the checking modes (e.g., `RoundingMode`) because those are required to preserve source language semantics.
**Alternatives Considered:** A simple string/JSON diff of the IR. Rejected because it would flag expected semantic mode differences as divergence, which contradicts the core design principles of `compylr`.

### 3. CI Integration
**Decision:** We will add a new test tier to `tests/conformance.rs` (or create `tests/divergence.rs`) that runs both frontends on equivalent `fixtures/accepted/` files, normalizes the IR, calculates `D`, and fails if `D > threshold`.

## Risks / Trade-offs

- **Risk:** The normalization pass might be too aggressive and mask real structural bugs. → *Mitigation:* Ensure normalization rules are strictly commutative and identity-preserving, validated by execution tests.
- **Risk:** Forcing identical IR structures might cause a frontend to adopt non-idiomatic behaviors. → *Mitigation:* We will rely on execution tests to prove that semantics are preserved. If identical IR breaks execution, we accept the higher divergence score `D` for that construct.
