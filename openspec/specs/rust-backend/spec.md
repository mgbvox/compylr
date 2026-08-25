## Purpose

Translates compylr IR into Rust source text. This is where the IR's deliberately abstract type
model meets concrete spellings, and where the arithmetic semantics each node declares must be
reproduced rather than delegated to Rust's same-named operators, which are one choice among
several and disagree with the others on negative and integer operands.

## Requirements

### Requirement: Concrete type spellings

The backend SHALL map each IR type to a Rust type. The mapping SHALL live in the backend
alone: no IR type carries a Rust spelling, so a second backend can choose different ones for
the same IR. Collection spellings SHALL be derived from their parameters, recursively. The mapping
SHALL be derived from the IR's semantic types only, so a unit produced by any frontend spells the
same way.

| IR type | Rust type |
| --- | --- |
| integer | `i64` |
| float | `f64` |
| bool | `bool` |
| string | `String` |
| unit | `()` |
| sequence of `T` | `Vec<T>` |
| mapping from `K` to `V` | `HashMap<K, V>` |
| set of `T` | `HashSet<T>` |
| tuple of `T1..Tn` | `(T1, .., Tn)` |

#### Scenario: Each type is spelled

- **WHEN** a function's parameters and return type cover all five scalar IR types
- **THEN** the emitted Rust uses `i64`, `f64`, `bool`, `String`, and `()` respectively

#### Scenario: Each collection type is spelled

- **WHEN** a function's parameters cover a sequence, mapping, set, and tuple
- **THEN** the emitted Rust uses `Vec`, `HashMap`, `HashSet`, and a tuple type respectively

#### Scenario: Nested collections spell recursively

- **WHEN** a parameter typed as a mapping from strings to sequences of integers is emitted
- **THEN** the emitted Rust spells it `HashMap<String, Vec<i64>>`

#### Scenario: The IR is unchanged by emission

- **WHEN** a unit is emitted twice
- **THEN** the IR is identical before and after, carrying no Rust-specific information

#### Scenario: Spelling does not depend on the producing frontend

- **WHEN** two units with identical types record different producing frontends
- **THEN** the emitted Rust type spellings are identical

### Requirement: Function emission

The backend SHALL emit each function in a unit as a Rust function carrying its name, its
parameters in source order with their spelled types, and its declared return type.

Every emitted function SHALL be fallible, yielding either the declared return type or a runtime
error. This is uniform rather than decided per function: any body can contain a division or an
arithmetic overflow, including the body of a function that returns nothing, so a signature that
became fallible only when the backend judged failure possible would change shape on an unrelated
edit and force every caller to change with it.

#### Scenario: Function with parameters and a return type

- **WHEN** a function taking two integers and returning an integer is emitted
- **THEN** the Rust signature names both parameters with type `i64`, and the function yields an
  `i64` on success

#### Scenario: Function returning unit

- **WHEN** a function annotated `-> None` is emitted
- **THEN** the emitted Rust function yields no value on success

#### Scenario: A unit-returning function can still report failure

- **WHEN** a function annotated `-> None` contains a division by zero
- **THEN** its signature is able to carry the failure, rather than the failure being unreportable

#### Scenario: Every function in the unit appears

- **WHEN** a unit holding three functions is emitted
- **THEN** the output contains all three, in the unit's deterministic order

### Requirement: Statement emission

The backend SHALL emit each IR statement form: a return of an expression, a return of unit,
and a local binding. A binding SHALL be emitted with its type stated explicitly rather than
inferred by the Rust compiler, so that a mismatch between the IR's type and Rust's inference
is a compile error rather than a silent behavior change.

#### Scenario: Return of an expression

- **WHEN** a function whose body is a single return of an expression is emitted
- **THEN** the emitted body evaluates that expression as the function's result

#### Scenario: Local binding carries its type

- **WHEN** a binding of an integer-typed initializer is emitted
- **THEN** the emitted Rust binding states the type `i64` explicitly

#### Scenario: A body with no value to return

- **WHEN** a function whose body is `pass` and whose return type is unit is emitted
- **THEN** the emitted function body produces no value and compiles

### Requirement: Expression emission

The backend SHALL emit each IR expression form: literals of every type, name references,
negation, the float-promotion node, binary operations, and calls to other functions in the
same unit. String literals SHALL be emitted with escaping such that the emitted Rust string
denotes exactly the characters in the IR literal.

#### Scenario: Literals of every type

- **WHEN** integer, float, boolean, and string literals are emitted
- **THEN** each appears as a Rust literal denoting the same value

#### Scenario: String literal containing characters that need escaping

- **WHEN** a string literal containing a double quote, a backslash, and a newline is emitted
- **THEN** the emitted Rust string literal denotes exactly those characters

#### Scenario: Promotion node

- **WHEN** an expression wrapping an integer operand in the float-promotion node is emitted
- **THEN** the emitted Rust converts that operand to `f64`

#### Scenario: Call to another function in the unit

- **WHEN** a function calls another function in the same unit
- **THEN** the emitted Rust calls it by name with the arguments in order

#### Scenario: Nesting is preserved

- **WHEN** an expression nests arithmetic inside a comparison inside a call argument
- **THEN** the emitted Rust evaluates it in the same grouping as the IR, regardless of Rust's
  operator precedence

### Requirement: True division always yields a float

A division node declaring float promotion yields a floating-point result even for integer operands,
whereas Rust's `/` between two integers is integer division. The backend SHALL emit code that
converts both operands to floating point before dividing whenever the node declares promotion.

#### Scenario: Integer operands

- **WHEN** a division of `7` by `2` declaring float promotion is emitted and executed
- **THEN** the result is `3.5`, not the `3` that Rust's `/` would produce

#### Scenario: Result type is float

