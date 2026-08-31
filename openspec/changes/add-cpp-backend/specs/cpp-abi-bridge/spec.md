## Purpose

Defines the single C-ABI export surface generated C++ presents to any calling runtime, and the
contract a per-frontend loader implements against it — so that making C++ callable from a new
source language costs one loader rather than one whole bridge, while bridges still resolve by the
`(source, target)` pair.

## ADDED Requirements

### Requirement: One target-side export surface serves every source language

Generated C++ SHALL be made callable through a **single** C-ABI export surface that is generated
once per unit and is independent of which source language will call it. That surface SHALL contain
no name, spelling, type, or convention belonging to any source language.

A source language SHALL be added to the set that can call C++ by supplying only a loader for that
surface. Adding one SHALL NOT require changing the export surface, the C++ backend, or any existing
loader.

This is the canonical-C-ABI hub the bridge model
[defers rather than forecloses](../../../../../crates/compylr-core/src/bridge.rs#L18).

#### Scenario: The export surface names no source language

- **GIVEN** a unit lowered from any frontend
- **WHEN** the C-ABI export surface is generated for it
- **THEN** the generated surface contains no spelling, keyword, or type belonging to a source
  language

#### Scenario: Two source languages receive the same target-side artifact

- **GIVEN** two units holding the same IR, one lowered from Python and one from TypeScript
- **WHEN** each is emitted through its C++ bridge
- **THEN** every file the two artifacts share by path is byte-identical
- **AND** the files that differ are only the loader and its type declarations

#### Scenario: Adding a source language does not disturb the others

- **GIVEN** a registered `(source, cpp)` bridge for each of two source languages
- **WHEN** a third source language's loader is added
- **THEN** neither existing bridge's emitted artifact changes

### Requirement: A pair bridge is still resolved by its pair

Each `(source, cpp)` combination that can be called SHALL be registered as its own bridge and SHALL
resolve by the pair, as every other bridge does. Sharing a target-side surface SHALL NOT be
observable in how a bridge is selected, and an unregistered pair SHALL still report that the pair is
not bridged rather than that the target is unavailable.

#### Scenario: Python to C++ resolves

- **WHEN** the `(python, cpp)` pair is resolved from the bridge registry
- **THEN** resolution succeeds and returns a bridge whose source is `python` and whose target is
  `cpp`

#### Scenario: TypeScript to C++ resolves

- **WHEN** the `(typescript, cpp)` pair is resolved from the bridge registry
- **THEN** resolution succeeds and returns a bridge whose source is `typescript` and whose target
  is `cpp`

#### Scenario: An unregistered pair is still unbridged

- **GIVEN** a source language with no registered C++ loader
- **WHEN** that pair is resolved from the bridge registry
- **THEN** resolution fails naming both languages
- **AND** the failure says the pair is not bridged, not that the target is unavailable

### Requirement: Values cross the boundary by an explicit marshalling contract

The export surface SHALL define, for every IR type, how a value of that type crosses the boundary
in each direction, and a loader SHALL implement only that contract.

Scalars and text SHALL cross by value. A collection SHALL cross by value, so a mutation a callee
performs on a collection parameter is not observable to the caller. An instance SHALL cross as an
opaque handle whose lifetime the calling runtime owns, so that a mutated attribute **is** observable
to the caller on the next call — the contrast the accepted subset already draws.

Memory allocated on one side of the boundary SHALL be released through the surface that allocated
it, and no loader SHALL free memory it did not allocate.

#### Scenario: A scalar crosses and returns

- **GIVEN** a built artifact exporting a function that adds two integers
- **WHEN** the loader calls it with `2` and `3`
- **THEN** the calling runtime receives `5`

#### Scenario: Text crosses as UTF-8

- **GIVEN** a built artifact exporting a function that returns the text it was given
- **WHEN** the loader calls it with a string holding a non-ASCII character
- **THEN** the calling runtime receives an equal string

#### Scenario: A collection parameter is a copy

- **GIVEN** a built artifact exporting a function that appends to a sequence parameter
- **WHEN** the loader calls it with a sequence
- **THEN** the caller's own sequence is unchanged

#### Scenario: An instance is a handle whose state persists

- **GIVEN** a built artifact exporting a class whose method increments an attribute
- **WHEN** the loader constructs an instance and calls that method twice
- **THEN** reading the attribute reflects both calls

#### Scenario: A handle is released exactly once

- **GIVEN** a built artifact and an instance the loader has constructed
- **WHEN** the calling runtime releases that instance
- **THEN** the artifact's allocation for it is freed
- **AND** no further release of the same handle occurs

### Requirement: A returned failure becomes the calling language's own failure

The export surface SHALL carry a failure out of a call as data rather than as an exception, and each
loader SHALL translate that data into the idiomatic failure of its own runtime, carrying the
message.

A call that succeeds SHALL be distinguishable from one that failed without inspecting the returned
value.

#### Scenario: A failure reaches Python as an exception

- **GIVEN** a built artifact exporting a function that divides under a behavior that reports
  division by zero
- **WHEN** the Python loader calls it with a zero divisor
- **THEN** a Python exception is raised whose message names division by zero

#### Scenario: A failure reaches TypeScript as a thrown Error

- **GIVEN** the same built artifact
- **WHEN** the TypeScript loader calls it with a zero divisor
- **THEN** an `Error` is thrown whose message names division by zero

#### Scenario: Success and failure are distinguishable

- **GIVEN** a built artifact exporting a fallible function whose success value may be zero
- **WHEN** the loader calls it and it succeeds returning zero
- **THEN** the loader reports success rather than a failure

### Requirement: The loadable name distinguishes builds, not only programs

The name a built C++ artifact is loaded under SHALL encode the program's fingerprint **and** a tag
over the rest of the build key, so that the same program built for a different target or under a
different pass configuration does not collide with an artifact a process has already loaded.

#### Scenario: The name carries the fingerprint

- **GIVEN** a unit and a build key
- **WHEN** the artifact is emitted through a C++ bridge
- **THEN** the name it is loaded under contains the unit's fingerprint

#### Scenario: Two pass configurations do not collide

- **GIVEN** one unit
- **WHEN** it is emitted under two different pass configurations
- **THEN** the two artifacts are loaded under different names

### Requirement: A bridge crate links no host runtime and parses no source language

A crate implementing a `(source, cpp)` bridge generates loader source as **text**. It SHALL NOT
depend on the host language's runtime and SHALL NOT depend on a parser for any source language,
for the same reason the existing bridges do not: a crate below the host layer that linked one would
only work where that language is present.

This is enforced by
[`crate_boundaries.rs`](../../../../../crates/compylr-host-python/tests/crate_boundaries.rs#L133)
rather than by convention.

#### Scenario: No bridge crate links a host runtime

- **WHEN** the workspace manifests are read
- **THEN** no crate implementing a C++ bridge declares a dependency on a host language runtime

#### Scenario: No bridge crate parses a source language

- **WHEN** the workspace manifests are read
- **THEN** no crate implementing a C++ bridge declares a dependency on a source-language parser
