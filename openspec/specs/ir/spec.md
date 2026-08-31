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

- **GIVEN** three functions parsed from three separate sources
- **WHEN** they are added to one unit
- **THEN** the unit contains all three functions

#### Scenario: Function added to an existing unit

- **GIVEN** a unit already holding three functions
- **WHEN** a fourth is added
- **THEN** the unit contains four functions
- **AND** the three existing functions are unchanged

#### Scenario: Duplicate function name is refused

- **GIVEN** a unit already containing a function of a given name
- **WHEN** another function of that name is added
- **THEN** the unit refuses the addition and reports the conflicting name

#### Scenario: Empty unit

- **GIVEN** a unit that has had no functions added
- **WHEN** it is inspected
- **THEN** it contains no functions and is still a valid unit

### Requirement: Deterministic unit ordering

The IR SHALL expose a unit's functions in an order determined solely by their content, not by
the order in which they were added, so that downstream output and fingerprints are stable
across runs. Functions SHALL be ordered by name. Within a function, parameter order and
statement order SHALL follow the source.

#### Scenario: Addition order does not affect unit order

- **GIVEN** the same three functions
- **WHEN** they are added to two units in different orders
- **THEN** both units expose their functions in the same order

#### Scenario: Source order preserved within a function

- **GIVEN** a function with two parameters and a three-statement body
- **WHEN** it is represented in IR
- **THEN** the parameters and statements appear in the order written in the source

### Requirement: Function structure

Each IR function SHALL carry its name, an ordered sequence of parameters, a return type, and
a body of statements. Each parameter SHALL carry its name and its type.

#### Scenario: Function signature is preserved

- **GIVEN** a function with two parameters and a declared return type
- **WHEN** it is represented in IR
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

- **GIVEN** a value declared with the Python annotation `int`
- **WHEN** its IR type is derived
- **THEN** its IR type is the 64-bit signed integer type

#### Scenario: Floating-point annotation

- **GIVEN** a value declared with the Python annotation `float`
- **WHEN** its IR type is derived
- **THEN** its IR type is the 64-bit binary floating-point type

#### Scenario: Boolean annotation

- **GIVEN** a value declared with the Python annotation `bool`
- **WHEN** its IR type is derived
- **THEN** its IR type is the boolean type

#### Scenario: String annotation

- **GIVEN** a value declared with the Python annotation `str`
- **WHEN** its IR type is derived
- **THEN** its IR type is the UTF-8 text string type

#### Scenario: None return annotation

- **GIVEN** a function declaring the return annotation `None`
- **WHEN** its IR return type is derived
- **THEN** its IR return type is the unit type

#### Scenario: Integer and floating-point types are distinct

- **GIVEN** the integer type and the floating-point type
- **WHEN** they are compared
- **THEN** they are different types, so a backend can tell which representation to emit

#### Scenario: Sequence annotation

- **GIVEN** a value declared with the Python annotation `list[int]`
- **WHEN** its IR type is derived
- **THEN** its IR type is a sequence whose element type is the integer type

#### Scenario: Mapping annotation

- **GIVEN** a value declared with the Python annotation `dict[str, int]`
- **WHEN** its IR type is derived
- **THEN** its IR type is a mapping from the string type to the integer type

#### Scenario: Set annotation

- **GIVEN** a value declared with the Python annotation `set[int]`
- **WHEN** its IR type is derived
- **THEN** its IR type is a set whose element type is the integer type

#### Scenario: Tuple annotation carries a type per position

- **GIVEN** a value declared with the Python annotation `tuple[int, str]`
- **WHEN** its IR type is derived
- **THEN** its IR type is a two-element tuple whose first position is the integer type and whose
  second is the string type

#### Scenario: Collections nest

- **GIVEN** a value declared with the Python annotation `dict[str, list[int]]`
- **WHEN** its IR type is derived
- **THEN** its IR type is a mapping from the string type to a sequence of the integer type

#### Scenario: Collections of different element types are distinct

- **GIVEN** a sequence of integers and a sequence of strings
- **WHEN** they are compared
- **THEN** they are different types

#### Scenario: Unsupported annotation has no representation

- **GIVEN** an annotation outside the supported set, such as `complex` or a type variable
- **WHEN** an IR type is sought for it
- **THEN** the type model provides no IR type for it