- **WHEN** a function returning the result of a promoting division on two integers is emitted
- **THEN** the emitted Rust function returns `f64`

#### Scenario: Promotion is read from the node

- **WHEN** an integer division node that does not declare promotion is emitted
- **THEN** the emitted Rust does not convert its operands to floating point

### Requirement: Remaining operators

The backend SHALL emit addition, subtraction, and multiplication for numeric operands;
addition for string operands as concatenation; and the six comparisons, each yielding `bool`.

#### Scenario: String concatenation

- **WHEN** `"a" + "b"` is emitted and executed
- **THEN** the result is the string `ab`

#### Scenario: Comparisons yield bool

- **WHEN** each of the six comparison operators is emitted
- **THEN** each produces a `bool`

### Requirement: Arithmetic failures are recoverable, not panics

A generated function SHALL NOT abort the process on an arithmetic failure **that its node declares
reported**. Dividing by zero and exceeding the range of `i64` SHALL each produce a recoverable error
that the caller can observe and act on wherever the node declares that the failure is reported,
because those are conditions Python reports to the program rather than crashes.

Where a node declares the failure unchecked, the program has declined to define it and the backend
SHALL emit Rust's own operator, whose behavior on failure is Rust's. That is not an exception to this
requirement but its boundary: a recoverable error is what a *reported* failure produces.

#### Scenario: Integer division by zero

- **WHEN** a generated function evaluates `x // 0` from a node declaring the failure reported
- **THEN** it returns a recoverable error identifying division by zero, and the process
  continues running

#### Scenario: Remainder by zero

- **WHEN** a generated function evaluates `x % 0` from a node declaring the failure reported
- **THEN** it returns a recoverable error identifying division by zero

#### Scenario: Overflow is detected rather than wrapped

- **WHEN** a generated function computes a value exceeding the range of `i64` from a node declaring
  the failure reported
- **THEN** it returns a recoverable error identifying overflow, rather than wrapping to a
  negative number

#### Scenario: Errors propagate through calls

- **WHEN** a generated function calls another generated function that fails
- **THEN** the failure propagates to the outermost caller rather than being discarded

#### Scenario: A reported caller of an unchecked callee still propagates

- **WHEN** a function lowered under the default behavior calls one lowered under the target's
  behavior
- **THEN** the call compiles and any failure the callee reports still propagates

### Requirement: Emission is deterministic

The backend SHALL produce byte-identical output for the same unit across runs and across
addition orders, so that a rebuild decision made on the fingerprint is never contradicted by
the generated source. This SHALL hold for **every** file emitted, and the set of file names
itself SHALL be determined by the unit alone.

#### Scenario: Same unit, repeated emission

- **WHEN** the same unit is emitted twice in one process
- **THEN** the two outputs are byte-identical

#### Scenario: Addition order does not change output

- **WHEN** the same functions are added to two units in different orders and both are emitted
- **THEN** the two outputs are byte-identical

#### Scenario: The file set is stable

- **WHEN** two different units are emitted
- **THEN** both produce the same file names, differing only in contents

### Requirement: Emitted source is valid Rust

Output SHALL compile without errors or warnings under the same lint settings the project
applies to its own code, so that a malformed emission is caught at build time rather than
surfacing as an unexplained failure to the user. The files SHALL compile **together**, as the
crate they describe, rather than each being separately valid.

#### Scenario: Every accepted fixture compiles

- **WHEN** each accepted Python fixture is lowered and emitted
- **THEN** the resulting Rust compiles cleanly

#### Scenario: The crate root reaches every other file

- **WHEN** an emitted crate is compiled from its root file
- **THEN** every other emitted file is reached through a module declaration, so none is dead
  weight on disk

### Requirement: A function's docstring is emitted as documentation

When a function carries a docstring, the backend SHALL emit it as a doc comment on the generated
function. The generated source is written to disk for people to read, and a translated function
stripped of the explanation its author wrote is harder to check against the original than it
needs to be.

The emitted text SHALL denote the same characters as the docstring, including when it contains
characters that would otherwise end or escape a comment.

#### Scenario: A docstring reaches the generated source

- **WHEN** a function with a docstring is emitted
- **THEN** the generated Rust carries that text as a doc comment on the function

#### Scenario: A function without a docstring emits none

- **WHEN** a function with no docstring is emitted
- **THEN** no doc comment is emitted for it

#### Scenario: A multi-line docstring stays readable

- **WHEN** a function whose docstring spans several lines is emitted
- **THEN** each line appears in the doc comment, and the result compiles

#### Scenario: A docstring cannot break out of its comment

- **WHEN** a docstring containing a newline, a `*/`, and a backslash is emitted
- **THEN** the generated Rust still compiles and the comment denotes the original characters

#### Scenario: Emission stays deterministic

- **WHEN** the same documented function is emitted twice
- **THEN** the two outputs are byte-identical

### Requirement: Emission produces a named set of files

The backend SHALL emit a crate as a mapping from relative path to contents, rather than as one
source string. Each file SHALL hold one concern:

| File | Holds |
| --- | --- |
| `src/lib.rs` | module declarations and the module registration, and nothing that grows with the program |
| `src/generated.rs` | the translated functions, and nothing else |
| `src/bindings.rs` | the Python-boundary wrappers and the mapping from runtime failures to exceptions |
| `src/compat.rs` | the helpers reproducing Python's semantics |

The division exists to be **read**. Generated source is written to disk so a user can check what
their Python became; a single file that opens with two hundred identical lines in every project
buries the twelve lines they came for.

#### Scenario: The crate is emitted as separate files

- **WHEN** a unit is emitted
- **THEN** the result names each file separately rather than concatenating them

#### Scenario: Translated code stands alone

- **WHEN** the file holding translated functions is read
- **THEN** it contains the functions and nothing else — no helpers, no boundary code, no
  lint allowances

