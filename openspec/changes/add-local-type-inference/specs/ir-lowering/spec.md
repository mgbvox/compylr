## MODIFIED Requirements

### Requirement: Require complete type annotations

Lowering SHALL reject any function whose parameters or return type are not explicitly
annotated. Parameters and returns are the boundary a future binding generator reads, so they
stay explicit even where a value could be inferred. A local binding SHALL carry an explicit
type annotation only when its initializer's type is not determined by the rules in "Infer
local binding types from their initializer". The diagnostic SHALL name the offending
parameter, function, or variable.

#### Scenario: Unannotated parameter

- **WHEN** lowering a function declared as `def add(a, b: int) -> int:`
- **THEN** lowering fails with a diagnostic naming parameter `a` as missing a type annotation

#### Scenario: Missing return annotation

- **WHEN** lowering a function declared as `def add(a: int, b: int):`
- **THEN** lowering fails with a diagnostic naming function `add` as missing a return type
  annotation

#### Scenario: Unannotated local assignment from a literal

- **WHEN** lowering a function body containing `x = 1`
- **THEN** lowering succeeds and the IR body binds `x` with the IR integer type, reversing the
  previous rule that required an annotation here

#### Scenario: Annotated local assignment is accepted

- **WHEN** lowering a function body containing `x: int = 1`
- **THEN** lowering succeeds and the IR body binds `x` with the IR integer type

#### Scenario: Undetermined initializer still requires an annotation

- **WHEN** lowering a function body containing `b = helper(a)`
- **THEN** lowering fails with a diagnostic naming `b` as requiring an explicit type
  annotation, because a call's type is not determined during lowering

### Requirement: Reject unsupported type annotations

Lowering SHALL reject any annotation outside the supported set, and the diagnostic SHALL
report the annotation as written in the source. The supported set is `int`, `float`, `bool`,
and `str`, plus `None` as a return annotation only.

#### Scenario: Floating-point annotation is accepted

- **WHEN** lowering a function whose parameter is annotated `float`
- **THEN** lowering succeeds and the parameter has the IR floating-point type

#### Scenario: Unsupported scalar annotation

- **WHEN** lowering a function whose parameter is annotated `complex`
- **THEN** lowering fails with a diagnostic reporting `complex` as an unsupported type

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

#### Scenario: True division is accepted

- **WHEN** lowering an expression using `/`
- **THEN** lowering succeeds, because true division is now part of the supported subset

#### Scenario: Unsupported operator is rejected

- **WHEN** lowering an expression using an operator outside the supported set, such as
  exponentiation or a bitwise operator
- **THEN** lowering fails with a diagnostic naming the operator as unsupported

#### Scenario: Out-of-range integer literal is rejected

- **WHEN** lowering an integer literal too large to be represented as an `i64`
- **THEN** lowering fails with a diagnostic reporting that the literal exceeds the supported
  integer range, rather than silently truncating it

#### Scenario: Non-finite float literal is not producible

- **WHEN** lowering any floating-point literal written in source
- **THEN** lowering succeeds, since Python source cannot spell infinity or NaN as a literal

## ADDED Requirements

### Requirement: Infer local binding types from their initializer

When a local binding has no annotation, lowering SHALL infer its type from the initializer
whenever that type is determined, and SHALL reject the binding otherwise. An initializer's
type is determined when it is built only from literals, references to names already bound
with a known type, negation, arithmetic, and comparisons — composed to any depth. An
initializer whose type is not determined, which today means any expression containing a call,
SHALL still require an annotation.

Inference never guesses: each form above has exactly one possible result type given its
operands, so this computes an answer that was already fixed rather than choosing among
candidates. Direct aliasing (`b = a`) is a case of this rule, not a rule of its own.

#### Scenario: String literal is inferred

- **WHEN** lowering a body containing `a = "x"`
- **THEN** lowering succeeds and `a` is bound with the IR string type

#### Scenario: Integer literal is inferred

