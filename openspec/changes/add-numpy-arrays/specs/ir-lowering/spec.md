## ADDED Requirements

### Requirement: Array annotations are lowered by rank and storage

Lowering SHALL accept the ranked array annotation, SHALL reject an unranked or unsupported-storage
annotation with a located diagnostic, and SHALL type indexing and shape from the declared rank.

#### Scenario: A ranked parameter annotation lowers

- **GIVEN** a parameter annotated as a ranked array of a supported storage
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering succeeds
- **AND** the parameter carries the array type

#### Scenario: An unranked annotation names the ranked form

- **GIVEN** an unranked array annotation
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming the ranked spelling

#### Scenario: Shape is typed from the rank

- **GIVEN** a read of an array's shape
- **WHEN** the expression is lowered
- **THEN** its type is a tuple of integers whose length is the declared rank

#### Scenario: Length yields the first extent

- **GIVEN** a length applied to an array
- **WHEN** the expression is lowered
- **THEN** it yields the extent of the first dimension, as it does for the outer sequence of a
  nested sequence

#### Scenario: An array local must be an alias of a parameter

- **GIVEN** a binding whose initializer is an array parameter
- **WHEN** the statement is lowered
- **THEN** the binding is an alias governed by the existing alias rules
- **AND** it does not copy

#### Scenario: An array return type is refused

- **GIVEN** a function declaring an array return type
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming array creation as not yet supported

### Requirement: The mutation rules of arrays and collections are distinguished at the diagnostic

Lowering SHALL accept an element assignment into an array parameter and SHALL continue to refuse
one into a sequence parameter. Because the two rules differ for a representational reason, the
existing refusal SHALL name the contrast rather than leaving the user to discover it.

#### Scenario: Mutating an array parameter is accepted

- **GIVEN** a function whose body contains

  ```python
  values[i] = values[i] * factor
  ```

  where `values` is an array parameter

- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering succeeds, in deliberate contrast to the same assignment into a sequence
  parameter

#### Scenario: Mutating a sequence parameter stays refused

- **GIVEN** the same body where `values` is a sequence parameter
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering fails with the existing diagnostic, unchanged in what it refuses

#### Scenario: The collection refusal explains the contrast

- **GIVEN** an element assignment into a sequence parameter
- **WHEN** the diagnostic is produced
- **THEN** it names the contrast with an array parameter, so the user learns which representation
  permits it
- **AND** it still explains that a collection parameter is a copy

### Requirement: Whole-array arithmetic is refused rather than reinterpreted

An arithmetic operator applied to two arrays SHALL be refused with a located diagnostic naming
vectorized operations as unsupported. The scalar operators the IR carries SHALL NOT be reused to
mean an element-wise operation over arrays.

#### Scenario: Adding two arrays is refused

- **GIVEN** a function whose body adds two array parameters
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming vectorized operations as unsupported
- **BUT** the scalar addition operator is not reused to mean an element-wise one

#### Scenario: Element-wise work written as a loop is accepted

- **GIVEN** a function that indexes two array parameters inside a loop and combines the elements
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering succeeds, because each operation is over scalars
