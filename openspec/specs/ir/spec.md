## Purpose

Defines compylr's intermediate representation: a program model that is independent of both
Python and any target language, the semantic type model its backends map onto concrete types,
and the unit that aggregates independently-compiled functions into the single shared build
artifact a project produces. This is the contract every backend and the decorator runtime
consume.

## Requirements

### Requirement: Unit aggregates functions incrementally

Because every function a project marks for compilation is exposed by one shared build
artifact, the IR SHALL model a compilation unit as a collection of functions assembled from
one or more independently-parsed sources. It SHALL be possible to add a function to an
existing unit without re-supplying the functions already in it. Function names SHALL be
unique within a unit.

#### Scenario: Unit assembled from separate sources

- **WHEN** three functions parsed from three separate sources are added to one unit
- **THEN** the unit contains all three functions

#### Scenario: Function added to an existing unit

- **WHEN** a fourth function is added to a unit that already holds three
- **THEN** the unit contains four functions
- **AND** the three existing functions are unchanged

#### Scenario: Duplicate function name is refused

- **WHEN** a function is added to a unit that already contains a function of the same name
- **THEN** the unit refuses the addition and reports the conflicting name

#### Scenario: Empty unit

- **WHEN** a unit has had no functions added
- **THEN** it contains no functions and is still a valid unit

### Requirement: Deterministic unit ordering

The IR SHALL expose a unit's functions in an order determined solely by their content, not by
the order in which they were added, so that downstream output and fingerprints are stable
across runs. Functions SHALL be ordered by name. Within a function, parameter order and
statement order SHALL follow the source.

#### Scenario: Addition order does not affect unit order

- **WHEN** the same three functions are added to two units in different orders
- **THEN** both units expose their functions in the same order

#### Scenario: Source order preserved within a function

- **WHEN** a function with two parameters and a three-statement body is represented in IR
- **THEN** the parameters and statements appear in the order written in the source

### Requirement: Function structure

Each IR function SHALL carry its name, an ordered sequence of parameters, a return type, and
a body of statements. Each parameter SHALL carry its name and its type.

#### Scenario: Function signature is preserved

- **WHEN** a function with two parameters and a declared return type is represented in IR
- **THEN** the IR function carries both parameter names and types in declaration order
- **AND** it carries the declared return type

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

### Requirement: Target-language independence

The IR SHALL NOT name, spell, or otherwise encode constructs specific to any single target
language. Choosing the concrete type spelling, operator syntax, and value representation for
a target is the responsibility of that target's backend, defined by its own capability. Rust
is the first backend compylr will implement, but the IR SHALL remain expressible by a
backend for another imperative target such as Go, C++, or TypeScript.

#### Scenario: No target syntax in the IR

- **WHEN** the IR type model and node definitions are inspected
- **THEN** no target language's type spellings or syntax appear in them

#### Scenario: Backend supplies the mapping

- **WHEN** a backend renders an IR function for a specific target
- **THEN** it derives every concrete type spelling from the IR's semantic types, without
  reading the original Python source

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

### Requirement: Statement forms

The IR SHALL support exactly three statement forms in this slice: returning a value,
returning no value, and binding a local name to a value with an explicit declared type. Each
local binding SHALL carry the bound name, its declared type, and the bound expression.

#### Scenario: Value return

- **WHEN** a function body returns an expression
- **THEN** the IR body contains a return statement carrying that expression

#### Scenario: Bare return

- **WHEN** a function body returns nothing, or reaches a no-op statement
- **THEN** the IR body contains a statement that produces no value

#### Scenario: Typed local binding

- **WHEN** a function body binds a name with an explicit type and an initial value
- **THEN** the IR body contains a binding statement carrying the name, the type, and the
  initializing expression

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

### Requirement: Stable structural fingerprint

