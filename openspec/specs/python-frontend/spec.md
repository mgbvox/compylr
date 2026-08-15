## Purpose

Turns a Python source file on disk into a parsed syntax tree, and turns any failure to do so
into a structured, inspectable error that the rest of the compiler can report instead of
crashing on.

## Requirements

### Requirement: Parse a Python source file

The frontend SHALL accept a filesystem path to a Python source file and produce a parsed
syntax tree for the module it contains.

#### Scenario: Valid Python file is parsed

- **WHEN** the frontend is given a path to a file containing syntactically valid Python
- **THEN** it returns a successful result carrying the parsed module tree

#### Scenario: Empty file is parsed

- **WHEN** the frontend is given a path to a file containing no statements
- **THEN** it returns a successful result carrying a module tree with an empty body

### Requirement: Report unreadable input as a structured error

The frontend MUST NOT panic when the input file cannot be read. It SHALL return a failure
that identifies the failure as an input/output problem and names the offending path.

#### Scenario: File does not exist

- **WHEN** the frontend is given a path that does not exist on disk
- **THEN** it returns a failure identified as an input/output problem
- **AND** the human-readable message contains the requested path

#### Scenario: Path refers to a directory

- **WHEN** the frontend is given a path that exists but is a directory
- **THEN** it returns a failure identified as an input/output problem rather than panicking

### Requirement: Report invalid syntax as a structured error

The frontend MUST NOT panic on syntactically invalid Python. It SHALL return a failure that
is distinguishable from an input/output problem and that carries the location of the syntax
error within the source.

#### Scenario: Malformed Python source

- **WHEN** the frontend parses a file whose contents are not valid Python
- **THEN** it returns a failure identified as a syntax problem
- **AND** the failure carries the source position at which parsing failed

#### Scenario: Caller can distinguish failure kinds

- **WHEN** a caller receives a frontend failure
- **THEN** the caller can determine whether it was an input/output problem or a syntax
  problem without inspecting message text

### Requirement: Failures are human-readable

Every frontend failure SHALL render as a single-line, human-readable message suitable for
display to a user, and SHALL integrate with the language's standard error reporting so it
can be propagated by callers.

#### Scenario: Failure is displayed

- **WHEN** a frontend failure is rendered for display
- **THEN** the message names what went wrong and the file involved
- **AND** the message does not expose internal debug formatting of the underlying cause
