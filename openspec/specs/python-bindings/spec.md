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
an extension module cannot be reliably re-imported under a name already in use. The build identity
the name encodes SHALL distinguish builds that differ only in the target language or in the pass
configuration that produced them, so that switching either does not collide with an already-loaded
module.

#### Scenario: Every function is exposed

- **GIVEN** a unit holding three compiled functions
- **WHEN** it is built and imported
- **THEN** all three are accessible as attributes of the module

#### Scenario: Callers never name the module

- **GIVEN** a project with a marked function
- **WHEN** a user calls it
- **THEN** no import of the generated module appears in their code

#### Scenario: A rebuilt unit loads in a process that already loaded its predecessor

- **GIVEN** a process that has already loaded a build
- **WHEN** a function is marked and calling it forces a rebuild
- **THEN** the rebuilt unit is loaded and used in that same process

#### Scenario: Nothing beyond the unit is exposed

- **GIVEN** a compiled unit
- **WHEN** its module is imported
- **THEN** only the unit's functions and standard module attributes are present, so helper
  code emitted by the backend is not reachable as public API

#### Scenario: Builds differing only in configuration do not collide

- **GIVEN** one unit built twice under different pass configurations
- **WHEN** both are loaded in one process
- **THEN** each is loaded under its own module name

### Requirement: Parameter names are preserved

Generated functions SHALL accept arguments by keyword under the names written in the Python
source, as well as positionally, because a caller replacing an interpreted function with a
compiled one must not have to change how it calls.

#### Scenario: Keyword call

- **GIVEN** a compiled function `add(a, b)`
- **WHEN** it is called as `add(b=2, a=1)`
- **THEN** it returns the same result as `add(1, 2)`

#### Scenario: Positional call

- **GIVEN** a compiled function `add(a, b)`
- **WHEN** it is called as `add(1, 2)`
- **THEN** arguments bind in source order

### Requirement: Value conversion across the boundary

Each IR type SHALL convert between its Python and Rust representations in both directions:
integers, floats, booleans, and strings as arguments and as return values, the unit type as `None`
on return, and the collection types as their Python counterparts — sequences as `list`, mappings as
`dict`, sets as `set`, and tuples as `tuple` — nested to any depth.

#### Scenario: Each type round-trips

- **GIVEN** a compiled function taking a scalar parameter and returning it unchanged
- **WHEN** it is called with a Python value of that type
- **THEN** calling it with a Python value of that type returns an equal Python value of the
  same type

#### Scenario: Each collection type round-trips

- **GIVEN** a compiled function taking a `list`, `dict`, `set`, or `tuple` and returning it
  unchanged
- **WHEN** it is called
- **THEN** calling it returns an equal Python value of the same kind

#### Scenario: Nested collections round-trip

- **GIVEN** a compiled function taking a `dict[str, list[int]]` and returning it unchanged
- **WHEN** it is called
- **THEN** the nesting and the values are preserved

#### Scenario: Unit return

- **GIVEN** a compiled function annotated `-> None`
- **WHEN** it is called
- **THEN** it returns `None`

#### Scenario: Booleans are not integers at the boundary

- **GIVEN** a compiled function declaring a `bool` return
- **WHEN** it is called
- **THEN** the returned value is a Python `bool`, consistent with the IR's rule that booleans
  are not numbers

#### Scenario: A tuple returns as a tuple, not a list

- **GIVEN** a compiled function declaring a `tuple[int, str]` return
- **WHEN** it is called
- **THEN** the returned value is a Python `tuple`

### Requirement: Wrong argument types raise TypeError

A compiled function SHALL raise `TypeError` when given an argument that does not match its
declared parameter type, rather than coercing it or failing later, because the compiled
function's contract is exactly the annotations the user wrote. This SHALL include a collection
whose kind matches but whose elements do not.

#### Scenario: String passed where an integer is declared

- **GIVEN** a compiled function declaring an `int` parameter
- **WHEN** it is called with `"x"`
- **THEN** it raises `TypeError`

#### Scenario: Wrong argument count

- **GIVEN** a compiled function taking two parameters
- **WHEN** it is called with one argument
- **THEN** it raises `TypeError`

#### Scenario: Wrong collection kind

- **GIVEN** a compiled function declaring a `list[int]` parameter
- **WHEN** it is called with a `set`
- **THEN** it raises `TypeError`

