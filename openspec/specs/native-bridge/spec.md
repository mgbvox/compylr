## Purpose

Exposes the compiler — frontend, lowering, and backends — to Python as an extension module, so
the Python package can compile source text in-process instead of shelling out. It is also the
boundary where compylr's diagnostics stop being Rust errors and become Python exceptions.

## Requirements

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

### Requirement: Sources are assembled into one unit

The bridge SHALL combine every supplied source into a single compilation unit before emitting,
so that a call from a function in one source to a function in another resolves. Resolution
SHALL NOT depend on the order the sources are supplied.

Signatures SHALL be gathered from **every** source before any body is lowered, so that a call
across sources is typed rather than left undetermined. This is not an optimisation: the decorator
captures each function as its own source, so a call between two decorated functions is always a
cross-source call, and without this the inference the compiler offers would work everywhere except
through its primary interface.

#### Scenario: Call across two sources

- **WHEN** two sources are compiled together and a function in the first calls a function in
  the second
- **THEN** compilation succeeds

#### Scenario: A cross-source call is typed

- **WHEN** a binding in one source is initialised by calling a function defined in another
- **THEN** the binding takes the callee's return type and needs no annotation

#### Scenario: Order independence

- **WHEN** the same two sources are compiled in both orders
- **THEN** both succeed and report the same fingerprint

#### Scenario: A callee in no source is still reported

- **WHEN** every source has been supplied and a binding's initializer still cannot be typed
- **THEN** compilation fails, since deferring a check is not the same as skipping it

#### Scenario: Duplicate function names across sources

- **WHEN** two sources each define a function of the same name
- **THEN** compilation fails reporting the conflicting name

### Requirement: Diagnostics become Python exceptions

Frontend and lowering failures SHALL be raised as Python exceptions carrying the diagnostic
message and its `line:column` location. A caller SHALL be able to distinguish a syntax error
from a rejection of an otherwise-parseable program, because the two call for different fixes.

#### Scenario: Syntax error

- **WHEN** source text that is not valid Python is compiled
- **THEN** an exception identifying it as a syntax error is raised, carrying the location

#### Scenario: Program outside the supported subset

- **WHEN** valid Python that lowering rejects is compiled
- **THEN** an exception identifying the unsupported construct is raised, carrying the location

#### Scenario: Location is preserved

- **WHEN** a rejection occurs on the third line of a source
- **THEN** the raised exception reports line 3 and the column of the offending construct

#### Scenario: Exceptions are catchable by category

- **WHEN** a caller wants to handle any compylr compilation failure
- **THEN** a single exception type covers both syntax errors and subset rejections

### Requirement: Backend names form a registry

The bridge SHALL accept a backend name and SHALL distinguish three cases: a name it can
compile, a name reserved for a target that is not implemented yet, and a name it does not
recognize. Reserved-but-unimplemented names SHALL fail with an error saying so, not with an
unknown-name error, so that a user who asks for a planned target learns it is planned.

#### Scenario: Implemented backend

- **WHEN** the `rust` backend is requested
- **THEN** compilation proceeds

#### Scenario: Reserved but unimplemented backend

- **WHEN** the `typescript` backend is requested
- **THEN** compilation fails with an error stating that the backend is not implemented yet

#### Scenario: Unrecognized backend

- **WHEN** a backend name that is not in the registry is requested
- **THEN** compilation fails with an error naming the available backends

### Requirement: The fingerprint is exposed

The bridge SHALL report the compiled unit's fingerprint, computed over the IR rather than over
the source text, so that callers can decide whether a rebuild is required without repeating
that computation or inspecting source.

#### Scenario: Formatting does not change the fingerprint

- **WHEN** the same functions are compiled twice, the second time with added comments, blank
  lines, and different indentation
- **THEN** both compilations report the same fingerprint

#### Scenario: A changed body changes the fingerprint

- **WHEN** a function's body is edited to compute something different and recompiled
- **THEN** the reported fingerprint differs

### Requirement: Failures carry a machine-readable category

A compilation failure SHALL carry a stable identifier for what kind of rule was broken, alongside
its message and location. Callers that act differently on different failures SHALL be able to
branch on that identifier.

The identifier SHALL be distinct from the human-readable message. A caller matching on message
text is broken by any rewording, which makes the message unimprovable — and one caller, the
decorator, needs to recognise exactly one category in order to defer it, without recognising any
other.

#### Scenario: A subset violation reports its category

- **WHEN** a program is rejected for an unsupported construct
- **THEN** the failure carries an identifier naming that category

#### Scenario: Categories are distinguishable

- **WHEN** two programs are rejected for different reasons
- **THEN** their identifiers differ

#### Scenario: The identifier is not the message

- **WHEN** a failure's identifier and message are compared
- **THEN** the identifier is a stable token rather than the prose shown to a user

#### Scenario: A binding that cannot yet be typed has its own category

- **WHEN** a binding's initializer calls a function the supplied sources do not define
- **THEN** the failure's category distinguishes it from an annotation the user simply omitted,
  because one may become resolvable with more sources and the other never will

#### Scenario: A syntax error needs no category

- **WHEN** a source fails to parse
- **THEN** the failure is identifiable as a syntax error without carrying a subset category

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
