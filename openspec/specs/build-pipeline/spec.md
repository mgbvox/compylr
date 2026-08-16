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

- **WHEN** a project marks three functions for compilation
- **THEN** exactly one build is performed and one extension module is produced

#### Scenario: A fourth function joins the existing three

- **WHEN** a fourth function is marked and the project is run again
- **THEN** the single shared artifact is rebuilt to contain all four, rather than a second
  artifact being produced

#### Scenario: Compiled functions can call each other

- **WHEN** one marked function calls another marked function
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

### Requirement: Build artifacts are isolated from the user's source

Generated files SHALL live under a single predictable directory that is separate from the
user's own source, so that they can be inspected, deleted, or excluded from version control as
a unit, and so no generated file is ever mistaken for hand-written code.

The directory SHALL be a property of the **project**, not of the shell. It SHALL be located by
searching upward from the working directory for a project marker, so that running the same project
from a subdirectory reuses the same artifacts instead of building a second copy. When no marker is
found, the working directory SHALL be used, so a script in an unmarked directory still works.

#### Scenario: All generated files share one root

- **WHEN** a build completes
- **THEN** every file it generated is under one directory

#### Scenario: Deleting the directory is safe

- **WHEN** the directory is deleted and the project is run again
- **THEN** the project rebuilds from scratch and behaves identically

#### Scenario: Running from a subdirectory reuses the same artifacts

- **WHEN** a project is built once from its root and then run again from a subdirectory
- **THEN** the second run reuses the existing artifacts and does not invoke the toolchain

#### Scenario: An existing artifact directory is itself a marker

- **WHEN** a project has been built before and is run again from a subdirectory beneath it
- **THEN** the existing directory is found rather than a new one created

#### Scenario: No marker falls back to the working directory

- **WHEN** a script is run from a directory with no project marker above it
- **THEN** artifacts are created under the working directory

#### Scenario: The search does not escape into unrelated directories

- **WHEN** the search reaches the filesystem root without finding a marker
- **THEN** it stops and falls back, rather than selecting an arbitrary ancestor

#### Scenario: An explicit location overrides discovery

- **WHEN** a caller states where artifacts should live
- **THEN** that location is used and no search is performed

### Requirement: A successful build yields an importable module

A build SHALL produce an extension module importable by the interpreter running the project,
without the user taking any further step.

#### Scenario: Import after build

- **WHEN** a build completes
- **THEN** the compiled module can be imported in the same process that triggered the build

#### Scenario: Available on a later run

- **WHEN** the project is run again in a new process
- **THEN** the compiled module is importable without rebuilding

### Requirement: Rebuilds are keyed on the IR fingerprint

The decision to rebuild SHALL compare the current unit's fingerprint against the fingerprint of
the last successful build. A rebuild SHALL occur when they differ and SHALL be skipped when
they match. Because the fingerprint is computed over the IR rather than the source text,
changes that do not alter meaning SHALL NOT trigger a rebuild.

#### Scenario: Unchanged project skips the build

- **WHEN** a project is run twice with no changes
- **THEN** the second run does not invoke the toolchain

#### Scenario: Reformatting does not trigger a rebuild

- **WHEN** comments are added and a marked function is reindented, with no change in meaning
- **THEN** the next run does not invoke the toolchain

#### Scenario: An edit triggers a rebuild

- **WHEN** a marked function's body is changed to compute something different
- **THEN** the next run rebuilds

#### Scenario: Marking an additional function triggers a rebuild

- **WHEN** a function is newly marked for compilation
- **THEN** the next run rebuilds

#### Scenario: A failed build is not recorded as successful

- **WHEN** a build fails and the project is run again with no changes
- **THEN** the build is attempted again rather than skipped

### Requirement: Build failures are reported, never swallowed

When the toolchain fails, the pipeline SHALL raise an error that includes the toolchain's own
output. It SHALL NOT fall back to interpreted execution silently: a user who asked for compiled
code and got interpreted code without being told would be measuring the wrong thing.

#### Scenario: Toolchain reports a compile error

- **WHEN** the generated source fails to compile
- **THEN** an error is raised that includes the toolchain's diagnostics

#### Scenario: No silent fallback

- **WHEN** a build fails
- **THEN** the failure surfaces to the caller rather than execution continuing with the
  interpreted function

### Requirement: A missing toolchain is diagnosed clearly

Compiling requires build tools that are not guaranteed to be present. When a required tool is
missing, the pipeline SHALL say which one and how to install it, rather than surfacing a
file-not-found error.

#### Scenario: Rust toolchain absent

- **WHEN** a build is attempted with no Rust compiler available
- **THEN** the error names the missing toolchain and states how to install it

#### Scenario: Build tool absent

- **WHEN** a build is attempted with the extension-module build tool unavailable
- **THEN** the error names it and states how to install it

#### Scenario: The check happens before work is wasted

- **WHEN** required tools are missing
- **THEN** the failure is reported before a build is attempted
