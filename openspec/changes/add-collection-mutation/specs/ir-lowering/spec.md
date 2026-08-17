## ADDED Requirements

### Requirement: Element assignment

Lowering SHALL accept assigning to an element of a sequence or a mapping. The index SHALL be an
integer for a sequence and the key type for a mapping, and the value SHALL match the element or
value type, with promotion applying as elsewhere. Assigning to an element of a set, a tuple, or a
scalar SHALL be rejected.

#### Scenario: Sequence element assignment

- **WHEN** lowering `xs[0] = 1` where `xs` is a local sequence of integers
- **THEN** lowering succeeds

#### Scenario: Mapping element assignment

- **WHEN** lowering `d["a"] = 1` where `d` is a local mapping from strings to integers
- **THEN** lowering succeeds

#### Scenario: A wrong value type is rejected

- **WHEN** lowering `xs[0] = "a"` where `xs` holds integers
- **THEN** lowering fails reporting both types

#### Scenario: A wrong index type is rejected

- **WHEN** lowering `xs["a"] = 1` where `xs` is a sequence
- **THEN** lowering fails reporting the index type

#### Scenario: Promotion applies

- **WHEN** lowering `xs[0] = 1` where `xs` holds floats
- **THEN** lowering succeeds and the value carries an explicit conversion

#### Scenario: A tuple is immutable

- **WHEN** lowering an assignment to a tuple element
- **THEN** lowering fails, matching Python, where tuples cannot be assigned into

#### Scenario: A set has no elements to assign

- **WHEN** lowering an assignment to a set element
- **THEN** lowering fails

### Requirement: Mutation is confined to locals

Lowering SHALL reject mutating a collection that arrived as a **parameter**, whether by element
assignment or by appending. The diagnostic SHALL explain that a collection parameter is a copy, so
the mutation could not be observed by the caller.

Collections cross the boundary by value. A compiled function mutating a parameter would leave its
caller's collection unchanged, where an interpreted function would have modified it — a wrong
answer with no error. Confining mutation to locals makes that unreachable rather than documented.

A collection built locally and returned is unaffected, which is the shape this change exists to
enable.

#### Scenario: A local collection may be mutated

- **WHEN** lowering a body that binds an empty sequence, appends to it, and returns it
- **THEN** lowering succeeds

#### Scenario: A parameter may not be mutated

- **WHEN** lowering a body that appends to one of its sequence parameters
- **THEN** lowering fails, explaining that the parameter is a copy and the caller would not see it

#### Scenario: Assigning into a parameter is rejected

- **WHEN** lowering a body that assigns to an element of a mapping parameter
- **THEN** lowering fails

#### Scenario: Reading a parameter is unaffected

- **WHEN** lowering a body that reads elements of a parameter without mutating it
- **THEN** lowering succeeds

#### Scenario: A local copied from a parameter may be mutated

- **WHEN** a body binds a local to a parameter and mutates the local
- **THEN** lowering succeeds, because the local is the function's own value

### Requirement: Append

Lowering SHALL accept `append` on a local sequence, with one argument whose type matches the
element type. Any other method SHALL remain rejected, and the diagnostic SHALL name the method.

#### Scenario: Appending lowers

- **WHEN** lowering `xs.append(1)` where `xs` is a local sequence of integers
- **THEN** lowering succeeds

#### Scenario: A wrong element type is rejected

- **WHEN** lowering `xs.append("a")` where `xs` holds integers
- **THEN** lowering fails reporting both types

#### Scenario: Wrong arity is rejected

- **WHEN** lowering `xs.append()` or `xs.append(1, 2)`
- **THEN** lowering fails reporting the argument count

#### Scenario: Appending to a non-sequence is rejected

- **WHEN** lowering `d.append(1)` where `d` is a mapping
- **THEN** lowering fails reporting the type

#### Scenario: Another method is rejected by name

- **WHEN** lowering `xs.pop()`
- **THEN** lowering fails with a diagnostic naming `pop` as unsupported

### Requirement: Membership

Lowering SHALL accept `in` and `not in` over a sequence, mapping, set, or string, yielding a
boolean. Membership in a mapping SHALL test its **keys**, matching Python. The value's type SHALL
match what the container holds — its element type, its key type, or a string for a string.

#### Scenario: Membership yields a boolean

- **WHEN** lowering `x in xs` where `xs` is a sequence of integers and `x` an integer
- **THEN** the expression's type is boolean

#### Scenario: Mapping membership tests keys

- **WHEN** lowering `k in d` where `d` maps strings to integers
- **THEN** `k` must be a string, matching Python

#### Scenario: Negated membership

- **WHEN** lowering `x not in xs`
- **THEN** the expression's type is boolean

#### Scenario: A mismatched value type is rejected

- **WHEN** lowering `"a" in xs` where `xs` holds integers
- **THEN** lowering fails reporting both types

#### Scenario: Membership in a scalar is rejected

- **WHEN** lowering `x in n` where `n` is an integer
- **THEN** lowering fails reporting the type

#### Scenario: Membership in a string tests substrings

- **WHEN** lowering `a in s` where both are strings
- **THEN** the expression's type is boolean, matching Python's substring test