#### Scenario: Wrong element type

- **GIVEN** a compiled function declaring a `list[int]` parameter
- **WHEN** it is called with `["a"]`
- **THEN** it raises `TypeError`

#### Scenario: Wrong tuple length

- **GIVEN** a compiled function declaring a `tuple[int, str]` parameter
- **WHEN** it is called with a tuple of the wrong length
- **THEN** it raises `TypeError`

### Requirement: Arithmetic failures raise the Python exception the interpreter would

The recoverable errors the backend emits SHALL surface as the exception types Python raises
for the same conditions, so that existing error handling around a function keeps working when
that function is compiled.

#### Scenario: Division by zero

- **GIVEN** a compiled function that divides or takes a remainder
- **WHEN** its divisor is zero
- **THEN** it raises `ZeroDivisionError`

#### Scenario: Integer overflow

- **GIVEN** a compiled function computing a value outside the range of a 64-bit signed integer
- **WHEN** it is called
- **THEN** it raises `OverflowError`

#### Scenario: Index out of range

- **GIVEN** a compiled function reading past the end of a sequence, in either direction
- **WHEN** it is called
- **THEN** it raises `IndexError`

#### Scenario: Missing mapping key

- **GIVEN** a compiled function reading a key that is not present in a mapping
- **WHEN** it is called
- **THEN** it raises `KeyError`

#### Scenario: The process survives

- **GIVEN** a caller catching one of these exceptions
- **WHEN** execution continues
- **THEN** execution continues normally and later calls still work

#### Scenario: Failure inside a nested call

- **GIVEN** a compiled function calling another that divides by zero
- **WHEN** the outer function is called
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

- **GIVEN** a caller holding a list
- **WHEN** it is passed to a compiled function and inspected afterwards
- **THEN** the list is unchanged

#### Scenario: A compiled function cannot mutate a parameter at all

- **GIVEN** a function attempting to mutate a collection parameter
- **WHEN** it is marked
- **THEN** it is rejected, so no program exists in which the divergence could be observed

#### Scenario: A returned collection is independent

- **GIVEN** a compiled function returning a collection
- **WHEN** the caller modifies the result
- **THEN** nothing inside the compiled module is affected

#### Scenario: A locally built collection is returned by value

- **GIVEN** a compiled function that builds a collection and returns it
- **WHEN** it is called
- **THEN** the caller receives an independent Python object holding the built contents

#### Scenario: Large collections still convert correctly

- **GIVEN** a sequence of many thousands of elements
- **WHEN** a compiled function is called with it
- **THEN** it returns the correct result, the conversion cost being proportional to the size

### Requirement: A returned mapping does not preserve insertion order

Python dictionaries iterate in insertion order. The mapping a compiled function returns SHALL NOT
be relied upon to do so: its order is unspecified and may differ between runs of the same program.

This is a **known, accepted divergence** rather than an oversight. It follows from the map type the
Rust backend uses, and it is documented here so that a caller who iterates a returned mapping,
compares its key order, or snapshots it in a test knows the result is not stable. Callers that need
a defined order must sort explicitly.

#### Scenario: Contents are correct regardless of order

- **GIVEN** a compiled function returning a mapping
- **WHEN** it is called
- **THEN** it contains exactly the expected keys and values

#### Scenario: Key order is not guaranteed

- **GIVEN** a compiled function returning a mapping built from keys inserted in a known order
- **WHEN** it is called
- **THEN** the returned dictionary's iteration order is not guaranteed to match that order

#### Scenario: Lookup is unaffected

- **GIVEN** a mapping returned from a compiled function
- **WHEN** a caller reads a key from it
- **THEN** the value is correct, since only ordering is affected

#### Scenario: Sequence and tuple order IS preserved

- **GIVEN** a compiled function returning a sequence or a tuple
- **WHEN** it is called
- **THEN** the order of the elements matches the order they were produced in, because only
  mappings and sets are unordered

### Requirement: A class is exposed to Python as a type

A compiled class SHALL be exposed as a Python type on the compiled module, constructible from
Python, with its methods callable as ordinary methods.

#### Scenario: The type is exposed

- **GIVEN** a unit containing a class
- **WHEN** it is built and imported
- **THEN** the class is accessible as an attribute of the module

#### Scenario: It is constructible

- **GIVEN** an exposed compiled class
- **WHEN** it is called with the arguments its constructor declares
- **THEN** an instance is returned

