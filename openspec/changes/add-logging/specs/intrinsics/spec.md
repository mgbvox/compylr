## ADDED Requirements

### Requirement: An effectful operation may be gated by a level

The registry SHALL allow an effectful operation to declare a level that determines whether it is
performed. A backend SHALL emit the level test before any work the operation requires, and SHALL
take the level from the registry rather than from the operation's name.

#### Scenario: A gated operation records its level

- **GIVEN** a gated operation in the registry
- **WHEN** it is looked up
- **THEN** it declares the level that gates it

#### Scenario: The level reaches the backend as data

- **GIVEN** a backend emitting a gated operation
- **WHEN** the target level is selected
- **THEN** it is selected from the declared level
- **AND** an operation added later with a new name needs no backend change beyond a table entry

#### Scenario: An ungated effectful operation is unaffected

- **GIVEN** an effectful operation declaring no level
- **WHEN** it is emitted
- **THEN** it is performed unconditionally, as output is

#### Scenario: Adding a gated module needs no IR change

- **GIVEN** a second module of gated effectful operations
- **WHEN** it is added to the registry
- **THEN** it reuses the existing effectful statement form
- **AND** the artifact version does not move
