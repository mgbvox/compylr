## ADDED Requirements

### Requirement: Behavior travels with each source

The bridge SHALL accept a behavior alongside each source text, so that members of one project
marked with different behaviors can be compiled into one unit. A source supplied with no behavior
SHALL be lowered under the source language's stance on every axis.

The behavior SHALL be supplied per source rather than per call, because the decorator captures each
marked member as its own source and a per-call setting could not express a project whose members
differ.

#### Scenario: Each source keeps its own behavior

- **WHEN** two sources are compiled together, one with the source language's behavior and one with
  the target's
- **THEN** each resulting function carries the modes of the behavior its own source was given

#### Scenario: An omitted behavior is the source language's

- **WHEN** a source is compiled with no behavior supplied
- **THEN** it is lowered under the source language's stance on every axis

#### Scenario: A cross-behavior call still resolves

- **WHEN** a function in one source under one behavior calls a function in another source under a
  different behavior
- **THEN** the call is typed and resolved exactly as a same-behavior call would be

### Requirement: Behavior can be validated without compiling

The bridge SHALL expose a way to check a behavior against a source and target language pair and
report whether it is valid, without lowering any source. This is what allows the decorator to
reject a bad behavior as it runs, rather than at a build reached much later.

The check SHALL distinguish the same cases the behavior model does: a language compylr does not
know, a language it knows that is not one of the two here, and an axis that does not exist.

#### Scenario: A valid behavior checks clean

- **WHEN** a behavior naming only the source and target languages is checked for that pair
- **THEN** the check succeeds

#### Scenario: An invalid language is reported

- **WHEN** a behavior naming a third language is checked
- **THEN** the check fails with a message naming the two languages that would have been accepted

#### Scenario: The check compiles nothing

- **WHEN** a behavior is checked
- **THEN** no source is parsed and no target source is generated

#### Scenario: The failure category is machine-readable

- **WHEN** a behavior check fails
- **THEN** the failure carries a stable category, so a caller can branch on it without matching
  prose

## MODIFIED Requirements

### Requirement: Compilation is reachable from Python

The compiler SHALL be importable from Python and SHALL accept a collection of Python source
texts, each with its behavior, together with a backend name, returning the artifacts of a
successful compilation. It SHALL accept source TEXT rather than file paths, because the decorator
obtains source by introspecting a live function object and no file may correspond to it.

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

#### Scenario: Behavior changes the fingerprint

- **WHEN** the same source is compiled twice under two different behaviors
- **THEN** the two compilations report different fingerprints
