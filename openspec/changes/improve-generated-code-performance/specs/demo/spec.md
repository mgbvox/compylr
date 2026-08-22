## ADDED Requirements

### Requirement: The benchmark reports spread, not a single best

The demo's benchmark SHALL report a measure of run-to-run variation alongside each timing, and
SHALL state the noise floor its results should be read against.

A single best-of-N figure hides instability rather than removing it. `sorting.merge_sort` has been
observed at 160, 202, 235, 256, 264 and 277 us across runs of binaries that were in some cases
byte-identical — a spread wider than most of the improvements anyone would want to measure. A
reader given only the best of those six cannot tell a real 10% gain from the harness moving.

Where a reported difference does not exceed the noise floor, the benchmark SHALL say so rather than
presenting the ratio as a result.

#### Scenario: Every timing carries a spread

- **WHEN** the benchmark reports a workload's timing
- **THEN** it reports a measure of run-to-run variation with it

#### Scenario: The noise floor is stated

- **WHEN** the benchmark prints its results
- **THEN** it states the noise floor those results should be read against

#### Scenario: An unresolvable difference is named

- **WHEN** two figures differ by less than the noise floor
- **THEN** the benchmark reports the difference as not resolvable rather than as a result

#### Scenario: An unstable workload is visible as unstable

- **WHEN** a workload's variation is wide enough that its figure cannot be relied on
- **THEN** the benchmark's output shows that, rather than presenting a stable-looking number

### Requirement: The demo covers the shapes where compiling loses

The demo SHALL include workloads that exercise the costs of crossing the boundary, not only
workloads that reward a fast body. A benchmark composed only of compute-bound loops reports that
compiling always wins, which is not true.

Specifically it SHALL include a workload whose body does asymptotically less work than converting
its arguments costs, because that is the shape where compiling loses most sharply and the shape a
user is least likely to anticipate.

#### Scenario: A conversion-dominated workload is measured

- **WHEN** the benchmark runs
- **THEN** it includes a workload whose cost is dominated by converting its arguments rather than
  by its body

#### Scenario: Text collections are represented

- **WHEN** the benchmark runs
- **THEN** it includes a workload taking a collection of text, whose per-element crossing cost is
  the highest of the supported element types

### Requirement: A recorded speedup is guarded against regression

The repository SHALL check that generated code does not regress on workloads whose performance is a
recorded property of the build.

Every defect this change addresses produced correct answers, so no correctness test could have
caught any of them. The demo is the only instrument that sees them, which makes it the only place a
guard can live.

#### Scenario: A regression fails the repository's checks

- **WHEN** a change makes a guarded workload significantly slower than its recorded figure
- **THEN** the repository's checks fail

#### Scenario: The guard does not fire on noise

- **WHEN** a guarded workload varies within the stated noise floor
- **THEN** the check passes, so the guard does not become flaky
