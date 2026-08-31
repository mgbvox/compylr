## Purpose

Measuring how far two IR units are from being the same program, so that divergence between what
different frontends emit for equivalent source is a tracked number rather than an impression. The
capability owns both halves of that measurement: the normalized form units are compared in, and the
divergence score and its enforcement.

## Requirements

### Requirement: Normalization is a comparison-time view

Comparison SHALL be performed over a **normalized** form of a unit that standardizes differences
carrying no meaning — the ordering of independent local bindings, and the operand order of
commutative operations.

Normalizing SHALL NOT alter the unit a backend emits from, and SHALL NOT change a unit's
fingerprint. The normalized form exists to be compared and is never compiled: a program must not
change because the project measures it.

Operand order SHALL be normalized only where both operands are free of side effects. Two programs
that call in different orders are different programs, and normalizing them together would report
agreement that does not exist.

#### Scenario: Normalizing does not change the compiled program

- **GIVEN** a unit that is about to be compared with another
- **WHEN** it is normalized for comparison
- **THEN** the unit that reaches the backend is unchanged
- **AND** its fingerprint is unchanged

#### Scenario: Independent orderings normalize together

- **GIVEN** two units differing only in the order of independent local bindings, or in the operand
  order of a commutative operation whose operands are free of side effects
- **WHEN** both are normalized
- **THEN** their normalized forms are identical

#### Scenario: Reordering an effectful operand is refused

- **GIVEN** two units differing in the operand order of a commutative operation, where one operand
  calls a function
- **WHEN** both are normalized
- **THEN** that operation's operand order is left alone
- **AND** the two units do not normalize together

### Requirement: A frontend's IR does not depend on the backend

For one source program and one frontend, the IR SHALL be identical regardless of which backend the
compilation is directed at. This is an invariant rather than a quantity to minimize: a frontend is
defined to be unaware of the target, so any difference here is a target leak in the frontend, which
is a defect and not a score.

#### Scenario: The same source lowered for two targets

- **GIVEN** one source file and one frontend
- **WHEN** it is lowered for two different backends
- **THEN** the two units are identical
- **BUT** if they differ, the suite fails naming the differing node rather than recording a score

### Requirement: Divergence ignores what the IR is required to preserve

The divergence score `D` SHALL be computed over normalized units and SHALL disregard every
difference the IR carries deliberately:

- the semantic modes an operation resolved to — overflow and division checking, division rounding,
  remainder sign convention, index origin, and the units text length is counted in;
- source spans, which record positions in files that are not the same file;
- documentation, which carries no runtime meaning.

Two units that differ only in these SHALL score `D == 0`. Disregarding them is what makes `D` a
measure of structural divergence rather than a restatement of the fact that the sources were written
in different languages.

#### Scenario: Modes differ, structure does not

- **GIVEN** two units with identical structure whose operations resolved to different checking or
  rounding modes
- **WHEN** their divergence is measured
- **THEN** `D` is 0

#### Scenario: Spans and documentation differ

- **GIVEN** two structurally identical units carrying different spans and different docstrings
- **WHEN** their divergence is measured
- **THEN** `D` is 0

#### Scenario: Structure differs

- **GIVEN** two units expressing the same computation through different structures, one looping
  with a cursor where the other iterates a sequence
- **WHEN** their divergence is measured
- **THEN** `D` is greater than 0

### Requirement: Divergence is reported with its location

A comparison SHALL report, alongside the score, which members and which nodes account for it. A
bare number says that the frontends disagree without saying where, and the point of the measurement
is to be acted on.

#### Scenario: A nonzero score is explained

- **GIVEN** two units scoring `D` greater than 0
- **WHEN** the comparison is reported
- **THEN** the report names each member that contributed
- **AND** it names what differed within each

### Requirement: Cross-language divergence is recorded and may not increase

**Members** accepted by two frontends under the **same name** SHALL be treated as a pair stating
the same program in two languages. Pairing is per member and not per file: two corpora may share a
filename without stating the same programs, and a file-level score would be dominated by members
one corpus does not define at all. For every pair the divergence between the two frontends' units
SHALL be measured and **recorded in the repository**, generated from a real run rather than written
by hand.

A member only one corpus defines SHALL be recorded as missing **coverage** rather than counted as
divergence, and SHALL be recorded by name. Counting it as divergence would mean a corpus scored
better by expressing less; leaving it out entirely would let a pair be dropped silently.

A check SHALL fail when a measured score exceeds its recorded score. Lowering a score is a change to
the recorded table, made by regenerating it. There SHALL be no hand-chosen threshold: the baseline
is whatever the project currently achieves, and the only permitted direction is down.

Reducing divergence SHALL NOT be permitted to break agreement with the source language. A change
that lowers `D` while any accepted fixture stops answering what its oracle answers is a regression,
not an improvement.

#### Scenario: A pair is measured

- **GIVEN** a member name appearing in the accepted corpus of two frontends
- **WHEN** the corpora are compared
- **THEN** the pair's divergence is measured
- **AND** it appears in the recorded table

#### Scenario: A member only one corpus defines

- **GIVEN** a member appearing in one frontend's accepted corpus and not the other's
- **WHEN** the corpora are compared
- **THEN** it is recorded by name as missing coverage
- **BUT** it contributes nothing to the score

#### Scenario: Divergence increases

- **GIVEN** a change that raises the divergence of a recorded pair above its recorded score
- **WHEN** the check runs
- **THEN** it fails
- **AND** it names the pair and both scores

#### Scenario: Divergence decreases

- **GIVEN** a change that lowers a pair's divergence, with the recorded table regenerated
- **WHEN** the check runs
- **THEN** it passes against the new table

#### Scenario: The recorded table drifts

- **GIVEN** a recorded table edited by hand to a value no run produces
- **WHEN** the check runs
- **THEN** it fails
