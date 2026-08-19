## ADDED Requirements

### Requirement: Control flow is emitted

The backend SHALL emit conditionals, both loop forms, and both loop controls, preserving the
nesting the IR carries.

#### Scenario: A conditional is emitted

- **WHEN** a conditional with an alternative is emitted and executed
- **THEN** the branch matching the test runs and the other does not

#### Scenario: A conditional without an alternative is emitted

- **WHEN** a conditional with no alternative is emitted and executed with a false test
- **THEN** neither branch's effects occur and execution continues after it

#### Scenario: A while loop is emitted

- **WHEN** a loop counting to ten is emitted and executed
- **THEN** the counter ends at ten

#### Scenario: A loop that never runs

- **WHEN** a loop whose test is false at entry is emitted and executed
- **THEN** its body does not run

#### Scenario: Loop control is emitted

- **WHEN** a loop containing `break` and `continue` is emitted and executed
- **THEN** it terminates and skips iterations as Python would

#### Scenario: Nesting is preserved

- **WHEN** a loop containing a conditional containing a loop is emitted and executed
- **THEN** the result matches the interpreted original

### Requirement: Ranges match Python, including a negative step

The backend SHALL emit iteration over a range that produces exactly the values Python produces,
for any combination of start, stop, and step. Rust's `..` counts upward by one and cannot express
a negative step, so a range SHALL NOT be emitted as one.

A step of zero SHALL be a recoverable error rather than a loop that never terminates, matching
Python, which raises for it.

#### Scenario: A simple range

- **WHEN** `for i in range(3)` is emitted and executed
- **THEN** the values are 0, 1, 2

#### Scenario: A bounded range

- **WHEN** `for i in range(2, 5)` is emitted and executed
- **THEN** the values are 2, 3, 4

#### Scenario: A stepped range

- **WHEN** `for i in range(0, 6, 2)` is emitted and executed
- **THEN** the values are 0, 2, 4

#### Scenario: A negative step counts down

- **WHEN** `for i in range(3, 0, -1)` is emitted and executed
- **THEN** the values are 3, 2, 1 — which Rust's `..` cannot produce

#### Scenario: An empty range

- **WHEN** `for i in range(5, 0)` is emitted and executed
- **THEN** the body does not run

#### Scenario: A zero step is recoverable

- **WHEN** a range with a step of zero is evaluated
- **THEN** a recoverable error is returned, rather than the loop running forever

### Requirement: Iterating a collection yields what Python yields

The backend SHALL emit iteration over a sequence yielding its elements in order, over a set
yielding its elements, and over a mapping yielding its **keys**.

Iteration SHALL NOT consume the collection: a name may be iterated and then read again, on the
same terms as every other read.

#### Scenario: Sequence order is preserved

- **WHEN** a sequence is iterated and its elements collected
- **THEN** they appear in the order the sequence holds

#### Scenario: A mapping yields keys

- **WHEN** a mapping is iterated
- **THEN** the loop variable takes each key, matching Python

#### Scenario: A collection is not consumed by iteration

- **WHEN** a function iterates a sequence parameter and then takes its length
- **THEN** the emitted Rust compiles

#### Scenario: Mapping and set order is not guaranteed

- **WHEN** a mapping or set is iterated
- **THEN** the order is unspecified and may differ between runs, consistent with the map type the
  backend uses

### Requirement: A reassigned local is emitted as mutable

The backend SHALL emit a local that is assigned more than once as a mutable binding, and one that
is not as an immutable binding, so that generated code carries no avoidable warnings under the
lint settings the project applies to its own code.

#### Scenario: A rebound local compiles

- **WHEN** a function incrementing a counter is emitted
- **THEN** the emitted Rust compiles

#### Scenario: A local bound once is not mutable

- **WHEN** a function binding a local once is emitted
- **THEN** the emitted binding is not marked mutable

#### Scenario: A reassigned parameter compiles

- **WHEN** a function assigning to its own parameter is emitted
- **THEN** the emitted Rust compiles

#### Scenario: Emitted control flow carries no warnings

- **WHEN** every accepted fixture using control flow is emitted and compiled with warnings denied
- **THEN** it compiles cleanly