### Requirement: Target-language independence

The IR SHALL NOT name, spell, or otherwise encode constructs specific to any single target
language, **nor to any single source language**. Choosing the concrete type spelling, operator
syntax, and value representation for a target is the responsibility of that target's backend,
defined by its own capability; choosing how a construct is spelled back to a programmer in
diagnostics is the responsibility of the frontend that read it. Rust is the first backend compylr
implements and Python the first frontend, but the IR SHALL remain producible by a frontend for
another imperative source language and expressible by a backend for another imperative target, such
as Go, C++, or TypeScript.

#### Scenario: No target syntax in the IR

- **GIVEN** the IR type model and node definitions
- **WHEN** they are inspected
- **THEN** no target language's type spellings or syntax appear in them

#### Scenario: No source syntax in the IR

- **GIVEN** the IR type model and node definitions
- **WHEN** they are inspected
- **THEN** no source language's type spellings, operator spellings, or keywords appear in them

#### Scenario: Backend supplies the mapping

- **GIVEN** an IR function and a specific target
- **WHEN** the backend renders it
- **THEN** it derives every concrete type spelling from the IR's semantic types, without
  reading the original source

#### Scenario: Frontend supplies the spelling in diagnostics

- **GIVEN** a diagnostic needing to quote a type or operator in the programmer's own language
- **WHEN** the spelling is chosen
- **THEN** the spelling comes from the frontend that read the source, not from the IR

### Requirement: Statement forms

The IR SHALL support exactly three statement forms in this slice: returning a value,
returning no value, and binding a local name to a value with an explicit declared type. Each
local binding SHALL carry the bound name, its declared type, and the bound expression.

#### Scenario: Value return

- **GIVEN** a function body returning an expression
- **WHEN** it is represented in IR
- **THEN** the IR body contains a return statement carrying that expression

#### Scenario: Bare return

- **GIVEN** a function body returning nothing, or reaching a no-op statement
- **WHEN** it is represented in IR
- **THEN** the IR body contains a statement that produces no value

#### Scenario: Typed local binding

- **GIVEN** a function body binding a name with an explicit type and an initial value
- **WHEN** it is represented in IR
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

- **GIVEN** a literal integer, floating-point number, boolean, or string in a function body
- **WHEN** it is represented in IR
- **THEN** the IR represents it as a literal expression carrying that value

#### Scenario: Floating-point literals compare and hash by value

- **GIVEN** two floating-point literals written identically in source
- **WHEN** they are compared
- **THEN** they are equal and produce the same fingerprint contribution, so that a
  floating-point literal does not prevent a function from being fingerprinted

#### Scenario: Collection literal

- **GIVEN** a sequence, mapping, set, or tuple literal in a function body
- **WHEN** it is represented in IR
- **THEN** the IR represents it as a literal of that kind carrying its element expressions in
  source order

#### Scenario: Subscript expression

- **GIVEN** a collection being subscripted
- **WHEN** it is represented in IR
- **THEN** the IR represents it as a subscript expression carrying the subscripted expression and
  the index expression

#### Scenario: Length expression

- **GIVEN** `len` applied to a collection or string
- **WHEN** it is represented in IR
- **THEN** the IR represents it as a length expression carrying the operand, distinct from a call

#### Scenario: Binary operation

- **GIVEN** two expressions combined with a supported arithmetic or comparison operator
- **WHEN** they are represented in IR
- **THEN** the IR represents it as a binary expression carrying the operator and both operand
  expressions

#### Scenario: True division is distinct from floor division

- **GIVEN** the true-division and floor-division operators
- **WHEN** they are compared
- **THEN** they are distinct operators, because they produce different values for the same
  operands

#### Scenario: Nested expressions

- **GIVEN** an expression containing sub-expressions several levels deep
- **WHEN** it is represented in IR
- **THEN** the IR preserves the nesting and the grouping implied by the source

#### Scenario: Call expression

- **GIVEN** a function called with two arguments
- **WHEN** it is represented in IR
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

- **GIVEN** two sources differing only in comments, formatting, and layout
- **WHEN** both are lowered and fingerprinted
- **THEN** both IR functions produce the same fingerprint

