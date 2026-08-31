## Purpose

Turns generated target source into an importable extension module: where a project's build
artifacts live on disk, how the intermediate IR and target source are preserved for
inspection, when a rebuild is required, and what the user is told when the build cannot run.

## Requirements

### Requirement: One shared artifact per project

All functions a project marks for compilation SHALL be compiled into ONE build artifact, not
one per function. Compiling N functions SHALL invoke the toolchain once, because a per-function
artifact would multiply build cost by N and prevent compiled functions from calling each other.

#### Scenario: Three functions, one build

- **GIVEN** a project marking three functions for compilation
- **WHEN** the project is built
- **THEN** exactly one build is performed and one extension module is produced

#### Scenario: A fourth function joins the existing three

- **GIVEN** a project with three marked functions already built
- **WHEN** a fourth is marked and the project is run again
- **THEN** the single shared artifact is rebuilt to contain all four, rather than a second
  artifact being produced

#### Scenario: Compiled functions can call each other

- **GIVEN** a project where one marked function calls another
- **WHEN** the project is built
- **THEN** the built artifact resolves the call internally

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

- **GIVEN** a project that has been built
- **WHEN** the artifact directory is inspected
- **THEN** the unit's IR is present on disk in a documented, readable format

#### Scenario: Target source is written

- **GIVEN** a project built for the `rust` backend
- **WHEN** the artifact directory is inspected
- **THEN** every generated file is present on disk at its reported path

#### Scenario: A stale file is removed

- **GIVEN** a build that wrote a file the next build does not
- **WHEN** the later build runs
- **THEN** the later build removes it, rather than leaving a file no longer part of the crate

#### Scenario: Hand-written files in the crate are untouched

- **GIVEN** a crate directory also holding the build manifest and build configuration
- **WHEN** a build runs against it
- **THEN** those are preserved, since pruning applies to generated source rather than to
  everything present

#### Scenario: Artifacts survive a skipped rebuild

- **GIVEN** a project whose artifact is up to date
- **WHEN** a run reuses it without rebuilding
- **THEN** the IR and target source from the previous build are still readable

#### Scenario: Artifacts reflect the current unit

- **GIVEN** a project with an edited marked function
- **WHEN** the project is rebuilt
- **THEN** the IR and target source on disk describe the edited function

### Requirement: Build artifacts are isolated from the user's source

Generated files SHALL live under a single predictable directory that is separate from the
user's own source, so that they can be inspected, deleted, or excluded from version control as
a unit, and so no generated file is ever mistaken for hand-written code.

The directory SHALL be a property of the **project**, not of the shell. It SHALL be located by
searching upward from the working directory for a project marker, so that running the same project
from a subdirectory reuses the same artifacts instead of building a second copy. When no marker is
found, the working directory SHALL be used, so a script in an unmarked directory still works.

#### Scenario: All generated files share one root

- **GIVEN** a project that has been built
- **WHEN** the generated files are located
- **THEN** every file it generated is under one directory

#### Scenario: Deleting the directory is safe

- **GIVEN** a project whose artifact directory has been deleted
- **WHEN** the project is run again
- **THEN** the project rebuilds from scratch and behaves identically

#### Scenario: Running from a subdirectory reuses the same artifacts

- **GIVEN** a project already built once from its root
- **WHEN** it is run again from a subdirectory
- **THEN** the second run reuses the existing artifacts and does not invoke the toolchain

#### Scenario: An existing artifact directory is itself a marker

- **GIVEN** a project built before, with an artifact directory above the working directory
- **WHEN** it is run again from a subdirectory beneath it
- **THEN** the existing directory is found rather than a new one created

#### Scenario: No marker falls back to the working directory

- **GIVEN** a script in a directory with no project marker above it
- **WHEN** it is run
- **THEN** artifacts are created under the working directory

#### Scenario: The search does not escape into unrelated directories

- **GIVEN** a search that reaches the filesystem root without finding a marker
- **WHEN** the location is decided
- **THEN** it stops and falls back, rather than selecting an arbitrary ancestor

#### Scenario: An explicit location overrides discovery

- **GIVEN** a caller stating where artifacts should live
- **WHEN** the location is decided
- **THEN** that location is used and no search is performed

### Requirement: A successful build yields an importable module

A build SHALL produce an extension module importable by the interpreter running the project,
without the user taking any further step.

#### Scenario: Import after build

- **GIVEN** a project that has just been built
- **WHEN** the compiled module is imported in the same process
- **THEN** the compiled module can be imported in the same process that triggered the build

#### Scenario: Available on a later run

- **GIVEN** a project built in an earlier process
- **WHEN** it is run again in a new process
- **THEN** the compiled module is importable without rebuilding

### Requirement: Rebuilds are keyed on the IR fingerprint

The decision to rebuild SHALL compare the current unit's fingerprint against the fingerprint of
the last successful build. A rebuild SHALL occur when they differ and SHALL be skipped when
they match. Because the fingerprint is computed over the IR rather than the source text,
changes that do not alter meaning SHALL NOT trigger a rebuild.

#### Scenario: Unchanged project skips the build

- **GIVEN** a project that has been built and not changed
- **WHEN** it is run again
- **THEN** the second run does not invoke the toolchain

