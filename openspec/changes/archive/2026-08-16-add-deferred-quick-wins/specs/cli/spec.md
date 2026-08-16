## Purpose

The command-line entry point for inspecting what a Python file compiles to, without a build, an
interpreter, or a decorator. It is the tool for answering "what does this actually become?" —
during development of the compiler itself, and for a user diagnosing why their function was
rejected.

## ADDED Requirements

### Requirement: A file is compiled and reported

The CLI SHALL accept a path to a Python file, run it through the pipeline, and report the result.
Invoked with no path, it SHALL explain its usage and exit unsuccessfully.

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

### Requirement: Diagnostics carry their location

A rejection SHALL be reported with the `line:column` at which it occurred and a message naming
the construct, matching the diagnostic a user would get from the decorator. Somebody debugging a
rejection should not get a different answer depending on which entry point they used.

#### Scenario: A subset violation reports where it is

- **WHEN** the CLI is run on a file whose third line is outside the subset
- **THEN** the reported location names line 3

#### Scenario: A syntax error is reported as such

- **WHEN** the CLI is run on a file that is not valid Python
- **THEN** it reports a syntax error with a location

#### Scenario: Diagnostics go to the error stream

- **WHEN** a file is rejected
- **THEN** the diagnostic is written to the error stream, leaving the output stream empty for
  redirection

### Requirement: The output form is selectable

The CLI SHALL accept a flag choosing what to report: a human-readable summary of the compiled
unit, the IR artifact, or the generated target source. The summary SHALL be the default, since it
is the smallest useful answer.

Being able to read the generated source without a build is the point: producing it otherwise means
running a full toolchain build and locating the file it wrote.

#### Scenario: The default is a summary

- **WHEN** the CLI is run with no output flag
- **THEN** it reports the unit fingerprint and each function's name, parameter count, and return
  type

#### Scenario: The IR can be emitted

- **WHEN** the CLI is asked for the IR
- **THEN** it writes the IR artifact, in the same form the build pipeline writes to disk

#### Scenario: The generated source can be emitted

- **WHEN** the CLI is asked for the generated source
- **THEN** it writes the target source for the selected backend, without performing a build

#### Scenario: Emitted output is written to the output stream

- **WHEN** any form is emitted
- **THEN** it goes to the output stream, so it can be redirected to a file or piped

#### Scenario: An unrecognized output form is refused

- **WHEN** the CLI is asked for a form it does not produce
- **THEN** it reports the accepted forms and exits unsuccessfully

### Requirement: The backend is selectable

The CLI SHALL accept a backend name, defaulting to the implemented one, and SHALL report a
reserved or unknown name with the same distinction the rest of compylr makes: a reserved target is
reported as planned, an unrecognized one as unknown.

#### Scenario: The default backend is used

- **WHEN** the CLI emits generated source with no backend named
- **THEN** it uses the implemented default

#### Scenario: A reserved backend is reported as planned

- **WHEN** the CLI is asked to emit for a reserved but unimplemented backend
- **THEN** it reports that the backend is not implemented yet and exits unsuccessfully

#### Scenario: An unknown backend lists what is available

- **WHEN** the CLI is asked to emit for a name that is not a backend
- **THEN** it names the available backends and exits unsuccessfully
