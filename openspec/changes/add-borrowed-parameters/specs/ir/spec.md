## ADDED Requirements

### Requirement: A parameter carries a passing mode

A parameter SHALL carry a passing mode alongside its name and type, stating whether the value is
owned by the function or borrowed from the caller, and where borrowed, whether it may be mutated.
Owned SHALL be the mode a parameter constructed without a decision receives. The mode SHALL survive
the artifact and SHALL contribute to [`Unit::fingerprint`](../../../../../crates/compylr-ir/src/ir.rs#L1299).

#### Scenario: The mode is part of a parameter

- **GIVEN** a lowered unit
- **WHEN** a function's parameters are inspected
- **THEN** each carries a passing mode as well as a name and a type

#### Scenario: A parameter constructed without a decision is owned

- **GIVEN** a parameter built without the analysis having run
- **WHEN** its mode is inspected
- **THEN** it is owned, which is correct for every program

#### Scenario: The mode survives the artifact

- **GIVEN** a unit whose parameters carry modes
- **WHEN** it is written to an artifact and read back
- **THEN** every parameter's mode is recovered unchanged

#### Scenario: The mode contributes to the fingerprint

- **GIVEN** two units differing only in one parameter's passing mode
- **WHEN** their fingerprints are compared
- **THEN** the fingerprints differ, because the two produce different target signatures

#### Scenario: The mode names no target language

- **GIVEN** a passing mode in a lowered unit
- **WHEN** it is inspected
- **THEN** it describes ownership by meaning
- **BUT** it carries no target-language spelling

### Requirement: The artifact format advances for the passing mode

The on-disk artifact version SHALL advance, and an artifact written before parameters carried a
mode SHALL be refused rather than read as though every mode were absent. See
[`ARTIFACT_VERSION`](../../../../../crates/compylr-ir/src/ir.rs#L58).

#### Scenario: An older artifact is refused

- **GIVEN** an artifact written before parameters carried a mode
- **WHEN** the artifact is loaded
- **THEN** loading fails with a version mismatch
- **AND** the project rebuilds automatically
