## ADDED Requirements

### Requirement: Namespaced intrinsic expression form

The IR SHALL support an expression form representing an operation drawn from a named module,
carrying the module name, the operation name, its argument expressions in order, and — for an
operation whose result is undefined on some inputs — a checking mode. The form SHALL be distinct
from a call to a named function, for the reason the length and range forms are: a call is resolved
against the unit, and an operation whose meaning depended on what else was compiled would not have
a fixed meaning.

#### Scenario: Intrinsic expression

- **WHEN** an operation from a supported module is applied to arguments
- **THEN** the IR represents it as an intrinsic expression carrying the module, the operation, and
  the arguments in order

#### Scenario: An intrinsic is not a call

- **WHEN** an intrinsic expression and a call expression naming the same text are compared
- **THEN** they are distinct forms, and unit validation resolves only the call

#### Scenario: A constant is an intrinsic with no arguments

- **WHEN** a module constant such as a mathematical constant appears in an expression
- **THEN** the IR represents it as an intrinsic expression carrying an empty argument list

#### Scenario: An intrinsic survives the artifact

- **WHEN** a unit containing an intrinsic expression is written to an artifact and read back
- **THEN** the recovered unit contains the same module, operation, arguments, and checking mode

#### Scenario: The intrinsic form contributes to the fingerprint

- **WHEN** two units differ only in the operation named by an intrinsic
- **THEN** their fingerprints differ, so a program that changed does not reuse a cached build

#### Scenario: The form names no target language

- **WHEN** an intrinsic expression is inspected
- **THEN** it carries a module and operation described by meaning, and no target-language spelling

### Requirement: The artifact format advances for the intrinsic form

The on-disk artifact version SHALL advance, and an artifact written by an earlier version SHALL be
refused with an explanation rather than deserialized into a unit missing the added information.

#### Scenario: An older artifact is refused

- **WHEN** an artifact written before the intrinsic form existed is loaded
- **THEN** loading fails with a message naming the version mismatch, and the project rebuilds

#### Scenario: The rebuild is automatic

- **WHEN** a project built by an earlier compylr version is built again after upgrading
- **THEN** the recorded compylr version marks the build state stale and the project rebuilds
  without the user taking any action
