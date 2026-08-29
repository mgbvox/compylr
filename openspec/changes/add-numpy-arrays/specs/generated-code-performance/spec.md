## ADDED Requirements

### Requirement: An array argument costs no per-element conversion

The cost of passing an array across the boundary SHALL NOT grow with the number of elements, and
this SHALL be measured rather than asserted.

#### Scenario: Call setup is constant in the element count

- **WHEN** the same compiled function is called with arrays of increasing size
- **THEN** the measured setup cost before the body runs does not grow with the element count

#### Scenario: An array beats the equivalent sequence argument

- **WHEN** the same computation is measured taking a sequence parameter and taking an array
  parameter
- **THEN** the array version's boundary cost is materially lower, and the measurement is recorded

#### Scenario: Element access is not slower than the target's own indexing

- **WHEN** a compiled loop reads array elements under an unchecked mode
- **THEN** the emitted access is a direct indexed read with no per-element helper call

#### Scenario: Measurements start from a clean build

- **WHEN** any array measurement is taken
- **THEN** the build directories are removed first, so a previously cached build is not measured
