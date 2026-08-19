## Purpose

Translates compylr IR into Rust source text. This is where the IR's deliberately abstract type
model meets concrete spellings, and where Python's arithmetic semantics must be reproduced
rather than delegated to Rust's same-named operators, which disagree on negative and integer
operands.

## Requirements

### Requirement: Concrete type spellings

The backend SHALL map each IR type to a Rust type. The mapping SHALL live in the backend
alone: no IR type carries a Rust spelling, so a second backend can choose different ones for
the same IR. Collection spellings SHALL be derived from their parameters, recursively.

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

### Requirement: Floor division preserves Python semantics

Python's `//` rounds toward negative infinity; Rust's `/` truncates toward zero. The backend
SHALL emit code that floors, so that the two disagree on no input. This SHALL hold for
integer and floating-point operands alike.

#### Scenario: Negative dividend

- **WHEN** `-7 // 2` is emitted and executed
- **THEN** the result is `-4`, not the `-3` that Rust's `/` would produce

#### Scenario: Negative divisor

- **WHEN** `7 // -2` is emitted and executed
- **THEN** the result is `-4`

#### Scenario: Exact division is unaffected

- **WHEN** `-6 // 2` is emitted and executed
- **THEN** the result is `-3`

#### Scenario: Floating-point floor division

- **WHEN** `-7.0 // 2.0` is emitted and executed
- **THEN** the result is `-4.0`

### Requirement: Remainder preserves Python semantics

Python's `%` takes the sign of the divisor; Rust's `%` takes the sign of the dividend. The
backend SHALL emit code matching Python for every combination of operand signs.

#### Scenario: Negative dividend

- **WHEN** `-7 % 2` is emitted and executed
- **THEN** the result is `1`, not the `-1` that Rust's `%` would produce

#### Scenario: Negative divisor

- **WHEN** `7 % -2` is emitted and executed
- **THEN** the result is `-1`

#### Scenario: Remainder and floor division stay consistent

- **WHEN** any operand pair is evaluated for both `//` and `%`
- **THEN** `(a // b) * b + (a % b)` equals `a`

### Requirement: True division always yields a float

Python's `/` between two integers yields a float; Rust's `/` between two integers is integer
division. The backend SHALL emit code that converts both operands to floating point before
dividing.

#### Scenario: Integer operands

- **WHEN** `7 / 2` is emitted and executed
- **THEN** the result is `3.5`, not the `3` that Rust's `/` would produce

#### Scenario: Result type is float

- **WHEN** a function returning the result of `/` on two integers is emitted
- **THEN** the emitted Rust function returns `f64`

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

A generated function SHALL NOT abort the process on an arithmetic failure. Dividing by zero
and exceeding the range of `i64` SHALL each produce a recoverable error that the caller can
observe and act on, because these are conditions Python reports to the program rather than
crashes.

#### Scenario: Integer division by zero

- **WHEN** a generated function evaluates `x // 0`
- **THEN** it returns a recoverable error identifying division by zero, and the process
  continues running

#### Scenario: Remainder by zero

- **WHEN** a generated function evaluates `x % 0`
- **THEN** it returns a recoverable error identifying division by zero

#### Scenario: Overflow is detected rather than wrapped

- **WHEN** a generated function computes a value exceeding the range of `i64`
- **THEN** it returns a recoverable error identifying overflow, rather than wrapping to a
  negative number

#### Scenario: Errors propagate through calls

- **WHEN** a generated function calls another generated function that fails
- **THEN** the failure propagates to the outermost caller rather than being discarded

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