#### Scenario: Changed body changes the fingerprint

- **GIVEN** a function edited so it computes something different
- **WHEN** it is fingerprinted
- **THEN** its fingerprint differs from the fingerprint before the edit

#### Scenario: Changed signature changes the fingerprint

- **GIVEN** a function whose parameter type or return type has changed
- **WHEN** it is fingerprinted
- **THEN** its fingerprint differs from the fingerprint before the change

#### Scenario: Adding a function changes the unit fingerprint

- **GIVEN** a unit containing three functions
- **WHEN** a fourth is added
- **THEN** the unit fingerprint differs from its previous value
- **AND** the fingerprints of the three original functions are unchanged

#### Scenario: Unit fingerprint ignores addition order

- **GIVEN** the same set of functions
- **WHEN** they are assembled into two units in different orders
- **THEN** both units produce the same fingerprint

### Requirement: IR values are self-contained and inspectable

IR values SHALL NOT borrow from the parsed Python source: an IR value SHALL remain valid and
usable after the source text and its parse tree have been released. IR values SHALL support
structural equality comparison and a stable textual rendering, so that tests can assert on a
whole IR tree.

#### Scenario: IR outlives its source

- **GIVEN** a function represented in IR
- **WHEN** the original source text and parse tree are discarded
- **THEN** the IR value remains fully usable

#### Scenario: Structural comparison

- **GIVEN** two IR values built from equivalent programs
- **WHEN** they are compared for structural equality
- **THEN** comparing them for structural equality reports them equal

#### Scenario: Stable rendering

- **GIVEN** one IR value
- **WHEN** it is rendered textually twice
- **THEN** both renderings are identical

### Requirement: A unit serializes to a durable artifact

The IR SHALL be serializable to a durable, self-describing artifact and SHALL be reconstructible
from it. This belongs to the IR rather than to any one backend: the IR is the stage every
backend consumes, so an on-disk form of it is what makes the pipeline inspectable between
lowering and code generation regardless of which target is being emitted.

The artifact SHALL carry a format version, and a reader SHALL refuse an artifact whose version it
does not understand, naming both the version found and the version expected. Adding a mode to a node
changes the serialized shape, so the version SHALL advance whenever it does.

#### Scenario: A unit is written and read back

- **GIVEN** a unit
- **WHEN** it is serialized and then deserialized
- **THEN** the result compares structurally equal to the original

#### Scenario: The artifact describes every construct

- **GIVEN** a unit containing every supported type, statement form, and expression form
- **WHEN** it is round-tripped
- **THEN** each construct is represented in the artifact and survives a round trip

#### Scenario: Fingerprint survives a round trip

- **GIVEN** a unit
- **WHEN** it is serialized, deserialized, and its fingerprint recomputed
- **THEN** the fingerprint equals that of the original unit

#### Scenario: Float literals survive exactly

- **GIVEN** a unit containing float literals, including negative zero
- **WHEN** it is round-tripped
- **THEN** each literal is bit-for-bit identical to the original, consistent with the IR's rule
  that float literals compare by bit pattern

#### Scenario: The artifact carries no target-language information

- **GIVEN** an artifact written from a unit
- **WHEN** it is inspected
- **THEN** it names IR types and operators only, containing no Rust or other target spellings

#### Scenario: An artifact written before checking modes is refused

- **GIVEN** an artifact written under the previous format version
- **WHEN** it is read
- **THEN** it is refused with a message naming the version found and the version expected, rather
  than being read as though every operation reported its failures

### Requirement: Serialization is deterministic

Serializing the same unit SHALL produce byte-identical output across runs and regardless of the
order functions were added, so that an artifact can be compared, cached, or checked into version
control without spurious differences.

#### Scenario: Repeated serialization

- **GIVEN** one unit
- **WHEN** it is serialized twice
- **THEN** the two outputs are byte-identical

#### Scenario: Addition order does not affect the artifact

- **GIVEN** the same functions
- **WHEN** they are assembled into two units in different orders and both are serialized
- **THEN** the two outputs are byte-identical

#### Scenario: Formatting changes do not affect the artifact

- **GIVEN** sources differing only in comments, blank lines, and formatting
- **WHEN** each is lowered and serialized
- **THEN** the output is byte-identical to that of the unit lowered from the original sources

