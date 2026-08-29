## ADDED Requirements

### Requirement: Effectful intrinsic statement form

The IR SHALL support a statement form carrying an intrinsic operation that produces no value and is
performed for its effect on the outside world, together with its argument expressions in order and
the rendering convention in force. The form SHALL be distinct from the existing effect statement,
whose meaning is a unit-returning method call on a value the program owns.

#### Scenario: Effectful intrinsic statement

- **WHEN** a program performs an output operation
- **THEN** the IR body contains a statement carrying the module, the operation, its arguments in
  order, and the rendering convention

#### Scenario: Distinct from a method effect

- **WHEN** an effectful intrinsic statement and the existing effect statement are compared
- **THEN** they are distinct forms, and the existing form still carries only a method call

#### Scenario: An effectful operation has no result

- **WHEN** an effectful intrinsic is looked up in the registry
- **THEN** it declares no result type, and using it where a value is required is not representable

#### Scenario: An effect statement survives the artifact

- **WHEN** a unit containing an effectful intrinsic statement is written and read back
- **THEN** the recovered unit carries the same operation, arguments, and rendering convention

#### Scenario: The rendering convention contributes to the fingerprint

- **WHEN** two units differ only in the rendering convention carried by an output statement
- **THEN** their fingerprints differ, because the two produce different observable output