- **WHEN** lowering a body containing `b = 3`
- **THEN** lowering succeeds and `b` is bound with the IR integer type

#### Scenario: Floating-point literal is inferred

- **WHEN** lowering a body containing `c = 1.3`
- **THEN** lowering succeeds and `c` is bound with the IR floating-point type

#### Scenario: Boolean literal is inferred

- **WHEN** lowering a body containing `d = True`
- **THEN** lowering succeeds and `d` is bound with the IR boolean type

#### Scenario: Alias of a parameter is inferred

- **WHEN** lowering `def foo(a: int) -> int:` whose body contains `b = a` and returns `b`
- **THEN** lowering succeeds
- **AND** the IR binds `b` with the IR integer type

#### Scenario: Chained aliases are inferred

- **WHEN** lowering a body where `b = a` is followed by `c = b`, with `a` a boolean parameter
- **THEN** lowering succeeds and both `b` and `c` are bound with the IR boolean type

#### Scenario: Arithmetic expression is inferred

- **WHEN** lowering a body containing `b = a + 1`, where `a` is an integer parameter
- **THEN** lowering succeeds and `b` is bound with the IR integer type

#### Scenario: Comparison expression is inferred as boolean

- **WHEN** lowering a body containing `b = a < 10`, where `a` is an integer parameter
- **THEN** lowering succeeds and `b` is bound with the IR boolean type

#### Scenario: Negation preserves the operand type

- **WHEN** lowering a body containing `b = -c`, where `c` is a floating-point local
- **THEN** lowering succeeds and `b` is bound with the IR floating-point type

#### Scenario: Deeply nested expression is inferred

- **WHEN** lowering a body containing `b = (a + 1) * 2 - 3`, where `a` is an integer parameter
- **THEN** lowering succeeds and `b` is bound with the IR integer type

#### Scenario: Reference to an unbound name is unresolved

- **WHEN** lowering a body containing `b = q` where `q` is neither a parameter nor a
  previously bound local
- **THEN** lowering fails with a diagnostic reporting `q` as unresolved, not as a missing
  annotation

#### Scenario: Explicit annotation still wins over inference

- **WHEN** lowering a body containing `b: int = a` where `a` is an integer parameter
- **THEN** lowering succeeds using the declared type


### Requirement: Operator type rules

Lowering SHALL determine an expression's type from its operator and operand types, and SHALL
reject operand types the operator does not accept. Arithmetic operators SHALL accept numeric
operands and produce a numeric result; true division SHALL always produce a floating-point
result even when both operands are integers; string concatenation with `+` SHALL produce a
string; and every comparison SHALL produce a boolean. Booleans SHALL NOT be usable as numbers,
so that a backend never has to decide what a boolean means in arithmetic.

#### Scenario: Integer arithmetic yields an integer

- **WHEN** lowering `a + b`, `a - b`, `a * b`, `a // b`, or `a % b` with two integer operands
- **THEN** the expression's type is the IR integer type

#### Scenario: Floating-point arithmetic yields a float

- **WHEN** lowering an arithmetic expression with two floating-point operands
- **THEN** the expression's type is the IR floating-point type

#### Scenario: True division always yields a float

- **WHEN** lowering `a / b` with two integer operands
- **THEN** lowering succeeds and the expression's type is the IR floating-point type

#### Scenario: Floor division of integers stays an integer

- **WHEN** lowering `a // b` with two integer operands
- **THEN** the expression's type is the IR integer type, distinguishing it from `a / b`

#### Scenario: String concatenation yields a string

- **WHEN** lowering `a + b` with two string operands
- **THEN** the expression's type is the IR string type

#### Scenario: Comparison yields a boolean

- **WHEN** lowering any supported comparison between two operands of compatible type
- **THEN** the expression's type is the IR boolean type

#### Scenario: Arithmetic on a string and a number is rejected