#### Scenario: The crate root does not grow with the program

- **WHEN** units of one function and of fifty functions are emitted
- **THEN** their crate roots are the same size

#### Scenario: Boundary code is separate from translated code

- **WHEN** a unit is emitted
- **THEN** the Python-boundary wrappers are in a different file from the translated functions

#### Scenario: The helpers are identical across projects

- **WHEN** two unrelated units are emitted
- **THEN** the file holding the Python-semantics helpers is byte-identical in both, since it
  depends on nothing about the program

#### Scenario: Emitting the same unit yields the same file set

- **WHEN** a unit is emitted twice
- **THEN** both results name exactly the same files

### Requirement: What is generated does not change

Rearranging output into files SHALL NOT change the code that is generated. The same functions,
helpers, and wrappers SHALL be produced, so a compiled artifact behaves exactly as before and no
fingerprint moves.

This is a readability change. Anything that alters behavior belongs in a change that says so.

#### Scenario: Fingerprints are unaffected

- **WHEN** a unit is fingerprinted before and after this change
- **THEN** the fingerprint is the same, because it is computed over the IR and not the output

#### Scenario: The compiled result is unchanged

- **WHEN** a unit is compiled and called before and after this change
- **THEN** every function returns the same values, including on the operands where Python and
  Rust semantics diverge

#### Scenario: The same helpers are present

- **WHEN** the emitted files are taken together
- **THEN** they contain the same helper definitions the single file previously did

### Requirement: Collection literals are emitted

The backend SHALL emit sequence, mapping, set, and tuple literals as constructions of the
corresponding Rust type, preserving element order as written for sequences and tuples.

#### Scenario: Sequence literal

- **WHEN** `[1, 2, 3]` is emitted and executed
- **THEN** the result is a sequence of those three values in that order

#### Scenario: Mapping literal

- **WHEN** `{"a": 1, "b": 2}` is emitted and executed
- **THEN** the result maps each key to its value

#### Scenario: Set literal

- **WHEN** `{1, 2, 2}` is emitted and executed
- **THEN** the result contains two distinct elements, matching Python's de-duplication

#### Scenario: Tuple literal

- **WHEN** `(1, "a")` is emitted and executed
- **THEN** the result is a pair carrying both values in order

#### Scenario: An empty literal is emitted from its declared type

- **WHEN** a binding annotated as a sequence of integers is initialised with an empty literal
- **THEN** the emitted Rust constructs an empty `Vec<i64>`

#### Scenario: Nested literals are emitted

- **WHEN** a mapping literal whose values are sequence literals is emitted and executed
- **THEN** the nesting is preserved in the result

### Requirement: Indexing preserves Python semantics

Python indexes a sequence from the end for a negative index; Rust does not, and would either
fail to compile or wrap into an enormous positive index. The backend SHALL emit code that resolves
a negative index against the sequence's length, so that `xs[-1]` is the last element.

Reading past the end of a sequence, or a key that is not in a mapping, SHALL produce a recoverable
error rather than a panic, because Python reports both to the program.

#### Scenario: Negative index counts from the end

- **WHEN** `xs[-1]` is emitted and executed against a three-element sequence
- **THEN** the result is the third element

#### Scenario: Negative index reaching the first element

- **WHEN** `xs[-3]` is emitted and executed against a three-element sequence
- **THEN** the result is the first element

#### Scenario: Positive index is unaffected

- **WHEN** `xs[0]` is emitted and executed
- **THEN** the result is the first element

#### Scenario: Index past the end is recoverable

- **WHEN** `xs[5]` is evaluated against a three-element sequence
- **THEN** a recoverable error identifying an out-of-range index is returned, and the process
  continues running

#### Scenario: Negative index past the start is recoverable

- **WHEN** `xs[-5]` is evaluated against a three-element sequence
- **THEN** a recoverable error identifying an out-of-range index is returned

#### Scenario: A missing mapping key is recoverable

- **WHEN** a key not present in a mapping is read
- **THEN** a recoverable error identifying the missing key is returned

#### Scenario: A tuple index is resolved at emission

- **WHEN** `t[1]` is emitted
- **THEN** the emitted Rust selects the second position directly and cannot fail at runtime

#### Scenario: Index errors propagate through calls

- **WHEN** a generated function calls another that reads past the end of a sequence
- **THEN** the failure propagates to the outermost caller

### Requirement: Length counts what Python counts

The backend SHALL emit a length that matches Python's. For a string this SHALL be the number of
characters, **not** the number of bytes: Rust's native string length counts UTF-8 bytes, so a
string containing any non-ASCII character would otherwise report a larger length than Python does.

#### Scenario: Length of a sequence, mapping, set, and tuple

- **WHEN** `len` is emitted for each and executed
- **THEN** each result is the number of elements

#### Scenario: Length of an ASCII string

- **WHEN** `len("abc")` is emitted and executed
- **THEN** the result is 3

#### Scenario: Length of a non-ASCII string counts characters

- **WHEN** `len("é")` is emitted and executed
- **THEN** the result is 1, not the 2 bytes its UTF-8 encoding occupies

#### Scenario: Length of a tuple is resolved at emission

- **WHEN** `len(t)` is emitted for a tuple
- **THEN** the emitted Rust uses the tuple's fixed length

### Requirement: Collections are emitted without moving a value that is used again

A collection is not copyable in Rust, so emitting it positionally where it is consumed would move
it, and a value used twice would fail to compile. The backend SHALL emit collections such that a
name may be read any number of times, on the same terms already applied to strings.

#### Scenario: A sequence parameter is read twice

- **WHEN** a function subscripts the same sequence parameter twice
- **THEN** the emitted Rust compiles

#### Scenario: A collection is passed to a call and read afterwards

- **WHEN** a function passes a sequence to another function and then takes its length
- **THEN** the emitted Rust compiles

