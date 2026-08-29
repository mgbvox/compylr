## MODIFIED Requirements

### Requirement: Reject constructs outside the subset

Lowering SHALL reject any statement or expression outside the supported subset, including
control flow, class statements, and top-level statements other than function definitions and
imports of supported modules. The diagnostic SHALL name the unsupported construct. The single
exception is a leading docstring, defined in "Docstrings are accepted and carry no runtime meaning".

#### Scenario: Control flow is rejected

- **WHEN** lowering a function body containing an `if` statement
- **THEN** lowering fails with a diagnostic naming the conditional as unsupported

#### Scenario: Top-level statement is rejected

- **WHEN** lowering a source containing an `if __name__ == '__main__':` guard
- **THEN** lowering fails with a diagnostic reporting that only function definitions and imports
  are permitted at top level

#### Scenario: A module-level docstring is still rejected

- **WHEN** lowering a source whose first statement is a module-level string literal
- **THEN** lowering fails, because the docstring exception applies only inside a function body

#### Scenario: An import of a supported module is accepted

- **WHEN** lowering a source containing an import of a supported module, with or without an alias
- **THEN** lowering succeeds and the module name is available as a namespace in every function body
  of that source

#### Scenario: Import is rejected

- **WHEN** lowering a source containing an import of a module the registry does not support
- **THEN** lowering fails with a located diagnostic naming the module and listing the supported
  ones, so the construct that was refused outright is now refused only where it cannot be honoured

#### Scenario: A from-import is rejected

- **WHEN** lowering a source containing `from math import sqrt`
- **THEN** lowering fails with a located diagnostic naming the supported import form, because a
  bare name in the body cannot be distinguished from a function defined in the same source

#### Scenario: Non-simple parameter forms are rejected

- **WHEN** lowering a function declaring variadic parameters (`*args` or `**kwargs`),
  keyword-only or positional-only parameters, or a parameter with a default value
- **THEN** lowering fails with a diagnostic naming the unsupported parameter form

#### Scenario: Decorated or async function is rejected

- **WHEN** lowering a function that carries a decorator or is declared `async def`
- **THEN** lowering fails with a diagnostic naming the unsupported function form

#### Scenario: True division is accepted

- **WHEN** lowering an expression using `/`
- **THEN** lowering succeeds, because true division is now part of the supported subset

#### Scenario: Unsupported operator is rejected

- **WHEN** lowering an expression using an operator outside the supported set, such as
  exponentiation or a bitwise operator
- **THEN** lowering fails with a diagnostic naming the operator as unsupported

#### Scenario: Exponentiation stays rejected even though a power operation exists

- **WHEN** lowering an expression using `**`
- **THEN** lowering fails, because `math.pow` always yields a float and is a different operation
  from Python's integer-preserving `**`

#### Scenario: Out-of-range integer literal is rejected

- **WHEN** lowering an integer literal too large to be represented as an `i64`
- **THEN** lowering fails with a diagnostic reporting that the literal exceeds the supported
  integer range, rather than silently truncating it

#### Scenario: Non-finite float literal is not producible

- **WHEN** lowering any floating-point literal written in source
- **THEN** lowering succeeds, since Python source cannot spell infinity as a literal

## ADDED Requirements

### Requirement: An imported module binds a namespace, not a value

Lowering SHALL treat an imported module name as a namespace usable only as the receiver of an
attribute access. A module name SHALL NOT be bound to a local, passed as an argument, returned,
stored in a collection or attribute, or compared. Each such use SHALL be a located diagnostic
explaining that a module is not a value.

#### Scenario: A module name cannot be bound

- **WHEN** lowering `m = math`
- **THEN** lowering fails with a located diagnostic reporting that a module is not a value

#### Scenario: A module name cannot be passed or returned

- **WHEN** lowering a call that passes an imported module as an argument, or a `return` of one
- **THEN** lowering fails with a located diagnostic reporting that a module is not a value

#### Scenario: An alias names the same namespace

- **WHEN** lowering `import math as m` and a body using `m.sqrt(x)`
- **THEN** the intrinsic resolves to the same operation `math.sqrt` resolves to

#### Scenario: An alias does not leak the original name

- **WHEN** a source contains `import math as m` and a body using `math.sqrt(x)`
- **THEN** lowering fails, because only the alias was introduced

#### Scenario: A module namespace is scoped to its source

- **WHEN** one source imports a module and another source in the same unit does not
- **THEN** the second source's functions cannot use that module, and using it is an unbound-name
  diagnostic

#### Scenario: An unknown attribute of a module is located

- **WHEN** lowering an attribute of a supported module that the registry does not list
- **THEN** lowering fails with a located diagnostic naming the module and the attribute

### Requirement: An intrinsic call is typed from the registry

Lowering SHALL determine an intrinsic's argument requirements and result type from the registry,
SHALL apply the same numeric promotion it applies elsewhere, and SHALL reject a mismatch with a
located diagnostic. An intrinsic result SHALL determine the type of a local bound to it, in the
same way a call to a function in the same source does.

#### Scenario: A binding infers its type from an intrinsic

- **WHEN** lowering a local bound to an intrinsic result with no annotation
- **THEN** the binding takes the result type the registry declares

#### Scenario: An integer argument is promoted

- **WHEN** lowering an operation declared over floating-point applied to an integer expression
- **THEN** lowering inserts the widening it inserts for any other numeric promotion

#### Scenario: A mismatched argument is a located diagnostic

- **WHEN** lowering an intrinsic applied to an argument of a type the signature does not accept
- **THEN** lowering fails with a located diagnostic naming the operation, the expected type, and
  the supplied type

#### Scenario: An intrinsic result must still satisfy a declared type

- **WHEN** a function declaring an integer return returns a floating-point intrinsic result
- **THEN** lowering fails with the existing declared-versus-inferred diagnostic

#### Scenario: The checking mode comes from the resolved behavior

- **WHEN** lowering a fallible intrinsic
- **THEN** its checking mode is taken from the resolved behavior, in the same way a fallible
  arithmetic operation's mode is, and not from the operation's name