#### Scenario: Reformatting does not trigger a rebuild

- **GIVEN** a built project whose marked function gains comments and is reindented, with no
  change in meaning
- **WHEN** it is run again
- **THEN** the next run does not invoke the toolchain

#### Scenario: An edit triggers a rebuild

- **GIVEN** a built project whose marked function now computes something different
- **WHEN** it is run again
- **THEN** the next run rebuilds

#### Scenario: Marking an additional function triggers a rebuild

- **GIVEN** a built project with a newly marked function
- **WHEN** it is run again
- **THEN** the next run rebuilds

#### Scenario: A failed build is not recorded as successful

- **GIVEN** a project whose build failed and which has not changed
- **WHEN** it is run again
- **THEN** the build is attempted again rather than skipped

### Requirement: Build failures are reported, never swallowed

When the toolchain fails, the pipeline SHALL raise an error that includes the toolchain's own
output. It SHALL NOT fall back to interpreted execution silently: a user who asked for compiled
code and got interpreted code without being told would be measuring the wrong thing.

#### Scenario: Toolchain reports a compile error

- **GIVEN** generated source that fails to compile
- **WHEN** the build runs
- **THEN** an error is raised that includes the toolchain's diagnostics

#### Scenario: No silent fallback

- **GIVEN** a build that fails
- **WHEN** the failure is handled
- **THEN** the failure surfaces to the caller rather than execution continuing with the
  interpreted function

### Requirement: A missing toolchain is diagnosed clearly

Compiling requires build tools that are not guaranteed to be present. When a required tool is
missing, the pipeline SHALL say which one and how to install it, rather than surfacing a
file-not-found error.

#### Scenario: Rust toolchain absent

- **GIVEN** a machine with no Rust compiler available
- **WHEN** a build is attempted
- **THEN** the error names the missing toolchain and states how to install it

#### Scenario: Build tool absent

- **GIVEN** a machine without the extension-module build tool
- **WHEN** a build is attempted
- **THEN** the error names it and states how to install it

#### Scenario: The check happens before work is wasted

- **GIVEN** a machine missing a required tool
- **WHEN** a build is attempted
- **THEN** the failure is reported before a build is attempted

### Requirement: A build can be driven without a call

The pipeline SHALL be able to build a project's artifact without any marked function having been
called, so that the build can be performed ahead of time.

Building ahead SHALL produce exactly what calling would have produced: the same artifact, keyed on
the same fingerprint, so a later run reuses it rather than rebuilding.

#### Scenario: Building ahead produces a usable artifact

- **GIVEN** a project with no marked function yet called
- **WHEN** the project is built ahead of time
- **THEN** the artifact is written and a later run reuses it

#### Scenario: The fingerprint is the same either way

- **GIVEN** one project
- **WHEN** it is built ahead of time and, separately, by calling a function
- **THEN** both record the same fingerprint

#### Scenario: The artifact directory is the project's

- **GIVEN** a project being built from a different working directory
- **WHEN** it is built ahead of time
- **THEN** the artifacts land in the project's own directory, found the same way a run finds it

#### Scenario: An already-current project is not rebuilt

- **GIVEN** a project whose artifact is current
- **WHEN** it is built ahead of time
- **THEN** building ahead does not invoke the toolchain

#### Scenario: Toolchain requirements are unchanged

- **GIVEN** a machine missing a required build tool
- **WHEN** a project is built ahead of time
- **THEN** the same diagnostic is reported as when a call triggers the build

### Requirement: The generated crate is built under an explicit release profile

The generated crate's manifest SHALL declare its own release profile rather than inheriting
Cargo's defaults. The artifact is built once per fingerprint and imported on every subsequent run,
so build time is the cheap side of that trade and run time is the expensive one.

The profile SHALL at minimum enable link-time optimization and a single codegen unit. This is not a
generic "make it faster" setting: the runtime helpers are emitted into a different module from the
code that calls them, and at Cargo's default of sixteen codegen units they are frequently not
inlined — which matters here in particular because every arithmetic operation in the supported
subset is emitted as a trait call by design.

The profile SHALL NOT select a target CPU. A generated crate may be copied to another machine, and
an artifact that faults on an unsupported instruction is a worse outcome than a slower one.

#### Scenario: The manifest declares a release profile

- **GIVEN** a project being compiled
- **WHEN** the generated crate's manifest is written
- **THEN** it contains a release profile section declaring link-time optimization and a single
  codegen unit

#### Scenario: The build still succeeds end to end

- **GIVEN** a generated crate carrying the release profile
- **WHEN** the project is compiled
- **THEN** the crate builds and the resulting module imports and runs as before

#### Scenario: The artifact stays portable

- **GIVEN** a project being compiled
- **WHEN** the generated crate's manifest and cargo configuration are written
- **THEN** neither pins a target CPU, so the built artifact does not depend on the machine that
  built it

#### Scenario: Panics still reach Python as exceptions

- **GIVEN** a generated crate carrying the release profile
- **WHEN** the profile's settings are read
- **THEN** it preserves unwinding, because the bridge converts a panic into a Python exception and
  aborting would terminate the interpreter instead
