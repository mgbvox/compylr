## ADDED Requirements

### Requirement: Arrays are exercised against numpy as the oracle

Array support SHALL be exercised by accepted fixtures whose expected answers come from running the
same code interpreted, and SHALL include a case in which a write through an array parameter is
observed by the caller.

#### Scenario: Answers come from the interpreted run

- **WHEN** a driver exercises an array fixture
- **THEN** the expected answer is what the interpreted run produces, and no expected value is
  written into the driver

#### Scenario: A caller-visible mutation is asserted

- **WHEN** a fixture writes through a mutably bound array parameter
- **THEN** the driver asserts on the caller's array after the call, not on the return value

#### Scenario: A strided argument is covered

- **WHEN** the corpus is checked
- **THEN** a fixture passes a non-contiguous array, so the strided path is exercised rather than
  assumed

#### Scenario: Both ranks are covered

- **WHEN** the corpus is checked
- **THEN** accepted fixtures cover rank one and rank two, and a supported rank with no fixture fails
  the suite

#### Scenario: Floating-point answers are compared within a tolerance

- **WHEN** an array fixture produces a floating-point answer
- **THEN** agreement is checked within a stated tolerance, since reduction order may differ

### Requirement: The refused array shapes are in the rejected corpus

Every array shape this change refuses SHALL appear in the rejected corpus and SHALL fail before
lowering produces any IR.

#### Scenario: The refused shapes each have a program

- **WHEN** the rejected corpus is checked
- **THEN** it contains an unranked annotation, an unsupported storage, a partial index, an array
  return, and an array stored into an attribute

#### Scenario: Overlap is exercised at the boundary

- **WHEN** the suite runs
- **THEN** a case calls a compiled function with the same array for a mutable and another array
  parameter, and asserts the call is refused
