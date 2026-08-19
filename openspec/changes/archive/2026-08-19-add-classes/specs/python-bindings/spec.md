## ADDED Requirements

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
