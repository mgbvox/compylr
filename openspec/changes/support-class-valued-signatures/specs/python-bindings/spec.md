## ADDED Requirements

### Requirement: Direct class values cross free-function boundaries

A top-level compiled free function SHALL accept an instance of a compiled class as a direct
parameter and SHALL return an instance of a compiled class as a direct result. The boundary SHALL
use the same Python-visible compiled type that construction and methods expose, rather than exposing
or asking Python to convert the target backend's inner representation.

Passing an existing instance SHALL borrow the state held by that Python object rather than copy it.
A free function that mutates the parameter directly or through a mutating method SHALL therefore
change the same instance the caller passed, and a read-only free function SHALL observe its current
state. A returned inner instance SHALL be placed into the stable Python-visible wrapper for its
declared class before it is returned.

This initial conversion SHALL apply only when the class value is the direct parameter or result.
An instance nested in a collection boundary type SHALL be rejected with a source-located diagnostic
before target source is emitted, rather than producing bindings that fail to compile.

#### Scenario: Existing instance is read without copying

- **WHEN** Python passes a compiled `Tally` instance to a free function declared `read(t: Tally)`
- **THEN** the function observes the current state of that exact Python-held instance

#### Scenario: Existing instance is mutated without copying

- **WHEN** Python passes a compiled `Tally` instance to a free function that mutates `t`
- **THEN** a later method call on the same Python object observes the mutation

#### Scenario: Class-valued return uses the exposed type

- **WHEN** Python calls a compiled free function declared `build(start: int) -> Tally`
- **THEN** the result is an instance of the same compiled `Tally` type exposed by the module and
  its methods observe the state produced inside `build`

#### Scenario: Returned instances remain independent

- **WHEN** a class-valued free function is called twice and one returned instance is mutated
- **THEN** the other returned instance is unaffected

#### Scenario: Nested class conversion is rejected before emission

- **WHEN** a Python-boundary signature contains `list[Tally]`, `dict[str, Tally]`, or another
  container with an instance type at any depth
- **THEN** compilation fails with a diagnostic at that annotation before any Rust source is emitted

#### Scenario: Generated bindings compile for both directions

- **WHEN** one unit contains a free function taking a direct `Tally` and another returning one
- **THEN** the generated Python extension builds and both functions are callable
