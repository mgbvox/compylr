## Purpose

Defines the Python-facing surface generated onto compiled functions: how a unit becomes an
importable extension module, how values cross the boundary in each direction, and how a
compiled function's failures appear to Python code as ordinary exceptions.

## ADDED Requirements

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
integers, floats, booleans, and strings as arguments and as return values, and the unit type
as `None` on return.

#### Scenario: Each type round-trips

- **WHEN** a compiled function takes a parameter of a given type and returns it unchanged
- **THEN** calling it with a Python value of that type returns an equal Python value of the
  same type

#### Scenario: Unit return

- **WHEN** a compiled function annotated `-> None` is called
- **THEN** it returns `None`

#### Scenario: Booleans are not integers at the boundary

- **WHEN** a compiled function declaring a `bool` return is called
- **THEN** the returned value is a Python `bool`, consistent with the IR's rule that booleans
  are not numbers

### Requirement: Wrong argument types raise TypeError

A compiled function SHALL raise `TypeError` when given an argument that does not match its
declared parameter type, rather than coercing it or failing later, because the compiled
function's contract is exactly the annotations the user wrote.

#### Scenario: String passed where an integer is declared

- **WHEN** a compiled function declaring an `int` parameter is called with `"x"`
- **THEN** it raises `TypeError`

#### Scenario: Wrong argument count

- **WHEN** a compiled function taking two parameters is called with one argument
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

#### Scenario: The process survives

- **WHEN** a compiled function raises either of these and the caller catches it
- **THEN** execution continues normally and later calls still work

#### Scenario: Failure inside a nested call

- **WHEN** a compiled function calls another compiled function that divides by zero
- **THEN** `ZeroDivisionError` propagates to the original Python caller
