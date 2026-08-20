## ADDED Requirements

### Requirement: Integer division honors the declared rounding mode

The backend SHALL emit integer division that rounds the way the node declares, not the way Rust's
`/` happens to round. A node declaring rounding toward negative infinity SHALL floor; a node
declaring rounding toward zero SHALL truncate. This SHALL hold for integer and floating-point
operands alike.

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

### Requirement: Remainder honors the declared sign convention

The backend SHALL emit a remainder whose sign follows the convention the node declares. A node
declaring the sign of the divisor SHALL NOT be emitted as Rust's `%`, which takes the sign of the
dividend; a node declaring the sign of the dividend MAY be.

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

### Requirement: The Rust backend declares what it preserves

The Rust backend SHALL declare the semantic guarantees it preserves, and SHALL be usable only with
frontends whose requirements its declaration covers. At minimum it SHALL declare that an arithmetic
result outside the range of its integer type is reported rather than wrapped, that division by zero
is reported, and that floating-point arithmetic is not reordered.

#### Scenario: Declaration covers the Python frontend

- **WHEN** the Python frontend's requirements are checked against the Rust backend's declaration
- **THEN** every required guarantee is covered and compilation proceeds

#### Scenario: The declaration is checked, not assumed

- **WHEN** a frontend requires a guarantee the Rust backend does not declare
- **THEN** compilation fails before emission, naming the guarantee

### Requirement: Rust post-processing is limited to meaning-preserving transformations

Transformations the backend applies to generated Rust after emission SHALL be limited to those that
do not change what the code computes, unless configuration explicitly permits otherwise. Formatting
generated source for readability SHALL be permitted unconditionally, SHALL happen outside emission,
and SHALL be a no-op when the formatter is unavailable. Build settings that would violate a declared
guarantee SHALL NOT be applied by default.

#### Scenario: Formatting is applied when writing source out

- **WHEN** generated Rust is written to disk for a human to read
- **THEN** it is formatted, and the formatting is not part of emission

#### Scenario: A missing formatter costs readability only

- **WHEN** no formatter is available
- **THEN** the unformatted source is written and the build succeeds

#### Scenario: Guarantee-violating build settings are withheld

- **WHEN** a build setting would allow arithmetic to wrap rather than report
- **THEN** it is not applied, because the frontend requires overflow be reported

## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: Floor division preserves Python semantics

**Reason**: The backend no longer knows that its input came from Python. It renders whatever rounding
the node declares, which is what allows the same backend to serve a source language that truncates.

**Migration**: Replaced by "Integer division honors the declared rounding mode". Behavior for the
Python frontend is unchanged — Python declares rounding toward negative infinity, so `-7 // 2` is
still `-4`. Existing fixtures and their expected values carry over unchanged.

### Requirement: Remainder preserves Python semantics

**Reason**: Same as above — the sign convention is read from the node rather than assumed to be
Python's.

**Migration**: Replaced by "Remainder honors the declared sign convention". Behavior for the Python
frontend is unchanged — Python declares the sign of the divisor, so `-7 % 2` is still `1`.
