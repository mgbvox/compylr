## ADDED Requirements

### Requirement: Records are compared by level, logger, and message, not by formatted line

A fixture that records SHALL have its records captured structurally from both the interpreted and
the compiled run, and compared by level, logger name, message, and order. The comparison SHALL NOT
compare formatted output lines, which carry timestamps and handler-dependent formatting. The logger
name SHALL be part of the comparison, because an attribution divergence is otherwise invisible to
it.

#### Scenario: Records are captured from both tiers

- **GIVEN** a driver exercising a fixture function that records
- **WHEN** the driver runs
- **THEN** records from the interpreted run and from the compiled run are captured and compared by
  level, logger name, message, and order

#### Scenario: Timestamps do not enter the comparison

- **GIVEN** records captured from both tiers
- **WHEN** they are compared
- **THEN** the comparison covers level, logger name, message, and order only
- **AND** the suite does not fail on the time at which it ran

#### Scenario: An attribution divergence fails the suite

- **GIVEN** a compiled run attributing a record to a different logger than the interpreted run
- **WHEN** the suite compares the two
- **THEN** the suite fails naming the differing logger
- **BUT** it does not pass because the level and message matched

#### Scenario: A suppressed record is absent from both

- **GIVEN** an effective level that suppresses a record
- **WHEN** both runs are compared
- **THEN** neither produces it
- **AND** the comparison confirms the absence rather than ignoring it

#### Scenario: Every supported level is exercised

- **GIVEN** the accepted corpus
- **WHEN** it is checked
- **THEN** a fixture records at every supported level
- **AND** a level with no fixture fails the suite
