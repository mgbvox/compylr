## MODIFIED Requirements

### Requirement: Every implemented backend renders the shared conformance corpus

compylr SHALL maintain a corpus of IR units, independent of any source language, that every
implemented backend is required to render. Adding a backend SHALL NOT require writing a new corpus,
and a backend that cannot render a corpus entry SHALL fail visibly rather than emitting code that
does not build.

Coverage SHALL be measured over **positions as well as forms**. A backend renders the same statement
differently depending on where it appears — a function body, a constructor, a method with a shared
receiver, a method with a mutable receiver, and a loop body are each rendered by their own path — and
a corpus that recorded only which forms appeared would report full coverage while leaving those paths
untested. Where a form is not legal in a position, the corpus SHALL NOT be required to contain it.

#### Scenario: The corpus covers every IR form

- **WHEN** the corpus is checked against the IR's node forms
- **THEN** every statement form, expression form, and type is exercised by at least one entry

#### Scenario: The corpus covers every form in every position it is legal in

- **WHEN** the corpus is checked against the positions a backend renders separately
- **THEN** each statement form appears in every position where it is legal, and its absence from a
  position it is legal in fails the check

#### Scenario: An illegal position is not required

- **WHEN** a form cannot appear in a position, such as returning a value from a constructor
- **THEN** the check does not require a corpus entry for that combination

#### Scenario: Every implemented backend is checked

- **WHEN** the conformance check runs
- **THEN** it runs each corpus entry through every backend the registry reports as implemented,
  enumerated from the registry rather than from a hand-maintained list

#### Scenario: An unrenderable form is a failure

- **WHEN** a backend cannot render a corpus entry
- **THEN** the conformance check fails and names the entry and the backend
