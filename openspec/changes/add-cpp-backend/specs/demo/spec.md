## ADDED Requirements

### Requirement: Every bridged pair has a demo at the same standard

The repository SHALL contain one demonstration project per registered `(source, target)` bridge, and
each SHALL meet the same standard as the others: the same algorithmic breadth, the same nth-prime
depth, an asserted coverage claim, a benchmark comparing compiled against interpreted, and
verification by this repository's suite.

A pair whose demo is absent, or whose demo is held to a weaker standard than the others, SHALL be a
failure of this requirement rather than a gap someone notices later. Parity is the point: a second
target that is demonstrated less thoroughly than the first tells a reader nothing about whether the
pipeline is actually neutral.

The set of demos SHALL be derived from the bridge registry rather than from a list a test
maintains, so that a bridge registered without a demo fails visibly.

#### Scenario: Each registered pair has a demo

- **GIVEN** the bridge registry
- **WHEN** the repository's demo projects are enumerated
- **THEN** every registered pair has one

#### Scenario: A new bridge without a demo fails

- **GIVEN** a bridge registered for a pair with no demonstration project
- **WHEN** the repository's suite runs
- **THEN** it fails naming the pair that has no demo

#### Scenario: The demos cover the same ground

- **GIVEN** two demonstration projects for two different pairs
- **WHEN** the members each project marks for compilation are compared
- **THEN** each project covers the same algorithmic breadth and the same nth-prime depth

#### Scenario: Each demo asserts its own coverage claim

- **GIVEN** a demonstration project whose README states which IR forms its build exercises
- **WHEN** that project's own suite runs
- **THEN** the claim is checked against the IR the build actually produced
- **AND** a form the claim names but the build does not reach fails the check

#### Scenario: Each demo reports compiled against interpreted

- **GIVEN** a demonstration project
- **WHEN** its benchmark is run
- **THEN** it reports, per workload, the compiled timing and the interpreted timing
- **AND** it reports whether both modes returned the same answer

## MODIFIED Requirements

### Requirement: The demo is a real project

The repository SHALL contain demonstration projects that are each complete on their own terms: an
own project definition, an own package, own tests, and a README explaining what it shows. Each SHALL
depend on compylr as a package, the way any consumer would.

They are meant to be copied. A snippet in a README cannot be run, and a fixture is a file chosen to
exercise one rule; neither answers "what does using this look like?"

Each demonstration project SHALL be written in the source language of the pair it demonstrates and
SHALL compile to that pair's target, so that "what does using this look like?" is answered for every
pair compylr claims to bridge rather than only for the first one.

#### Scenario: It stands alone

- **WHEN** a demo directory is inspected
- **THEN** it contains a project definition, a package, tests, and a README

#### Scenario: It depends on compylr as a consumer

- **WHEN** a demo's dependencies are inspected
- **THEN** compylr appears as a dependency, rather than the demo reaching into the repository's
  internals

#### Scenario: It can be run

- **WHEN** a demo is installed and run following its own README
- **THEN** it produces its documented output

#### Scenario: A demo names the pair it demonstrates

- **GIVEN** a demonstration project
- **WHEN** its README is read
- **THEN** it names the source language it is written in and the target it compiles to

### Requirement: The demo is verified by this repository

The repository's own test suite SHALL build each demo and check its output, so that a demo which
stops compiling fails the build rather than misleading a reader.

Because it compiles a project, this check SHALL be grouped with the repository's other slow tests, so
that keeping it does not make the fast suite unusable.

A demo whose target toolchain is unavailable on the machine SHALL be reported as skipped, naming the
missing toolchain, rather than reported as passing. A skipped demo is not a verified demo.

#### Scenario: The suite builds the demo

- **WHEN** the repository's test suite runs
- **THEN** each demo is built and its implementations are exercised

#### Scenario: A broken demo fails the build

- **WHEN** a demo implementation stops compiling
- **THEN** the repository's suite fails

#### Scenario: The check is grouped with slow tests

- **WHEN** the fast suite is run
- **THEN** the demo check is excluded, and it is available when slow tests are requested

#### Scenario: The README's claims are checked

- **WHEN** a demo's README states which features it demonstrates
- **THEN** the suite exercises each of them, so the claim cannot drift from the code

#### Scenario: A missing target toolchain is a skip, not a pass

- **GIVEN** a machine without the toolchain a demo's target requires
- **WHEN** the repository's suite runs that demo's check
- **THEN** the check reports itself skipped and names the missing toolchain
- **BUT** it does not report success