#### Scenario: Methods are callable

- **GIVEN** an instance of a compiled class
- **WHEN** a method is called on it
- **THEN** it runs the compiled implementation and returns its result

#### Scenario: Arguments convert on the same terms as functions

- **GIVEN** an instance of a compiled class
- **WHEN** a method is called with arguments of the declared types
- **THEN** each converts as it would for a free function, including collections

#### Scenario: Wrong argument types raise TypeError

- **GIVEN** an instance of a compiled class
- **WHEN** a method or constructor is called with an argument of the wrong type
- **THEN** it raises `TypeError`

#### Scenario: Failures raise what Python would

- **GIVEN** an instance of a compiled class
- **WHEN** a method divides by zero, reads a missing key, or overflows
- **THEN** it raises the same exception the equivalent free function would

### Requirement: Instance state persists across calls

An instance held by Python SHALL retain its attributes between method calls, so that a method
mutating an attribute is observed by a later call on the same object.

This is the property the whole change exists for. A compiled object whose state reset between calls
would be indistinguishable from a free function, and a cache built on it would never hit.

#### Scenario: A mutation is observed by a later call

- **GIVEN** an instance whose method increments a counter attribute
- **WHEN** that method is called three times
- **THEN** a method reading the counter reports three

#### Scenario: Two instances are independent

- **GIVEN** two instances of a compiled class
- **WHEN** one is mutated
- **THEN** the other is unaffected

#### Scenario: A cache hits

- **GIVEN** an instance whose method memoizes into a mapping attribute
- **WHEN** it is called twice with the same argument
- **THEN** the second call observes the cached entry

#### Scenario: An instance survives being stored by the caller

- **GIVEN** a caller keeping an instance in a Python data structure
- **WHEN** a method is called on it later
- **THEN** the accumulated state is intact

### Requirement: Python bindings are the bridge for one source/target pair

Generating the code that makes a compiled unit callable from Python SHALL be the responsibility of a
component registered for the pair `(python, rust)`, not of the Rust backend and not of the Python
frontend. The Rust backend SHALL remain able to generate target source with no Python bridge
present, and adding a second target SHALL NOT change this component.

#### Scenario: The bridge is selected by the pair

- **GIVEN** a unit lowered by the Python frontend
- **WHEN** a callable artifact is requested for the Rust target
- **THEN** the `(python, rust)` bridge is selected and generates the binding layer

#### Scenario: The backend generates without the bridge

- **GIVEN** a unit lowered by the Python frontend
- **WHEN** target source is requested without a callable artifact
- **THEN** the Rust backend emits it, and no Python-specific code is generated

#### Scenario: A second target does not touch this bridge

- **GIVEN** a workspace to which a backend for another target has been added
- **WHEN** the `(python, rust)` bridge is inspected
- **THEN** the `(python, rust)` bridge is unchanged

### Requirement: An unbridged pair is reported as such

Requesting a callable artifact for a source and target that have no registered bridge SHALL fail
with an error naming both languages and stating that generation is available but calling back is
not. It SHALL NOT be reported as an unknown backend, an unknown frontend, or an internal error.

#### Scenario: Generation succeeds, bridging does not

- **GIVEN** a pair whose backend is implemented but which has no bridge
- **WHEN** a callable artifact is requested
- **THEN** the failure names both languages and distinguishes itself from an unknown-target failure

#### Scenario: A caller can branch on the case

- **GIVEN** a caller holding a bridging failure
- **WHEN** it needs to distinguish an unbridged pair from an unimplemented target
- **THEN** it can do so from the failure's kind without matching on rendered text

### Requirement: The binding layer is generated from the IR alone

The bridge SHALL derive every exposed name, signature, and conversion from the IR, without reading
the original Python source and without depending on the Python parser. Error mapping SHALL be
derived from the errors the IR's operations can produce, so that a target error has one Python
exception regardless of which frontend construct produced it.

#### Scenario: No source is consulted

- **GIVEN** one unit, in memory and read back from its serialized artifact
- **WHEN** a binding layer is generated from each
- **THEN** it is identical to the one generated from the same unit in memory

#### Scenario: The bridge does not depend on the parser

- **GIVEN** the workspace manifests
- **WHEN** the bridge component's dependencies are inspected
- **THEN** it does not depend on a Python parser

### Requirement: The cost of crossing the boundary is stated