### Requirement: Collection types constrain their parameters

The IR SHALL restrict a mapping's key type and a set's element type to those that can be compared
and hashed: the integer, string, and boolean types. Floating-point SHALL NOT be usable as a
mapping key or a set element.

This is not an arbitrary narrowing. A floating-point key is a hazard in Python — `nan` is never
equal to itself, so a `nan` key can never be retrieved — and most target languages cannot hash a
float at all. Excluding it keeps every backend able to render the type.

#### Scenario: Integer, string, and boolean keys are representable

- **GIVEN** mappings keyed by the integer, string, and boolean types
- **WHEN** their IR types are sought
- **THEN** each has an IR type

#### Scenario: A floating-point key has no representation

- **GIVEN** a mapping keyed by the floating-point type
- **WHEN** its IR type is sought
- **THEN** the type model provides no IR type for it

#### Scenario: A floating-point set element has no representation

- **GIVEN** a set of the floating-point type
- **WHEN** its IR type is sought
- **THEN** the type model provides no IR type for it

#### Scenario: A collection value type is unrestricted

- **GIVEN** a mapping from the string type to the floating-point type
- **WHEN** its IR type is sought
- **THEN** it has an IR type, because only keys and set elements need hashing

### Requirement: Collection literals and subscripts survive the artifact

Every new type and expression form SHALL serialize to the durable artifact and be reconstructible
from it, deterministically, on the same terms as the existing forms. An artifact that could not
describe a collection would make the IR unreadable for exactly the programs this change exists to
support.

#### Scenario: A unit using every collection form round-trips

- **GIVEN** a unit containing each collection type, a literal, a subscript, and a length
- **WHEN** it is round-tripped
- **THEN** the result compares structurally equal to the original

#### Scenario: Nested types round-trip

- **GIVEN** a unit containing a mapping from strings to sequences of integers
- **WHEN** it is round-tripped
- **THEN** the nesting is preserved

#### Scenario: A collection artifact stays target-neutral

- **GIVEN** an artifact describing collections
- **WHEN** it is inspected
- **THEN** it names IR types only, containing no `Vec`, `HashMap`, `HashSet`, or other target
  spelling

#### Scenario: Serialization stays deterministic

- **GIVEN** a unit using collections
- **WHEN** it is serialized twice
- **THEN** the two outputs are byte-identical

### Requirement: Control-flow statement forms

The IR SHALL support conditional execution, bounded and unbounded repetition, and the two loop
controls: a conditional carrying a test, a body, and an optional alternative; a loop carrying a
test and a body; a loop carrying a bound name, an iterable expression, and a body; and statements
that abandon or restart the enclosing loop.

`elif` SHALL be represented as a conditional nested in the alternative of another, since that is
what it means; the IR gains no separate form for it.

#### Scenario: Conditional with no alternative

- **GIVEN** a function body containing an `if` with no `else`
- **WHEN** it is represented in IR
- **THEN** the IR contains a conditional carrying the test and the body, with no alternative

#### Scenario: Conditional with an alternative

- **GIVEN** a function body containing an `if`/`else`
- **WHEN** it is represented in IR
- **THEN** the IR contains a conditional carrying both branches

#### Scenario: elif nests

- **GIVEN** a function body containing `if`/`elif`/`else`
- **WHEN** it is represented in IR
- **THEN** the IR represents the `elif` as a conditional inside the first one's alternative

#### Scenario: Conditional test is a boolean

- **GIVEN** a conditional represented in the IR
- **WHEN** its test is examined
- **THEN** its test is an expression, and the type rules require that expression to be a boolean

#### Scenario: Unbounded loop

- **GIVEN** a function body containing a `while`
- **WHEN** it is represented in IR
- **THEN** the IR contains a loop carrying the test and the body

#### Scenario: Iterating loop

- **GIVEN** a function body containing a `for`
- **WHEN** it is represented in IR
- **THEN** the IR contains a loop carrying the bound name, the iterable, and the body

#### Scenario: Loop control

- **GIVEN** a loop body containing `break` or `continue`
- **WHEN** it is represented in IR
- **THEN** the IR contains the corresponding statement

#### Scenario: Bodies nest

