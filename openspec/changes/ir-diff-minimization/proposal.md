## Why

To achieve a truly universal Intermediate Representation (IR), we must minimize divergence when representing equivalent operations across different source languages (e.g., Python vs TypeScript). If existing language frontends diverge structurally on identical concepts, we lose the value of a shared middle-end and optimization pipeline. By strictly enforcing that backend-agnostic frontends emit equivalent structural shapes for identical operations (while explicitly preserving necessary semantic checking modes), we maximize shared paths and minimize the N x M cost of language scaling.

## What Changes

- Introduce an IR Canonicalization/Normalization Pass that strips superficial differences (like variable naming, order of independent declarations, or commutative operand ordering) from the IR to prepare it for comparison.
- Introduce a Semantic IR Difference Checker that measures the structural divergence `D(I_python, I_ts)` between two normalized IRs, while gracefully ignoring expected semantic mode differences (e.g. integer overflow behavior).
- Hook the Difference Checker into the CI and fixture corpus to iteratively track and minimize `D(I_python, I_ts)` across our existing language boundaries without breaking execution correctness.

## Capabilities

### New Capabilities
- `ir-diff-checker`: A semantic differencing engine that compares two IR units for structural and semantic divergence.

### Modified Capabilities
- `ir-optimization`: Adding a required IR canonicalization/normalization pass to standardize IR structures prior to diffing.
- `fixture-corpus`: Expanding the corpus requirements to enforce cross-language IR divergence tracking.

## Impact

- **Tooling:** New CLI or library tools to calculate and assert on the IR difference metric.
- **Testing:** The CI pipeline will now assert on the degree of IR divergence across language frontends, providing a forcing function to tighten the supported subsets into shared behaviors.
