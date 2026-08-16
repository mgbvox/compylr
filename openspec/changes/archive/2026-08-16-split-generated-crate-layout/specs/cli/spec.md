## MODIFIED Requirements

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

## ADDED Requirements

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
