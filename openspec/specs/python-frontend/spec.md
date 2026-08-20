## Purpose

The source language component for Python: turns Python source text into IR, declares what Python
means by each operator and what it needs a target to preserve, and owns how a type or operator is
spelled back to the programmer in a diagnostic. Any failure along the way becomes a structured,
located error the rest of the compiler can report instead of crashing on.

## Requirements

### Requirement: Parse a Python source file

The frontend SHALL accept Python source text and produce a parsed syntax tree for the module it
contains, and SHALL additionally accept a filesystem path to a source file as a convenience over the
same behavior. Source text is the primary input because the decorator supplies the result of reading
a function's own source, not a path.

#### Scenario: Valid Python source is parsed

- **WHEN** the frontend is given syntactically valid Python source text
- **THEN** it returns a successful result carrying the parsed module tree

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

### Requirement: The Python frontend is a registered component

The Python frontend SHALL be selectable by name through the frontend registry and SHALL be reachable
only through the shared frontend interface. No component outside it SHALL depend on it directly, and
the Python parser it uses SHALL NOT be a dependency of the IR, the optimization passes, or any
backend.

#### Scenario: Selected by name

- **WHEN** the frontend named `python` is resolved
- **THEN** resolution succeeds and lowering Python source through it produces the same IR as before

#### Scenario: The parser is confined to the frontend

- **WHEN** the dependencies of the IR, the pass pipeline, and each backend are inspected
- **THEN** none of them depends on a Python parser

#### Scenario: Building a backend does not build a Python parser

- **WHEN** a target backend is built on its own
- **THEN** the Python parser is not compiled

### Requirement: The Python frontend declares Python's semantics on the IR it produces

When lowering, the Python frontend SHALL set each operator's declared semantics to what Python
means: integer division rounds toward negative infinity, remainder takes the sign of the divisor,
and true division promotes integer operands to floating point. It SHALL NOT rely on any other
component defaulting to Python's interpretation.

#### Scenario: Floor division is declared

- **WHEN** `a // b` is lowered
- **THEN** the resulting node declares rounding toward negative infinity

#### Scenario: Remainder is declared

- **WHEN** `a % b` is lowered
- **THEN** the resulting node declares the sign of the divisor

#### Scenario: True division is declared

- **WHEN** `a / b` is lowered with integer operands
- **THEN** the resulting node declares float promotion

### Requirement: The Python frontend owns Python spellings

Rendering a type or an operator in the way a Python programmer wrote it SHALL be the Python
frontend's responsibility. Diagnostics naming a type SHALL use Python's spelling, and no such
spelling SHALL be obtainable from the IR itself.

#### Scenario: A type is named the way the programmer wrote it

- **WHEN** a diagnostic reports a mismatch involving a mapping from strings to integers
- **THEN** it names the type `dict[str, int]`

#### Scenario: An operator is named the way the programmer wrote it

- **WHEN** a diagnostic reports a problem with floor division
- **THEN** it names the operator `//`

#### Scenario: The IR offers no Python spelling

- **WHEN** the IR's public surface is inspected
- **THEN** it exposes no way to render a type or operator in Python

### Requirement: The Python frontend declares the guarantees Python requires

The Python frontend SHALL declare the semantic guarantees that must survive to the target for the
compiled function to still mean what the Python source meant. At minimum these SHALL include: an
arithmetic result outside the target's integer range is reported rather than wrapped or truncated,
division by zero is reported rather than undefined, and floating-point arithmetic is not reordered.
A backend that does not preserve these SHALL NOT be usable with this frontend.

#### Scenario: Guarantees are declared

- **WHEN** the Python frontend is asked what it requires preserved
- **THEN** it lists overflow reporting, division-by-zero reporting, and floating-point ordering

#### Scenario: A backend lacking a guarantee is refused

- **WHEN** compilation is attempted with a backend that does not declare one of these guarantees
- **THEN** compilation fails before emission, naming the guarantee

### Requirement: The Python frontend declares Python's container semantics

When lowering, the Python frontend SHALL set each container operation's declared semantics to what
Python means: a subscript counts a negative index from the end of a sequence, and a length counts
code points. It SHALL NOT rely on any other component defaulting to Python's interpretation.

#### Scenario: Subscripting declares counting from either end

- **WHEN** `xs[i]` is lowered
- **THEN** the resulting node declares that a negative index counts from the end

#### Scenario: Length declares code points

- **WHEN** `len(s)` is lowered
- **THEN** the resulting node declares that it counts code points

#### Scenario: The declaration is asserted, not the node's name

- **WHEN** the lowered form of a subscript or a length is examined
- **THEN** its meaning is determined by the declared mode rather than by which variant it is