- **GIVEN** a loop containing a conditional containing another loop
- **WHEN** it is represented in IR
- **THEN** the IR preserves the nesting

### Requirement: Range expression

The IR SHALL support a range as an expression form carrying a start, a stop, and a step. All three
SHALL be present in the IR even when the source omitted them, so that a backend never has to know
Python's defaulting rules.

A range is a distinct form rather than a call, for the same reason length is: a call is resolved
against the unit, so leaving it as one would make its meaning depend on what else was compiled.

#### Scenario: Range carries all three components

- **GIVEN** `range(n)`
- **WHEN** it is represented in the IR
- **THEN** it carries a start of zero, a stop of `n`, and a step of one

#### Scenario: Explicit bounds are preserved

- **GIVEN** `range(a, b, c)`
- **WHEN** it is represented in the IR
- **THEN** it carries each component as written

#### Scenario: A range is not a call

- **GIVEN** a unit containing a range
- **WHEN** it is validated
- **THEN** validation does not attempt to resolve `range` as a function

### Requirement: Control flow survives the artifact

Every new statement and expression form SHALL serialize to the durable artifact and be
reconstructible from it, deterministically, on the same terms as the existing forms.

#### Scenario: A unit using every control-flow form round-trips

- **GIVEN** a unit containing a conditional, both loop forms, both loop controls, and a range
- **WHEN** it is round-tripped
- **THEN** the result compares structurally equal to the original

#### Scenario: Nesting survives

- **GIVEN** a unit containing a loop inside a conditional inside a loop
- **WHEN** it is round-tripped
- **THEN** the nesting is preserved

#### Scenario: A control-flow artifact stays target-neutral

- **GIVEN** an artifact describing control flow
- **WHEN** it is inspected
- **THEN** it names IR forms only, containing no target-language loop or branch syntax

### Requirement: Element assignment and membership forms

The IR SHALL support assigning to one element of a collection, as a statement carrying the
collection, the index or key, and the value; and testing membership, as an expression carrying the
value and the container.

It SHALL also support appending to a sequence. Appending is represented explicitly rather than as a
general method call: there is exactly one supported method, and a general form would need a method
signature table before anything needed one.

#### Scenario: Element assignment

- **GIVEN** a body assigning to a collection element
- **WHEN** it is represented in IR
- **THEN** the IR carries the collection, the index or key, and the value

#### Scenario: Membership

- **GIVEN** a body testing membership
- **WHEN** it is represented in IR
- **THEN** the IR carries the value and the container

#### Scenario: Negated membership

- **GIVEN** a body testing `not in`
- **WHEN** it is represented in IR
- **THEN** the IR represents it as the negation of a membership test rather than as its own form

#### Scenario: Append

- **GIVEN** a body appending to a sequence
- **WHEN** it is represented in IR
- **THEN** the IR carries the sequence and the value, as a form distinct from a call

#### Scenario: Appending is not resolved as a call

- **GIVEN** a unit containing an append
- **WHEN** it is validated
- **THEN** validation does not attempt to resolve `append` as a function in the unit

#### Scenario: Assignment, membership, and append survive the artifact

- **GIVEN** a unit containing element assignment, membership, and append
- **WHEN** it is round-tripped
- **THEN** the result compares structurally equal to the original

### Requirement: A unit holds classes as well as functions

The IR SHALL model a class as a member of a compilation unit, carrying its name, its attributes
with their types in declaration order, and its methods. Classes and functions SHALL share one
namespace: a unit SHALL refuse a class whose name is already taken by a function, and the reverse.

A unit's ordering and fingerprint guarantees SHALL extend to classes: members SHALL be exposed in
an order determined by content rather than by addition order, and a unit's fingerprint SHALL cover
each class's structure.

#### Scenario: A class is a unit member

- **GIVEN** a class and a unit
- **WHEN** the class is added to the unit
- **THEN** the unit contains it, alongside any functions

#### Scenario: Names are shared across kinds

- **GIVEN** a unit already containing a function of a given name
- **WHEN** a class of that name is added
- **THEN** the unit refuses the addition and reports the conflicting name

#### Scenario: Ordering is content-determined

- **GIVEN** the same classes and functions
- **WHEN** they are added to two units in different orders
- **THEN** both expose their members in the same order

