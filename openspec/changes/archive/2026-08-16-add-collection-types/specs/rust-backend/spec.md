## MODIFIED Requirements

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

## ADDED Requirements

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
