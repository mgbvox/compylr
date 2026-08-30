## MODIFIED Requirements

### Requirement: The Python frontend owns Python spellings

Rendering a type or an operator in the way a Python programmer wrote it SHALL belong to **the
Python language**, and SHALL be stated once, in a place that neither reads Python source nor writes
it. Both the component that reads Python and the component that writes it SHALL use that one
statement. No such spelling SHALL be obtainable from the IR itself.

Stating it once is the requirement, not an implementation preference: a type named `dict[str, int]`
in a diagnostic and something else in generated Python would be two answers to one question, and the
component that reads a language has no better claim to how the language is spelled than the
component that writes it.

#### Scenario: A type is named the way the programmer wrote it

- **WHEN** a diagnostic reports a mismatch involving a mapping from strings to integers
- **THEN** it names the type `dict[str, int]`

#### Scenario: An operator is named the way the programmer wrote it

- **WHEN** a diagnostic reports a problem with floor division
- **THEN** it names the operator `//`

#### Scenario: The IR offers no Python spelling

- **WHEN** the IR's public surface is inspected
- **THEN** it exposes no way to render a type or operator in Python

#### Scenario: Reading and writing Python agree on a spelling

- **WHEN** a type is named in a diagnostic and the same type is annotated in generated Python
- **THEN** the two spellings are identical

#### Scenario: The spelling does not require a parser

- **WHEN** a component that writes Python but never reads it asks for a spelling
- **THEN** it obtains one without depending on anything that parses Python

### Requirement: The Python frontend declares Python's semantics on the IR it produces

When lowering, the Python frontend SHALL set each operator's declared semantics to what the
**resolved behavior** says for that axis, and SHALL NOT rely on any other component defaulting to
Python's interpretation. Where the resolved behavior takes Python's stance — which is the default —
the declared semantics SHALL be Python's: integer division rounds toward negative infinity,
remainder takes the sign of the divisor, true division promotes integer operands to floating point,
and every fallible operation reports its failure.

What Python means SHALL be stated once, as a property of the language, and SHALL be read rather than
restated by every component that needs it. A component that reads Python and a component that writes
Python that disagreed about what Python means would be a contradiction the compiler could not
detect.

#### Scenario: Floor division is declared

- **WHEN** `a // b` is lowered under Python's stance
- **THEN** the resulting node declares rounding toward negative infinity

#### Scenario: Remainder is declared

- **WHEN** `a % b` is lowered under Python's stance
- **THEN** the resulting node declares the sign of the divisor

#### Scenario: True division is declared

- **WHEN** `a / b` is lowered with integer operands
- **THEN** the resulting node declares float promotion, under every behavior

#### Scenario: The behavior selects the rounding, not the frontend

- **WHEN** `a // b` is lowered under a behavior taking the target's stance on integer division
- **THEN** the resulting node declares that target's rounding, and the frontend consults no
  constant of its own

#### Scenario: One declaration serves both directions

- **WHEN** the component that reads Python and the component that writes Python are each asked what
  Python means on any axis
- **THEN** they give the same answer, from the same declaration
