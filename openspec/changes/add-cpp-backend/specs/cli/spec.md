## MODIFIED Requirements

### Requirement: The backend is selectable
The CLI SHALL accept a backend name, defaulting to the implemented one, and SHALL report a
reserved or unknown name with the same distinction the rest of compylr makes: a reserved target is
reported as planned, an unrecognized one as unknown. The CLI SHALL accept `--backend go` to emit
Go source code, and `--backend cpp` to emit C++ source code.

The set of names the CLI accepts SHALL be taken from the backend registry rather than from a list
the CLI maintains, so that a backend added to the registry is selectable without the CLI being
edited.

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

#### Scenario: Emitting C++ backend source from CLI
- **GIVEN** a source file the frontend accepts
- **WHEN** the CLI is run with `--backend cpp` and asked for the generated source
- **THEN** it writes the translated C++ to the output stream
- **AND** it performs no build

#### Scenario: The accepted names come from the registry
- **GIVEN** a backend registered as implemented
- **WHEN** the CLI is asked to emit for that backend's name
- **THEN** it is accepted without the CLI having been edited to name it

### Requirement: The whole crate can be written to a directory

The CLI SHALL be able to write every generated file to a directory named by the caller, so that
what compylr would build can be compiled, diffed, or committed without running a build first.

The directory SHALL be required rather than defaulting: writing several files somewhere the user
did not name is a side effect a command-line tool should not have.

What is written SHALL be complete for the **selected backend**, including that target's own build
manifest, so the tree builds with no file added by hand whatever the target's build system is.

#### Scenario: Every file is written

- **WHEN** the CLI is asked to write a crate to a directory
- **THEN** each generated file appears under that directory at its relative path

#### Scenario: The result compiles

- **WHEN** a crate written this way is compiled
- **THEN** it builds, because the files written are exactly the ones the build pipeline would use

#### Scenario: The written tree carries the selected target's manifest

- **GIVEN** a source file the frontend accepts
- **WHEN** a crate is written with the `cpp` backend selected
- **THEN** the directory holds that target's build manifest
- **AND** building it requires no file to be added by hand

#### Scenario: The destination is required

- **WHEN** the CLI is asked for the crate form with no destination
- **THEN** it reports that a destination is needed and exits unsuccessfully

#### Scenario: A missing directory is created

- **WHEN** the named destination does not exist
- **THEN** it is created, rather than failing on a path the user clearly intended

#### Scenario: Nothing is written to the output stream

- **WHEN** a crate is written to a directory
- **THEN** the output stream carries at most a report of what was written, never source, since the
  source went to files

#### Scenario: Writing a crate performs no build

- **WHEN** a crate is written
- **THEN** no toolchain is invoked, so the command works on a machine with none of the selected
  target's build tools installed
