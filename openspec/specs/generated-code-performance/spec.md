## Purpose

Governs changes made to generated code for speed: that an optimization is applied unconditionally
because nothing can observe it, that a claimed speedup is a measurement taken against the
benchmark harness's stated noise floor rather than an intuition, and that a speedup once recorded
is guarded against silently regressing.

The defects this capability exists to catch all produced correct answers, so no correctness test in
the repository could see them. Measurement is the only instrument that can, which is why what
counts as a measurement is specified here rather than left to whoever is claiming the number.

## Requirements

### Requirement: An optimization preserves observable behavior

A change made to generated code for performance SHALL NOT change any answer the generated code
produces, any failure it reports, or any value observable from the calling host. Where a faster
form would change an answer, it is not an optimization and belongs to a change about semantics.

This is the line that separates this capability from a behavior selection: an optimization is
applied unconditionally and needs no one's permission, precisely because nothing can observe it.

#### Scenario: An optimization is applied without being requested

- **GIVEN** a construct that an optimization covers
- **WHEN** the compiler emits code for it
- **THEN** the optimization is applied
- **BUT** no setting, flag, or annotation from the user was needed to get it

#### Scenario: A behavior-changing rewrite is refused

- **GIVEN** a candidate emission that is faster than the current one
- **WHEN** it would change a result, a reported failure, or the kind of that failure
- **THEN** it is not applied under this capability

#### Scenario: The subset is unchanged

- **GIVEN** the set of programs compylr accepts before an optimization lands
- **WHEN** the optimization lands
- **THEN** exactly the same programs are accepted
- **AND** no diagnostic has been added or removed

### Requirement: A performance claim is measured, and measured against noise

A speedup claimed for generated code SHALL be supported by a measurement taken from the demo
benchmark, and the measurement SHALL be compared against the harness's noise floor. A difference
that does not exceed the noise floor SHALL NOT be reported as a speedup.

The reason is recorded rather than assumed: `sorting.merge_sort` has been observed returning 160us
and 277us from *byte-identical* builds, so a claim taken from a single run of a single workload can
be pure harness variance.

#### Scenario: A claim cites a measurement

- **GIVEN** a change to generated code that claims a speedup
- **WHEN** the claim is made
- **THEN** it names the workload it was measured on
- **AND** it names the before and after timings it came from

#### Scenario: A difference inside the noise is not a claim

- **GIVEN** a measured difference between two builds
- **WHEN** the difference does not exceed the harness's stated noise floor
- **THEN** it is reported as not resolvable
- **BUT** it is not reported as a speedup

#### Scenario: A rejected candidate is recorded with its measurement

- **GIVEN** a candidate optimization that has been measured
- **WHEN** the measurement shows it does not pay for itself
- **THEN** the measurement is recorded
- **AND** the candidate is not re-proposed on intuition later

### Requirement: Measurement is taken against a rebuilt artifact

Because rebuild decisions key off the IR fingerprint, an emission change does not invalidate a
cached build. Any measurement of an emission change SHALL be taken after the artifact directories
are removed, so the measurement describes the new emission rather than the previous build.

#### Scenario: A stale artifact does not become a measurement

- **GIVEN** a change to emission and a cached build predating it
- **WHEN** generated code is measured
- **THEN** the measurement comes from an artifact rebuilt after the change
- **BUT** never from the cached build, whose fingerprint the change did not move

### Requirement: A known speedup does not silently regress

The defects this capability addresses were invisible to every correctness test, because each
produced correct answers. Correctness tests therefore cannot guard them. The repository SHALL check
that generated code does not regress on a workload whose performance is a recorded property.

#### Scenario: A regression is caught by the repository

- **GIVEN** a workload whose performance is a recorded property
- **WHEN** a change makes it significantly slower than its recorded figure
- **THEN** the repository's checks fail rather than passing quietly

#### Scenario: The guard tolerates noise

- **GIVEN** a guarded workload
- **WHEN** it varies within the harness's noise floor
- **THEN** the check passes, so the guard does not become a flaky test
