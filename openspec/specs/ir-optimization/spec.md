## Purpose

Defines the transformation stage between lowering and emission: how IR is verified before any target
sees it, how target-agnostic and pair-directed passes are configured and ordered, and the rule that
keeps a pass correct for every source language rather than only for the one it was written against.

## Requirements

### Requirement: IR is verified before it reaches a backend

Every unit SHALL be checked for well-formedness after lowering and before emission. Verification
SHALL be independent of which frontend produced the unit. A unit that fails verification SHALL be
rejected with a diagnostic identifying what is malformed, rather than being passed to a backend that
would emit code that does not build.

#### Scenario: A well-formed unit passes

- **GIVEN** a unit produced by lowering an accepted program
- **WHEN** it is verified
- **THEN** verification succeeds
- **AND** the unit is unchanged

#### Scenario: A malformed unit is rejected

- **GIVEN** a unit referring to a function that is in neither the unit nor its declared external
  signatures
- **WHEN** it is verified
- **THEN** verification fails
- **AND** the diagnostic names the offending reference
- **BUT** no backend has been asked to emit from it

#### Scenario: Verification does not know the source language

- **GIVEN** one malformed unit
- **WHEN** it is verified as though produced by each frontend in turn
- **THEN** the same failure is reported every time

### Requirement: Passes run in a configurable pipeline

Transformations over the IR SHALL be organized as an ordered pipeline of named passes that a caller
can configure. It SHALL be possible to run no optimization passes at all, and doing so SHALL produce
the same observable program behavior as running the full set. Pass selection SHALL be reportable, so
that a build can be explained.

#### Scenario: Default pipeline

- **GIVEN** a compilation supplying no pass configuration
- **WHEN** the unit is compiled
- **THEN** verification runs
- **AND** the documented default set of passes runs

#### Scenario: Optimization disabled

- **GIVEN** a pipeline configured to run no optimization passes
- **WHEN** each accepted fixture is compiled and run
- **THEN** verification still runs
- **AND** every fixture answers what it answers under the full set of passes

#### Scenario: The pipeline is reportable

- **GIVEN** a unit and a pass configuration
- **WHEN** the unit is compiled
- **THEN** the caller can read the names of the passes that ran, in order

### Requirement: A pass preserves observable behavior

A pass SHALL NOT change what a program computes, including its error behavior. A pass SHALL derive
every decision from what the IR declares, never from an assumption about which source language
produced it. A pass that cannot establish that a transformation is safe SHALL leave the IR
unchanged.

#### Scenario: Declared semantics drive the transformation

- **GIVEN** a division of two integer literals carrying a declared rounding mode
- **WHEN** a pass folds it
- **THEN** the folded value is the one that rounding mode gives
- **AND** the same literals under a different declared mode fold to a different value

#### Scenario: An error is not optimized away

- **GIVEN** an operation whose constant operands would fail at runtime, such as division by zero
  or a result outside the integer range
- **WHEN** a folding pass reaches it
- **THEN** the operation is left in place
- **AND** the failure still reaches the caller at runtime

#### Scenario: Uncertainty means no change

- **GIVEN** a transformation a pass cannot establish as behavior-preserving
- **WHEN** the pass runs
- **THEN** the IR is returned unchanged

### Requirement: Constant folding is available as a target-agnostic pass

The default pipeline SHALL include folding of operations whose operands are all literals, for the
arithmetic and comparison forms the IR defines. Folding SHALL respect the semantics declared on each
node and SHALL produce a literal of the type the operation's declared semantics require.

#### Scenario: Arithmetic on literals folds

- **GIVEN** a function body containing an addition of two integer literals
- **WHEN** the default pipeline runs
- **THEN** a single integer literal stands in its place

#### Scenario: Division folds to the declared type

- **GIVEN** a division of two integer literals declaring float promotion
- **WHEN** the default pipeline runs
- **THEN** the result is a float literal

#### Scenario: Non-constant operands are untouched

- **GIVEN** an operation with an operand that is not a literal
- **WHEN** the default pipeline runs
- **THEN** the operation is unchanged

#### Scenario: Folding is observable in the artifact

- **GIVEN** a unit containing foldable arithmetic
- **WHEN** its IR artifact is written
- **THEN** the folded form appears in it

### Requirement: Passes may be directed at a source/target pair

The pipeline SHALL support passes selected by the combination of source language and target
language, running after target-agnostic passes and before emission. A pair-directed pass SHALL
operate on the IR rather than on target source text, and SHALL be subject to the same
behavior-preservation rule as any other pass. When no pass is registered for a pair, the pipeline
SHALL run the target-agnostic passes alone.

#### Scenario: A pair-directed pass runs for its pair

- **GIVEN** a pair that has a directed pass registered
- **WHEN** a unit is compiled for that pair
- **THEN** the directed pass runs after the target-agnostic passes

#### Scenario: The same pass does not run for another pair

- **GIVEN** one unit and a pass registered for a different pair
- **WHEN** the unit is compiled for this pair
- **THEN** that pass does not run

#### Scenario: No pass registered

- **GIVEN** a pair with no directed passes
- **WHEN** a unit is compiled for it
- **THEN** compilation succeeds with the target-agnostic passes alone

### Requirement: Optimization does not change the program's fingerprint

The fingerprint identifying a program SHALL be computed from the IR as lowering produced it, before
optimization, so that turning a pass on does not read as a change to the user's code. Because the
same program can be built by different pass configurations, the pass configuration SHALL be recorded
alongside the fingerprint in build state, and a build made under a different configuration SHALL NOT
be reused.

#### Scenario: Pass configuration does not alter the fingerprint

- **GIVEN** one source file
- **WHEN** it is lowered and fingerprinted with optimization enabled and again with it disabled
- **THEN** the two fingerprints are identical

#### Scenario: Build state records the configuration

- **GIVEN** a project being built
- **WHEN** the build completes
- **THEN** the recorded build state identifies the pass configuration that produced it

#### Scenario: A build under a different configuration is not reused

- **GIVEN** a project whose source has not changed since a recorded build
- **WHEN** it is compiled under a different pass configuration
- **THEN** the artifact is rebuilt
- **BUT** the fingerprint has not moved
