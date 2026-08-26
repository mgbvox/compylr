## ADDED Requirements

### Requirement: IR Canonicalization Pass
The optimizer SHALL include a normalization pass that canonicalizes superficial differences (such as commutative operand ordering and decoupled local variable assignments) into a standard structural form.

#### Scenario: Running the canonicalizer
- **WHEN** the canonicalization pass runs over an IR unit
- **THEN** it outputs a normalized IR unit ready for semantic differencing.
