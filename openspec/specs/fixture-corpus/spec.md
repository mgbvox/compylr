## Purpose

The corpus of Python programs this repository compiles on purpose, and the evidence it produces:
that every accepted program answers what CPython answers, that a refused construct cannot start
compiling silently, and that what the documentation claims about the accepted subset is counted from
the corpus rather than remembered beside it.

A corpus belongs to a frontend. The requirements below are stated over the Python corpus, whose
oracle is CPython; a second frontend's corpus is paired with it by member name and measured for
divergence, which is the `ir-diff-checker` capability rather than this one.

## Requirements

### Requirement: Every accepted fixture is driven

Each program in the accepted corpus SHALL have exactly one **driver**, which names the calls that
exercise that fixture: which members to invoke and with what arguments. Running a driver against a
fixture SHALL produce a **transcript** — the results of those calls, rendered so that two runs of
the same calls over the same values are textually identical.

A driver SHALL be readable by more than one runner, because agreement is established at two tiers
and both tiers must exercise the same calls. Two independent statements of the same calls would be
free to drift, and a tier exercising calls the other does not is a tier reporting on a different
program.

A driver SHALL contain no expected values. Expected output is what CPython produces from the same
driver and the same fixture, so there is nothing for a person to type incorrectly.

A driver SHALL exercise every function and every class the fixture defines. A member no driver calls
contributes nothing to the evidence, and the corpus's purpose is evidence.

Drivers SHALL live outside the accepted-fixture directory. That directory's contents are compiler
inputs enumerated by other checks, and a driver sitting among them would join those enumerations.

#### Scenario: Every accepted fixture has a driver

- **GIVEN** the accepted corpus
- **WHEN** it is enumerated
- **THEN** each fixture has exactly one driver
- **AND** the suite fails naming any fixture without one

#### Scenario: A driver reaches every member

- **GIVEN** a fixture and its driver
- **WHEN** the driver runs against the fixture
- **THEN** every function and class the fixture defines has been called

#### Scenario: A driver produces output

- **GIVEN** a driver and its fixture
- **WHEN** the driver runs under CPython
- **THEN** it writes at least one line to standard output

#### Scenario: Drivers do not join the fixture enumerations

- **GIVEN** the checks that derive their work from the accepted-fixture directory
- **WHEN** they enumerate it
- **THEN** no driver appears among the fixtures they find

### Requirement: CPython is the oracle for every accepted fixture

For each accepted fixture, the transcript its driver produces when the fixture's members run
**translated** SHALL be identical to the transcript the same driver produces when they run
**interpreted**.

The interpreted transcript SHALL be produced by running the driver against the unmodified fixture
under CPython, in a process where compilation is off, so that nothing about the comparison depends
on the compiler being correct.

A disagreement SHALL fail, reporting the fixture, both transcripts, and their difference.

#### Scenario: Translated and interpreted transcripts agree

- **GIVEN** an accepted fixture and its driver
- **WHEN** the driver is run both interpreted and translated
- **THEN** the two transcripts are identical

#### Scenario: A disagreement is reported in full

- **GIVEN** an accepted fixture whose two transcripts differ
- **WHEN** the check runs
- **THEN** it fails naming the fixture
- **AND** it shows both transcripts and their difference

#### Scenario: The oracle does not consult the compiler

- **GIVEN** a process with compilation disabled
- **WHEN** the interpreted transcript is produced
- **THEN** CPython has run the fixture's own source
- **BUT** nothing about the transcript depended on the compiler being correct

### Requirement: Agreement is checked at both the translation and the call boundary

Agreement SHALL be established at two tiers, because a translated program can be right and the way
its values cross into the host language can be wrong, and neither tier observes the other's failure.

The **translation tier** SHALL exercise the generated target source directly, without the host
language's calling convention. It SHALL cover the whole accepted corpus and SHALL run as part of the
ordinary test suite.

The **boundary tier** SHALL exercise the same corpus through the host bridge, as a user reaches it.
It SHALL cover the whole accepted corpus and MAY be marked as slow, because it builds an extension.

Neither tier SHALL be treated as standing in for the other.

#### Scenario: The translation tier covers the corpus

- **GIVEN** the accepted corpus
- **WHEN** the ordinary test suite runs
- **THEN** every fixture has been checked for agreement through generated target source

#### Scenario: The boundary tier covers the corpus

