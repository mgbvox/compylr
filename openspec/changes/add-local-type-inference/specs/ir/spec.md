## MODIFIED Requirements

### Requirement: Type model

The IR SHALL define a closed set of types described by their semantics rather than by any
target language's spelling, covering exactly the annotations supported in this slice: a
64-bit signed integer, a 64-bit binary floating-point number, a boolean, a UTF-8 text string,
and a unit type denoting the absence of a value. Each type SHALL carry enough meaning for a
backend to choose a concrete representation without consulting the Python source. Any Python
annotation outside this set SHALL NOT be representable in the IR.

#### Scenario: Integer annotation

- **WHEN** a value is declared with the Python annotation `int`
- **THEN** its IR type is the 64-bit signed integer type

#### Scenario: Floating-point annotation

- **WHEN** a value is declared with the Python annotation `float`
- **THEN** its IR type is the 64-bit binary floating-point type

#### Scenario: Boolean annotation

- **WHEN** a value is declared with the Python annotation `bool`
- **THEN** its IR type is the boolean type

#### Scenario: String annotation

- **WHEN** a value is declared with the Python annotation `str`
- **THEN** its IR type is the UTF-8 text string type

#### Scenario: None return annotation

- **WHEN** a function declares the return annotation `None`
- **THEN** its IR return type is the unit type

#### Scenario: Integer and floating-point types are distinct

- **WHEN** the integer type and the floating-point type are compared
- **THEN** they are different types, so a backend can tell which representation to emit

#### Scenario: Unsupported annotation has no representation

- **WHEN** an annotation such as `complex`, `list[int]`, or a type variable is considered
- **THEN** the type model provides no IR type for it

### Requirement: Expression forms

The IR SHALL support exactly these expression forms in this slice: integer, floating-point,
boolean, and string literals; references to a bound name; arithmetic negation; the binary
arithmetic operations add, subtract, multiply, true-divide, floor-divide, and remainder; the
comparisons equal, not equal, less than, less than or equal, greater than, and greater than
or equal; and calls to a named function with an ordered list of argument expressions.

#### Scenario: Literal expression

- **WHEN** a literal integer, floating-point number, boolean, or string appears in a function
  body
- **THEN** the IR represents it as a literal expression carrying that value

#### Scenario: Floating-point literals compare and hash by value

- **WHEN** two floating-point literals written identically in source are compared
- **THEN** they are equal and produce the same fingerprint contribution, so that a
  floating-point literal does not prevent a function from being fingerprinted

#### Scenario: Binary operation

- **WHEN** two expressions are combined with a supported arithmetic or comparison operator
- **THEN** the IR represents it as a binary expression carrying the operator and both operand
  expressions

#### Scenario: True division is distinct from floor division

- **WHEN** the true-division and floor-division operators are compared
- **THEN** they are distinct operators, because they produce different values for the same
  operands

#### Scenario: Nested expressions

- **WHEN** an expression contains sub-expressions several levels deep
- **THEN** the IR preserves the nesting and the grouping implied by the source

#### Scenario: Call expression

- **WHEN** a function is called with two arguments
- **THEN** the IR represents it as a call expression carrying the callee name and both
  argument expressions in order

### Requirement: Operators carry Python semantics

Arithmetic and comparison operators in the IR SHALL denote Python's semantics, which differ
from several target languages' native operators. In particular, integer floor division
rounds toward negative infinity and the remainder takes the sign of the divisor, whereas
common target languages truncate toward zero; and true division always produces a
floating-point result, whereas the same spelling between two integers is integer division in
many target languages. Backends SHALL be responsible for emitting code that preserves the
IR's semantics rather than mapping operators to same-named native ones.

#### Scenario: Floor division semantics are specified

- **WHEN** the IR's floor-division operator is interpreted
- **THEN** it denotes division rounding toward negative infinity, independent of how any
  target language's division operator behaves

#### Scenario: Remainder semantics are specified

- **WHEN** the IR's remainder operator is interpreted
- **THEN** it denotes a result taking the sign of the divisor, independent of how any target
  language's remainder operator behaves

#### Scenario: True division semantics are specified

- **WHEN** the IR's true-division operator is applied to two integer operands
- **THEN** it denotes a floating-point result, so a backend emitting a native integer
  division for the same spelling would be wrong
