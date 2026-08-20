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

- **WHEN** a unit produced by lowering an accepted program is verified
- **THEN** verification succeeds and the unit is unchanged

#### Scenario: A malformed unit is rejected

- **WHEN** a unit contains a reference to a function that is in neither the unit nor its declared
  external signatures
- **THEN** verification fails and reports the offending reference

#### Scenario: Verification does not know the source language

- **WHEN** the same malformed unit is presented as though produced by any frontend
- **THEN** verification reports the same failure

### Requirement: Passes run in a configurable pipeline

Transformations over the IR SHALL be organized as an ordered pipeline of named passes that a caller
can configure. It SHALL be possible to run no optimization passes at all, and doing so SHALL produce
the same observable program behavior as running the full set. Pass selection SHALL be reportable, so
that a build can be explained.

#### Scenario: Default pipeline

- **WHEN** no pass configuration is supplied
- **THEN** verification runs and a documented default set of passes runs

#### Scenario: Optimization disabled

- **WHEN** the pipeline is configured to run no optimization passes
- **THEN** verification still runs, and the resulting program behaves identically to the optimized
  one on every accepted fixture

#### Scenario: The pipeline is reportable

- **WHEN** a unit is compiled
- **THEN** the names of the passes that ran, in order, are available to the caller

### Requirement: A pass preserves observable behavior

A pass SHALL NOT change what a program computes, including its error behavior. A pass SHALL derive
every decision from what the IR declares, never from an assumption about which source language
produced it. A pass that cannot establish that a transformation is safe SHALL leave the IR
unchanged.

#### Scenario: Declared semantics drive the transformation

- **WHEN** a pass folds a division of two integer literals
- **THEN** the folded value is computed using the rounding mode declared on that node, so the same
  literals under a different declared mode fold to a different value

#### Scenario: An error is not optimized away

- **WHEN** a pass encounters an operation whose constant operands would fail at runtime, such as
  division by zero or a result outside the integer range
- **THEN** it leaves the operation in place so the failure still reaches the caller

#### Scenario: Uncertainty means no change

- **WHEN** a pass cannot determine that a transformation preserves behavior
- **THEN** the IR is returned unchanged

### Requirement: Constant folding is available as a target-agnostic pass

The default pipeline SHALL include folding of operations whose operands are all literals, for the
arithmetic and comparison forms the IR defines. Folding SHALL respect the semantics declared on each
node and SHALL produce a literal of the type the operation's declared semantics require.

#### Scenario: Arithmetic on literals folds

- **WHEN** a function body contains an addition of two integer literals
- **THEN** the emitted IR contains a single integer literal in its place

#### Scenario: Division folds to the declared type

- **WHEN** a division declaring float promotion is applied to two integer literals
- **THEN** the result is a float literal

#### Scenario: Non-constant operands are untouched

- **WHEN** an operation has an operand that is not a literal
- **THEN** the operation is unchanged

#### Scenario: Folding is observable in the artifact

- **WHEN** the IR artifact for a unit containing foldable arithmetic is written
- **THEN** the folded form appears in it

### Requirement: Passes may be directed at a source/target pair

The pipeline SHALL support passes selected by the combination of source language and target
language, running after target-agnostic passes and before emission. A pair-directed pass SHALL
operate on the IR rather than on target source text, and SHALL be subject to the same
behavior-preservation rule as any other pass. When no pass is registered for a pair, the pipeline
SHALL run the target-agnostic passes alone.

#### Scenario: A pair-directed pass runs for its pair

- **WHEN** a unit is compiled from a source language to a target that has a pair-directed pass
- **THEN** that pass runs after the target-agnostic passes

#### Scenario: The same pass does not run for another pair

- **WHEN** the same unit is compiled to a different target
- **THEN** the pass registered for the first pair does not run

#### Scenario: No pass registered

- **WHEN** a pair has no directed passes
- **THEN** compilation succeeds with the target-agnostic passes alone

### Requirement: Optimization does not change the program's fingerprint

The fingerprint identifying a program SHALL be computed from the IR as lowering produced it, before
optimization, so that turning a pass on does not read as a change to the user's code. Because the
same program can be built by different pass configurations, the pass configuration SHALL be recorded
alongside the fingerprint in build state, and a build made under a different configuration SHALL NOT
be reused.

#### Scenario: Pass configuration does not alter the fingerprint

- **WHEN** the same source is lowered and fingerprinted with optimization enabled and disabled
- **THEN** the fingerprint is identical

#### Scenario: Build state records the configuration

- **WHEN** a project is built
- **THEN** the recorded build state identifies the pass configuration that produced it

#### Scenario: A build under a different configuration is not reused

- **WHEN** a project whose source has not changed is compiled with a different pass configuration
  than the one recorded
- **THEN** the cached artifact is rebuilt rather than reused
