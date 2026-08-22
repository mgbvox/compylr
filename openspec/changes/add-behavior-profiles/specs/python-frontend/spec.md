## ADDED Requirements

### Requirement: The Python frontend declares Python's stance on every behavior axis

The Python frontend SHALL declare, for every behavior axis, what Python means by that operation.
The declaration SHALL be complete — no axis unanswered — and SHALL describe Python only, naming no
other language and no target.

Python's stance SHALL be: integer arithmetic reports a result outside the integer range; integer
division rounds toward negative infinity and reports a zero divisor; exact division reports a zero
divisor; remainder takes the sign of the divisor and reports a zero divisor; a subscript counts a
negative index from the end and reports an index out of range or a key that is absent; a length
counts code points.

#### Scenario: The stance is complete

- **WHEN** the Python frontend is asked what Python means on each axis
- **THEN** it answers for every axis defined by the behavior model

#### Scenario: The stance names only Python

- **WHEN** the Python frontend's declared stance is inspected
- **THEN** it describes Python's meanings and refers to no other language

#### Scenario: The default behavior reproduces today's output

- **WHEN** a source is lowered under a behavior resolved entirely from the Python frontend's stance
- **THEN** every declared mode on every node matches what the frontend produced before behavior
  selection existed

## MODIFIED Requirements

### Requirement: The Python frontend declares Python's semantics on the IR it produces

When lowering, the Python frontend SHALL set each operator's declared semantics to what the
**resolved behavior** says for that axis, and SHALL NOT rely on any other component defaulting to
Python's interpretation. Where the resolved behavior takes Python's stance — which is the default —
the declared semantics SHALL be Python's: integer division rounds toward negative infinity,
remainder takes the sign of the divisor, true division promotes integer operands to floating point,
and every fallible operation reports its failure.

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

### Requirement: The Python frontend declares the guarantees Python requires

The Python frontend SHALL declare the semantic guarantees that must survive to the target for a
compiled function to still mean what the Python source meant, **under a given resolved behavior**.
Under Python's own stance these SHALL include: an arithmetic result outside the target's integer
range is reported rather than wrapped or truncated, division by zero is reported rather than
undefined, and floating-point arithmetic is not reordered. A backend that does not preserve the
guarantees a unit requires SHALL NOT be usable for it.

Where a resolved behavior takes the target's stance on an axis, the frontend SHALL NOT declare the
guarantee that axis would otherwise have required. Requiring a failure be reported for an operation
the user asked to leave undefined would refuse the very thing they asked for.

#### Scenario: Guarantees are declared

- **WHEN** the Python frontend is asked what Python requires preserved under Python's own stance
- **THEN** it lists overflow reporting, division-by-zero reporting, and floating-point ordering

#### Scenario: A backend lacking a guarantee is refused

- **WHEN** compilation is attempted with a backend that does not declare a guarantee the unit
  requires
- **THEN** compilation fails before emission, naming the guarantee

#### Scenario: A behavior drops the guarantee it waives

- **WHEN** the resolved behavior takes the target's stance on integer overflow
- **THEN** the unit does not require that integer overflow be reported

#### Scenario: Float ordering is not an axis and is never dropped

- **WHEN** any behavior is resolved
- **THEN** the unit still requires that floating-point arithmetic not be reordered, because
  reassociation is a target transformation rather than an operation the programmer wrote

### Requirement: The Python frontend declares Python's container semantics

When lowering, the Python frontend SHALL set each container operation's declared semantics to what
the **resolved behavior** says for that axis, and SHALL NOT rely on any other component defaulting
to Python's interpretation. Where the resolved behavior takes Python's stance, a subscript counts a
negative index from the end and reports a failure, and a length counts code points.

#### Scenario: Subscripting declares counting from either end

- **WHEN** `xs[i]` is lowered under Python's stance
- **THEN** the resulting node declares that a negative index counts from the end, and that a
  failure is reported

#### Scenario: Length declares code points

- **WHEN** `len(s)` is lowered under Python's stance
- **THEN** the resulting node declares that it counts code points

#### Scenario: The declaration is asserted, not the node's name

- **WHEN** the lowered form of a subscript or a length is examined
- **THEN** its meaning is determined by the declared mode rather than by which variant it is

#### Scenario: The behavior selects the container semantics

- **WHEN** `xs[i]` and `len(s)` are lowered under a behavior taking the target's stance on
  indexing and on text length
- **THEN** the nodes declare that target's index origin, checking mode, and text units