Every IR function SHALL expose a fingerprint derived solely from its structure — name,
parameter names and types, return type, and body. Two functions with identical structure
SHALL produce identical fingerprints, and a change to any of those components SHALL produce a
different fingerprint. A unit SHALL expose a fingerprint derived from the fingerprints of the
functions it contains. Fingerprints SHALL NOT depend on source formatting, comments, or the
order in which functions were added to the unit.

#### Scenario: Identical functions fingerprint identically

- **WHEN** the same function is lowered from two sources that differ only in comments,
  blank lines, and indentation width
- **THEN** both IR functions produce the same fingerprint

#### Scenario: Changed body changes the fingerprint

- **WHEN** a function's body is edited so it computes something different
- **THEN** its fingerprint differs from the fingerprint before the edit

#### Scenario: Changed signature changes the fingerprint

- **WHEN** a function's parameter type or return type is changed
- **THEN** its fingerprint differs from the fingerprint before the change

#### Scenario: Adding a function changes the unit fingerprint

- **WHEN** a fourth function is added to a unit containing three
- **THEN** the unit fingerprint differs from its previous value
- **AND** the fingerprints of the three original functions are unchanged

#### Scenario: Unit fingerprint ignores addition order

- **WHEN** the same set of functions is assembled into two units in different orders
- **THEN** both units produce the same fingerprint

### Requirement: IR values are self-contained and inspectable

IR values SHALL NOT borrow from the parsed Python source: an IR value SHALL remain valid and
usable after the source text and its parse tree have been released. IR values SHALL support
structural equality comparison and a stable textual rendering, so that tests can assert on a
whole IR tree.

#### Scenario: IR outlives its source

- **WHEN** a function is represented in IR and the original source text and parse tree are
  then released
- **THEN** the IR value remains fully usable

#### Scenario: Structural comparison

- **WHEN** two IR values are built from equivalent programs
- **THEN** comparing them for structural equality reports them equal

#### Scenario: Stable rendering

- **WHEN** the same IR value is rendered textually twice
- **THEN** both renderings are identical

### Requirement: A unit serializes to a durable artifact

The IR SHALL be serializable to a durable, self-describing artifact and SHALL be reconstructible
from it. This belongs to the IR rather than to any one backend: the IR is the stage every
backend consumes, so an on-disk form of it is what makes the pipeline inspectable between
lowering and code generation regardless of which target is being emitted.

#### Scenario: A unit is written and read back

- **WHEN** a unit is serialized and then deserialized
- **THEN** the result compares structurally equal to the original

#### Scenario: The artifact describes every construct

- **WHEN** a unit containing every supported type, statement form, and expression form is
  serialized
- **THEN** each construct is represented in the artifact and survives a round trip

#### Scenario: Fingerprint survives a round trip

- **WHEN** a unit is serialized, deserialized, and its fingerprint recomputed
- **THEN** the fingerprint equals that of the original unit

#### Scenario: Float literals survive exactly

- **WHEN** a unit containing float literals, including negative zero, is round-tripped
- **THEN** each literal is bit-for-bit identical to the original, consistent with the IR's rule
  that float literals compare by bit pattern

#### Scenario: The artifact carries no target-language information

- **WHEN** an artifact is inspected
- **THEN** it names IR types and operators only, containing no Rust or other target spellings

### Requirement: Serialization is deterministic

Serializing the same unit SHALL produce byte-identical output across runs and regardless of the
order functions were added, so that an artifact can be compared, cached, or checked into version
control without spurious differences.

#### Scenario: Repeated serialization

- **WHEN** the same unit is serialized twice
- **THEN** the two outputs are byte-identical

#### Scenario: Addition order does not affect the artifact

- **WHEN** the same functions are assembled into two units in different orders and both are
  serialized
- **THEN** the two outputs are byte-identical

#### Scenario: Formatting changes do not affect the artifact

- **WHEN** a unit is lowered from sources differing only in comments, blank lines, and
  indentation, and serialized
- **THEN** the output is byte-identical to that of the unit lowered from the original sources

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

