## ADDED Requirements

### Requirement: A build can be driven without a call

The pipeline SHALL be able to build a project's artifact without any marked function having been
called, so that the build can be performed ahead of time.

Building ahead SHALL produce exactly what calling would have produced: the same artifact, keyed on
the same fingerprint, so a later run reuses it rather than rebuilding.

#### Scenario: Building ahead produces a usable artifact

- **WHEN** a project is built without any marked function being called
- **THEN** the artifact is written and a later run reuses it

#### Scenario: The fingerprint is the same either way

- **WHEN** a project is built ahead of time and, separately, by calling a function
- **THEN** both record the same fingerprint

#### Scenario: The artifact directory is the project's

- **WHEN** a project is built ahead of time from a different working directory
- **THEN** the artifacts land in the project's own directory, found the same way a run finds it

#### Scenario: An already-current project is not rebuilt

- **WHEN** the artifact is current
- **THEN** building ahead does not invoke the toolchain

#### Scenario: Toolchain requirements are unchanged

- **WHEN** a required build tool is missing
- **THEN** the same diagnostic is reported as when a call triggers the build
