## ADDED Requirements

### Requirement: An array argument costs no per-element conversion

The cost of passing an array across the boundary SHALL NOT grow with the number of elements, and
this SHALL be measured rather than asserted. This is the claim the change exists to make, so an
unmeasured version of it is not enough.

#### Scenario: Call setup is constant in the element count

- **GIVEN** one compiled function taking an array parameter
- **WHEN** it is called with arrays of increasing size
- **THEN** the measured setup cost before the body runs does not grow with the element count

#### Scenario: An array beats the equivalent sequence argument

- **GIVEN** one computation written twice, taking a sequence parameter and taking an array parameter
- **WHEN** both are measured
- **THEN** the array version's boundary cost is materially lower
- **AND** the measurement is recorded

#### Scenario: Element access is not slower than the target's own indexing

- **GIVEN** a compiled loop reading array elements under the unchecked mode
- **WHEN** the emitted source is inspected
- **THEN** the access is a direct indexed read
- **BUT** there is no per-element helper call

#### Scenario: Measurements start from a clean build

- **GIVEN** a working tree with existing build directories
- **WHEN** any array measurement is taken
- **THEN** `.compylr/` and the demo's are removed first, so a previously cached build is not
  measured