- **GIVEN** the accepted corpus
- **WHEN** the full check runs
- **THEN** every fixture has been checked for agreement through the host bridge

#### Scenario: A conversion defect is caught where a translation defect is not

- **GIVEN** a value that is translated correctly but converted incorrectly at the host boundary
- **WHEN** both tiers run
- **THEN** the boundary tier fails
- **BUT** the translation tier passes

### Requirement: Failing to build or run is a failure, not an omission

A fixture that fails to translate, fails to build, fails to run, or produces a transcript that
disagrees SHALL fail the suite.

The suite SHALL skip only for a **missing toolchain** — a fact about the machine rather than about
the program — and a skip SHALL name the tool that was absent.

A build SHALL be performed with the target's warnings denied, so that output which merely happens to
compile is distinguished from output fit to ship.

#### Scenario: Output that no longer builds fails

- **GIVEN** a fixture whose generated source no longer builds
- **WHEN** the suite runs
- **THEN** it fails
- **BUT** it does not skip

#### Scenario: A missing toolchain skips and says so

- **GIVEN** a machine without a tool a tier requires
- **WHEN** that tier runs
- **THEN** it skips
- **AND** the skip names the missing tool

#### Scenario: A warning is a failure

- **GIVEN** generated source that builds but emits a warning
- **WHEN** the build is evaluated
- **THEN** it is treated as failed

### Requirement: A refused construct cannot start compiling silently

Each program in the rejected corpus SHALL continue to be refused, and the refusal SHALL remain the
one the corpus records for it.

A rejected program that **begins to lower** SHALL fail the suite. Growing the accepted subset is a
decision, and this requirement is what makes it one: the failure is cleared by moving the program
into the accepted corpus and giving it a driver, not by editing an allowance.

#### Scenario: A rejected program that starts lowering fails

- **GIVEN** a program in the rejected corpus
- **WHEN** it lowers successfully
- **THEN** the suite fails
- **AND** it names the program and the rejection it was recording

#### Scenario: Clearing the failure moves the program

- **GIVEN** a construct that has become supported
- **WHEN** the failure is cleared
- **THEN** its program has moved into the accepted corpus with a driver
- **AND** the rejected corpus no longer lists it
- **BUT** no allowance has been added to the check

### Requirement: The frontend produces a located diagnostic for arbitrary Python

The frontend SHALL be exercised over a corpus of **ordinary Python not written for this compiler**,
substantially larger than the curated fixtures and including source this repository does not own.

For every program in that corpus, the outcome SHALL be either a lowered unit or a diagnostic
carrying a source position. A panic SHALL fail. A failure without a position SHALL fail.

The check SHALL report the proportion of top-level members the frontend accepted, so that growth in
the accepted subset is a measured quantity rather than an impression.

#### Scenario: No input panics

- **GIVEN** a corpus of ordinary Python not written for this compiler
- **WHEN** the frontend is run over it
- **THEN** no program causes a panic

#### Scenario: Every rejection is located

- **GIVEN** a program in that corpus that the frontend rejects
- **WHEN** the diagnostic is read
- **THEN** it carries a source position

#### Scenario: Acceptance is reported as a number

- **GIVEN** a completed run over that corpus
- **WHEN** the check reports
- **THEN** it says how many of the corpus's top-level members lowered, out of how many

### Requirement: The documented subset is generated from the corpus

The description of the accepted subset that this repository publishes SHALL be **generated** from
the corpus, not maintained alongside it.

Regenerating it SHALL be idempotent, and a mode SHALL exist that verifies the published text matches
what regeneration would produce without measuring anything. That verification SHALL run wherever the
project's other documentation checks run.

Generation SHALL derive from evidence of success: a construct SHALL be reported as accepted only
because a fixture exercising it translated, built, ran, and agreed with CPython.

#### Scenario: Regeneration is idempotent

- **GIVEN** a published subset description
- **WHEN** it is regenerated twice
- **THEN** the second regeneration changes nothing

#### Scenario: Drift fails the check

- **GIVEN** a published subset description differing from what regeneration would produce
- **WHEN** the verification mode runs
- **THEN** it fails
- **AND** it names what differs

#### Scenario: A claim rests on a passing fixture

- **GIVEN** a subset description reporting a construct as accepted
- **WHEN** the claim is traced
- **THEN** a fixture exercising that construct exists
- **AND** it agrees with CPython
