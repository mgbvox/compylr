## ADDED Requirements

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

## MODIFIED Requirements

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
