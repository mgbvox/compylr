## Purpose

Consumes the `compylr` Intermediate Representation and generates idiomatic, semantics-preserving Go source code.

## ADDED Requirements

### Requirement: Go Code Emission
The backend SHALL accept a `compylr` IR unit and emit Go source code that strictly preserves the operational semantics defined in the IR modes.

#### Scenario: Code emission
- **WHEN** provided with a valid IR unit
- **THEN** it produces Go code that correctly implements the operations regardless of which frontend produced the IR.
