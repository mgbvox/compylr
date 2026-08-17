## ADDED Requirements

### Requirement: A project can be compiled ahead of time

The command line SHALL provide a command that takes a project root, finds every marked function and
class beneath it, builds the shared artifact once, and exits. A later run of that project SHALL
find the artifact current and start without building.

The command SHALL be available as `compylr` on the user's path once the package is installed.

#### Scenario: A project is compiled

- **WHEN** the command is run against a project containing marked functions
- **THEN** the artifact is built and the command exits successfully

#### Scenario: A later run does not build

- **WHEN** a project is precompiled and then run
- **THEN** the run does not invoke the toolchain

#### Scenario: An already-current project is not rebuilt

- **WHEN** the command is run twice against an unchanged project
- **THEN** the second run reports the artifact was already current and does not build

#### Scenario: A change is picked up

- **WHEN** a marked function is edited and the command run again
- **THEN** it rebuilds

#### Scenario: Reformatting is not a change

- **WHEN** comments or indentation are altered and the command run again
- **THEN** it does not rebuild, because the decision keys off the IR

### Requirement: Discovery imports the project

The command SHALL discover marked functions and classes by importing each module beneath the root,
so that the decorators run and register exactly as they do at runtime.

This is why the command is a Python entry point rather than the Rust binary: importing is the only
discovery mechanism that cannot disagree with the runtime, because it *is* the runtime's mechanism.
A separate static notion of what a decorator looks like would drift on aliases, re-exports, and
conditional decoration.

Importing runs module-level code. The command SHALL say so in its help, because a user may
reasonably expect a compiler not to execute what it compiles.

#### Scenario: Every marked function is found

- **WHEN** a project spreads marked functions across several modules
- **THEN** all of them are included in the one build

#### Scenario: Marked classes are found

- **WHEN** a project marks a class
- **THEN** it is included alongside marked functions

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

### Requirement: The command reports what it did

The command SHALL report the modules it imported, the functions and classes it found, whether it
built or reused a current artifact, and how long it took.

A precompile that quietly finds nothing looks exactly like one that succeeded, and the failure only
appears later as a slow first run — which is far from its cause.

#### Scenario: A successful build is reported

- **WHEN** a project is compiled
- **THEN** the output names the counts found and reports that a build occurred

#### Scenario: Finding nothing is reported clearly

- **WHEN** the root contains no marked functions or classes
- **THEN** the command says so rather than reporting success with nothing done

#### Scenario: Reuse is distinguished from building

- **WHEN** the artifact is already current
- **THEN** the output distinguishes that from having built

#### Scenario: Build failures carry the toolchain's output

- **WHEN** the build fails
- **THEN** the toolchain's diagnostics are included and the command exits unsuccessfully

#### Scenario: Exit status distinguishes the outcomes

- **WHEN** the command succeeds, fails to build, or finds nothing
- **THEN** each is distinguishable from the exit status alone, so it can be used in a script

#### Scenario: A missing root is reported

- **WHEN** the command is given a path that does not exist
- **THEN** it reports the path and exits unsuccessfully
