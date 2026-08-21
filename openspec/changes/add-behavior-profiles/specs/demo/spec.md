## ADDED Requirements

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
