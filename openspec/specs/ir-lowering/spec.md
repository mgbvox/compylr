## Purpose

Translates parsed Python into compylr IR while enforcing the strict, fully annotated Python
subset the compiler accepts, and validates the assembled unit so that every call resolves.
Anything outside the subset is rejected with a precise, located diagnostic instead of a guess.

## Requirements

### Requirement: Lower a parsed source to IR functions

Lowering SHALL accept a parsed Python source and produce one IR function per top-level
function definition it contains, preserving the structure of each body. Lowering a source
SHALL NOT require knowledge of any other source.

#### Scenario: Single annotated function

- **WHEN** lowering a source containing `def add(a: int, b: int) -> int:` whose body returns
  `a + b`
- **THEN** lowering succeeds
- **AND** it yields one function named `add` with two integer parameters, an integer return
  type, and a body returning the sum of both parameter references

#### Scenario: Multiple functions in one source

- **WHEN** lowering a source defining three annotated functions
- **THEN** lowering yields all three functions, in source order

#### Scenario: Supported statement and expression coverage

- **WHEN** lowering a function that uses a typed local binding, arithmetic, a comparison, a
  string literal, and a call
- **THEN** lowering succeeds and each construct is present in the resulting IR body

#### Scenario: Empty source

- **WHEN** lowering a source containing no statements
- **THEN** lowering succeeds and yields no functions

### Requirement: Require complete type annotations

Lowering SHALL reject any function whose parameters or return type are not explicitly
annotated. A local binding SHALL carry an explicit type annotation unless its type is fixed by
the alias rule in "Infer local types from direct aliases". The diagnostic SHALL name the
offending parameter, function, or variable.

#### Scenario: Unannotated parameter

- **WHEN** lowering a function declared as `def add(a, b: int) -> int:`
- **THEN** lowering fails with a diagnostic naming parameter `a` as missing a type annotation

#### Scenario: Missing return annotation

- **WHEN** lowering a function declared as `def add(a: int, b: int):`
- **THEN** lowering fails with a diagnostic naming function `add` as missing a return type
  annotation

#### Scenario: Unannotated local assignment from a literal

- **WHEN** lowering a function body containing `x = 1`
- **THEN** lowering fails with a diagnostic naming `x` as requiring an explicit type
  annotation

#### Scenario: Annotated local assignment is accepted

- **WHEN** lowering a function body containing `x: int = 1`
- **THEN** lowering succeeds and the IR body binds `x` with the IR integer type

### Requirement: Infer local types from direct aliases

When a local binding's initializer is a bare reference to a name already bound with a known
type, lowering SHALL infer the binding's type from that name instead of requiring an
annotation. This is deliberately the only inference performed: aliasing cannot be ambiguous
because the source type is already fixed, so it buys the common `b = a` case without pulling a
general inference engine into the compiler. Every other unannotated initializer SHALL still be
rejected, so the "strict annotated subset" promise holds everywhere the answer is not already
determined.

#### Scenario: Alias of a parameter is inferred

- **WHEN** lowering `def foo(a: int) -> int:` whose body contains `b = a` and returns `b`
- **THEN** lowering succeeds
- **AND** the IR binds `b` with the IR integer type

#### Scenario: Alias of a previously bound local is inferred

- **WHEN** lowering a body that binds `x: str = "hi"` and then contains `y = x`
- **THEN** lowering succeeds and the IR binds `y` with the IR string type

#### Scenario: Chained aliases are inferred

- **WHEN** lowering a body where `b = a` is followed by `c = b`, with `a` a boolean parameter
- **THEN** lowering succeeds and both `b` and `c` are bound with the IR boolean type

#### Scenario: Unannotated binding from an expression is still rejected

- **WHEN** lowering a body containing `b = a + 1`, where `a` is an annotated parameter
- **THEN** lowering fails with a diagnostic naming `b` as requiring an explicit type
  annotation, because the initializer is not a bare name

#### Scenario: Unannotated binding from a call is still rejected

- **WHEN** lowering a body containing `b = helper(a)`
- **THEN** lowering fails with a diagnostic naming `b` as requiring an explicit type
  annotation

#### Scenario: Alias of an unbound name is unresolved

- **WHEN** lowering a body containing `b = q` where `q` is neither a parameter nor a
  previously bound local
- **THEN** lowering fails with a diagnostic reporting `q` as unresolved, not as a missing
  annotation

#### Scenario: Explicit annotation still wins over inference

- **WHEN** lowering a body containing `b: int = a` where `a` is an integer parameter
- **THEN** lowering succeeds using the declared type

#### Scenario: Annotation conflicting with the aliased type is rejected

- **WHEN** lowering a body containing `b: str = a` where `a` is an integer parameter
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

### Requirement: Reject unsupported type annotations

Lowering SHALL reject any annotation outside the supported set, and the diagnostic SHALL
report the annotation as written in the source.

#### Scenario: Unsupported scalar annotation

- **WHEN** lowering a function whose parameter is annotated `float`
- **THEN** lowering fails with a diagnostic reporting `float` as an unsupported type

#### Scenario: Unsupported generic annotation

