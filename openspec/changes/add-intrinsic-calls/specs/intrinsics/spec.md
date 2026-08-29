## Purpose

Defines how a compiled program names operations it did not itself compile: what an intrinsic
operation is, how a module namespace is introduced and resolved, and what the compiler reports for
a module or operation it does not support.

## ADDED Requirements

### Requirement: An intrinsic is resolved against a registry, not against the unit

An intrinsic operation SHALL be identified by a module name and an operation name, and SHALL be
resolved against a registry of supported operations. Resolution SHALL NOT consult the functions or
classes of the unit being compiled, and the unit SHALL NOT be able to change what an intrinsic
means.

#### Scenario: A user function of the same name does not shadow an intrinsic

- **WHEN** a unit defines a function named `sqrt` and another function calls `math.sqrt(x)`
- **THEN** the intrinsic resolves to the registry entry for `math.sqrt`, and the user's `sqrt` is
  unaffected and still callable as `sqrt(x)`

#### Scenario: An intrinsic does not require a matching unit member

- **WHEN** a unit contains exactly one function whose body calls `math.floor(x)`
- **THEN** validation succeeds, and no diagnostic reports a call to a function that exists nowhere

#### Scenario: Resolution does not depend on compilation order

- **WHEN** the same function using `math.sqrt` is compiled alone and again alongside other
  decorated functions
- **THEN** both compilations produce the same intrinsic operation with the same meaning

### Requirement: The registry carries each operation's signature

The registry SHALL record, for every supported operation, the number and types of its parameters
and the type of its result. Lowering SHALL check an intrinsic call against that signature and SHALL
report a mismatch as a located diagnostic naming the operation.

#### Scenario: Wrong arity is a located diagnostic

- **WHEN** lowering `math.sqrt(1.0, 2.0)`
- **THEN** lowering fails with a located diagnostic reporting that `math.sqrt` takes one argument

#### Scenario: Wrong argument type is a located diagnostic

- **WHEN** lowering `math.sqrt("four")`
- **THEN** lowering fails with a located diagnostic naming the expected and supplied types

#### Scenario: The result type is known without consulting a backend

- **WHEN** lowering `x = math.floor(2.7)` with no annotation on `x`
- **THEN** the binding's type is determined from the registry signature, in the same way a call to
  a function in the same source determines one

### Requirement: An unsupported module or operation names what is supported

The compiler SHALL reject an import of an unsupported module, and a use of an unsupported operation
of a supported module, with a located diagnostic. The diagnostic for an unsupported module SHALL
list the modules that are supported.

#### Scenario: An unsupported module lists the supported ones

- **WHEN** lowering a source containing `import json`
- **THEN** lowering fails with a located diagnostic reporting that `json` is not supported yet, and
  listing the supported modules

#### Scenario: An unsupported operation of a supported module names the operation

- **WHEN** lowering `math.erf(x)`
- **THEN** lowering fails with a located diagnostic naming `math.erf` as unsupported, rather than
  reporting that the module is unknown

#### Scenario: The refusal is a diagnostic, not a backend failure

- **WHEN** an unsupported module or operation appears anywhere in a source
- **THEN** the failure is a located diagnostic produced before any target source exists, and never
  a complaint about generated code

### Requirement: A fallible intrinsic declares whether the program defines its failure

An operation whose result is undefined for some inputs SHALL carry a checking mode stating whether
the program defines that failure, in the same way a fallible arithmetic operation does. A backend
SHALL emit from the mode and SHALL NOT infer it from the operation's name.

#### Scenario: A reported domain failure is recoverable

- **WHEN** an intrinsic carrying a reported checking mode is evaluated on an input outside its
  domain
- **THEN** the failure surfaces as a recoverable error carrying a message naming the operation, and
  not as a panic or an abort

#### Scenario: An undefined domain failure emits the target's own operation

- **WHEN** an intrinsic carrying an unchecked mode is emitted
- **THEN** the target's own operation is emitted directly, with no domain test around it

#### Scenario: An infallible operation carries no mode

- **WHEN** an operation is total over its parameter types, such as `math.fabs`
- **THEN** it carries no checking mode, and no backend emits a domain test for it

### Requirement: A backend supplies spellings and may decline a module

A backend SHALL map each supported operation onto its own target-native operation. A backend that
has no mapping for a module SHALL fail with a diagnostic reporting the mapping as planned, distinct
from the backend itself being unknown or unimplemented.

#### Scenario: A backend without a mapping reports it as planned

- **WHEN** a program using `math` is compiled with a backend that has no `math` mapping
- **THEN** compilation fails reporting that the mapping for that module and backend is planned,
  and the message does not claim the backend is unknown

#### Scenario: Spellings do not reach the IR

- **WHEN** an intrinsic is represented in the IR
- **THEN** it names a module and an operation by meaning, and carries no target-language spelling
