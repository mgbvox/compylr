## ADDED Requirements

### Requirement: Printed output is compared against CPython's

A fixture that produces output SHALL have its output captured from both the interpreted and the
compiled run, and the two SHALL be compared as text. A difference SHALL fail the suite, and no
expected output SHALL be written into the driver.

#### Scenario: Output is captured from both tiers

- **WHEN** a driver exercises a fixture function that prints
- **THEN** the output of the interpreted run and of the compiled run are both captured and compared

#### Scenario: A rendering difference fails the suite

- **WHEN** a compiled run renders a boolean or a float differently from the interpreted run
- **THEN** the suite fails naming the differing line, rather than comparing only return values

#### Scenario: Ordering is part of the comparison

- **WHEN** a fixture interleaves output from a compiled function with output from its driver
- **THEN** the comparison covers the order of the lines, not only their contents

#### Scenario: An unordered container fixture stays in the rejected corpus

- **WHEN** the corpus contains a program printing a mapping or a set
- **THEN** it appears under the rejected corpus and never begins lowering
