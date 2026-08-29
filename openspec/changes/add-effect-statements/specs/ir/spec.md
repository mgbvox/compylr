## ADDED Requirements

### Requirement: Effectful intrinsic statement form

The IR SHALL support a statement form carrying an intrinsic operation that produces no value and is
performed for its effect on the outside world, together with its argument expressions in order and
the rendering convention in force. The form SHALL be distinct from
[`Stmt::Effect`](../../../../../crates/compylr-ir/src/ir.rs#L761), whose meaning is a unit-returning
method call on a value the program owns.

#### Scenario: An output operation is an effectful intrinsic statement

- **GIVEN** a lowered unit whose body performs an output operation
- **WHEN** the unit's IR is inspected
- **THEN** the body contains a statement carrying the module, the operation, its arguments in
  order, and the rendering convention

#### Scenario: Distinct from a method effect

- **GIVEN** a unit containing both an effectful intrinsic statement and the existing effect
  statement
- **WHEN** the two are compared
- **THEN** they are distinct forms
- **AND** the existing form still carries only a method call

#### Scenario: An effectful operation has no result

- **GIVEN** an effectful operation in the registry
- **WHEN** it is looked up
- **THEN** it declares no result type
- **AND** using it where a value is required is not representable

#### Scenario: An effect statement survives the artifact

- **GIVEN** a unit containing an effectful intrinsic statement
- **WHEN** the unit is written to an artifact and read back
- **THEN** the recovered unit carries the same operation, arguments, and rendering convention

#### Scenario: The rendering convention contributes to the fingerprint

- **GIVEN** two units differing only in the rendering convention carried by an output statement
- **WHEN** their fingerprints are compared
- **THEN** the fingerprints differ, because the two produce different observable output

#### Scenario: The form names no target language

- **GIVEN** an effectful intrinsic statement in a lowered unit
- **WHEN** the statement is inspected
- **THEN** the convention names a rendering stance by meaning
- **BUT** it carries no target-language formatter

### Requirement: The artifact format advances for the effect statement form

The on-disk artifact version SHALL advance, and an artifact written by an earlier version SHALL be
refused with an explanation rather than deserialized into a unit missing the added information. See
[`ARTIFACT_VERSION`](../../../../../crates/compylr-ir/src/ir.rs#L58).

#### Scenario: An older artifact is refused

- **GIVEN** an artifact written before the effect statement form existed
- **WHEN** the artifact is loaded
- **THEN** loading fails with a message naming the version mismatch
- **AND** the project rebuilds

#### Scenario: A program that prints nothing is unaffected

- **GIVEN** a program containing no output statement
- **WHEN** it is emitted before and after this change
- **THEN** the emitted files are byte-identical
