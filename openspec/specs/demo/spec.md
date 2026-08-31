## Purpose

Defines the demonstration project at `./demo/`: what it must contain, that its implementations must
compile rather than silently fall back to the interpreter, that they must agree with each other and
with a plain interpreted reference, and that its correctness is checked by this repository's own
suite rather than by its own alone.

A demo that stops compiling and nobody notices is worse than no demo. It is also the only executable
check that the capabilities compose — every other capability is tested in isolation, and interaction
is where a compiler with control flow, mutation, classes, and a build pipeline is most likely to be
wrong.

## Requirements

### Requirement: The demo is a real project

The repository SHALL contain a demonstration project that is complete on its own terms: its own
project definition, its own package, its own tests, and a README explaining what it shows. It SHALL
depend on compylr as a package, the way any consumer would.

It is meant to be copied. A snippet in a README cannot be run, and a fixture is a file chosen to
exercise one rule; neither answers "what does using this look like?"

#### Scenario: It stands alone

- **GIVEN** the demo directory
- **WHEN** it is inspected
- **THEN** it contains a project definition, a package, tests, and a README

#### Scenario: It depends on compylr as a consumer

- **GIVEN** the demo's dependency declarations
- **WHEN** they are inspected
- **THEN** compylr appears as a dependency, rather than the demo reaching into the repository's
  internals

#### Scenario: It can be run

- **GIVEN** a machine following the demo's own README
- **WHEN** the demo is installed and run
- **THEN** it produces its documented output

### Requirement: Three nth-prime implementations that each compile

The demo SHALL implement nth-prime three ways — recursive, iterative, and memoized — and each SHALL
be compiled by compylr rather than interpreted.

The three are chosen because they reach different parts of the subset: recursion with a base case;
iteration with reassignment and a locally built collection; and an object holding a mutable cache.
Together they are the smallest set that exercises branching, looping, mutation, and state.

#### Scenario: The recursive implementation compiles

- **GIVEN** a demo containing a recursive nth-prime
- **WHEN** the demo is built
- **THEN** the recursive implementation is compiled, using recursion and a base case

#### Scenario: The iterative implementation compiles

- **GIVEN** a demo containing an iterative nth-prime
- **WHEN** the demo is built
- **THEN** the iterative implementation is compiled, using a loop, a reassigned counter, and a
  collection it builds

#### Scenario: The memoized implementation compiles

- **GIVEN** a demo containing a memoized nth-prime
- **WHEN** the demo is built
- **THEN** the memoized implementation is compiled as a class holding a mutable cache

#### Scenario: None falls back to the interpreter

- **GIVEN** a built demo
- **WHEN** each implementation is called
- **THEN** the compiled implementation runs, not the original Python — a demo that silently
  interpreted would demonstrate nothing

#### Scenario: One build covers all three

- **GIVEN** a demo containing all three implementations
- **WHEN** the demo is built
- **THEN** a single shared artifact contains all three, as it does for any project

### Requirement: The three implementations agree

The demo SHALL assert that all three produce the same answer for every n over a range, and that the
answer matches a plain interpreted reference implementation.

Three implementations that each compile and disagree is the failure this demo exists to catch:
each would pass its own test, and only comparing them reveals it.

#### Scenario: They agree with each other

- **GIVEN** the three implementations and a range of n
- **WHEN** all three are evaluated for every n in the range
- **THEN** every answer matches

#### Scenario: They agree with an interpreted reference

- **GIVEN** the three implementations and an uncompiled reference
- **WHEN** their answers are compared
- **THEN** every answer matches

#### Scenario: Known values are correct

- **GIVEN** a built demo
- **WHEN** the first few primes are requested
- **THEN** the results are 2, 3, 5, 7, 11

#### Scenario: The memoized implementation actually caches

- **GIVEN** the memoized implementation
- **WHEN** the same n is requested twice
- **THEN** the second request is served from its cache, so the demonstration is of memoization
  rather than of a class that recomputes

#### Scenario: An invalid n is handled

- **GIVEN** the three implementations
- **WHEN** an n below one is requested
- **THEN** each implementation behaves as its documented contract says, rather than looping forever
  or returning a wrong answer

### Requirement: The demo demonstrates precompiling

The demo's documented flow SHALL include compiling it ahead of time, and its README SHALL show the
difference that makes to the first run.

Precompiling is most visible in exactly this situation — a project someone has just obtained, where
the first run is the run they judge it by.

#### Scenario: Precompiling is documented

- **GIVEN** the demo's README
- **WHEN** it is read
- **THEN** it shows compiling the project ahead of time and then running it

#### Scenario: A precompiled run does not build

- **GIVEN** a demo that has been precompiled
- **WHEN** it is run
- **THEN** the run performs no build

#### Scenario: The difference is shown as measurements

- **GIVEN** the demo's README
- **WHEN** it describes the benefit of precompiling
- **THEN** it gives measured timings rather than an unquantified claim

