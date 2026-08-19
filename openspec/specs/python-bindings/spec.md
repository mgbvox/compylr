## Purpose

Defines the Python-facing surface generated onto compiled functions: how a unit becomes an
importable extension module, how values cross the boundary in each direction, and how a
compiled function's failures appear to Python code as ordinary exceptions.

## Requirements

### Requirement: The unit becomes a single importable module

A compiled unit SHALL be exposed as ONE Python extension module containing every function in
the unit. The module's name SHALL NOT be part of the user-facing API: callers reach compiled
functions through the objects they marked, never by importing the module themselves. Keeping
the name an implementation detail is what allows it to encode build identity, which in turn is
what allows a rebuilt unit to be loaded by a process that has already loaded an earlier one —
an extension module cannot be reliably re-imported under a name already in use.

#### Scenario: Every function is exposed

- **WHEN** a unit holding three compiled functions is built and imported
- **THEN** all three are accessible as attributes of the module

#### Scenario: Callers never name the module

- **WHEN** a user calls a marked function
- **THEN** no import of the generated module appears in their code

#### Scenario: A rebuilt unit loads in a process that already loaded its predecessor

- **WHEN** a function is marked after a build has occurred, and calling it forces a rebuild
- **THEN** the rebuilt unit is loaded and used in that same process

#### Scenario: Nothing beyond the unit is exposed

- **WHEN** a compiled module is imported
- **THEN** only the unit's functions and standard module attributes are present, so helper
  code emitted by the backend is not reachable as public API

### Requirement: Parameter names are preserved

Generated functions SHALL accept arguments by keyword under the names written in the Python
source, as well as positionally, because a caller replacing an interpreted function with a
compiled one must not have to change how it calls.

#### Scenario: Keyword call

- **WHEN** a compiled `add(a, b)` is called as `add(b=2, a=1)`
- **THEN** it returns the same result as `add(1, 2)`

#### Scenario: Positional call

- **WHEN** a compiled `add(a, b)` is called as `add(1, 2)`
- **THEN** arguments bind in source order

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

### Requirement: Collections cross the boundary by value

A collection passed to a compiled function SHALL be converted into an independent value. A
compiled function therefore SHALL NOT be able to affect a collection its caller still holds.

This differs from calling an interpreted Python function, which receives a reference. Mutation now
exists in the subset, so the difference would be observable — a compiled function mutating a
parameter would leave its caller's collection unchanged where an interpreted one would not, which
is a wrong answer with no error.

Mutation is therefore **confined to locals**: lowering rejects mutating a parameter. The divergence
is unreachable rather than documented, and this requirement records why the restriction exists, so
that relaxing it later has to supply reference semantics first.

#### Scenario: The caller's list is unaffected

- **WHEN** a caller passes a list to a compiled function and inspects it afterwards
- **THEN** the list is unchanged

#### Scenario: A compiled function cannot mutate a parameter at all

- **WHEN** a function attempting to mutate a collection parameter is marked
- **THEN** it is rejected, so no program exists in which the divergence could be observed

#### Scenario: A returned collection is independent

- **WHEN** a compiled function returns a collection and the caller modifies the result
- **THEN** nothing inside the compiled module is affected

#### Scenario: A locally built collection is returned by value

- **WHEN** a compiled function builds a collection and returns it
- **THEN** the caller receives an independent Python object holding the built contents

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

### Requirement: A class is exposed to Python as a type

A compiled class SHALL be exposed as a Python type on the compiled module, constructible from
Python, with its methods callable as ordinary methods.

#### Scenario: The type is exposed

- **WHEN** a unit containing a class is built and imported
- **THEN** the class is accessible as an attribute of the module

#### Scenario: It is constructible

- **WHEN** the exposed type is called with the arguments `__init__` declares
- **THEN** an instance is returned

#### Scenario: Methods are callable

- **WHEN** a method is called on an instance
- **THEN** it runs the compiled implementation and returns its result

#### Scenario: Arguments convert on the same terms as functions

- **WHEN** a method is called with arguments of the declared types
- **THEN** each converts as it would for a free function, including collections

#### Scenario: Wrong argument types raise TypeError

- **WHEN** a method or constructor is called with an argument of the wrong type
- **THEN** it raises `TypeError`

#### Scenario: Failures raise what Python would

- **WHEN** a method divides by zero, reads a missing key, or overflows
- **THEN** it raises the same exception the equivalent free function would

### Requirement: Instance state persists across calls

An instance held by Python SHALL retain its attributes between method calls, so that a method
mutating an attribute is observed by a later call on the same object.

This is the property the whole change exists for. A compiled object whose state reset between calls
would be indistinguishable from a free function, and a cache built on it would never hit.

#### Scenario: A mutation is observed by a later call

- **WHEN** a method increments a counter attribute and is called three times
- **THEN** a method reading the counter reports three

#### Scenario: Two instances are independent

- **WHEN** two instances are constructed and one is mutated
- **THEN** the other is unaffected

#### Scenario: A cache hits

- **WHEN** a method that memoizes into a mapping attribute is called twice with the same argument
- **THEN** the second call observes the cached entry

#### Scenario: An instance survives being stored by the caller

- **WHEN** a caller keeps an instance in a Python data structure and calls a method later
- **THEN** the accumulated state is intact
