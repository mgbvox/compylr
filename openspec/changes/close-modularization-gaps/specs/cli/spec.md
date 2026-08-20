## MODIFIED Requirements

### Requirement: Discovery imports the project

The command SHALL discover marked functions and classes by importing each module beneath the root,
so that the decorators run and register exactly as they do at runtime.

This is why the command is a Python entry point rather than the Rust binary: importing is the only
discovery mechanism that cannot disagree with the runtime, because it *is* the runtime's mechanism.
A separate static notion of what a decorator looks like would drift on aliases, re-exports, and
conditional decoration.

"Exactly as they do at runtime" includes packages. A module inside a package SHALL be imported such
that its relative imports resolve, which means the package it belongs to, and every package above
that, SHALL exist by the time it is imported. Discovery SHALL NOT depend on the order the filesystem
happens to enumerate files in.

Importing runs module-level code. The command SHALL say so in its help, because a user may
reasonably expect a compiler not to execute what it compiles.

#### Scenario: Every marked function is found

- **WHEN** a project spreads marked functions across several modules
- **THEN** all of them are included in the one build

#### Scenario: Marked classes are found

- **WHEN** a project marks a class
- **THEN** it is included alongside marked functions

#### Scenario: A package's own module imports

- **WHEN** the root contains a package whose `__init__.py` imports its siblings relatively
- **THEN** the package imports successfully and is not reported as a failure

#### Scenario: A nested package imports

- **WHEN** a marked function lives in a package inside another package
- **THEN** it is found, and every package above it resolves

#### Scenario: Enumeration order does not decide success

- **WHEN** a subpackage's name sorts before its parent's own module file
- **THEN** it imports successfully anyway

#### Scenario: Only the given root is imported

- **WHEN** the command is run against a subdirectory of a larger project
- **THEN** modules outside it are not imported

#### Scenario: Non-source directories are skipped

- **WHEN** the root contains virtual environments, caches, or build output
- **THEN** they are not imported

#### Scenario: A module that cannot be imported is reported

- **WHEN** one module raises on import
- **THEN** the command reports which module and why, and continues with the others

#### Scenario: The help says that importing executes code

- **WHEN** the command's help is shown
- **THEN** it states that discovery imports the project's modules
