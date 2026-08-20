## Purpose

The command-line entry point for inspecting what a Python file compiles to, without a build, an
interpreter, or a decorator. It is the tool for answering "what does this actually become?" —
during development of the compiler itself, and for a user diagnosing why their function was
rejected.

## Requirements

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
unit, the IR artifact, the translated target code, or the complete generated crate. The summary
SHALL be the default, since it is the smallest useful answer.

Being able to read the generated source without a build is the point: producing it otherwise means
running a full toolchain build and locating the file it wrote.

Where a backend emits several files, the target-code form SHALL print **only the translated
functions** — the part a reader is looking for, and the part that stays useful piped into a pager
or a search. Printing every file as one stream would produce something that no longer compiles when
redirected to a single file, quietly breaking the obvious use of the flag.

#### Scenario: The default is a summary

- **WHEN** the CLI is run with no output flag
- **THEN** it reports the unit fingerprint and each function's name, parameter count, and return
  type

#### Scenario: The IR can be emitted

- **WHEN** the CLI is asked for the IR
- **THEN** it writes the IR artifact, in the same form the build pipeline writes to disk

#### Scenario: The generated source can be emitted

- **WHEN** the CLI is asked for the generated source
- **THEN** it writes the translated functions for the selected backend, without performing a build

#### Scenario: Only the translated code is printed

- **WHEN** the generated source is emitted for a unit of one function
- **THEN** the output holds that function and not the helpers, boundary code, or crate root

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

### Requirement: The whole crate can be written to a directory

The CLI SHALL be able to write every generated file to a directory named by the caller, so that
what compylr would build can be compiled, diffed, or committed without running a build first.

The directory SHALL be required rather than defaulting: writing several files somewhere the user
did not name is a side effect a command-line tool should not have.

#### Scenario: Every file is written

- **WHEN** the CLI is asked to write a crate to a directory
- **THEN** each generated file appears under that directory at its relative path

#### Scenario: The result compiles

- **WHEN** a crate written this way is compiled
- **THEN** it builds, because the files written are exactly the ones the build pipeline would use

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
- **THEN** no toolchain is invoked, so the command works on a machine with no Rust installed

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