#### Scenario: A class contributes to the fingerprint

- **GIVEN** a unit containing a class
- **WHEN** a method body is changed
- **THEN** the unit's fingerprint differs from its previous value

#### Scenario: A unit without classes fingerprints unchanged

- **GIVEN** a unit containing only functions
- **WHEN** it is fingerprinted
- **THEN** the value is what it was before classes existed, so existing caches stay valid

#### Scenario: Attribute order follows declaration

- **GIVEN** a class declaring three attributes
- **WHEN** it is represented in the IR
- **THEN** they appear in the order declared

### Requirement: Instance types

The type model SHALL gain an instance type naming a class. It SHALL be usable wherever a type is,
including as a collection's parameter, and SHALL be distinct from every scalar and from every other
class's instance type.

An instance type SHALL NOT be usable as a mapping key or set element: the type model restricts
those to what can be compared and hashed, and an instance has no defined ordering or hash here.

#### Scenario: A class name is a type

- **GIVEN** a value declared with a class's name as its annotation
- **WHEN** its IR type is derived
- **THEN** its IR type is that class's instance type

#### Scenario: Two classes are distinct types

- **GIVEN** the instance types of two different classes
- **WHEN** they are compared
- **THEN** they are different types

#### Scenario: Instances nest in collections

- **GIVEN** a value declared as a sequence of a class
- **WHEN** its IR type is derived
- **THEN** its IR type is a sequence whose element type is that instance type

#### Scenario: An instance cannot be a key

- **GIVEN** a mapping keyed by an instance type
- **WHEN** its IR type is sought
- **THEN** the type model provides no IR type for it

#### Scenario: An instance is not trivially copyable

- **GIVEN** an instance type
- **WHEN** its copyability is considered
- **THEN** it is treated as a type that must be cloned where consumed, like a collection

### Requirement: Attribute and construction forms

The IR SHALL support reading an attribute, assigning an attribute, and constructing an instance.
Construction SHALL carry the class name and its arguments, distinct from a call to a function.

#### Scenario: Attribute read

- **GIVEN** an attribute being read
- **WHEN** it is represented in IR
- **THEN** the IR carries the object expression and the attribute name

#### Scenario: Attribute assignment

- **GIVEN** an attribute being assigned
- **WHEN** it is represented in IR
- **THEN** the IR carries the object expression, the attribute name, and the value

#### Scenario: Construction is distinct from a call

- **GIVEN** a class being constructed
- **WHEN** it is represented in IR
- **THEN** the IR represents it as a construction carrying the class name, not as a function call

#### Scenario: Attribute and construction forms survive the artifact

- **GIVEN** a unit containing a class, attribute access, attribute assignment, and construction
- **WHEN** it is round-tripped
- **THEN** the result compares structurally equal to the original

#### Scenario: A class artifact stays target-neutral

- **GIVEN** an artifact describing a class
- **WHEN** it is inspected
- **THEN** it names IR forms only, containing no target-language struct or trait syntax

### Requirement: Operators carry declared semantics

Every arithmetic operator in the IR that admits more than one reasonable interpretation SHALL carry
its interpretation explicitly on the node, rather than relying on a convention inherited from one
source language. Specifically: integer division SHALL carry a rounding mode, remainder SHALL carry a
sign convention, division that promotes its operands SHALL say so, and every operator that can fail
SHALL carry a checking mode. A frontend SHALL set these to whatever the resolved behavior says; a
backend SHALL reproduce exactly what the node declares, without knowing which frontend produced it.

The two rounding modes SHALL be *toward negative infinity* and *toward zero*. The two remainder sign
conventions SHALL be *sign of the divisor* and *sign of the dividend*. These pairs cover the
behavior of the languages in compylr's supported list; a source language needing a third SHALL add
it to the IR rather than encode it in its frontend.

#### Scenario: Rounding mode is explicit

- **GIVEN** an integer division node
- **WHEN** it is inspected
- **THEN** its rounding mode is readable from the node itself

#### Scenario: The same operator can mean either rounding

- **GIVEN** two integer division nodes declaring different rounding modes
- **WHEN** they are compared
- **THEN** they are distinguishable, and a backend renders each differently