- **WHEN** lowering `a + b` where `a` is a string and `b` is an integer
- **THEN** lowering fails with a diagnostic reporting the operand types

#### Scenario: Arithmetic on booleans is rejected

- **WHEN** lowering `a + b` where both operands are booleans
- **THEN** lowering fails with a diagnostic reporting that booleans are not numeric

#### Scenario: Negating a non-numeric value is rejected

- **WHEN** lowering `-a` where `a` is a string
- **THEN** lowering fails with a diagnostic reporting the operand type

#### Scenario: Comparing unrelated types is rejected

- **WHEN** lowering `a < b` where `a` is a string and `b` is an integer
- **THEN** lowering fails with a diagnostic reporting the operand types

### Requirement: Numeric promotion

When one operand of an arithmetic or comparison expression is an integer and the other is a
floating-point number, lowering SHALL treat the expression as floating-point, matching
Python. The IR SHALL record the promotion explicitly rather than leaving a backend to infer
it from operand types, so that a target whose native operators do not promote emits the
correct conversion.

#### Scenario: Mixed arithmetic promotes to float

- **WHEN** lowering `a + b` where `a` is an integer and `b` is a floating-point number
- **THEN** lowering succeeds and the expression's type is the IR floating-point type

#### Scenario: Mixed comparison is permitted

- **WHEN** lowering `a < b` where `a` is an integer and `b` is a floating-point number
- **THEN** lowering succeeds and the expression's type is the IR boolean type

#### Scenario: Promotion is visible in the IR

- **WHEN** an integer operand is used where the expression's type is floating-point
- **THEN** the IR makes the conversion explicit, so a backend does not need to re-derive it

### Requirement: Check declared types against inferred types

Where a binding carries an annotation and its initializer's type is determined, lowering
SHALL reject a disagreement between the two. Likewise, where a returned expression's type is
determined, lowering SHALL reject a disagreement with the function's declared return type.
Catching these here is strictly better than handing a backend IR it cannot render.

#### Scenario: Annotation conflicting with the initializer is rejected

- **WHEN** lowering a body containing `b: str = 1`
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

#### Scenario: Annotation conflicting with an aliased name is rejected

- **WHEN** lowering a body containing `b: str = a` where `a` is an integer parameter
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

#### Scenario: Returned value conflicting with the declared return type is rejected

- **WHEN** lowering `def f() -> int:` whose body returns `"x"`
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

#### Scenario: Returning a value from a unit function is rejected

- **WHEN** lowering `def f() -> None:` whose body returns `1`
- **THEN** lowering fails with a diagnostic reporting that the function returns no value

#### Scenario: Integer initializer for a float annotation is accepted

- **WHEN** lowering a body containing `c: float = 1`
- **THEN** lowering succeeds, because numeric promotion makes an integer acceptable where a
  float is declared

#### Scenario: Float initializer for an integer annotation is rejected

- **WHEN** lowering a body containing `n: int = 1.5`
- **THEN** lowering fails, because narrowing a float to an integer would silently lose
  information

#### Scenario: Undetermined initializer is not type checked

- **WHEN** lowering a body containing `b: int = helper(a)`
- **THEN** lowering succeeds using the declared type, because the initializer's type is not
  determined during lowering

## REMOVED Requirements

### Requirement: Infer local types from direct aliases

**Reason**: Superseded by "Infer local binding types from their initializer". The old rule
asserted that aliasing is "deliberately the only inference performed" and that "every other
unannotated initializer SHALL still be rejected" — both directly contradicted by this change.
Editing it in place would have preserved scenario titles such as "Unannotated binding from an
expression is still rejected" whose outcomes now invert, leaving misleading text in the main
spec. Retiring it and stating the general rule afresh keeps the spec readable.

**Migration**: No source changes are required. Every program the alias rule accepted is still
accepted, because a bare name reference is one of the determined initializer forms in the new
requirement. Programs that previously needed an annotation on a literal or arithmetic
initializer may now drop it, but are not obliged to.
