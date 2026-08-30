## MODIFIED Requirements

### Requirement: Reject constructs outside the subset

Lowering SHALL reject any statement or expression outside the supported subset, including
control flow, class statements, and top-level statements other than function definitions and
imports of supported modules. The diagnostic SHALL name the unsupported construct. The single
exception is a leading docstring, defined in "Docstrings are accepted and carry no runtime meaning".

#### Scenario: Control flow is rejected

- **GIVEN** a function body containing an `if` statement
- **WHEN** it is lowered by a frontend whose subset excludes control flow
- **THEN** lowering fails with a diagnostic naming the conditional as unsupported

#### Scenario: Top-level statement is rejected

- **GIVEN** a source containing an `if __name__ == '__main__':` guard
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that only function definitions and imports
  are permitted at top level

#### Scenario: A module-level docstring is still rejected

- **GIVEN** a source whose first statement is a module-level string literal
- **WHEN** it is lowered
- **THEN** lowering fails, because the docstring exception applies only inside a function body

#### Scenario: An import of a supported module is accepted

- **GIVEN** a source whose first statement is

  ```python
  import math
  ```

- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering succeeds
- **AND** the module name is available as a namespace in every function body of that source

#### Scenario: Import is rejected

- **GIVEN** a source importing a module the registry does not support
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming the module
- **AND** the diagnostic lists the supported modules, so the construct that was refused outright is
  now refused only where it cannot be honoured

#### Scenario: A from-import is rejected

- **GIVEN** a source containing

  ```python
  from math import sqrt
  ```

- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming the supported import form, because a
  bare name in the body cannot be distinguished from a function defined in the same source

#### Scenario: Non-simple parameter forms are rejected

- **GIVEN** a function declaring variadic, keyword-only, or defaulted parameters
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming the unsupported parameter form

#### Scenario: Decorated or async function is rejected

- **GIVEN** a function carrying a decorator or declared `async def`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming the unsupported function form

#### Scenario: True division is accepted

- **GIVEN** an expression using `/`
- **WHEN** it is lowered
- **THEN** lowering succeeds, because true division is now part of the supported subset

#### Scenario: Unsupported operator is rejected

- **GIVEN** an expression using an operator outside the supported set
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming the operator as unsupported

#### Scenario: Exponentiation stays rejected even though a power operation exists

- **GIVEN** a source whose body contains `2 ** 10`
- **AND** a registry that supports `math.pow`
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails, because `math.pow` always yields a float and is a different operation
  from Python's integer-preserving `**`

#### Scenario: Out-of-range integer literal is rejected

- **GIVEN** an integer literal too large to be represented as an `i64`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that the literal exceeds the supported
  integer range, rather than silently truncating it

#### Scenario: Non-finite float literal is not producible

- **GIVEN** any floating-point literal written in source
- **WHEN** it is lowered
- **THEN** lowering succeeds, since Python source cannot spell infinity as a literal

## ADDED Requirements

### Requirement: An imported module binds a namespace, not a value

Lowering SHALL treat an imported module name as a namespace usable only as the receiver of an
attribute access. A module name SHALL NOT be bound to a local, passed as an argument, returned,
stored in a collection or attribute, or compared. Each such use SHALL be a located diagnostic
explaining that a module is not a value. The rejection replaces the blanket refusal in
[`lower.rs`](../../../../../crates/compylr-frontend-python/src/lower.rs#L585).

#### Scenario Outline: A module name outside receiver position is a located diagnostic

- **GIVEN** a source that imports `math` and whose body contains `<use>`
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic reporting that a module is not a value

**Examples:**

| use              |
| ---------------- |
| `m = math`       |
| `f(math)`        |
| `return math`    |
| `xs.append(math)`|
| `math == math`   |

#### Scenario: An alias names the same namespace

- **GIVEN** a source containing

  ```python
  import math as m


  def root(x: float) -> float:
      return m.sqrt(x)
  ```

- **WHEN** the source is lowered by the `python` frontend
- **THEN** the intrinsic resolves to the same operation `math.sqrt` resolves to

#### Scenario: An alias does not leak the original name

- **GIVEN** a source containing `import math as m` and a body using `math.sqrt(x)`
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails, because only the alias was introduced

#### Scenario: A module namespace is scoped to its source

- **GIVEN** a unit of two sources, only the first of which imports a module
- **WHEN** a function in the second source uses that module
- **THEN** lowering fails with an unbound-name diagnostic

#### Scenario: An unknown attribute of a module is located

- **GIVEN** a source that imports `math` and names an attribute the registry does not list
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming the module and the attribute

### Requirement: An intrinsic call is typed from the registry

Lowering SHALL determine an intrinsic's argument requirements and result type from the registry,
SHALL apply the same numeric promotion it applies elsewhere, and SHALL reject a mismatch with a
located diagnostic. An intrinsic result SHALL determine the type of a local bound to it, in the
same way a call to a function in the same source does. The checking mode SHALL come from the
resolved behavior and never from the operation's name.

#### Scenario: A binding infers its type from an intrinsic

- **GIVEN** a source whose body contains `n = math.floor(2.7)` with no annotation on `n`
- **WHEN** the source is lowered by the `python` frontend
- **THEN** the binding takes the result type the registry declares
- **AND** `n` is bound at the integer type

#### Scenario: An integer argument is promoted

- **GIVEN** a source applying an operation declared over floating-point to an integer expression
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering inserts the widening it inserts for any other numeric promotion

#### Scenario: A mismatched argument is a located diagnostic

- **GIVEN** a source applying an intrinsic to an argument of a type the signature does not accept
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming the operation, the expected type, and
  the supplied type

#### Scenario: An intrinsic result must still satisfy a declared type

- **GIVEN** a function declaring an integer return whose body returns a floating-point intrinsic
  result
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with the existing declared-versus-inferred diagnostic

#### Scenario: The checking mode comes from the resolved behavior

- **GIVEN** a source whose body contains a fallible intrinsic
- **WHEN** the source is lowered by the `python` frontend
- **THEN** the intrinsic's checking mode is taken from the resolved behavior, in the same way a
  fallible arithmetic operation's mode is
- **BUT** it is not taken from the operation's name
