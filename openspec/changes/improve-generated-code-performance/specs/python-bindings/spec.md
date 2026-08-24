## ADDED Requirements

### Requirement: The cost of crossing the boundary is stated

The per-element cost of converting a collection across the boundary SHALL be documented where users
meet it, because it is a property of compiling rather than of any program they wrote, and nothing
in their source suggests it.

A collection parameter is converted element by element on **every call**, so a compiled function
can be slower than the interpreted one purely by being called — most sharply when the body does
less work than the conversion. A binary search over 2000 elements converts all of them to perform
about eleven comparisons, and runs roughly 16x slower compiled than interpreted as a result.

#### Scenario: The cost is documented

- **WHEN** a user reads the demo's documentation
- **THEN** it states that a collection parameter costs time proportional to its length on every
  call, even when the function's body does not

#### Scenario: The documentation names when compiling loses

- **WHEN** the documentation describes what compiling is worth
- **THEN** it says that a function doing less work than its arguments cost to convert may be slower
  compiled, rather than implying compiled is always at least as fast