### Requirement: Control-flow statement forms

The IR SHALL support conditional execution, bounded and unbounded repetition, and the two loop
controls: a conditional carrying a test, a body, and an optional alternative; a loop carrying a
test and a body; a loop carrying a bound name, an iterable expression, and a body; and statements
that abandon or restart the enclosing loop.

`elif` SHALL be represented as a conditional nested in the alternative of another, since that is
what it means; the IR gains no separate form for it.

#### Scenario: Conditional with no alternative

- **WHEN** a function body contains an `if` with no `else`
- **THEN** the IR contains a conditional carrying the test and the body, with no alternative

#### Scenario: Conditional with an alternative

- **WHEN** a function body contains an `if`/`else`
- **THEN** the IR contains a conditional carrying both branches

#### Scenario: elif nests

- **WHEN** a function body contains `if`/`elif`/`else`
- **THEN** the IR represents the `elif` as a conditional inside the first one's alternative

#### Scenario: Conditional test is a boolean

- **WHEN** a conditional is represented in the IR
- **THEN** its test is an expression, and the type rules require that expression to be a boolean

#### Scenario: Unbounded loop

- **WHEN** a function body contains a `while`
- **THEN** the IR contains a loop carrying the test and the body

#### Scenario: Iterating loop

- **WHEN** a function body contains a `for`
- **THEN** the IR contains a loop carrying the bound name, the iterable, and the body

#### Scenario: Loop control

- **WHEN** a loop body contains `break` or `continue`
- **THEN** the IR contains the corresponding statement

#### Scenario: Bodies nest

- **WHEN** a loop contains a conditional containing another loop
- **THEN** the IR preserves the nesting

### Requirement: Range expression

The IR SHALL support a range as an expression form carrying a start, a stop, and a step. All three
SHALL be present in the IR even when the source omitted them, so that a backend never has to know
Python's defaulting rules.

A range is a distinct form rather than a call, for the same reason length is: a call is resolved
against the unit, so leaving it as one would make its meaning depend on what else was compiled.

#### Scenario: Range carries all three components

- **WHEN** `range(n)` is represented in the IR
- **THEN** it carries a start of zero, a stop of `n`, and a step of one

#### Scenario: Explicit bounds are preserved

- **WHEN** `range(a, b, c)` is represented in the IR
- **THEN** it carries each component as written

#### Scenario: A range is not a call

- **WHEN** a unit containing a range is validated
- **THEN** validation does not attempt to resolve `range` as a function

### Requirement: Control flow survives the artifact

Every new statement and expression form SHALL serialize to the durable artifact and be
reconstructible from it, deterministically, on the same terms as the existing forms.

#### Scenario: A unit using every control-flow form round-trips

- **WHEN** a unit containing a conditional, both loop forms, both loop controls, and a range is
  serialized and deserialized
- **THEN** the result compares structurally equal to the original

#### Scenario: Nesting survives

- **WHEN** a unit containing a loop inside a conditional inside a loop is round-tripped
- **THEN** the nesting is preserved

#### Scenario: The artifact stays target-neutral

- **WHEN** an artifact describing control flow is inspected
- **THEN** it names IR forms only, containing no target-language loop or branch syntax

### Requirement: Element assignment and membership forms

The IR SHALL support assigning to one element of a collection, as a statement carrying the
collection, the index or key, and the value; and testing membership, as an expression carrying the
value and the container.

It SHALL also support appending to a sequence. Appending is represented explicitly rather than as a
general method call: there is exactly one supported method, and a general form would need a method
signature table before anything needed one.

#### Scenario: Element assignment

- **WHEN** a body assigns to a collection element
- **THEN** the IR carries the collection, the index or key, and the value

#### Scenario: Membership

- **WHEN** a body tests membership
- **THEN** the IR carries the value and the container

#### Scenario: Negated membership

- **WHEN** a body tests `not in`
- **THEN** the IR represents it as the negation of a membership test rather than as its own form

