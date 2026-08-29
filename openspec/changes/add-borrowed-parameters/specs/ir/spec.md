## ADDED Requirements

### Requirement: A parameter carries a passing mode

A parameter SHALL carry a passing mode alongside its name and type, stating whether the value is
owned by the function or borrowed from the caller, and where borrowed, whether it may be mutated.
The mode SHALL survive the artifact and SHALL contribute to the fingerprint.

#### Scenario: The mode is part of a parameter

- **WHEN** a function's parameters are inspected
- **THEN** each carries a passing mode as well as a name and a type

#### Scenario: The mode survives the artifact

- **WHEN** a unit is written to an artifact and read back
- **THEN** every parameter's mode is recovered unchanged

#### Scenario: The mode contributes to the fingerprint

- **WHEN** two units differ only in one parameter's passing mode
- **THEN** their fingerprints differ, because the two produce different target signatures

#### Scenario: The mode names no target language

- **WHEN** a passing mode is inspected
- **THEN** it describes ownership by meaning and carries no target-language spelling

#### Scenario: The artifact version advances

- **WHEN** an artifact written before parameters carried a mode is loaded
- **THEN** loading fails with a version mismatch and the project rebuilds automatically
