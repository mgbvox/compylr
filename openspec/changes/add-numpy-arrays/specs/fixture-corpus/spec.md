## ADDED Requirements

### Requirement: Arrays are exercised against numpy as the oracle

Array support SHALL be exercised by accepted fixtures in
[`accepted/`](../../../../../frontends/python/fixtures/accepted/) whose expected answers come from
running the same code interpreted, and SHALL include a case in which a write through an array
parameter is observed by the caller.

#### Scenario: Answers come from the interpreted run

- **GIVEN** a driver exercising an array fixture
- **WHEN** the driver runs
- **THEN** the expected answer is what the interpreted run produces
- **AND** no expected value is written into the driver

#### Scenario: A caller-visible mutation is asserted

- **GIVEN** a fixture that writes through a mutably bound array parameter
- **WHEN** the driver runs
- **THEN** it asserts on the caller's array after the call
- **BUT** it does not assert on the return value

#### Scenario: A strided argument is covered

- **GIVEN** the accepted corpus
- **WHEN** it is checked
- **THEN** a fixture passes a non-contiguous array, so the strided path is exercised rather than
  assumed

#### Scenario Outline: Both ranks are covered

- **GIVEN** the accepted corpus
- **WHEN** it is checked
- **THEN** a fixture covers rank <rank>
- **AND** a supported rank with no fixture fails the suite

**Examples:**

| rank |
| ---- |
| 1    |
| 2    |

#### Scenario: Floating-point answers are compared within a tolerance

- **GIVEN** an array fixture producing a floating-point answer
- **WHEN** agreement is checked
- **THEN** it is checked within a stated tolerance, since reduction order may differ from numpy's
  pairwise summation

### Requirement: The refused array shapes are in the rejected corpus

Every array shape this change refuses SHALL appear in
[`rejected/`](../../../../../frontends/python/fixtures/rejected/) and SHALL fail before lowering
produces any IR, under the inverted guard that corpus already carries.

#### Scenario Outline: Each refused shape has a program

- **GIVEN** the rejected corpus
- **WHEN** it is checked
- **THEN** it contains a program using <shape>
- **AND** that program never begins lowering

**Examples:**

| shape                              |
| ---------------------------------- |
| an unranked annotation             |
| an unsupported storage             |
| a partial index                    |
| an array return type               |
| an array stored into an attribute  |
| whole-array arithmetic             |

#### Scenario: Overlap is exercised at the boundary

- **GIVEN** a compiled function taking a mutable and another array parameter
- **WHEN** it is called with the same array for both
- **THEN** the suite asserts the call is refused
