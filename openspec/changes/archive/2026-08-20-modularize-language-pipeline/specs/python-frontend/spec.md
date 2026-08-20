## ADDED Requirements

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

## MODIFIED Requirements

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