#### Scenario: A collection is returned after being read

- **WHEN** a function reads an element of a sequence parameter and then returns the sequence
- **THEN** the emitted Rust compiles

### Requirement: Control flow is emitted

The backend SHALL emit conditionals, both loop forms, and both loop controls, preserving the
nesting the IR carries.

#### Scenario: A conditional is emitted

- **WHEN** a conditional with an alternative is emitted and executed
- **THEN** the branch matching the test runs and the other does not

#### Scenario: A conditional without an alternative is emitted

- **WHEN** a conditional with no alternative is emitted and executed with a false test
- **THEN** neither branch's effects occur and execution continues after it

#### Scenario: A while loop is emitted

- **WHEN** a loop counting to ten is emitted and executed
- **THEN** the counter ends at ten

#### Scenario: A loop that never runs

- **WHEN** a loop whose test is false at entry is emitted and executed
- **THEN** its body does not run

#### Scenario: Loop control is emitted

- **WHEN** a loop containing `break` and `continue` is emitted and executed
- **THEN** it terminates and skips iterations as Python would

#### Scenario: Nesting is preserved

- **WHEN** a loop containing a conditional containing a loop is emitted and executed
- **THEN** the result matches the interpreted original

### Requirement: Ranges match Python, including a negative step

The backend SHALL emit iteration over a range that produces exactly the values Python produces,
for any combination of start, stop, and step. Rust's `..` counts upward by one and cannot express
a negative step, so a range SHALL NOT be emitted as one.

A step of zero SHALL be a recoverable error rather than a loop that never terminates, matching
Python, which raises for it.

#### Scenario: A simple range

- **WHEN** `for i in range(3)` is emitted and executed
- **THEN** the values are 0, 1, 2

#### Scenario: A bounded range

- **WHEN** `for i in range(2, 5)` is emitted and executed
- **THEN** the values are 2, 3, 4

#### Scenario: A stepped range

- **WHEN** `for i in range(0, 6, 2)` is emitted and executed
- **THEN** the values are 0, 2, 4

#### Scenario: A negative step counts down

- **WHEN** `for i in range(3, 0, -1)` is emitted and executed
- **THEN** the values are 3, 2, 1 — which Rust's `..` cannot produce

#### Scenario: An empty range

- **WHEN** `for i in range(5, 0)` is emitted and executed
- **THEN** the body does not run

#### Scenario: A zero step is recoverable

- **WHEN** a range with a step of zero is evaluated
- **THEN** a recoverable error is returned, rather than the loop running forever

### Requirement: Iterating a collection yields what Python yields

The backend SHALL emit iteration over a sequence yielding its elements in order, over a set
yielding its elements, and over a mapping yielding its **keys**.

Iteration SHALL NOT consume the collection: a name may be iterated and then read again, on the
same terms as every other read.

#### Scenario: Sequence order is preserved

- **WHEN** a sequence is iterated and its elements collected
- **THEN** they appear in the order the sequence holds

#### Scenario: A mapping yields keys

- **WHEN** a mapping is iterated
- **THEN** the loop variable takes each key, matching Python

#### Scenario: A collection is not consumed by iteration

- **WHEN** a function iterates a sequence parameter and then takes its length
- **THEN** the emitted Rust compiles

#### Scenario: Mapping and set order is not guaranteed

- **WHEN** a mapping or set is iterated
- **THEN** the order is unspecified and may differ between runs, consistent with the map type the
  backend uses

### Requirement: A reassigned local is emitted as mutable

The backend SHALL emit a local that is assigned more than once as a mutable binding, and one that
is not as an immutable binding, so that generated code carries no avoidable warnings under the
lint settings the project applies to its own code.

#### Scenario: A rebound local compiles

- **WHEN** a function incrementing a counter is emitted
- **THEN** the emitted Rust compiles

#### Scenario: A local bound once is not mutable

- **WHEN** a function binding a local once is emitted
- **THEN** the emitted binding is not marked mutable

#### Scenario: A reassigned parameter compiles

- **WHEN** a function assigning to its own parameter is emitted
- **THEN** the emitted Rust compiles

#### Scenario: Emitted control flow carries no warnings

- **WHEN** every accepted fixture using control flow is emitted and compiled with warnings denied
- **THEN** it compiles cleanly

### Requirement: Mutation is emitted in place

The backend SHALL emit a mutated collection as a single binding that is modified, not as a value
that is copied and then modified. A collection that is mutated SHALL be bound mutably, and one that
is not SHALL NOT be.

The backend clones collections wherever they are consumed, so that a name read twice is not moved.
That rule must not apply to the target of a mutation: mutating a clone changes a value nothing
reads afterwards, which compiles cleanly and does nothing.

#### Scenario: Appending in a loop accumulates

- **WHEN** a function that binds an empty sequence, appends in a loop, and returns it is emitted
  and executed
- **THEN** the returned sequence holds every appended element

#### Scenario: Element assignment takes effect

- **WHEN** a function that assigns to an element and then reads it is emitted and executed
- **THEN** the read observes the assigned value

#### Scenario: A mutated collection is bound mutably

- **WHEN** a function that mutates a local collection is emitted
- **THEN** the emitted binding is mutable, and the source compiles

#### Scenario: An unmutated collection is not bound mutably

- **WHEN** a function that only reads a local collection is emitted
- **THEN** the emitted binding is not marked mutable, so no warning is produced

#### Scenario: Mutation and reading compose

- **WHEN** a function mutates a collection and then takes its length
- **THEN** the emitted Rust compiles and the length reflects the mutation

### Requirement: Assigning a mapping key inserts it

The backend SHALL emit assignment to a mapping key as an insertion. Reading a missing key is an
error; assigning to one is not, and Python creates it.

#### Scenario: Assigning a new key creates it