- **WHEN** lowering a function whose parameter is annotated `list[int]`
- **THEN** lowering fails with a diagnostic reporting the annotation as unsupported

#### Scenario: Type parameters are rejected

- **WHEN** lowering a function declared as `def f[T](a: T) -> T:`
- **THEN** lowering fails with a diagnostic reporting that type parameters are not yet
  supported

#### Scenario: None is rejected as a parameter type

- **WHEN** lowering a function whose parameter is annotated `None`
- **THEN** lowering fails, because `None` is supported only as a return annotation

### Requirement: Reject constructs outside the subset

Lowering SHALL reject any statement or expression outside the supported subset, including
control flow, class and import statements, and top-level statements other than function
definitions. The diagnostic SHALL name the unsupported construct.

#### Scenario: Control flow is rejected

- **WHEN** lowering a function body containing an `if` statement
- **THEN** lowering fails with a diagnostic naming the conditional as unsupported

#### Scenario: Top-level statement is rejected

- **WHEN** lowering a source containing an `if __name__ == '__main__':` guard
- **THEN** lowering fails with a diagnostic reporting that only function definitions are
  permitted at top level

#### Scenario: Import is rejected

- **WHEN** lowering a source containing an import statement
- **THEN** lowering fails with a diagnostic naming the import as unsupported

#### Scenario: Non-simple parameter forms are rejected

- **WHEN** lowering a function declaring variadic parameters (`*args` or `**kwargs`),
  keyword-only or positional-only parameters, or a parameter with a default value
- **THEN** lowering fails with a diagnostic naming the unsupported parameter form

#### Scenario: Decorated or async function is rejected

- **WHEN** lowering a function that carries a decorator or is declared `async def`
- **THEN** lowering fails with a diagnostic naming the unsupported function form

#### Scenario: Unsupported operator is rejected

- **WHEN** lowering an expression using an operator outside the supported set, such as true
  division
- **THEN** lowering fails with a diagnostic naming the operator as unsupported

#### Scenario: Out-of-range integer literal is rejected

- **WHEN** lowering an integer literal too large to be represented as an `i64`
- **THEN** lowering fails with a diagnostic reporting that the literal exceeds the supported
  integer range, rather than silently truncating it

### Requirement: Resolve local names during lowering

Lowering SHALL accept a name reference only when it refers to a parameter of the enclosing
function or to a local bound earlier in that function's body. References that resolve to
nothing SHALL be rejected.

#### Scenario: Parameter and local references resolve

- **WHEN** lowering a body that binds `x: int = a + 1` and then returns `x`
- **THEN** lowering succeeds

#### Scenario: Reference to an unbound name

- **WHEN** lowering a body that references a name that is neither a parameter nor a
  previously bound local
- **THEN** lowering fails with a diagnostic reporting the name as unresolved

#### Scenario: Reference before binding

- **WHEN** lowering a body that references a local before the statement that binds it
- **THEN** lowering fails with a diagnostic reporting the name as unresolved

#### Scenario: Rebinding an existing local is rejected

- **WHEN** lowering a body that assigns to a name already bound in that function, whether or
  not the new type matches
- **THEN** lowering fails with a diagnostic reporting that reassignment is not yet supported,
  keeping every IR binding a single introduction of a new name

#### Scenario: Binding over a parameter name is rejected

- **WHEN** lowering a body that assigns to a name that is already a parameter of the function
- **THEN** lowering fails with a diagnostic reporting that reassignment is not yet supported

### Requirement: Validate calls against the assembled unit

Because functions reach the compiler independently but share one build artifact, call targets
SHALL be resolved against the assembled unit rather than against the source that contained the
call. Validating a unit SHALL reject any call whose target is not a function in that unit, and
any call whose argument count differs from the target's parameter count.

#### Scenario: Call across sources resolves

- **WHEN** a function lowered from one source calls a function lowered from another source,
  and both have been added to the same unit
- **THEN** validating the unit succeeds

#### Scenario: Call to a function added later resolves

- **WHEN** a unit is validated after its called function has been added
- **THEN** validation succeeds regardless of the order the two functions were added

#### Scenario: Call to an unknown function

- **WHEN** validating a unit containing a call to a name that is not a function in the unit
- **THEN** validation fails with a diagnostic reporting the name as unresolved

#### Scenario: Argument count mismatch

- **WHEN** validating a unit containing a call that passes a different number of arguments
  than the target function declares
- **THEN** validation fails with a diagnostic reporting the expected and actual argument
  counts

### Requirement: Diagnostics are located and non-fatal to the process

Lowering and unit validation MUST NOT panic on any input that the frontend parsed
successfully. Every diagnostic SHALL carry the source position of the offending construct and
render as a human-readable message naming both the problem and its location.

#### Scenario: Rejection does not panic

- **WHEN** lowering any source that violates the subset rules
- **THEN** lowering returns a failure result and the process continues running

#### Scenario: Diagnostic carries a position

- **WHEN** lowering fails on a construct at a known position in the source
- **THEN** the diagnostic carries that source position

#### Scenario: First violation is reported

- **WHEN** lowering a source containing more than one subset violation
- **THEN** lowering fails reporting the first violation in source order