### Requirement: The demo is verified by this repository

The repository's own test suite SHALL build the demo and check its output, so that a demo which
stops compiling fails the build rather than misleading a reader.

Because it compiles a crate, this check SHALL be grouped with the repository's other slow tests, so
that keeping it does not make the fast suite unusable.

#### Scenario: The suite builds the demo

- **GIVEN** a checkout of this repository
- **WHEN** the test suite runs
- **THEN** the demo is built and its implementations are exercised

#### Scenario: A broken demo fails the build

- **GIVEN** a demo implementation that has stopped compiling
- **WHEN** the repository's suite runs
- **THEN** the repository's suite fails

#### Scenario: The check is grouped with slow tests

- **GIVEN** a checkout of this repository
- **WHEN** the fast suite is run
- **THEN** the demo check is excluded, and it is available when slow tests are requested

#### Scenario: The README's claims are checked

- **GIVEN** a README stating which features the demo demonstrates
- **WHEN** the suite runs
- **THEN** the suite exercises each of them, so the claim cannot drift from the code

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

- **GIVEN** a benchmark run
- **WHEN** it reports a workload's timing
- **THEN** it reports a measure of run-to-run variation with it

#### Scenario: The noise floor is stated

- **GIVEN** a benchmark run
- **WHEN** it prints its results
- **THEN** it states the noise floor those results should be read against

#### Scenario: An unresolvable difference is named

- **GIVEN** two figures differing by less than the noise floor
- **WHEN** the benchmark reports them
- **THEN** the benchmark reports the difference as not resolvable rather than as a result

#### Scenario: An unstable workload is visible as unstable

- **GIVEN** a workload whose variation is too wide for its figure to be relied on
- **WHEN** the benchmark reports it
- **THEN** the benchmark's output shows that, rather than presenting a stable-looking number

### Requirement: The demo covers the shapes where compiling loses

The demo SHALL include workloads that exercise the costs of crossing the boundary, not only
workloads that reward a fast body. A benchmark composed only of compute-bound loops reports that
compiling always wins, which is not true.

Specifically it SHALL include a workload whose body does asymptotically less work than converting
its arguments costs, because that is the shape where compiling loses most sharply and the shape a
user is least likely to anticipate.

#### Scenario: A conversion-dominated workload is measured

- **GIVEN** the demo's benchmark suite
- **WHEN** the benchmark runs
- **THEN** it includes a workload whose cost is dominated by converting its arguments rather than
  by its body

#### Scenario: Text collections are represented

- **GIVEN** the demo's benchmark suite
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

- **GIVEN** a guarded workload and a change making it significantly slower than its recorded
  figure
- **WHEN** the repository's checks run
- **THEN** the repository's checks fail

#### Scenario: The guard does not fire on noise

- **GIVEN** a guarded workload varying within the stated noise floor
- **WHEN** the check runs
- **THEN** the check passes, so the guard does not become flaky

### Requirement: The benchmark resolves the difference it reports

A comparison between two behaviors is only meaningful if the harness can distinguish them from
run-to-run variation. The demo's benchmark SHALL report a measure of run-to-run spread alongside
each timing rather than a single best-of figure alone, and the behavior comparison SHALL state the
observed noise floor so a reader can tell a real difference from an artifact of the harness.

Where the difference between two behaviors does not exceed that noise floor, the demo SHALL say the
difference was not resolvable, rather than reporting the ratio as a finding. A number the harness
cannot support is worse than no number, because it will be quoted.

#### Scenario: Spread is reported alongside each timing

- **GIVEN** a benchmark run
- **WHEN** it reports a timing
- **THEN** it reports a measure of run-to-run spread with it, not a single best-of figure alone

#### Scenario: The noise floor is stated for a behavior comparison

- **GIVEN** a benchmark run comparing two behaviors
- **WHEN** it reports the comparison
- **THEN** it states the noise floor the comparison should be read against

#### Scenario: An unresolvable difference is named as such

- **GIVEN** two behaviors whose timings differ by less than the harness's noise floor
- **WHEN** the comparison is reported
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

- **GIVEN** the demo's benchmark
- **WHEN** it is run
- **THEN** it reports the interpreted time, the time under the source language's behavior, and the
  time under the target language's

#### Scenario: The trade is stated

- **GIVEN** the demo's README
- **WHEN** it describes the behavior comparison
- **THEN** it says what the target's behavior gives up, not only what it saves

#### Scenario: Both behaviors produce the documented answer

- **GIVEN** the demo's documented inputs
- **WHEN** the same algorithm is run under both behaviors
- **THEN** both produce the answer the README states, so the comparison is between two correct
  programs

#### Scenario: The comparison is checked by this repository

- **GIVEN** a checkout of this repository
- **WHEN** the slow suite runs the demo
- **THEN** both behaviors are built and exercised, so one of them silently ceasing to compile fails
  the build
