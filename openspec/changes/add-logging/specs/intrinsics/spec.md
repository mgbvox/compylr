## ADDED Requirements

### Requirement: An effectful operation may be gated by a level

The registry SHALL allow an effectful operation to declare a level that determines whether it is
performed. A backend SHALL emit the level test before any work the operation requires, and SHALL
take the level from the registry rather than from the operation's name.

#### Scenario: A gated operation records its level

- **WHEN** a gated operation is looked up in the registry
- **THEN** it declares the level that gates it

#### Scenario: The level reaches the backend as data

- **WHEN** a backend emits a gated operation
- **THEN** it selects the target level from the declared level, and an operation added later with a
  new name needs no backend change beyond a table entry

#### Scenario: An ungated effectful operation is unaffected

- **WHEN** an effectful operation declaring no level is emitted
- **THEN** it is performed unconditionally, as output is
