## ADDED Requirements

### Requirement: Array annotations are lowered by rank and storage

Lowering SHALL accept the ranked array annotation, SHALL reject an unranked or unsupported-storage
annotation with a located diagnostic, and SHALL type indexing and shape from the declared rank.

#### Scenario: A ranked parameter annotation lowers

- **WHEN** lowering a parameter annotated as a ranked array of a supported storage
- **THEN** lowering succeeds and the parameter carries the array type

#### Scenario: An unranked annotation names the ranked form

- **WHEN** lowering an unranked array annotation
- **THEN** lowering fails with a located diagnostic naming the ranked spelling

#### Scenario: Shape is typed from the rank

- **WHEN** lowering a read of an array's shape
- **THEN** its type is a tuple of integers whose length is the declared rank

#### Scenario: Length yields the first extent

- **WHEN** lowering a length applied to an array
- **THEN** it yields the extent of the first dimension, as it does for the outer sequence of a
  nested sequence

#### Scenario: An array local must be an alias of a parameter

- **WHEN** lowering a binding whose initializer is an array parameter
- **THEN** the binding is an alias governed by the existing alias rules, and does not copy

#### Scenario: An array return type is refused

- **WHEN** lowering a function declaring an array return type
- **THEN** lowering fails with a located diagnostic naming array creation as not yet supported

#### Scenario: Mutating an array parameter is accepted

- **WHEN** lowering an element assignment into an array parameter
- **THEN** lowering succeeds, in deliberate contrast to the same assignment into a sequence
  parameter, which stays refused

#### Scenario: The collection refusal explains the contrast

- **WHEN** lowering an element assignment into a sequence parameter
- **THEN** the existing diagnostic is produced and names the contrast with an array parameter, so
  the user learns which representation permits it
