## ADDED Requirements

### Requirement: Mutation is emitted in place

The backend SHALL emit a mutated collection as a single binding that is modified, not as a value
that is copied and then modified. A collection that is mutated SHALL be bound mutably, and one that
is not SHALL NOT be.

The backend clones collections wherever they are consumed, so that a name read twice is not moved.
That rule must not apply to the target of a mutation: mutating a clone changes a value nothing
reads afterwards, which compiles cleanly and does nothing.

#### Scenario: Appending in a loop accumulates

- **WHEN** a function that binds an empty sequence, appends in a loop, and returns it is emitted
  and executed
- **THEN** the returned sequence holds every appended element

#### Scenario: Element assignment takes effect

- **WHEN** a function that assigns to an element and then reads it is emitted and executed
- **THEN** the read observes the assigned value

#### Scenario: A mutated collection is bound mutably

- **WHEN** a function that mutates a local collection is emitted
- **THEN** the emitted binding is mutable, and the source compiles

#### Scenario: An unmutated collection is not bound mutably

- **WHEN** a function that only reads a local collection is emitted
- **THEN** the emitted binding is not marked mutable, so no warning is produced

#### Scenario: Mutation and reading compose

- **WHEN** a function mutates a collection and then takes its length
- **THEN** the emitted Rust compiles and the length reflects the mutation

### Requirement: Assigning a mapping key inserts it

The backend SHALL emit assignment to a mapping key as an insertion. Reading a missing key is an
error; assigning to one is not, and Python creates it.

#### Scenario: Assigning a new key creates it

- **WHEN** a function assigns to a key not present and then reads it
- **THEN** the read succeeds and observes the assigned value

#### Scenario: Assigning an existing key replaces it

- **WHEN** a function assigns twice to the same key
- **THEN** the second value is observed

#### Scenario: Reading a missing key still fails

- **WHEN** a function reads a key that was never assigned
- **THEN** a recoverable error is returned, unchanged by this requirement

### Requirement: Membership is emitted for every container

The backend SHALL emit membership over sequences, mappings, sets, and strings, testing a mapping's
keys and a string's substrings, matching Python.

#### Scenario: Sequence membership

- **WHEN** membership over a sequence is emitted and executed
- **THEN** the result is true exactly when the value is present

#### Scenario: Mapping membership tests keys

- **WHEN** membership over a mapping is emitted and executed
- **THEN** the result reflects the keys, not the values

#### Scenario: Set membership

- **WHEN** membership over a set is emitted and executed
- **THEN** the result is true exactly when the element is present

#### Scenario: String membership is a substring test

- **WHEN** membership over a string is emitted and executed
- **THEN** it reports whether the first is a substring of the second, matching Python

#### Scenario: Negated membership

- **WHEN** `not in` is emitted and executed
- **THEN** the result is the negation of the corresponding membership test

#### Scenario: Membership does not consume the container

- **WHEN** a function tests membership and then reads the container
- **THEN** the emitted Rust compiles
