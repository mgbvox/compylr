## ADDED Requirements

### Requirement: Cross-Language Divergence Enforcement
The fixture runner SHALL calculate the IR divergence score `D(I_langA, I_langB)` for multi-language fixtures and assert that the score does not exceed an allowed threshold or zero.

#### Scenario: Running multi-language fixtures
- **WHEN** the test suite executes a fixture present in both Python and TypeScript corpora
- **THEN** it asserts that `D(I_python, I_ts) == 0` (or within allowed threshold), failing the suite if the structural divergence constraint is violated.