#### Scenario: Remainder sign convention is explicit

- **GIVEN** a remainder node
- **WHEN** it is inspected
- **THEN** its sign convention is readable from the node itself

#### Scenario: Promotion is explicit

- **GIVEN** a division node yielding a floating-point result from integer operands
- **WHEN** it is inspected
- **THEN** the promotion is declared on the node rather than implied by the operator's name

#### Scenario: No node's meaning depends on the source language

- **GIVEN** a unit and no knowledge of which frontend produced it
- **WHEN** the unit is interpreted
- **THEN** every operator's meaning is fully determined by the unit

#### Scenario: Failure handling is explicit

- **GIVEN** an operator that can fail
- **WHEN** it is inspected
- **THEN** whether the program defines its failure is readable from the node, independently of the
  operator's other declared modes

### Requirement: A unit records the frontend that produced it

A unit SHALL record the name of the frontend that lowered it, and the semantic guarantees **the
program** requires be preserved. This is what allows a pair-directed pass to be selected and a
backend's post-processing to be gated without any component re-deriving the source language from the
shape of the tree.

The recorded guarantees SHALL be derived from what the unit's own operations declare, not from a
fixed list belonging to the frontend. Two units produced by the same frontend MAY therefore record
different guarantees, because the guarantees describe what this program needs preserved rather than
what its language usually needs.

#### Scenario: The producing frontend is recorded

- **GIVEN** source lowered with a named frontend
- **WHEN** the resulting unit is inspected
- **THEN** the unit reports that frontend's name

#### Scenario: Required guarantees travel with the unit

- **GIVEN** a unit
- **WHEN** it is inspected
- **THEN** the guarantees the program requires preserved are readable from it

#### Scenario: The record survives the artifact

- **GIVEN** a unit recording its producing frontend
- **WHEN** it is serialized and read back
- **THEN** the producing frontend and its required guarantees are unchanged

#### Scenario: Guarantees follow the program, not the language

- **GIVEN** two units from the same frontend declaring different checking modes on their
  arithmetic
- **WHEN** their required guarantees are computed
- **THEN** the one whose arithmetic is unchecked does not record that integer overflow must be
  reported, and the other does

### Requirement: Container operations carry declared semantics

Reading an element of a sequence and measuring the length of a value each admit more than one
reasonable interpretation across the languages compylr supports, so each SHALL carry its
interpretation on the node rather than inherit one from whichever frontend happens to exist.

Specifically: a subscript SHALL carry an **index origin** and a **checking mode**, and a length
SHALL carry the **text units** it counts in. A frontend sets these to whatever the resolved behavior
says; a backend reproduces exactly what the node says.

The index origins SHALL be *from either end*, where a negative index counts backwards from the end,
and *from the start*, where a negative index is out of range. The text units SHALL be *code points*,
*UTF-8 bytes*, and *UTF-16 units*. These cover Python, Go, C++, and TypeScript; a language needing
another SHALL add it to the IR rather than encode it in its frontend.

Each mode describes one operand kind and SHALL be inert for the others: an index origin says nothing
about a mapping, whose index is a key rather than an offset, and text units say nothing about a
sequence, whose length is a count of elements. A subscript's checking mode is the exception: it
applies to every operand kind, because a sequence offset out of range and a mapping key that is
absent are the same question — whether the failure is a value the program handles.

#### Scenario: Index origin is explicit

- **GIVEN** a subscript node
- **WHEN** it is inspected
- **THEN** its index origin is readable from the node itself

#### Scenario: The same subscript can mean either origin

- **GIVEN** two subscript nodes declaring different index origins
- **WHEN** they are compared
- **THEN** they are distinguishable, and a backend renders each differently

#### Scenario: Text units are explicit

- **GIVEN** a length node
- **WHEN** it is inspected
- **THEN** the units it counts in are readable from the node itself

#### Scenario: All three unit readings are distinguishable

- **GIVEN** three length nodes declaring code points, UTF-8 bytes, and UTF-16 units
- **WHEN** they are compared
- **THEN** each is distinct from the others

#### Scenario: A declared container mode survives the artifact

- **GIVEN** a unit containing subscripts and lengths
- **WHEN** it is serialized and read back
- **THEN** every declared mode is unchanged

