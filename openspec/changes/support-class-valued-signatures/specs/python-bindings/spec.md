## ADDED Requirements

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

- **WHEN** Python passes a compiled `Tally` instance to a free function declared `read(t: Tally)`
- **THEN** the function observes the current state of that exact Python-held instance

#### Scenario: Existing instance is mutated without copying

- **WHEN** Python passes a compiled `Tally` instance to a free function that mutates `t`
- **THEN** a later method call on the same Python object observes the mutation

#### Scenario: Existing instance is forwarded without copying

- **WHEN** one compiled free function passes its direct instance parameter to another compatible
  borrowed instance parameter
- **THEN** both functions operate on the same Python-held state without cloning the inner instance

#### Scenario: Class-valued return uses the exposed type

- **WHEN** Python calls a compiled free function declared `build(start: int) -> Tally`
- **THEN** the result is an instance of the same compiled `Tally` type exposed by the module and
  its methods observe the state produced inside `build`

#### Scenario: A borrowed argument cannot become an owned return

- **WHEN** source declares `identity(t: Tally) -> Tally` and returns `t`
- **THEN** compilation fails with a source-located diagnostic before binding emission instead of
  cloning `t` into a second Python object

#### Scenario: A borrowed argument cannot be stored

- **WHEN** source stores a direct instance parameter in another owned value
- **THEN** compilation fails with a source-located diagnostic before binding emission

#### Scenario: Returned instances remain independent

- **WHEN** a class-valued free function is called twice and one returned instance is mutated
- **THEN** the other returned instance is unaffected

#### Scenario: Nested class conversion is rejected before emission

- **WHEN** a Python-boundary signature contains `list[Tally]`, `dict[str, Tally]`, or another
  container with an instance type at any depth
- **THEN** compilation fails with a diagnostic at that annotation before any Rust source is emitted

#### Scenario: Generated bindings compile for both directions

- **WHEN** one unit contains a free function taking a direct `Tally` and another returning a newly
  constructed one
- **THEN** the generated Python extension builds and both functions are callable
