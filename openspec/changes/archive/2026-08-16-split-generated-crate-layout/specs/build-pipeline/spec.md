## MODIFIED Requirements

### Requirement: Intermediate artifacts are written for inspection

The pipeline SHALL write both intermediates to disk on every build: the IR of the compiled
unit, and the generated target source. A user SHALL be able to read them without re-running
the compiler, because a transpiler whose intermediate stages are invisible cannot be debugged
or trusted.

Where a backend emits several files, **every** file SHALL be written, each at its reported
relative path. Files a previous build wrote and this one did not SHALL be removed, so a rename
in the emitter cannot leave behind a stale file that still compiles and quietly contradicts the
current source.

#### Scenario: IR artifact is written

- **WHEN** a build completes
- **THEN** the unit's IR is present on disk in a documented, readable format

#### Scenario: Target source is written

- **WHEN** a build completes for the `rust` backend
- **THEN** every generated file is present on disk at its reported path

#### Scenario: A stale file is removed

- **WHEN** a build writes a file that a later build does not
- **THEN** the later build removes it, rather than leaving a file no longer part of the crate

#### Scenario: Hand-written files in the crate are untouched

- **WHEN** a build runs against a crate directory that also holds the build manifest and build
  configuration
- **THEN** those are preserved, since pruning applies to generated source rather than to
  everything present

#### Scenario: Artifacts survive a skipped rebuild

- **WHEN** a run reuses an up-to-date artifact without rebuilding
- **THEN** the IR and target source from the previous build are still readable

#### Scenario: Artifacts reflect the current unit

- **WHEN** a marked function is edited and the project is rebuilt
- **THEN** the IR and target source on disk describe the edited function
