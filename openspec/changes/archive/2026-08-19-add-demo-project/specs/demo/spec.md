## Purpose

A complete, runnable project that demonstrates what compylr does, and that serves as the
repository's only executable check that its features compose rather than merely working in
isolation.

## ADDED Requirements

### Requirement: The demo is a real project

The repository SHALL contain a demonstration project that is complete on its own terms: its own
project definition, its own package, its own tests, and a README explaining what it shows. It SHALL
depend on compylr as a package, the way any consumer would.

It is meant to be copied. A snippet in a README cannot be run, and a fixture is a file chosen to
exercise one rule; neither answers "what does using this look like?"

#### Scenario: It stands alone

- **WHEN** the demo directory is inspected
- **THEN** it contains a project definition, a package, tests, and a README

#### Scenario: It depends on compylr as a consumer

- **WHEN** the demo's dependencies are inspected
- **THEN** compylr appears as a dependency, rather than the demo reaching into the repository's
  internals

#### Scenario: It can be run

- **WHEN** the demo is installed and run following its own README
- **THEN** it produces its documented output

### Requirement: Three nth-prime implementations that each compile

The demo SHALL implement nth-prime three ways — recursive, iterative, and memoized — and each SHALL
be compiled by compylr rather than interpreted.

The three are chosen because they reach different parts of the subset: recursion with a base case;
iteration with reassignment and a locally built collection; and an object holding a mutable cache.
Together they are the smallest set that exercises branching, looping, mutation, and state.

#### Scenario: The recursive implementation compiles

- **WHEN** the demo is built
- **THEN** the recursive implementation is compiled, using recursion and a base case

#### Scenario: The iterative implementation compiles

- **WHEN** the demo is built
- **THEN** the iterative implementation is compiled, using a loop, a reassigned counter, and a
  collection it builds

#### Scenario: The memoized implementation compiles

- **WHEN** the demo is built
- **THEN** the memoized implementation is compiled as a class holding a mutable cache

#### Scenario: None falls back to the interpreter

- **WHEN** each implementation is called
- **THEN** the compiled implementation runs, not the original Python — a demo that silently
  interpreted would demonstrate nothing

#### Scenario: One build covers all three

- **WHEN** the demo is built
- **THEN** a single shared artifact contains all three, as it does for any project

### Requirement: The three implementations agree

The demo SHALL assert that all three produce the same answer for every n over a range, and that the
answer matches a plain interpreted reference implementation.

Three implementations that each compile and disagree is the failure this demo exists to catch:
each would pass its own test, and only comparing them reveals it.

#### Scenario: They agree with each other

- **WHEN** all three are evaluated for every n over a range
- **THEN** every answer matches

#### Scenario: They agree with an interpreted reference

- **WHEN** their answers are compared against an uncompiled implementation
- **THEN** every answer matches

#### Scenario: Known values are correct

- **WHEN** the first few primes are requested
- **THEN** the results are 2, 3, 5, 7, 11

#### Scenario: The memoized implementation actually caches

- **WHEN** the same n is requested twice from the memoized implementation
- **THEN** the second request is served from its cache, so the demonstration is of memoization
  rather than of a class that recomputes

#### Scenario: An invalid n is handled

- **WHEN** an n below one is requested
- **THEN** each implementation behaves as its documented contract says, rather than looping forever
  or returning a wrong answer

### Requirement: The demo demonstrates precompiling

The demo's documented flow SHALL include compiling it ahead of time, and its README SHALL show the
difference that makes to the first run.

Precompiling is most visible in exactly this situation — a project someone has just obtained, where
the first run is the run they judge it by.

#### Scenario: Precompiling is documented

- **WHEN** the demo's README is read
- **THEN** it shows compiling the project ahead of time and then running it

#### Scenario: A precompiled run does not build

- **WHEN** the demo is precompiled and then run
- **THEN** the run performs no build

#### Scenario: The difference is shown as measurements

- **WHEN** the README describes the benefit
- **THEN** it gives measured timings rather than an unquantified claim

### Requirement: The demo is verified by this repository

The repository's own test suite SHALL build the demo and check its output, so that a demo which
stops compiling fails the build rather than misleading a reader.

Because it compiles a crate, this check SHALL be grouped with the repository's other slow tests, so
that keeping it does not make the fast suite unusable.

#### Scenario: The suite builds the demo

- **WHEN** the repository's test suite runs
- **THEN** the demo is built and its implementations are exercised

#### Scenario: A broken demo fails the build

- **WHEN** a demo implementation stops compiling
- **THEN** the repository's suite fails

#### Scenario: The check is grouped with slow tests

- **WHEN** the fast suite is run
- **THEN** the demo check is excluded, and it is available when slow tests are requested

#### Scenario: The README's claims are checked

- **WHEN** the demo's README states which features it demonstrates
- **THEN** the suite exercises each of them, so the claim cannot drift from the code
