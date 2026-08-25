## ADDED Requirements

### Requirement: An optimization preserves observable behavior

A change made to generated code for performance SHALL NOT change any answer the generated code
produces, any exception it raises, or any value observable from Python. Where a faster form would
change an answer, it is not an optimization and belongs to a change about semantics.

This is the line that separates this capability from a behavior selection: an optimization is
applied unconditionally and needs no one's permission, precisely because nothing can observe it.

#### Scenario: An optimization is applied without being requested

- **WHEN** the compiler emits code for a construct an optimization covers
- **THEN** it applies the optimization with no setting, flag, or annotation from the user

#### Scenario: A behavior-changing rewrite is refused

- **WHEN** a faster emission would change a result, an exception, or an exception's type
- **THEN** it is not applied under this capability

#### Scenario: The subset is unchanged

- **WHEN** an optimization lands
- **THEN** the set of accepted programs is exactly what it was, and no diagnostic is added or
  removed

### Requirement: A performance claim is measured, and measured against noise

A speedup claimed for generated code SHALL be supported by a measurement taken from the demo
benchmark, and the measurement SHALL be compared against the harness's noise floor. A difference
that does not exceed the noise floor SHALL NOT be reported as a speedup.

The reason is recorded rather than assumed: `sorting.merge_sort` has been observed returning 160us
and 277us from *byte-identical* builds, so a claim taken from a single run of a single workload can
be pure harness variance.

#### Scenario: A claim cites a measurement

- **WHEN** a change to generated code claims a speedup
- **THEN** the claim names the workload and the before and after timings it came from

#### Scenario: A difference inside the noise is not a claim

- **WHEN** a measured difference does not exceed the harness's stated noise floor
- **THEN** it is reported as not resolvable rather than as a speedup

#### Scenario: A rejected candidate is recorded with its measurement

- **WHEN** a candidate optimization is measured and does not pay for itself
- **THEN** the measurement is recorded, so the candidate is not re-proposed on intuition

### Requirement: Measurement is taken against a rebuilt artifact

Because rebuild decisions key off the IR fingerprint, an emission change does not invalidate a
cached build. Any measurement of an emission change SHALL be taken after the artifact directories
are removed, so the measurement describes the new emission rather than the previous build.

#### Scenario: A stale artifact does not become a measurement

- **WHEN** generated code is measured after a change to emission
- **THEN** the measurement is taken from an artifact rebuilt after that change, not from a cached
  one

### Requirement: A known speedup does not silently regress

The defects this capability addresses were invisible to every correctness test, because each
produced correct answers. Correctness tests therefore cannot guard them. The repository SHALL check
that generated code does not regress on a workload whose performance is a recorded property.

#### Scenario: A regression is caught by the repository

- **WHEN** a change makes a guarded workload significantly slower than its recorded figure
- **THEN** the repository's checks fail rather than passing quietly

#### Scenario: The guard tolerates noise

- **WHEN** a guarded workload varies within the harness's noise floor
- **THEN** the check passes, so the guard does not become a flaky test
