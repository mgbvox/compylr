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
target language's spelling, covering exactly the annotations supported in this slice: a
64-bit signed integer, a boolean, a UTF-8 text string, and a unit type denoting the absence
of a value. Each type SHALL carry enough meaning for a backend to choose a concrete
representation without consulting the Python source. Any Python annotation outside this set
SHALL NOT be representable in the IR.

#### Scenario: Integer annotation

- **WHEN** a value is declared with the Python annotation `int`
- **THEN** its IR type is the 64-bit signed integer type

#### Scenario: Boolean annotation

- **WHEN** a value is declared with the Python annotation `bool`
- **THEN** its IR type is the boolean type

#### Scenario: String annotation

- **WHEN** a value is declared with the Python annotation `str`
- **THEN** its IR type is the UTF-8 text string type

#### Scenario: None return annotation

- **WHEN** a function declares the return annotation `None`
- **THEN** its IR return type is the unit type

#### Scenario: Unsupported annotation has no representation

- **WHEN** an annotation such as `float`, `list[int]`, or a type variable is considered
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
common target languages truncate toward zero. Backends SHALL be responsible for emitting code
that preserves the IR's semantics rather than mapping operators to same-named native ones.

#### Scenario: Floor division semantics are specified

- **WHEN** the IR's floor-division operator is interpreted
- **THEN** it denotes division rounding toward negative infinity, independent of how any
  target language's division operator behaves

#### Scenario: Remainder semantics are specified

- **WHEN** the IR's remainder operator is interpreted
- **THEN** it denotes a result taking the sign of the divisor, independent of how any target
  language's remainder operator behaves

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

The IR SHALL support exactly these expression forms in this slice: integer, boolean, and
string literals; references to a bound name; arithmetic negation; the binary arithmetic
operations add, subtract, multiply, floor-divide, and remainder; the comparisons equal, not
equal, less than, less than or equal, greater than, and greater than or equal; and calls to a
named function with an ordered list of argument expressions.

#### Scenario: Literal expression

- **WHEN** a literal integer, boolean, or string appears in a function body
- **THEN** the IR represents it as a literal expression carrying that value

#### Scenario: Binary operation

- **WHEN** two expressions are combined with a supported arithmetic or comparison operator
- **THEN** the IR represents it as a binary expression carrying the operator and both operand
  expressions

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
