## ADDED Requirements

### Requirement: A borrowed text argument does not pay a per-element conversion cost

Where a text parameter is borrowed, the per-element cost of crossing the host boundary SHALL fall
relative to the owned path, and the improvement SHALL be measured rather than asserted.

#### Scenario: The text conversion cost falls

- **WHEN** a function taking a borrowed text parameter is called with a large text value
- **THEN** the measured per-element conversion cost is lower than for the same function taking it
  owned

#### Scenario: The measurement is taken from a clean build

- **WHEN** the improvement is measured
- **THEN** the build directories are removed first, so the measurement is not taken against a
  previously cached build

#### Scenario: Forwarding between compiled functions does not clone

- **WHEN** one compiled function passes a borrowed collection to another that borrows it
- **THEN** no clone occurs at the call

#### Scenario: No program becomes slower

- **WHEN** the demo is run before and after this change
- **THEN** no algorithm's time regresses beyond measurement noise
