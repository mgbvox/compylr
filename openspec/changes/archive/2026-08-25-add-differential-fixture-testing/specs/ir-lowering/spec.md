## MODIFIED Requirements

### Requirement: Diagnostics are located and non-fatal to the process

Lowering and unit validation MUST NOT panic on any input that the frontend parsed
successfully. Every diagnostic SHALL carry the source position of the offending construct and
render as a human-readable message naming both the problem and its location.

This SHALL hold for **arbitrary** Python, not only for programs written to exercise a rejection
rule. A curated rejection corpus demonstrates that each *known* refusal is located; it cannot
demonstrate that an unanticipated construct is refused rather than crashed on, because every program
in it was written by someone who already knew the answer. The property is therefore established
against Python that was not written for this compiler.

#### Scenario: Rejection does not panic

- **WHEN** lowering any source that violates the subset rules
- **THEN** lowering returns a failure result and the process continues running

#### Scenario: Diagnostic carries a position

- **WHEN** lowering fails on a construct at a known position in the source
- **THEN** the diagnostic carries that source position

#### Scenario: First violation is reported

- **WHEN** lowering a source containing more than one subset violation
- **THEN** lowering fails reporting the first violation in source order

#### Scenario: Python written for other purposes is refused rather than crashed on

- **WHEN** lowering a parsed program that was not written to exercise a subset rule
- **THEN** the outcome is a lowered unit or a failure carrying a source position, and never a panic
