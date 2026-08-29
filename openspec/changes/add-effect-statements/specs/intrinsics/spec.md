## ADDED Requirements

### Requirement: An operation may be effectful

The registry SHALL record whether an operation produces a result or is performed for its effect. An
effectful operation SHALL declare no result type, SHALL be usable only as a statement, and SHALL be
rejected with a located diagnostic where a value is required. The bare-expression rejection in
[`bare_expression_error`](../../../../../crates/compylr-frontend-python/src/lower.rs#L1677) SHALL
consult the registry rather than testing the expression's shape.

#### Scenario: An effectful operation is a statement

- **GIVEN** a source whose body contains a statement consisting of an effectful intrinsic call
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering succeeds
- **AND** it produces an effect statement

#### Scenario: An effectful operation in a value position is refused

- **GIVEN** a source whose body binds a local to an effectful intrinsic call
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic reporting that the operation produces no value

#### Scenario: A result-producing operation as a statement is still refused

- **GIVEN** a source whose body contains a statement consisting of an intrinsic that declares a
  result
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with the existing diagnostic for a discarded value, because the value is
  computed and thrown away

#### Scenario: Argument types are still checked

- **GIVEN** a source applying an effectful intrinsic to an argument whose type it does not accept
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming the operation and the type

#### Scenario: The carve-out is not a shape test

- **GIVEN** an effectful operation added to the registry after this change
- **WHEN** it appears as a bare statement
- **THEN** it is accepted without any edit to the lowering condition
