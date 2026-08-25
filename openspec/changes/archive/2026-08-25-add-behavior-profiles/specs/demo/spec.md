## ADDED Requirements

### Requirement: The benchmark resolves the difference it reports

A comparison between two behaviors is only meaningful if the harness can distinguish them from
run-to-run variation. The demo's benchmark SHALL report a measure of run-to-run spread alongside
each timing rather than a single best-of figure alone, and the behavior comparison SHALL state the
observed noise floor so a reader can tell a real difference from an artifact of the harness.

Where the difference between two behaviors does not exceed that noise floor, the demo SHALL say the
difference was not resolvable, rather than reporting the ratio as a finding. A number the harness
cannot support is worse than no number, because it will be quoted.

#### Scenario: Spread is reported alongside each timing

- **WHEN** the benchmark reports a timing
- **THEN** it reports a measure of run-to-run spread with it, not a single best-of figure alone

#### Scenario: The noise floor is stated

- **WHEN** the benchmark reports a behavior comparison
- **THEN** it states the noise floor the comparison should be read against

#### Scenario: An unresolvable difference is named as such

- **WHEN** two behaviors' timings differ by less than the harness's noise floor
- **THEN** the comparison reports that the difference was not resolvable, rather than presenting
  the ratio as a result

### Requirement: The demo measures what a behavior costs and buys

The demo SHALL compile at least one algorithm under both the source language's behavior and the
target language's, and its benchmark SHALL report both alongside the interpreted baseline. The
claim that the target's behavior is faster SHALL be measured rather than asserted.

The demo SHALL also state, for the algorithm it measures, what the target's behavior changes about
the answer — or that it changes nothing for these inputs — so that a reader sees the trade and not
only the number.

#### Scenario: Both behaviors are measured

- **WHEN** the demo's benchmark is run
- **THEN** it reports the interpreted time, the time under the source language's behavior, and the
  time under the target language's

#### Scenario: The trade is stated

- **WHEN** the demo's README describes the behavior comparison
- **THEN** it says what the target's behavior gives up, not only what it saves

#### Scenario: Both behaviors produce the documented answer

- **WHEN** the demo runs the same algorithm under both behaviors on its documented inputs
- **THEN** both produce the answer the README states, so the comparison is between two correct
  programs

#### Scenario: The comparison is checked by this repository

- **WHEN** the repository's slow suite runs the demo
- **THEN** both behaviors are built and exercised, so one of them silently ceasing to compile fails
  the build
