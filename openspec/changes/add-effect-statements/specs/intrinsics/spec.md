## ADDED Requirements

### Requirement: An operation may be effectful

The registry SHALL record whether an operation produces a result or is performed for its effect. An
effectful operation SHALL declare no result type, SHALL be usable only as a statement, and SHALL be
rejected with a located diagnostic where a value is required.

#### Scenario: An effectful operation is a statement

- **WHEN** lowering a statement consisting of an effectful intrinsic call
- **THEN** lowering succeeds and produces an effect statement

#### Scenario: An effectful operation in a value position is refused

- **WHEN** lowering a binding whose initializer is an effectful intrinsic call
- **THEN** lowering fails with a located diagnostic reporting that the operation produces no value

#### Scenario: A result-producing operation as a statement is still refused

- **WHEN** lowering a statement consisting of an intrinsic that declares a result
- **THEN** lowering fails with the existing diagnostic for a discarded value, because the value is
  computed and thrown away

#### Scenario: Argument types are still checked

- **WHEN** lowering an effectful intrinsic applied to an argument whose type it does not accept
- **THEN** lowering fails with a located diagnostic naming the operation and the type