- **WHEN** a function assigns to a key not present and then reads it
- **THEN** the read succeeds and observes the assigned value

#### Scenario: Assigning an existing key replaces it

- **WHEN** a function assigns twice to the same key
- **THEN** the second value is observed

#### Scenario: Reading a missing key still fails

- **WHEN** a function reads a key that was never assigned
- **THEN** a recoverable error is returned, unchanged by this requirement

### Requirement: Membership is emitted for every container

The backend SHALL emit membership over sequences, mappings, sets, and strings, testing a mapping's
keys and a string's substrings, matching Python.

#### Scenario: Sequence membership

- **WHEN** membership over a sequence is emitted and executed
- **THEN** the result is true exactly when the value is present

#### Scenario: Mapping membership tests keys

- **WHEN** membership over a mapping is emitted and executed
- **THEN** the result reflects the keys, not the values

#### Scenario: Set membership

- **WHEN** membership over a set is emitted and executed
- **THEN** the result is true exactly when the element is present

#### Scenario: String membership is a substring test

- **WHEN** membership over a string is emitted and executed
- **THEN** it reports whether the first is a substring of the second, matching Python

#### Scenario: Negated membership

- **WHEN** `not in` is emitted and executed
- **THEN** the result is the negation of the corresponding membership test

#### Scenario: Membership does not consume the container

- **WHEN** a function tests membership and then reads the container
- **THEN** the emitted Rust compiles

### Requirement: A class emits a struct and an implementation

The backend SHALL emit each class as a data type carrying its attributes as fields in declaration
order, and an implementation block carrying its methods. Attribute types SHALL use the same
spellings every other type does.

#### Scenario: Attributes become fields

- **WHEN** a class declaring three attributes is emitted
- **THEN** the emitted type carries three fields with the corresponding spellings

#### Scenario: Methods become an implementation

- **WHEN** a class with two methods is emitted
- **THEN** both appear in one implementation block for that type

#### Scenario: __init__ becomes a constructor

- **WHEN** a class is emitted
- **THEN** it carries a constructor initialising every field

#### Scenario: Methods are fallible

- **WHEN** a method is emitted
- **THEN** it yields either its declared return type or a runtime error, on the same terms as every
  free function

#### Scenario: Emission is deterministic

- **WHEN** the same unit containing classes is emitted twice
- **THEN** the two outputs are byte-identical

#### Scenario: Classes and functions are emitted into the same file

- **WHEN** a unit holding both is emitted
- **THEN** the translated file holds both, with nothing else added to the crate root

### Requirement: A method takes a mutable receiver only when it needs one

The backend SHALL emit a method that assigns to an attribute, or mutates a collection attribute,
with a mutable receiver, and every other method with a shared one.

Emitting a mutable receiver everywhere would make two methods unusable on the same object at once,
and the failure would be a borrow-checker complaint about generated code rather than a diagnostic
about the user's program.

#### Scenario: A mutating method compiles

- **WHEN** a method that assigns to an attribute is emitted
- **THEN** the emitted Rust compiles

#### Scenario: A reading method takes a shared receiver

- **WHEN** a method that only reads attributes is emitted
- **THEN** its receiver is shared, so it can be called while another borrow is held

#### Scenario: A method mutating a collection attribute is mutating

- **WHEN** a method that inserts into a mapping attribute is emitted
- **THEN** it takes a mutable receiver and the emitted Rust compiles

#### Scenario: A method calling a mutating method is mutating

- **WHEN** a method whose body calls another method that mutates is emitted
- **THEN** it also takes a mutable receiver, since it mutates transitively

#### Scenario: Reading and mutating compose

- **WHEN** a method reads an attribute, calls a mutating method, and reads again
- **THEN** the emitted Rust compiles

### Requirement: Attribute access and construction are emitted

The backend SHALL emit attribute reads, attribute assignments, and constructions. A collection or
instance attribute SHALL be read without being moved out of the object.

#### Scenario: An attribute read yields its value

- **WHEN** a method reading an integer attribute is emitted and executed
- **THEN** the value is the attribute's

#### Scenario: An attribute assignment persists

- **WHEN** a method assigns an attribute and a later call reads it
- **THEN** the later call observes the assigned value

#### Scenario: A collection attribute is not moved by a read

- **WHEN** a method reads a mapping attribute twice
- **THEN** the emitted Rust compiles

#### Scenario: Construction initialises every field

- **WHEN** a construction is emitted and executed
- **THEN** the resulting object's attributes hold what `__init__` assigned

#### Scenario: State outlives a call

- **WHEN** a method mutates an attribute and is called twice
- **THEN** the second call observes the first call's effect — which is what makes a cache possible

### Requirement: Integer division honors the declared rounding mode

The backend SHALL emit integer division that rounds the way the node declares, not the way Rust's
`/` happens to round. A node declaring rounding toward negative infinity SHALL floor; a node
declaring rounding toward zero SHALL truncate. This SHALL hold for integer and floating-point
operands alike. Where the node declares rounding toward zero **and** an unchecked zero divisor, the
emitted text SHALL be Rust's `/`, because that is exactly what Rust's `/` means.

#### Scenario: Negative dividend, flooring declared

- **WHEN** a division of `-7` by `2` declaring rounding toward negative infinity is emitted and
  executed
- **THEN** the result is `-4`, not the `-3` that Rust's `/` would produce

#### Scenario: Negative divisor, flooring declared

- **WHEN** a division of `7` by `-2` declaring rounding toward negative infinity is emitted and
  executed
- **THEN** the result is `-4`

#### Scenario: Negative dividend, truncation declared

- **WHEN** a division of `-7` by `2` declaring rounding toward zero is emitted and executed
- **THEN** the result is `-3`

#### Scenario: Exact division is unaffected

