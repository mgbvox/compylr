## MODIFIED Requirements

### Requirement: Value conversion across the boundary

Each IR type SHALL convert between its Python and Rust representations in both directions:
integers, floats, booleans, and strings as arguments and as return values, the unit type as `None`
on return, and the collection types as their Python counterparts — sequences as `list`, mappings as
`dict`, sets as `set`, and tuples as `tuple` — nested to any depth.

#### Scenario: Each type round-trips

- **WHEN** a compiled function takes a parameter of a given scalar type and returns it unchanged
- **THEN** calling it with a Python value of that type returns an equal Python value of the
  same type

#### Scenario: Each collection type round-trips

- **WHEN** a compiled function takes a `list`, `dict`, `set`, or `tuple` parameter and returns it
  unchanged
- **THEN** calling it returns an equal Python value of the same kind

#### Scenario: Nested collections round-trip

- **WHEN** a compiled function takes a `dict[str, list[int]]` and returns it unchanged
- **THEN** the nesting and the values are preserved

#### Scenario: Unit return

- **WHEN** a compiled function annotated `-> None` is called
- **THEN** it returns `None`

#### Scenario: Booleans are not integers at the boundary

- **WHEN** a compiled function declaring a `bool` return is called
- **THEN** the returned value is a Python `bool`, consistent with the IR's rule that booleans
  are not numbers

#### Scenario: A tuple returns as a tuple, not a list

- **WHEN** a compiled function declaring a `tuple[int, str]` return is called
- **THEN** the returned value is a Python `tuple`

### Requirement: Wrong argument types raise TypeError

A compiled function SHALL raise `TypeError` when given an argument that does not match its
declared parameter type, rather than coercing it or failing later, because the compiled
function's contract is exactly the annotations the user wrote. This SHALL include a collection
whose kind matches but whose elements do not.

#### Scenario: String passed where an integer is declared

- **WHEN** a compiled function declaring an `int` parameter is called with `"x"`
- **THEN** it raises `TypeError`

#### Scenario: Wrong argument count

- **WHEN** a compiled function taking two parameters is called with one argument
- **THEN** it raises `TypeError`

#### Scenario: Wrong collection kind

- **WHEN** a compiled function declaring a `list[int]` parameter is called with a `set`
- **THEN** it raises `TypeError`

#### Scenario: Wrong element type

- **WHEN** a compiled function declaring a `list[int]` parameter is called with `["a"]`
- **THEN** it raises `TypeError`

#### Scenario: Wrong tuple length

- **WHEN** a compiled function declaring a `tuple[int, str]` parameter is called with a
  three-element tuple
- **THEN** it raises `TypeError`

### Requirement: Arithmetic failures raise the Python exception the interpreter would

The recoverable errors the backend emits SHALL surface as the exception types Python raises
for the same conditions, so that existing error handling around a function keeps working when
that function is compiled.

#### Scenario: Division by zero

- **WHEN** a compiled function evaluates a division or remainder by zero
- **THEN** it raises `ZeroDivisionError`

#### Scenario: Integer overflow

- **WHEN** a compiled function computes a value outside the range of a 64-bit signed integer
- **THEN** it raises `OverflowError`

#### Scenario: Index out of range

- **WHEN** a compiled function reads past the end of a sequence, in either direction
- **THEN** it raises `IndexError`

#### Scenario: Missing mapping key

- **WHEN** a compiled function reads a key that is not present in a mapping
- **THEN** it raises `KeyError`

#### Scenario: The process survives

- **WHEN** a compiled function raises any of these and the caller catches it
- **THEN** execution continues normally and later calls still work

#### Scenario: Failure inside a nested call

- **WHEN** a compiled function calls another compiled function that divides by zero
- **THEN** `ZeroDivisionError` propagates to the original Python caller

## ADDED Requirements

### Requirement: Collections cross the boundary by value

A collection passed to a compiled function SHALL be converted into an independent value. A
compiled function therefore SHALL NOT be able to affect a collection its caller still holds.

This differs from calling an interpreted Python function, which receives a reference. Nothing in
the supported subset can mutate a collection, so the difference is currently unobservable — it is
specified now so that it is a decision on record rather than a surprise discovered later, and so
that adding mutation has to confront it deliberately.

#### Scenario: The caller's list is unaffected

- **WHEN** a caller passes a list to a compiled function and inspects it afterwards
- **THEN** the list is unchanged

#### Scenario: A returned collection is independent

- **WHEN** a compiled function returns a collection and the caller modifies the result
- **THEN** nothing inside the compiled module is affected

#### Scenario: Large collections still convert correctly

- **WHEN** a compiled function is called with a sequence of many thousands of elements
- **THEN** it returns the correct result, the conversion cost being proportional to the size

### Requirement: A returned mapping does not preserve insertion order

Python dictionaries iterate in insertion order. The mapping a compiled function returns SHALL NOT
be relied upon to do so: its order is unspecified and may differ between runs of the same program.

This is a **known, accepted divergence** rather than an oversight. It follows from the map type the
Rust backend uses, and it is documented here so that a caller who iterates a returned mapping,
compares its key order, or snapshots it in a test knows the result is not stable. Callers that need
a defined order must sort explicitly.

#### Scenario: Contents are correct regardless of order

- **WHEN** a compiled function returns a mapping
- **THEN** it contains exactly the expected keys and values

#### Scenario: Key order is not guaranteed

- **WHEN** a compiled function returns a mapping built from keys inserted in a known order
- **THEN** the returned dictionary's iteration order is not guaranteed to match that order

#### Scenario: Lookup is unaffected

- **WHEN** a caller reads a key from a returned mapping
- **THEN** the value is correct, since only ordering is affected

#### Scenario: Sequence and tuple order IS preserved

- **WHEN** a compiled function returns a sequence or a tuple
- **THEN** the order of the elements matches the order they were produced in, because only
  mappings and sets are unordered
