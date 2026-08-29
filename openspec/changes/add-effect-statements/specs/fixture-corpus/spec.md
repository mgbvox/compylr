## ADDED Requirements

### Requirement: Printed output is compared against CPython's

A fixture that produces output SHALL have its output captured from both the interpreted and the
compiled run, and the two SHALL be compared as text. A difference SHALL fail the suite, and no
expected output SHALL be written into the driver — what a program should print is what CPython
prints, as with every other answer a driver checks.

#### Scenario: Output is captured from both tiers

- **GIVEN** a driver exercising a fixture function that prints
- **WHEN** the driver runs
- **THEN** the output of the interpreted run and of the compiled run are both captured and compared

#### Scenario: A rendering difference fails the suite

- **GIVEN** a compiled run that renders a boolean or a float differently from the interpreted run
- **WHEN** the suite compares the two
- **THEN** the suite fails naming the differing line
- **BUT** it does not compare only return values

#### Scenario: Ordering is part of the comparison

- **GIVEN** a fixture interleaving output from a compiled function with output from its driver
- **WHEN** the suite compares the two runs
- **THEN** the comparison covers the order of the lines, not only their contents

#### Scenario: An unordered container fixture stays in the rejected corpus

- **GIVEN** a program that prints a mapping or a set
- **WHEN** the corpus suite runs
- **THEN** the program appears under
  [`rejected/`](../../../../../frontends/python/fixtures/rejected/)
- **AND** it never begins lowering