- **WHEN** a division of `-6` by `2` is emitted and executed under either rounding mode
- **THEN** the result is `-3`

#### Scenario: Floating-point division under flooring

- **WHEN** a division of `-7.0` by `2.0` declaring rounding toward negative infinity is emitted and
  executed
- **THEN** the result is `-4.0`

#### Scenario: Truncating and unchecked emits Rust's own division

- **WHEN** a division declaring rounding toward zero and an unchecked zero divisor is emitted
- **THEN** the emitted text is Rust's `/`, with no helper call and no `?`

### Requirement: Remainder honors the declared sign convention

The backend SHALL emit a remainder whose sign follows the convention the node declares. A node
declaring the sign of the divisor SHALL NOT be emitted as Rust's `%`, which takes the sign of the
dividend; a node declaring the sign of the dividend and an unchecked zero divisor SHALL be.

#### Scenario: Negative dividend, sign of divisor declared

- **WHEN** `-7 % 2` declaring the sign of the divisor is emitted and executed
- **THEN** the result is `1`, not the `-1` that Rust's `%` would produce

#### Scenario: Negative divisor, sign of divisor declared

- **WHEN** `7 % -2` declaring the sign of the divisor is emitted and executed
- **THEN** the result is `-1`

#### Scenario: Negative dividend, sign of dividend declared

- **WHEN** `-7 % 2` declaring the sign of the dividend is emitted and executed
- **THEN** the result is `-1`

#### Scenario: Remainder and division stay consistent

- **WHEN** any operand pair is evaluated for a division and a remainder declaring matching
  conventions
- **THEN** `(a / b) * b + (a % b)` equals `a`

#### Scenario: Sign of dividend and unchecked emits Rust's own remainder

- **WHEN** a remainder declaring the sign of the dividend and an unchecked zero divisor is emitted
- **THEN** the emitted text is Rust's `%`, with no helper call and no `?`

### Requirement: The Rust backend declares what it preserves

The Rust backend SHALL declare the semantic guarantees it preserves, and SHALL be usable only with
units whose requirements its declaration covers. At minimum it SHALL declare that an arithmetic
result outside the range of its integer type is reported rather than wrapped, that division by zero
is reported, and that floating-point arithmetic is not reordered.

A guarantee is a promise about what the backend does with a node that *asks* for it. Emitting Rust's
native `+` for a node that declares overflow unchecked SHALL NOT count against the overflow
guarantee, because that node did not ask for it.

#### Scenario: Declaration covers the Python frontend

- **WHEN** a unit lowered under Python's stance is checked against the Rust backend's declaration
- **THEN** every required guarantee is covered and compilation proceeds

#### Scenario: The declaration is checked, not assumed

- **WHEN** a unit requires a guarantee the Rust backend does not declare
- **THEN** compilation fails before emission, naming the guarantee

#### Scenario: A waived guarantee is not a violated one

- **WHEN** a unit whose arithmetic is unchecked is emitted as native Rust operators
- **THEN** the backend still declares that it preserves overflow reporting, because the nodes that
  ask for it still get it

### Requirement: Rust post-processing is limited to meaning-preserving transformations

Transformations the backend applies to generated Rust after emission SHALL be limited to those that
do not change what the code computes, unless configuration explicitly permits otherwise. Formatting
generated source for readability SHALL be permitted unconditionally, SHALL happen outside emission,
and SHALL be a no-op when the formatter is unavailable. Build settings that would violate a
guarantee **the unit requires** SHALL NOT be applied by default.

#### Scenario: Formatting is applied when writing source out

- **WHEN** generated Rust is written to disk for a human to read
- **THEN** it is formatted, and the formatting is not part of emission

#### Scenario: A missing formatter costs readability only

- **WHEN** no formatter is available
- **THEN** the unformatted source is written and the build succeeds

#### Scenario: Guarantee-violating build settings are withheld

- **WHEN** a build setting would allow arithmetic to wrap rather than report, and the unit requires
  overflow be reported
- **THEN** it is not applied, and the reason is reportable

#### Scenario: A unit that waives the guarantee may permit the setting

- **WHEN** a unit's behavior leaves every arithmetic operation unchecked, so it requires no overflow
  reporting
- **THEN** the setting is no longer withheld for that unit

### Requirement: Subscripting honors the declared index origin

The backend SHALL emit a sequence read that resolves a negative index the way the node declares. A
node declaring *from either end* SHALL count a negative index backwards from the end; a node
declaring *from the start* SHALL treat a negative index as out of range. A node declaring that a
failure is **reported** SHALL report a read outside the sequence rather than panicking; a node
declaring it **unchecked** SHALL emit Rust's own indexing, whose out-of-range behavior is Rust's.

#### Scenario: Negative index, counting from either end

- **WHEN** a sequence of three elements is read at index `-1` under an origin of *from either end*
- **THEN** the result is the last element

#### Scenario: Negative index, counting from the start

- **WHEN** the same read is emitted under an origin of *from the start* with the failure reported
- **THEN** the failure is reported as an index out of range

#### Scenario: A non-negative index is unaffected by the origin

- **WHEN** a sequence is read at index `1` under either origin
- **THEN** both produce the second element

#### Scenario: Reading past the end is reported under either origin

- **WHEN** a sequence of three elements is read at index `3` from a node declaring the failure
  reported, under either origin
- **THEN** the failure is reported rather than the process aborting

#### Scenario: An unchecked in-range read emits native indexing

- **WHEN** a sequence read declaring *from the start* and unchecked failure is emitted
- **THEN** the emitted text is Rust's own indexing, with no bounds resolution helper and no `?`

#### Scenario: An unchecked mapping read emits native indexing

- **WHEN** a mapping read declaring unchecked failure is emitted
- **THEN** the emitted text is Rust's own indexing of the map, rather than the helper that reports
  a missing key

