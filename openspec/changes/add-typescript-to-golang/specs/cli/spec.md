## MODIFIED Requirements

### Requirement: A file is compiled and reported
The CLI SHALL accept a path to a source file, run it through the pipeline, and report the result.
Invoked with no path, it SHALL explain its usage and exit unsuccessfully. The CLI SHALL support
selecting frontends including `--frontend typescript` to compile TypeScript source files in addition
to the default Python frontend.

Exit status SHALL distinguish success from failure, so the CLI can be used in a script without
parsing its output.

#### Scenario: A supported file is reported
- **WHEN** the CLI is run on a file inside the supported subset
- **THEN** it reports the result and exits successfully

#### Scenario: No arguments
- **WHEN** the CLI is run with no path
- **THEN** it prints usage and exits unsuccessfully

#### Scenario: A missing file
- **WHEN** the CLI is run on a path that does not exist
- **THEN** it reports the path it could not read and exits unsuccessfully

#### Scenario: A rejected program exits unsuccessfully
- **WHEN** the CLI is run on a file outside the supported subset
- **THEN** it exits unsuccessfully

#### Scenario: Compiling a TypeScript file via CLI
- **WHEN** the CLI is run with `--frontend typescript <file.ts>`
- **THEN** it compiles the TypeScript file and prints a human-readable summary of the unit

### Requirement: The backend is selectable
The CLI SHALL accept a backend name, defaulting to the implemented one, and SHALL report a
reserved or unknown name with the same distinction the rest of compylr makes: a reserved target is
reported as planned, an unrecognized one as unknown. The CLI SHALL accept `--backend go` to emit
Go source code.

#### Scenario: The default backend is used
- **WHEN** the CLI emits generated source with no backend named
- **THEN** it uses the implemented default

#### Scenario: A reserved backend is reported as planned
- **WHEN** the CLI is asked to emit for a reserved but unimplemented backend
- **THEN** it reports that the backend is not implemented yet and exits unsuccessfully

#### Scenario: An unknown backend lists what is available
- **WHEN** the CLI is asked to emit for a name that is not a backend
- **THEN** it names the available backends and exits unsuccessfully

#### Scenario: Emitting Go backend source from CLI
- **WHEN** the CLI is run with `--backend go --emit source <file>`
- **THEN** it outputs the generated Go source code to stdout