#### Scenario: Append

- **WHEN** a body appends to a sequence
- **THEN** the IR carries the sequence and the value, as a form distinct from a call

#### Scenario: Appending is not resolved as a call

- **WHEN** a unit containing an append is validated
- **THEN** validation does not attempt to resolve `append` as a function in the unit

#### Scenario: The new forms survive the artifact

- **WHEN** a unit containing element assignment, membership, and append is round-tripped
- **THEN** the result compares structurally equal to the original

### Requirement: A unit holds classes as well as functions

The IR SHALL model a class as a member of a compilation unit, carrying its name, its attributes
with their types in declaration order, and its methods. Classes and functions SHALL share one
namespace: a unit SHALL refuse a class whose name is already taken by a function, and the reverse.

A unit's ordering and fingerprint guarantees SHALL extend to classes: members SHALL be exposed in
an order determined by content rather than by addition order, and a unit's fingerprint SHALL cover
each class's structure.

#### Scenario: A class is a unit member

- **WHEN** a class is added to a unit
- **THEN** the unit contains it, alongside any functions

#### Scenario: Names are shared across kinds

- **WHEN** a class is added to a unit already containing a function of that name
- **THEN** the unit refuses the addition and reports the conflicting name

#### Scenario: Ordering is content-determined

- **WHEN** the same classes and functions are added to two units in different orders
- **THEN** both expose their members in the same order

#### Scenario: A class contributes to the fingerprint

- **WHEN** a method body is changed
- **THEN** the unit's fingerprint differs from its previous value

#### Scenario: A unit without classes fingerprints unchanged

- **WHEN** a unit containing only functions is fingerprinted
- **THEN** the value is what it was before classes existed, so existing caches stay valid

#### Scenario: Attribute order follows declaration

- **WHEN** a class declaring three attributes is represented in the IR
- **THEN** they appear in the order declared

### Requirement: Instance types

The type model SHALL gain an instance type naming a class. It SHALL be usable wherever a type is,
including as a collection's parameter, and SHALL be distinct from every scalar and from every other
class's instance type.

An instance type SHALL NOT be usable as a mapping key or set element: the type model restricts
those to what can be compared and hashed, and an instance has no defined ordering or hash here.

#### Scenario: A class name is a type

- **WHEN** a value is declared with a class's name as its annotation
- **THEN** its IR type is that class's instance type

#### Scenario: Two classes are distinct types

- **WHEN** the instance types of two different classes are compared
- **THEN** they are different types

#### Scenario: Instances nest in collections

- **WHEN** a value is declared as a sequence of a class
- **THEN** its IR type is a sequence whose element type is that instance type

#### Scenario: An instance cannot be a key

- **WHEN** a mapping keyed by an instance type is considered
- **THEN** the type model provides no IR type for it

#### Scenario: An instance is not trivially copyable

- **WHEN** the copyability of an instance type is considered
- **THEN** it is treated as a type that must be cloned where consumed, like a collection

### Requirement: Attribute and construction forms

The IR SHALL support reading an attribute, assigning an attribute, and constructing an instance.
Construction SHALL carry the class name and its arguments, distinct from a call to a function.

#### Scenario: Attribute read

- **WHEN** an attribute is read
- **THEN** the IR carries the object expression and the attribute name

#### Scenario: Attribute assignment

- **WHEN** an attribute is assigned
- **THEN** the IR carries the object expression, the attribute name, and the value

#### Scenario: Construction is distinct from a call

- **WHEN** a class is constructed
- **THEN** the IR represents it as a construction carrying the class name, not as a function call

#### Scenario: The new forms survive the artifact

- **WHEN** a unit containing a class, attribute access, attribute assignment, and construction is
  round-tripped
- **THEN** the result compares structurally equal to the original

#### Scenario: The artifact stays target-neutral

- **WHEN** an artifact describing a class is inspected
- **THEN** it names IR forms only, containing no target-language struct or trait syntax