### Requirement: Length honors the declared text units

The backend SHALL emit a length that counts in the units the node declares. For a value that is not
text the declaration SHALL make no difference, because the length of a collection is a count of its
elements under every reading. Where the node declares UTF-8 bytes, the emitted text SHALL be Rust's
own byte length, which is what Rust means by the length of a string.

#### Scenario: Each unit reading counts differently

- **WHEN** the length of a string containing a two-byte character is emitted under each of code
  points, UTF-8 bytes, and UTF-16 units
- **THEN** the three results differ where the readings differ, and the byte count exceeds the code
  point count

#### Scenario: A character outside the basic plane distinguishes all three

- **WHEN** the length of a string containing a character requiring a surrogate pair is emitted under
  each reading
- **THEN** code points, UTF-8 bytes, and UTF-16 units each report a different number

#### Scenario: A collection's length ignores the declaration

- **WHEN** the length of a sequence is emitted under any declared units
- **THEN** the result is the number of elements

#### Scenario: UTF-8 bytes emit Rust's own length

- **WHEN** the length of a string declaring UTF-8 bytes is emitted
- **THEN** the emitted text is Rust's own length of the string, with no counting helper

### Requirement: An accumulator that reads itself updates in place

Where a statement assigns to a name from an expression that reads that same name as the left
operand of an addition — the shape `x = x + y` — the backend SHALL emit an in-place update rather
than building a new value and rebinding it.

This is not a micro-optimization for text. Building a fresh value per iteration makes accumulation
quadratic, and CPython resizes in place when the target holds the only reference, so the current
emission is asymptotically *worse* than the interpreter it replaces. Measured on `text.joined`:
343.76us to 83.08us, a 4.1x difference that moves the workload from losing to the interpreter to
beating it.

The emission SHALL stay type-directed. The backend does not know an expression's type and must not
learn it here; the in-place form is selected through a trait whose implementations differ per type,
exactly as the existing addition is.

#### Scenario: String accumulation appends in place

- **WHEN** a `str` local is assigned from itself plus another value
- **THEN** the emitted code appends to the existing value rather than allocating a new one

#### Scenario: Numeric accumulation keeps its checking

- **WHEN** an `int` local is assigned from itself plus another value
- **THEN** the emitted code performs the same checked addition it does today, and still reports
  overflow

#### Scenario: The name must be the left operand

- **WHEN** the assigned name appears somewhere other than as the left operand of the addition
- **THEN** the ordinary emission is used, because the in-place form would read a value that has
  already been modified

### Requirement: A loop variable that is only read is borrowed

Where a `for` iterates a collection and the loop body never assigns to, moves, or mutates the loop
variable, the backend SHALL bind it by reference rather than cloning each element.

For a collection of scalars this costs nothing either way; for a collection of owned values it is
an allocation and a copy per element per loop. Measured on `text.total_length`, whose body is a
single length read per element: 88.52us to 59.43us.

Whether the body assigns to the loop variable is already computed, because it decides whether the
binding is emitted as mutable. The same answer decides this.

#### Scenario: A read-only loop variable is not cloned

- **WHEN** a loop body only reads its loop variable
- **THEN** the emitted loop binds it by reference

#### Scenario: A written loop variable is still owned

- **WHEN** a loop body assigns to its loop variable
- **THEN** the emitted loop binds an owned value, so the assignment is legal and does not affect
  what is iterated

#### Scenario: The runtime accepts a borrowed value wherever an owned one works

- **WHEN** a borrowed loop variable is passed to a runtime helper
- **THEN** the helper accepts it, so borrowing a loop variable never turns a working program into
  one that does not compile

### Requirement: A local returned in tail position is moved

Where a function's final statement returns a bare local name, the backend SHALL move that value
rather than cloning it. The function is ending and the original is about to be dropped, so the copy
has no reader.

The restriction to tail position is deliberate and load-bearing: a `return` nested inside a loop
that iterates the same name would move out of a value the loop borrows. Tail position is the last
statement at the top level of the body and therefore cannot sit inside any loop, which makes the
move safe by construction rather than by analysis.

#### Scenario: A returned collection is not copied

- **WHEN** a function's last statement returns a local holding a collection
- **THEN** the emitted code moves it into the result

#### Scenario: A return inside a loop is unchanged

- **WHEN** a `return` of a local appears anywhere other than tail position
- **THEN** the existing emission is used

#### Scenario: Returning a field still copies

- **WHEN** a function returns an attribute rather than a local
- **THEN** it is copied, because the instance outlives the call and must not be emptied

### Requirement: Generated maps and sets are parameterised over their hasher

The runtime's implementations for mapping and set types SHALL be generic over the hasher rather
than written against the standard library's default, and generated code SHALL select the hasher it
uses rather than inheriting one.

Today the hasher is not a choice at all: the implementations are written against the two-parameter
form of the container types, which silently pins the default hasher across every one of them. That
is a defect independent of which hasher is preferred — it means the decision cannot be expressed.

The selected default SHALL be a non-cryptographic hasher. Keys in generated code come from the
user's own program rather than from an untrusted source, and the interpreter being compared
against hashes small integers to themselves and caches a string's hash in the string. Measured:
`graphs.bfs_distances` 159.36us to 82.49us, which moves it from 0.7x to 1.4x against interpreted;
`graphs.topological_order` 421.48us to 271.33us.

A hasher has no observable semantics, so this is a performance choice and not a behavior axis. It
SHALL NOT be exposed as one.

#### Scenario: The runtime accepts any hasher

- **WHEN** the runtime's mapping and set implementations are compiled
- **THEN** they are generic over the hasher, and a container using a non-default hasher satisfies
  every one of them

#### Scenario: Container literals build with the selected hasher

