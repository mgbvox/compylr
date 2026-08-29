## ADDED Requirements

### Requirement: Namespaced intrinsic expression form

The IR SHALL support an expression form representing an operation drawn from a named module,
carrying the module name, the operation name, its argument expressions in order, and — for an
operation whose result is undefined on some inputs — a checking mode. The form SHALL be distinct
from [`Expr::Call`](../../../../../crates/compylr-ir/src/ir.rs#L605), for the reason
[`Expr::Len`](../../../../../crates/compylr-ir/src/ir.rs#L575) and
[`Expr::Range`](../../../../../crates/compylr-ir/src/ir.rs#L596) are: a call is resolved against
the unit, and an operation whose meaning depended on what else was compiled would not have a fixed
meaning.

#### Scenario: An operation from a supported module is an intrinsic expression

- **GIVEN** a lowered unit whose body applies an operation from a supported module to arguments
- **WHEN** the unit's IR is inspected
- **THEN** the operation is represented as an intrinsic expression
- **AND** it carries the module, the operation, and the arguments in order

#### Scenario: An intrinsic is not a call

- **GIVEN** an intrinsic expression and a call expression naming the same text
- **WHEN** the unit is validated
- **THEN** the two are distinct forms
- **AND** validation resolves only the call against the unit

#### Scenario: A constant is an intrinsic with no arguments

- **GIVEN** a lowered unit whose body names a module constant such as a mathematical constant
- **WHEN** the unit's IR is inspected
- **THEN** the constant is represented as an intrinsic expression carrying an empty argument list

#### Scenario: An intrinsic survives the artifact

- **GIVEN** a unit containing an intrinsic expression
- **WHEN** the unit is written to an artifact and read back
- **THEN** the recovered unit contains the same module, operation, arguments, and checking mode

#### Scenario Outline: The intrinsic form contributes to the fingerprint

- **GIVEN** two units differing only in the <field> named by an intrinsic
- **WHEN** their fingerprints are compared
- **THEN** the fingerprints differ, so a program that changed does not reuse a cached build

**Examples:**

| field         |
| ------------- |
| module        |
| operation     |
| checking mode |

#### Scenario: The form names no target language

- **GIVEN** an intrinsic expression in a lowered unit
- **WHEN** the expression is inspected
- **THEN** it carries a module and operation described by meaning
- **BUT** it carries no target-language spelling

### Requirement: The artifact format advances for the intrinsic form

The on-disk artifact version SHALL advance from 4 to 5, and an artifact written by an earlier
version SHALL be refused with an explanation rather than deserialized into a unit missing the added
information. See [`ARTIFACT_VERSION`](../../../../../crates/compylr-ir/src/ir.rs#L58).

#### Scenario: An older artifact is refused

- **GIVEN** an artifact written before the intrinsic form existed
- **WHEN** the artifact is loaded
- **THEN** loading fails with a message naming the version mismatch
- **AND** the project rebuilds

#### Scenario: The rebuild is automatic

- **GIVEN** a project built by an earlier compylr version
- **WHEN** the project is built again after upgrading
- **THEN** the recorded compylr version marks the build state stale
- **AND** the project rebuilds without the user taking any action
