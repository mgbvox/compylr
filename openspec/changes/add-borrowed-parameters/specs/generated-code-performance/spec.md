## ADDED Requirements

### Requirement: A borrowed text argument does not pay a per-element conversion cost

Where a text parameter is borrowed, the per-element cost of crossing the host boundary SHALL fall
relative to the owned path, and the improvement SHALL be measured rather than asserted. The saving
claimed for a collection parameter SHALL be the internal clone between compiled functions, not the
boundary conversion, which stays element by element.

#### Scenario: The text conversion cost falls

- **GIVEN** a function taking a borrowed text parameter
- **WHEN** it is called with a large text value and measured against the same function taking it
  owned
- **THEN** the measured per-element conversion cost is lower

#### Scenario: The measurement is taken from a clean build

- **GIVEN** a working tree with existing build directories
- **WHEN** the improvement is measured
- **THEN** `.compylr/` and the demo's are removed first, so the measurement is not taken against a
  previously cached build

#### Scenario: Forwarding between compiled functions does not clone

- **GIVEN** one compiled function passing a borrowed collection to another that borrows it
- **WHEN** the call is emitted
- **THEN** no clone occurs at the call

#### Scenario: A collection boundary is not claimed to improve

- **GIVEN** a compiled function taking a sequence parameter
- **WHEN** its boundary cost is measured before and after this change
- **THEN** the per-element conversion cost is unchanged, and no requirement claims otherwise

#### Scenario: No program becomes slower

- **GIVEN** the demo
- **WHEN** it is run before and after this change
- **THEN** no algorithm's time regresses beyond measurement noise
