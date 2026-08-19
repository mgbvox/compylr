## ADDED Requirements

### Requirement: A project can be compiled programmatically

The package SHALL expose an entry point that takes a project root, discovers everything marked
beneath it, builds once, and reports what it found and did.

The command-line form SHALL be a thin wrapper over it. Anything the command decides that the
programmatic form does not is a place the two can disagree, and a user debugging a precompile
should not have to work out which one they are looking at.

#### Scenario: A root is compiled programmatically

- **WHEN** the entry point is called with a project root containing marked functions
- **THEN** the artifact is built and a report is returned

#### Scenario: The report names what was found

- **WHEN** the entry point returns
- **THEN** the report carries the modules imported, the functions and classes found, and whether a
  build occurred

#### Scenario: An empty project is not an error

- **WHEN** the root contains nothing marked
- **THEN** the report says so and no build is attempted

#### Scenario: Import failures are reported rather than raised

- **WHEN** one module cannot be imported
- **THEN** the report carries the failure and the others are still processed

#### Scenario: A build failure raises

- **WHEN** the toolchain fails
- **THEN** the same error is raised as when a call triggers the build, carrying the toolchain output

#### Scenario: The command adds no behaviour of its own

- **WHEN** the same root is compiled through the command and through the entry point
- **THEN** both produce the same artifact and the same outcome
