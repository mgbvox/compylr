## ADDED Requirements

### Requirement: Python bindings are the bridge for one source/target pair

Generating the code that makes a compiled unit callable from Python SHALL be the responsibility of a
component registered for the pair `(python, rust)`, not of the Rust backend and not of the Python
frontend. The Rust backend SHALL remain able to generate target source with no Python bridge
present, and adding a second target SHALL NOT change this component.

#### Scenario: The bridge is selected by the pair

- **WHEN** a unit lowered by the Python frontend is compiled for the Rust target and a callable
  artifact is requested
- **THEN** the `(python, rust)` bridge is selected and generates the binding layer

#### Scenario: The backend generates without the bridge

- **WHEN** target source is requested without a callable artifact
- **THEN** the Rust backend emits it, and no Python-specific code is generated

#### Scenario: A second target does not touch this bridge

- **WHEN** a backend for another target is added
- **THEN** the `(python, rust)` bridge is unchanged

### Requirement: An unbridged pair is reported as such

Requesting a callable artifact for a source and target that have no registered bridge SHALL fail
with an error naming both languages and stating that generation is available but calling back is
not. It SHALL NOT be reported as an unknown backend, an unknown frontend, or an internal error.

#### Scenario: Generation succeeds, bridging does not

- **WHEN** a callable artifact is requested for a pair whose backend is implemented but whose bridge
  is not
- **THEN** the failure names both languages and distinguishes itself from an unknown-target failure

#### Scenario: A caller can branch on the case

- **WHEN** a caller needs to distinguish an unbridged pair from an unimplemented target
- **THEN** it can do so from the failure's kind without matching on rendered text

### Requirement: The binding layer is generated from the IR alone

The bridge SHALL derive every exposed name, signature, and conversion from the IR, without reading
the original Python source and without depending on the Python parser. Error mapping SHALL be
derived from the errors the IR's operations can produce, so that a target error has one Python
exception regardless of which frontend construct produced it.

#### Scenario: No source is consulted

- **WHEN** a binding layer is generated from a unit read back from its serialized artifact
- **THEN** it is identical to the one generated from the same unit in memory

#### Scenario: The bridge does not depend on the parser

- **WHEN** the bridge component's dependencies are inspected
- **THEN** it does not depend on a Python parser

## MODIFIED Requirements

### Requirement: The unit becomes a single importable module

A compiled unit SHALL be exposed as ONE Python extension module containing every function in
the unit. The module's name SHALL NOT be part of the user-facing API: callers reach compiled
functions through the objects they marked, never by importing the module themselves. Keeping
the name an implementation detail is what allows it to encode build identity, which in turn is
what allows a rebuilt unit to be loaded by a process that has already loaded an earlier one —
an extension module cannot be reliably re-imported under a name already in use. The build identity
the name encodes SHALL distinguish builds that differ only in the target language or in the pass
configuration that produced them, so that switching either does not collide with an already-loaded
module.

#### Scenario: Every function is exposed

- **WHEN** a unit holding three compiled functions is built and imported
- **THEN** all three are accessible as attributes of the module

#### Scenario: Callers never name the module

- **WHEN** a user calls a marked function
- **THEN** no import of the generated module appears in their code

#### Scenario: A rebuilt unit loads in a process that already loaded its predecessor

- **WHEN** a function is marked after a build has occurred, and calling it forces a rebuild
- **THEN** the rebuilt unit is loaded and used in that same process

#### Scenario: Nothing beyond the unit is exposed

- **WHEN** a compiled module is imported
- **THEN** only the unit's functions and standard module attributes are present, so helper
  code emitted by the backend is not reachable as public API

#### Scenario: Builds differing only in configuration do not collide

- **WHEN** the same unit is built twice under different pass configurations and both are loaded in
  one process
- **THEN** each is loaded under its own module name