The per-element cost of converting a collection across the boundary SHALL be documented where users
meet it, because it is a property of compiling rather than of any program they wrote, and nothing
in their source suggests it.

A collection parameter is converted element by element on **every call**, so a compiled function
can be slower than the interpreted one purely by being called — most sharply when the body does
less work than the conversion. A binary search over 2000 elements converts all of them to perform
about eleven comparisons, and runs roughly 16x slower compiled than interpreted as a result.

#### Scenario: The cost is documented

- **GIVEN** the demo's documentation
- **WHEN** a user reads it
- **THEN** it states that a collection parameter costs time proportional to its length on every
  call, even when the function's body does not

#### Scenario: The documentation names when compiling loses

- **GIVEN** the demo's documentation
- **WHEN** it describes what compiling is worth
- **THEN** it says that a function doing less work than its arguments cost to convert may be slower
  compiled, rather than implying compiled is always at least as fast

### Requirement: Direct class values cross free-function boundaries

A top-level compiled free function SHALL accept an instance of a compiled class as a direct
borrow-only parameter and SHALL return a newly owned instance of a compiled class as a direct
result. The boundary SHALL use the same Python-visible compiled type that construction and methods
expose, rather than exposing or asking Python to convert the target backend's inner representation.

Passing an existing instance SHALL borrow the state held by that Python object rather than copy it.
A free function that mutates the parameter directly or through a mutating method SHALL therefore
change the same instance the caller passed, and a read-only free function SHALL observe its current
state. The boundary SHALL permit the borrow to pass onward to another compatible borrowed
parameter. It SHALL NOT clone the inner value to satisfy an owned result or storage use. Such an
ownership escape SHALL have been rejected with a located diagnostic before bindings are emitted.

A newly owned returned inner instance—created in the function or returned from another
owned-producing call—SHALL be placed into the stable Python-visible wrapper for its declared class
before it is returned. Returning a borrowed parameter itself is outside this initial conversion.

This initial conversion SHALL apply only when the class value is the direct parameter or result.
An instance nested in a collection boundary type SHALL be rejected with a source-located diagnostic
before target source is emitted, rather than producing bindings that fail to compile.

#### Scenario: Existing instance is read without copying

- **GIVEN** a compiled `Tally` instance held by Python
- **WHEN** it is passed to a free function declared `read(t: Tally)`
- **THEN** the function observes the current state of that exact Python-held instance

#### Scenario: Existing instance is mutated without copying

- **GIVEN** a compiled `Tally` instance held by Python
- **WHEN** it is passed to a free function that mutates `t`
- **THEN** a later method call on the same Python object observes the mutation

#### Scenario: Existing instance is forwarded without copying

- **GIVEN** a compiled free function holding a direct instance parameter
- **WHEN** it passes that parameter to another compatible function
- **THEN** both functions operate on the same Python-held state without cloning the inner instance

#### Scenario: Class-valued return uses the exposed type

- **GIVEN** a compiled free function declared `build(start: int) -> Tally`
- **WHEN** Python calls it
- **THEN** the result is an instance of the same compiled `Tally` type exposed by the module and
  its methods observe the state produced inside `build`

#### Scenario: A borrowed argument cannot become an owned return

- **GIVEN** source declaring `identity(t: Tally) -> Tally` and returning `t`
- **WHEN** it is compiled
- **THEN** compilation fails with a source-located diagnostic before binding emission instead of
  cloning `t` into a second Python object

#### Scenario: A borrowed argument cannot be stored

- **GIVEN** source storing a direct instance parameter in another owned value
- **WHEN** it is compiled
- **THEN** compilation fails with a source-located diagnostic before binding emission

#### Scenario: Returned instances remain independent

- **GIVEN** a class-valued free function called twice
- **WHEN** one returned instance is mutated
- **THEN** the other returned instance is unaffected

#### Scenario: Nested class conversion is rejected before emission

- **GIVEN** a Python-boundary signature containing `list[Tally]`, `dict[str, Tally]`, or another
  nested class type
- **WHEN** it is compiled
- **THEN** compilation fails with a diagnostic at that annotation before any Rust source is emitted

#### Scenario: Generated bindings compile for both directions

- **GIVEN** one unit with a free function taking a direct `Tally` and another returning a newly
  built one
- **WHEN** the extension is built
- **THEN** the generated Python extension builds and both functions are callable
