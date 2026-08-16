## MODIFIED Requirements

### Requirement: Type model

The IR SHALL define a closed set of types described by their semantics rather than by any
target language's spelling. It SHALL cover the scalar types — a 64-bit signed integer, a 64-bit
binary floating-point number, a boolean, a UTF-8 text string, and a unit type denoting the absence
of a value — and the parameterised collection types: a sequence of one element type, a mapping from
one key type to one value type, a set of one element type, and a fixed-length tuple carrying an
element type per position.

The type model SHALL therefore be **recursive**: a collection's parameters are themselves types, to
any depth. Each type SHALL carry enough meaning for a backend to choose a concrete representation
without consulting the Python source. Any Python annotation outside this set SHALL NOT be
representable in the IR.

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

#### Scenario: Sequence annotation

- **WHEN** a value is declared with the Python annotation `list[int]`
- **THEN** its IR type is a sequence whose element type is the integer type

#### Scenario: Mapping annotation

- **WHEN** a value is declared with the Python annotation `dict[str, int]`
- **THEN** its IR type is a mapping from the string type to the integer type

#### Scenario: Set annotation

- **WHEN** a value is declared with the Python annotation `set[int]`
- **THEN** its IR type is a set whose element type is the integer type

#### Scenario: Tuple annotation carries a type per position

- **WHEN** a value is declared with the Python annotation `tuple[int, str]`
- **THEN** its IR type is a two-element tuple whose first position is the integer type and whose
  second is the string type

#### Scenario: Collections nest

- **WHEN** a value is declared with the Python annotation `dict[str, list[int]]`
- **THEN** its IR type is a mapping from the string type to a sequence of the integer type

#### Scenario: Collections of different element types are distinct

- **WHEN** a sequence of integers and a sequence of strings are compared
- **THEN** they are different types

#### Scenario: Unsupported annotation has no representation

- **WHEN** an annotation such as `complex`, `frozenset[int]`, or a type variable is considered
- **THEN** the type model provides no IR type for it

### Requirement: Expression forms

The IR SHALL support exactly these expression forms in this slice: integer, floating-point,
boolean, and string literals; collection literals for sequences, mappings, sets, and tuples;
references to a bound name; arithmetic negation; the binary arithmetic operations add, subtract,
multiply, true-divide, floor-divide, and remainder; the comparisons equal, not equal, less than,
less than or equal, greater than, and greater than or equal; subscripting a collection; the length
of a collection or string; and calls to a named function with an ordered list of argument
expressions.

#### Scenario: Literal expression

- **WHEN** a literal integer, floating-point number, boolean, or string appears in a function
  body
- **THEN** the IR represents it as a literal expression carrying that value

#### Scenario: Floating-point literals compare and hash by value

- **WHEN** two floating-point literals written identically in source are compared
- **THEN** they are equal and produce the same fingerprint contribution, so that a
  floating-point literal does not prevent a function from being fingerprinted

#### Scenario: Collection literal

- **WHEN** a sequence, mapping, set, or tuple literal appears in a function body
- **THEN** the IR represents it as a literal of that kind carrying its element expressions in
  source order

#### Scenario: Subscript expression

- **WHEN** a collection is subscripted
- **THEN** the IR represents it as a subscript expression carrying the subscripted expression and
  the index expression

#### Scenario: Length expression

- **WHEN** `len` is applied to a collection or string
- **THEN** the IR represents it as a length expression carrying the operand, distinct from a call

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

## ADDED Requirements

### Requirement: Collection types constrain their parameters

The IR SHALL restrict a mapping's key type and a set's element type to those that can be compared
and hashed: the integer, string, and boolean types. Floating-point SHALL NOT be usable as a
mapping key or a set element.

This is not an arbitrary narrowing. A floating-point key is a hazard in Python — `nan` is never
equal to itself, so a `nan` key can never be retrieved — and most target languages cannot hash a
float at all. Excluding it keeps every backend able to render the type.

#### Scenario: Integer, string, and boolean keys are representable

- **WHEN** mappings keyed by the integer, string, and boolean types are considered
- **THEN** each has an IR type

#### Scenario: A floating-point key has no representation

- **WHEN** a mapping keyed by the floating-point type is considered
- **THEN** the type model provides no IR type for it

#### Scenario: A floating-point set element has no representation

- **WHEN** a set of the floating-point type is considered
- **THEN** the type model provides no IR type for it

#### Scenario: A collection value type is unrestricted

- **WHEN** a mapping from the string type to the floating-point type is considered
- **THEN** it has an IR type, because only keys and set elements need hashing

### Requirement: Collection literals and subscripts survive the artifact

Every new type and expression form SHALL serialize to the durable artifact and be reconstructible
from it, deterministically, on the same terms as the existing forms. An artifact that could not
describe a collection would make the IR unreadable for exactly the programs this change exists to
support.

#### Scenario: A unit using every collection form round-trips

- **WHEN** a unit containing each collection type, literal, a subscript, and a length is serialized
  and deserialized
- **THEN** the result compares structurally equal to the original

#### Scenario: Nested types round-trip

- **WHEN** a unit containing a mapping from strings to sequences of integers is round-tripped
- **THEN** the nesting is preserved

#### Scenario: The artifact stays target-neutral

- **WHEN** an artifact describing collections is inspected
- **THEN** it names IR types only, containing no `Vec`, `HashMap`, `HashSet`, or other target
  spelling

#### Scenario: Serialization stays deterministic

- **WHEN** a unit using collections is serialized twice
- **THEN** the two outputs are byte-identical
