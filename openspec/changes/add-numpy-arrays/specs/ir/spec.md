## ADDED Requirements

### Requirement: Array type

The type model SHALL include an array type carrying its element storage and its rank. The type
SHALL name no target language and no source library, describing storage by meaning rather than by
any library's dtype spelling.

#### Scenario: An array type carries storage and rank

- **WHEN** an array type is inspected
- **THEN** it reports its element storage and its rank

#### Scenario: Arrays of different rank are unequal

- **WHEN** two array types over the same storage with different ranks are compared
- **THEN** they are unequal, and each renders distinguishably

#### Scenario: An array may not key a mapping

- **WHEN** an array type is tested for whether it may be a mapping key or set element
- **THEN** it may not, as collections may not

#### Scenario: An array is not trivially copyable

- **WHEN** an array type is tested for trivial copyability
- **THEN** it is not, so no backend treats reading one as a free copy

#### Scenario: The type survives the artifact

- **WHEN** a unit using array types is written and read back
- **THEN** every array type's storage and rank are recovered unchanged

#### Scenario: The artifact version advances

- **WHEN** an artifact written before the array type existed is loaded
- **THEN** loading fails with a version mismatch and the project rebuilds automatically