- **WHEN** a mapping or set literal is emitted
- **THEN** it constructs a container using the selected hasher rather than a form available only
  for the default one

#### Scenario: Iteration order remains unguaranteed

- **WHEN** a mapping or set is iterated in generated code
- **THEN** no order is guaranteed, exactly as before, and no test may depend on one

### Requirement: The runtime does not repeat work it has already done

Runtime helpers SHALL NOT perform work a caller or an earlier step has already performed.

Three instances are known and measured as a group at 2.7x on `text.word_count`'s body: resolving an
index validates the offset and then indexes through a checked operation that validates it again;
computing a text length under a code-point reading decodes the entire string on every call, where
the common case admits an exact shortcut; and the read-modify-write of a mapping entry performs
three separate lookups of the same key.

#### Scenario: An index is validated once

- **WHEN** a sequence element is read through the runtime
- **THEN** the offset is checked once, and an out-of-range index is still reported rather than
  panicking

#### Scenario: Text length keeps its declared units

- **WHEN** a text length is computed under any units the IR declares
- **THEN** the answer is exactly what it is today for every input, including non-ASCII text

#### Scenario: A mapping read-modify-write is not three lookups

- **WHEN** generated code reads a mapping entry, derives a new value, and stores it back under the
  same key
- **THEN** the emitted code does not hash that key three separate times

#### Scenario: A missing key still reports

- **WHEN** a mapping entry that is absent is read
- **THEN** it is reported exactly as it is today, and the fused form does not create it

### Requirement: The Rust backend declares Rust's stance on every behavior axis

The Rust backend SHALL declare, for every behavior axis, what Rust means by that operation. The
declaration SHALL be complete and SHALL describe Rust only.

Rust's stance SHALL be: integer arithmetic leaves overflow undefined by the program; integer
division rounds toward zero and leaves a zero divisor undefined by the program; exact division
leaves a zero divisor undefined by the program, yielding the IEEE-754 result; remainder takes the
sign of the dividend and leaves a zero divisor undefined by the program; a subscript treats a
negative index as out of range and leaves an out-of-range access undefined by the program; a length
counts UTF-8 bytes.

#### Scenario: The stance is complete

- **WHEN** the Rust backend is asked what Rust means on each axis
- **THEN** it answers for every axis defined by the behavior model

#### Scenario: The stance names only Rust

- **WHEN** the Rust backend's declared stance is inspected
- **THEN** it describes Rust's meanings and refers to no source language

### Requirement: A node declaring the target's meaning emits the target's own operator

Where a node's declared modes are exactly what Rust's own operator means, the backend SHALL emit
that operator rather than a helper that reproduces some other language's meaning. An unchecked
integer addition SHALL emit as Rust's `+`, an unchecked truncating division as Rust's `/`, an
unchecked remainder taking the sign of the dividend as Rust's `%`, and an unchecked subscript from
the start as Rust's own indexing.

The backend SHALL make this decision from the modes on the node alone. It SHALL NOT consult which
frontend produced the unit, and SHALL NOT infer that the target's meaning was intended from
anything other than the declared modes.

#### Scenario: Unchecked arithmetic emits the native operator

- **WHEN** an integer addition declaring unchecked overflow is emitted with a known integer
  expected type
- **THEN** the emitted text is Rust's `+` applied to the operands, with no helper call and no `?`

#### Scenario: Reported arithmetic still emits the helper

- **WHEN** an integer addition declaring reported overflow is emitted
- **THEN** the emitted text calls the helper that reports overflow, exactly as before

#### Scenario: The decision reads the node, not the frontend

- **WHEN** a hand-built unit with no recorded frontend declares unchecked truncating division
- **THEN** the emitted text is Rust's `/`

#### Scenario: A partially native node keeps its helper

- **WHEN** an integer division declares rounding toward negative infinity and unchecked failure
- **THEN** the emitted text still corrects the rounding, because Rust's `/` does not floor

### Requirement: An operand whose type is not known emits through an infallible helper

The backend does not annotate expressions with types and SHALL NOT re-derive them. Where the
expected type of an arithmetic operation is known to be integer or floating-point, the backend
SHALL emit Rust's operator directly. Where it is not known — as inside a comparison, whose operands
say nothing about the result type — the backend SHALL emit through a helper that dispatches on the
operand type and returns a value rather than a result.

Such a helper SHALL be infallible: it exists to select an implementation, not to check anything, and
SHALL compile to the same code the operator would.

#### Scenario: An unknown expected type dispatches

- **WHEN** an unchecked addition appears as an operand of a comparison
- **THEN** it is emitted through an infallible dispatching helper rather than as a bare operator

#### Scenario: The helper is infallible

- **WHEN** an infallible helper is used
- **THEN** the emitted expression carries no `?` and the enclosing statement needs no error path

#### Scenario: String concatenation still works under unchecked arithmetic

- **WHEN** two strings are added under a behavior declaring unchecked overflow
- **THEN** the emitted code concatenates them correctly, because a bare Rust `+` on two owned
  strings would not compile

### Requirement: Generated signatures do not depend on the behavior

Every generated function SHALL return a result type that can carry a failure, whatever behavior its
body was lowered under. A function whose operations are all unchecked SHALL still have the same
signature as one whose operations report, so that changing a behavior flag — or editing an unrelated
statement — never moves a signature.

#### Scenario: An all-unchecked function keeps its signature

- **WHEN** a function whose every operation is unchecked is emitted
- **THEN** its signature is the same fallible one it would have had under the default behavior

#### Scenario: The body carries no error propagation it does not need

- **WHEN** an all-unchecked function body is emitted
- **THEN** no `?` appears in it, and its returns are wrapped once at the boundary

#### Scenario: The bridge is unchanged by behavior

- **WHEN** functions lowered under different behaviors are exposed to Python
- **THEN** the generated bindings call every one of them the same way
