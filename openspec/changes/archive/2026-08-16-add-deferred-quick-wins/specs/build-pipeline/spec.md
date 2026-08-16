## MODIFIED Requirements

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
