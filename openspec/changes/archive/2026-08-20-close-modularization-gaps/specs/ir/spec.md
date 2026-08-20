## ADDED Requirements

### Requirement: Container operations carry declared semantics

Reading an element of a sequence and measuring the length of a value each admit more than one
reasonable interpretation across the languages compylr supports, so each SHALL carry its
interpretation on the node rather than inherit one from whichever frontend happens to exist.

Specifically: a subscript SHALL carry an **index origin**, and a length SHALL carry the **text
units** it counts in. A frontend sets these to whatever its source language means; a backend
reproduces exactly what the node says.

The index origins SHALL be *from either end*, where a negative index counts backwards from the end,
and *from the start*, where a negative index is out of range. The text units SHALL be *code points*,
*UTF-8 bytes*, and *UTF-16 units*. These cover Python, Go, C++, and TypeScript; a language needing
another SHALL add it to the IR rather than encode it in its frontend.

Each mode describes one operand kind and SHALL be inert for the others: an index origin says nothing
about a mapping, whose index is a key rather than an offset, and text units say nothing about a
sequence, whose length is a count of elements.

#### Scenario: Index origin is explicit

- **WHEN** a subscript node is inspected
- **THEN** its index origin is readable from the node itself

#### Scenario: The same subscript can mean either origin

- **WHEN** two subscript nodes declare different index origins
- **THEN** they are distinguishable, and a backend renders each differently

#### Scenario: Text units are explicit

- **WHEN** a length node is inspected
- **THEN** the units it counts in are readable from the node itself

#### Scenario: All three unit readings are distinguishable

- **WHEN** three length nodes declare code points, UTF-8 bytes, and UTF-16 units
- **THEN** each is distinct from the others

#### Scenario: A declared container mode survives the artifact

- **WHEN** a unit containing subscripts and lengths is serialized and read back
- **THEN** every declared mode is unchanged

#### Scenario: A declared container mode reaches the fingerprint

- **WHEN** two units differ only in a declared index origin, or only in declared text units
- **THEN** their fingerprints differ, because the mode is part of what the program computes

### Requirement: Container behavior that is not a mode is not parameterized

Where languages differ in the **shape** of an operation rather than in a setting on it, the IR SHALL
model the difference as a distinct form and SHALL NOT add a mode. In particular, reading a mapping
with a key that is absent SHALL always be an operation that reports the failure: a language whose
lookup instead yields a default value is performing a different operation, one that requires a
notion of a type's zero value the IR does not model, and its frontend SHALL lower it to a different
form rather than set a flag.

#### Scenario: A missing mapping key is reported

- **WHEN** a mapping is read with a key it does not contain
- **THEN** the operation reports the missing key, whichever frontend produced the unit

#### Scenario: No mode exists for behavior compylr's languages agree on

- **WHEN** the IR's node definitions are inspected
- **THEN** no mode is carried for iterating a mapping, testing membership, or assigning a mapping
  key, because the languages in the supported list agree on all three
