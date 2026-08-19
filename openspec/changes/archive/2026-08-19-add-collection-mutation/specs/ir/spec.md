## ADDED Requirements

### Requirement: Element assignment and membership forms

The IR SHALL support assigning to one element of a collection, as a statement carrying the
collection, the index or key, and the value; and testing membership, as an expression carrying the
value and the container.

It SHALL also support appending to a sequence. Appending is represented explicitly rather than as a
general method call: there is exactly one supported method, and a general form would need a method
signature table before anything needed one.

#### Scenario: Element assignment

- **WHEN** a body assigns to a collection element
- **THEN** the IR carries the collection, the index or key, and the value

#### Scenario: Membership

- **WHEN** a body tests membership
- **THEN** the IR carries the value and the container

#### Scenario: Negated membership

- **WHEN** a body tests `not in`
- **THEN** the IR represents it as the negation of a membership test rather than as its own form

#### Scenario: Append

- **WHEN** a body appends to a sequence
- **THEN** the IR carries the sequence and the value, as a form distinct from a call

#### Scenario: Appending is not resolved as a call

- **WHEN** a unit containing an append is validated
- **THEN** validation does not attempt to resolve `append` as a function in the unit

#### Scenario: The new forms survive the artifact

- **WHEN** a unit containing element assignment, membership, and append is round-tripped
- **THEN** the result compares structurally equal to the original
