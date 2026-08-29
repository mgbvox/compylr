## Purpose

Defines how a compiled program names operations it did not itself compile: what an intrinsic
operation is, how a module namespace is introduced and resolved, and what the compiler reports for
a module or operation it does not support.

## ADDED Requirements

### Requirement: An intrinsic is resolved against a registry, not against the unit

An intrinsic operation SHALL be identified by a module name and an operation name, and SHALL be
resolved against a registry of supported operations. Resolution SHALL NOT consult the functions or
classes of the unit being compiled, and the unit SHALL NOT be able to change what an intrinsic
means. This is the rule that already makes [`Expr::Len`](../../../../../crates/compylr-ir/src/ir.rs#L575)
and [`Expr::Range`](../../../../../crates/compylr-ir/src/ir.rs#L596) distinct forms rather than
calls, applied to a named module.

#### Scenario: A user function of the same name does not shadow an intrinsic

- **GIVEN** a unit whose source is

  ```python
  import math


  def sqrt(x: float) -> float:
      return x


  def use(x: float) -> float:
      return math.sqrt(x) + sqrt(x)
  ```

- **WHEN** the unit is lowered by the `python` frontend
- **THEN** `math.sqrt` resolves to the registry entry for the `math` module
- **AND** the plain `sqrt` resolves to the unit's own function
- **AND** neither resolution changes the other

#### Scenario: An intrinsic does not require a matching unit member

- **GIVEN** a unit containing exactly one function, whose body calls `math.floor(x)`
- **WHEN** the unit is validated
- **THEN** validation succeeds
- **AND** no diagnostic reports a call to a function that exists nowhere

#### Scenario: Resolution does not depend on compilation order

- **GIVEN** a function whose body calls `math.sqrt`
- **AND** a project containing other decorated functions
- **WHEN** that function is compiled alone and again alongside the others
- **THEN** both compilations produce the same intrinsic operation with the same meaning

### Requirement: The registry carries each operation's signature

The registry SHALL record, for every supported operation, the number and types of its parameters
and the type of its result. Lowering SHALL check an intrinsic call against that signature and SHALL
report a mismatch as a located diagnostic naming the operation. A result type SHALL be determined
from the registry alone, without consulting a backend.

#### Scenario Outline: A signature mismatch is a located diagnostic

- **GIVEN** a unit whose body contains `<expression>`
- **WHEN** the unit is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic
- **AND** the diagnostic names `<names>`

**Examples:**

| expression             | names                                     |
| ---------------------- | ----------------------------------------- |
| `math.sqrt(1.0, 2.0)`  | `math.sqrt` and that it takes one argument |
| `math.sqrt()`          | `math.sqrt` and that it takes one argument |
| `math.sqrt("four")`    | the expected and the supplied type         |
| `math.atan2(1.0)`      | `math.atan2` and that it takes two         |

#### Scenario: The result type is known without consulting a backend

- **GIVEN** a unit whose body contains `x = math.floor(2.7)` with no annotation on `x`
- **WHEN** the unit is lowered by the `python` frontend
- **THEN** the binding's type is determined from the registry signature, in the same way a call to
  a function in the same source determines one
- **AND** `x` is bound at the integer type

#### Scenario: An integer argument to a float operation is promoted by the existing path

- **GIVEN** a unit whose body contains `math.sqrt(4)`
- **WHEN** the unit is lowered by the `python` frontend
- **THEN** lowering succeeds
- **AND** the argument is widened by the same numeric promotion every other operation uses

### Requirement: An unsupported module or operation names what is supported

The compiler SHALL reject an import of an unsupported module, and a use of an unsupported operation
of a supported module, with a located diagnostic. The diagnostic for an unsupported module SHALL
list the modules that are supported. The refusal SHALL be produced before any target source exists.

#### Scenario: An unsupported module lists the supported ones

- **GIVEN** a source whose first line is `import json`
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic reporting that `json` is not supported yet
- **AND** the diagnostic lists the supported modules

#### Scenario: An unsupported operation of a supported module names the operation

- **GIVEN** a source that imports `math` and whose body contains `math.erf(x)`
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming `math.erf` as unsupported
- **BUT** the diagnostic does not report that the module is unknown

#### Scenario: The refusal is a diagnostic, not a backend failure

- **GIVEN** a source using an unsupported module or operation anywhere in its body
- **WHEN** the source is compiled
- **THEN** the failure is a located diagnostic produced before any target source exists
- **AND** it is never a complaint about generated code

### Requirement: A fallible intrinsic declares whether the program defines its failure

An operation whose result is undefined for some inputs SHALL carry a
[`Checked`](../../../../../crates/compylr-ir/src/ir.rs#L268) mode stating whether the program
defines that failure, in the same way a fallible arithmetic operation does. A backend SHALL emit
from the mode and SHALL NOT infer it from the operation's name. An operation total over its
parameter types SHALL carry no mode.

#### Scenario Outline: A domain failure is emitted from the mode, not the operation name

- **GIVEN** a unit whose body calls `math.sqrt` on a negative value, resolved to <mode>
- **WHEN** the unit is compiled for the `rust` backend and run
- **THEN** the result is <result>

**Examples:**

| mode        | result                                                      |
| ----------- | ----------------------------------------------------------- |
| `Reported`  | a recoverable error whose message names `math.sqrt`         |
| `Unchecked` | the target's own operation, with no domain test around it   |

#### Scenario: A reported domain failure is not a panic

- **GIVEN** an intrinsic carrying the reported checking mode
- **WHEN** it is evaluated on an input outside its domain
- **THEN** the failure surfaces as a recoverable error carrying a message naming the operation
- **BUT** it is not a panic and not an abort

#### Scenario: An infallible operation carries no mode

- **GIVEN** an operation total over its parameter types, such as `math.fabs`
- **WHEN** it is lowered and emitted
- **THEN** it carries no checking mode
- **AND** no backend emits a domain test for it

### Requirement: A backend supplies spellings and may decline a module

A backend SHALL map each supported operation onto its own target-native operation. A backend that
has no mapping for a module SHALL fail with a diagnostic reporting the mapping as planned, distinct
from the backend itself being unknown or unimplemented. The refusal SHALL attach to the
`(module, backend)` pair, so a backend with no mapping still compiles programs that use no module.

#### Scenario: A backend without a mapping reports it as planned

- **GIVEN** a program that imports `math`
- **WHEN** the program is compiled with a backend that has no `math` mapping
- **THEN** compilation fails reporting that the mapping for that module and backend is planned
- **BUT** the message does not claim the backend is unknown or unimplemented

#### Scenario: A backend without a mapping still compiles a program using no module

- **GIVEN** a program that imports nothing
- **WHEN** the program is compiled with a backend that has no `math` mapping
- **THEN** compilation succeeds

#### Scenario: Spellings do not reach the IR

- **GIVEN** a lowered unit containing an intrinsic
- **WHEN** the unit's IR is written as an artifact
- **THEN** the intrinsic names a module and an operation by meaning
- **AND** it carries no target-language spelling
