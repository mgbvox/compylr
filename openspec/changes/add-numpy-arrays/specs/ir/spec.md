## ADDED Requirements

### Requirement: Array type

[`Ty`](../../../../../crates/compylr-ir/src/ir.rs#L103) SHALL include an array type carrying its
element storage and its rank. The type SHALL name no target language and no source library,
describing storage by meaning rather than by any library's dtype spelling.

#### Scenario: An array type carries storage and rank

- **GIVEN** an array type in a lowered unit
- **WHEN** it is inspected
- **THEN** it reports its element storage and its rank

#### Scenario: Arrays of different rank are unequal

- **GIVEN** two array types over the same storage with different ranks
- **WHEN** they are compared
- **THEN** they are unequal
- **AND** each renders distinguishably

#### Scenario: An array may not key a mapping

- **GIVEN** an array type
- **WHEN** it is tested for whether it may be a mapping key or set element
- **THEN** it may not, as collections may not

#### Scenario: An array is not trivially copyable

- **GIVEN** an array type
- **WHEN** it is tested for trivial copyability
- **THEN** it is not, so no backend treats reading one as a free copy

#### Scenario: The type names no source library

- **GIVEN** an array type in a lowered unit
- **WHEN** it is inspected
- **THEN** its storage is described by meaning
- **BUT** it carries no library's dtype spelling and no target-language view type

#### Scenario: The type survives the artifact

- **GIVEN** a unit using array types
- **WHEN** it is written to an artifact and read back
- **THEN** every array type's storage and rank are recovered unchanged

#### Scenario Outline: Storage and rank contribute to the fingerprint

- **GIVEN** two units differing only in an array type's <field>
- **WHEN** their fingerprints are compared
- **THEN** the fingerprints differ, because the two produce different target signatures

**Examples:**

| field   |
| ------- |
| storage |
| rank    |

### Requirement: The artifact format advances for the array type

The on-disk artifact version SHALL advance, and an artifact written before the array type existed
SHALL be refused rather than read as a unit missing it. See
[`ARTIFACT_VERSION`](../../../../../crates/compylr-ir/src/ir.rs#L58).

#### Scenario: An older artifact is refused

- **GIVEN** an artifact written before the array type existed
- **WHEN** the artifact is loaded
- **THEN** loading fails with a version mismatch
- **AND** the project rebuilds automatically
