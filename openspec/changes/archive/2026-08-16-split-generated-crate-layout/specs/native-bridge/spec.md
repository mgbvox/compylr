## MODIFIED Requirements

### Requirement: Compilation is reachable from Python

The compiler SHALL be importable from Python and SHALL accept a collection of Python source
texts together with a backend name, returning the artifacts of a successful compilation. It
SHALL accept source TEXT rather than file paths, because the decorator obtains source by
introspecting a live function object and no file may correspond to it.

Generated target code SHALL be reported as a **mapping from relative path to contents**, since a
backend emits a crate of files rather than one source string. The paths SHALL be relative, so a
caller decides where the crate is written.

#### Scenario: Compiling one source

- **WHEN** a single source text containing one supported function is compiled for the `rust`
  backend
- **THEN** compilation succeeds and returns the generated target files, the IR artifact, and
  the unit fingerprint

#### Scenario: The generated files are reported individually

- **WHEN** a unit is compiled
- **THEN** each generated file is reported under its own relative path, rather than concatenated

#### Scenario: Paths are relative

- **WHEN** the reported paths are inspected
- **THEN** none is absolute, so the caller chooses where the crate lands

#### Scenario: Source text with no file behind it

- **WHEN** source text obtained by introspection, not read from disk, is compiled
- **THEN** compilation succeeds

#### Scenario: Compiling an empty collection

- **WHEN** no sources are supplied
- **THEN** compilation succeeds and reports an empty unit, rather than failing
