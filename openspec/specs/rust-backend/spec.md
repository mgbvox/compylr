## Purpose

Translates compylr IR into Rust source text. This is where the IR's deliberately abstract type
model meets concrete spellings, and where Python's arithmetic semantics must be reproduced
rather than delegated to Rust's same-named operators, which disagree on negative and integer
operands.

## Requirements

### Requirement: Concrete type spellings

The backend SHALL map each IR type to a Rust type. The mapping SHALL live in the backend
alone: no IR type carries a Rust spelling, so a second backend can choose different ones for
the same IR.

| IR type | Rust type |
| --- | --- |
| integer | `i64` |
| float | `f64` |
| bool | `bool` |
| string | `String` |
| unit | `()` |

#### Scenario: Each type is spelled

- **WHEN** a function's parameters and return type cover all five IR types
- **THEN** the emitted Rust uses `i64`, `f64`, `bool`, `String`, and `()` respectively

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