#### Scenario: A declared container mode reaches the fingerprint

- **GIVEN** two units differing only in a declared container mode
- **WHEN** they are fingerprinted
- **THEN** their fingerprints differ, because the mode is part of what the program computes

#### Scenario: A subscript's checking mode applies to mappings too

- **GIVEN** a mapping subscript declaring that its failure is unchecked
- **WHEN** it is inspected
- **THEN** the node says so, and a backend renders it differently from one that reports

### Requirement: Container behavior that is not a mode is not parameterized

Where languages differ in the **shape** of an operation rather than in a setting on it, the IR SHALL
model the difference as a distinct form and SHALL NOT add a mode. In particular, reading a mapping
with a key that is absent SHALL always be an operation that *fails*: a language whose lookup instead
yields a default value alongside a presence flag is performing a different operation, one that
requires a notion of a type's zero value the IR does not model, and its frontend SHALL lower it to a
different form rather than set a flag.

Whether that failure is reported to the program or left undefined is a separate question, answered
by the subscript's checking mode. The two are not in tension: the mode says how a failure surfaces,
and this requirement says that a missing key is a failure at all.

#### Scenario: A missing mapping key is reported

- **GIVEN** a mapping read with a key it does not contain, from a node declaring the failure
  reported
- **WHEN** the read is evaluated
- **THEN** the operation reports the missing key, whichever frontend produced the unit

#### Scenario: A missing key never yields a default value

- **GIVEN** a mapping read with a key it does not contain, under either checking mode
- **WHEN** the read is evaluated
- **THEN** the operation fails, and never yields the value type's zero in place of one

#### Scenario: No mode exists for behavior compylr's languages agree on

- **GIVEN** the IR's node definitions
- **WHEN** they are inspected
- **THEN** no mode is carried for iterating a mapping, testing membership, or assigning a mapping
  key, because the languages in the supported list agree on all three

#### Scenario: No mode exists for a range with a zero step

- **GIVEN** the IR's node definitions
- **WHEN** they are inspected
- **THEN** a range carries no mode for a zero step, because every supported language refuses one
  and the refusal exists so that a non-terminating loop stays diagnosable

### Requirement: Fallible operations declare whether the program defines their failure

Every IR operation that can fail on some inputs SHALL carry a **checking mode** stating whether the
program defines what happens when it does. The two modes SHALL be *reported*, where the failure
becomes a value the program can observe and handle, and *unchecked*, where the program declines to
define the result and whatever the target does is what happens.

The operations that SHALL carry it are: integer addition, subtraction, multiplication, and negation
(for a result outside the integer range); division and remainder (for a zero divisor); and
subscripting (for an index out of range or a key that is absent).

*Unchecked* is a statement about the program, not about the target. It says the program does not
define the result, which is why it is legible without knowing which backend will consume the unit —
one target may trap, another may wrap, and a third may do something else, and the unit is equally
true of all three.

#### Scenario: The checking mode is readable from the node

- **GIVEN** an addition, division, remainder, negation, or subscript node
- **WHEN** it is inspected
- **THEN** its checking mode is readable from the node itself

#### Scenario: The same operator can mean either mode

- **GIVEN** two addition nodes declaring different checking modes
- **WHEN** they are compared
- **THEN** they are distinguishable, and a backend renders each differently

#### Scenario: The mode composes with an existing mode

- **GIVEN** an integer division node
- **WHEN** it is inspected
- **THEN** its rounding mode and its checking mode are both readable, and the two are independent

#### Scenario: A checking mode survives the artifact

- **GIVEN** a unit containing both checking modes
- **WHEN** it is serialized and read back
- **THEN** every declared checking mode is unchanged

#### Scenario: A checking mode reaches the fingerprint

- **GIVEN** two units differing only in a declared checking mode
- **WHEN** they are fingerprinted
- **THEN** their fingerprints differ, because the mode is part of what the program computes

#### Scenario: An unchecked operation is not folded into a reported failure

- **GIVEN** a constant expression whose operation is declared unchecked and whose evaluation
  would fail
- **WHEN** a folding pass reaches it
- **THEN** the pass leaves the expression alone rather than turning it into a reported failure,
  because the program did not ask for one
