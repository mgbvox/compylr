## ADDED Requirements

### Requirement: A read-only text parameter crosses without being copied

A `str` parameter SHALL cross the boundary without allocating and copying a fresh owned string,
where the parameter is only read. The subset already guarantees that: mutating a parameter is a
rejected program, so every accepted function's parameters are read-only, and the compiler already
knows it.

This is the difference between the cheapest and most expensive scalar element the boundary handles.
Measured per element: an integer element costs roughly 4 ns to cross, and a text element roughly
42 ns — ten times as much, and enough that every workload in the demo taking a list of text loses
to the interpreter regardless of what its body does.

#### Scenario: A text argument is not copied on the way in

- **WHEN** a compiled function taking text is called
- **THEN** the boundary does not allocate a fresh owned copy of that text to pass it

#### Scenario: The value is still valid for the whole call

- **WHEN** a compiled function reads a text parameter anywhere in its body
- **THEN** the value is valid for the entire call, including inside nested calls it makes

#### Scenario: Text semantics are unchanged

- **WHEN** a text parameter is measured, compared, or tested for membership
- **THEN** every answer is what it is today, including for non-ASCII input

### Requirement: The cost of crossing the boundary is stated

The per-element cost of converting a collection across the boundary SHALL be documented where users
meet it, because it is a property of compiling rather than of any program they wrote, and nothing
in their source suggests it.

A collection parameter is converted element by element on **every call**, so a compiled function
can be slower than the interpreted one purely by being called — most sharply when the body does
less work than the conversion. A binary search over 2000 elements converts all of them to perform
about eleven comparisons, and runs roughly 16x slower compiled than interpreted as a result.

#### Scenario: The cost is documented

- **WHEN** a user reads the demo's documentation
- **THEN** it states that a collection parameter costs time proportional to its length on every
  call, even when the function's body does not

#### Scenario: The documentation names when compiling loses

- **WHEN** the documentation describes what compiling is worth
- **THEN** it says that a function doing less work than its arguments cost to convert may be slower
  compiled, rather than implying compiled is always at least as fast
