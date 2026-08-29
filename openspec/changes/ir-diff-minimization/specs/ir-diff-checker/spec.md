## Purpose

Measuring how far two IR units are from being the same program, so that divergence between what
different frontends emit for equivalent source is a tracked number rather than an impression. The
capability owns both halves of that measurement: the normalized form units are compared in, and the
divergence score and its enforcement.

## ADDED Requirements

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

- **WHEN** a unit is normalized for comparison
- **THEN** the unit that reaches the backend is unchanged, and its fingerprint is unchanged

#### Scenario: Independent orderings normalize together

- **WHEN** two units differ only in the order of independent local bindings, or in the operand
  order of a commutative operation over side-effect-free operands
- **THEN** their normalized forms are identical

#### Scenario: Reordering an effectful operand is refused

- **WHEN** a commutative operation has an operand that calls a function
- **THEN** normalization leaves that operation's operand order alone, and two units that differ in
  it do not normalize together

### Requirement: A frontend's IR does not depend on the backend

For one source program and one frontend, the IR SHALL be identical regardless of which backend the
compilation is directed at. This is an invariant rather than a quantity to minimize: a frontend is
defined to be unaware of the target, so any difference here is a target leak in the frontend, which
is a defect and not a score.

#### Scenario: The same source lowered for two targets

- **WHEN** one source file is lowered by one frontend for two different backends
- **THEN** the two units are identical, and the suite fails naming the differing node otherwise

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

- **WHEN** two units have identical structure and differ only in the checking or rounding modes
  their operations resolved to
- **THEN** `D` is 0

#### Scenario: Spans and documentation differ

- **WHEN** two units are structurally identical but carry different spans and different docstrings
- **THEN** `D` is 0

#### Scenario: Structure differs

- **WHEN** two units express the same computation through different structures, such as one
  looping with a cursor where the other iterates a sequence
- **THEN** `D` is greater than 0

### Requirement: Divergence is reported with its location

A comparison SHALL report, alongside the score, which members and which nodes account for it. A
bare number says that the frontends disagree without saying where, and the point of the measurement
is to be acted on.

#### Scenario: A nonzero score is explained

- **WHEN** two units score `D > 0`
- **THEN** the report names each member that contributed and what differed within it

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

- **WHEN** a member name appears in the accepted corpus of two frontends
- **THEN** the pair's divergence is measured and appears in the recorded table

#### Scenario: A member only one corpus defines

- **WHEN** a member appears in one frontend's accepted corpus and not the other's
- **THEN** it is recorded by name as missing coverage, and contributes nothing to the score

#### Scenario: Divergence increases

- **WHEN** a change raises the divergence of any recorded pair above its recorded score
- **THEN** the check fails, naming the pair and both scores

#### Scenario: Divergence decreases

- **WHEN** a change lowers a pair's divergence and the recorded table is regenerated
- **THEN** the check passes against the new table

#### Scenario: The recorded table drifts

- **WHEN** the recorded table is edited by hand to a value a run does not produce
- **THEN** the check fails
