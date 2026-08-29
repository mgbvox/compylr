## ADDED Requirements

### Requirement: Records are compared by level and message, not by formatted line

A fixture that records SHALL have its records captured structurally from both the interpreted and
the compiled run, and compared by level, message, and order. The comparison SHALL NOT compare
formatted output lines, which carry timestamps and handler-dependent formatting.

#### Scenario: Records are captured from both tiers

- **WHEN** a driver exercises a fixture function that records
- **THEN** records from the interpreted run and from the compiled run are captured and compared by
  level, message, and order

#### Scenario: Timestamps do not enter the comparison

- **WHEN** records are compared
- **THEN** the comparison covers level, message, and order only, so the suite does not fail on the
  time at which it ran

#### Scenario: A suppressed record is absent from both

- **WHEN** the effective level suppresses a record
- **THEN** neither run produces it, and the comparison confirms the absence rather than ignoring it

#### Scenario: Every supported level is exercised

- **WHEN** the corpus is checked
- **THEN** an accepted fixture records at every supported level, and a level with no fixture fails
  the suite
