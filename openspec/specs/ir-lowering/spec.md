## Purpose

Translates parsed Python into compylr IR while enforcing the strict, fully annotated Python
subset the compiler accepts, and validates the assembled unit so that every call resolves.
Anything outside the subset is rejected with a precise, located diagnostic instead of a guess.

## Requirements

### Requirement: Lower a parsed source to IR functions

Lowering SHALL accept a parsed Python source and a resolved behavior, and produce one IR function
per top-level function definition the source contains, preserving the structure of each body.
Lowering a source SHALL NOT require knowledge of any other source.

#### Scenario: Single annotated function

- **GIVEN** a source containing `def add(a: int, b: int) -> int:` whose body returns their sum
- **WHEN** it is lowered
- **THEN** lowering succeeds
- **AND** it yields one function named `add` with two integer parameters, an integer return
  type, and a body returning the sum of both parameter references

#### Scenario: Multiple functions in one source

- **GIVEN** a source defining three annotated functions
- **WHEN** it is lowered
- **THEN** lowering yields all three functions, in source order

#### Scenario: Supported statement and expression coverage

- **GIVEN** a function using a typed local binding, arithmetic, a comparison, a call, and a
  return
- **WHEN** it is lowered
- **THEN** lowering succeeds and each construct is present in the resulting IR body

#### Scenario: Empty source

- **GIVEN** a source containing no statements
- **WHEN** it is lowered
- **THEN** lowering succeeds and yields no functions

#### Scenario: The behavior travels with the source

- **GIVEN** two sources and two different behaviors
- **WHEN** they are lowered into one unit
- **THEN** each resulting function carries the modes of the behavior its own source was lowered
  under

### Requirement: Require complete type annotations

Lowering SHALL reject any function whose parameters or return type are not explicitly
annotated. Parameters and returns are the boundary a future binding generator reads, so they
stay explicit even where a value could be inferred. A local binding SHALL carry an explicit
type annotation only when its initializer's type is not determined by the rules in "Infer
local binding types from their initializer". The diagnostic SHALL name the offending
parameter, function, or variable.

#### Scenario: Unannotated parameter

- **GIVEN** a function declared as `def add(a, b: int) -> int:`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming parameter `a` as missing a type annotation

#### Scenario: Missing return annotation

- **GIVEN** a function declared as `def add(a: int, b: int):`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming function `add` as missing a return type
  annotation

#### Scenario: Unannotated local assignment from a literal

- **GIVEN** a function body containing `x = 1`
- **WHEN** it is lowered
- **THEN** lowering succeeds and the IR body binds `x` with the IR integer type, reversing the
  previous rule that required an annotation here

#### Scenario: Annotated local assignment is accepted

- **GIVEN** a function body containing `x: int = 1`
- **WHEN** it is lowered
- **THEN** lowering succeeds and the IR body binds `x` with the IR integer type

#### Scenario: Undetermined initializer still requires an annotation

- **GIVEN** a function body containing `b = helper(a)`, with `helper` defined nowhere in the
  sources
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming `b` as requiring an explicit type
  annotation, because a call's type is not determined during lowering

### Requirement: Infer local binding types from their initializer

When a local binding has no annotation, lowering SHALL infer its type from the initializer
whenever that type is determined, and SHALL reject the binding otherwise. An initializer's
type is determined when it is built only from literals, references to names already bound
with a known type, negation, arithmetic, comparisons, and **calls to functions whose signatures
are known** — composed to any depth.

Inference never guesses: each form above has exactly one possible result type given its
operands, so this computes an answer that was already fixed rather than choosing among
candidates. Direct aliasing (`b = a`) is a case of this rule, not a rule of its own.

#### Scenario: String literal is inferred

- **GIVEN** a body containing `a = "x"`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `a` is bound with the IR string type

#### Scenario: Integer literal is inferred

- **GIVEN** a body containing `b = 3`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `b` is bound with the IR integer type

#### Scenario: Floating-point literal is inferred

- **GIVEN** a body containing `c = 1.3`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `c` is bound with the IR floating-point type

#### Scenario: Boolean literal is inferred

- **GIVEN** a body containing `d = True`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `d` is bound with the IR boolean type

#### Scenario: Alias of a parameter is inferred

- **GIVEN** `def foo(a: int) -> int:` whose body contains `b = a` and returns `b`
- **WHEN** it is lowered
- **THEN** lowering succeeds
- **AND** the IR binds `b` with the IR integer type

#### Scenario: Alias of a previously bound local is inferred

- **GIVEN** a body binding `x: str = "hi"` and then containing `y = x`
- **WHEN** it is lowered
- **THEN** lowering succeeds and the IR binds `y` with the IR string type

#### Scenario: Chained aliases are inferred

- **GIVEN** a body where `b = a` is followed by `c = b`, with `a` a boolean parameter
- **WHEN** it is lowered
- **THEN** lowering succeeds and both `b` and `c` are bound with the IR boolean type

#### Scenario: Arithmetic expression is inferred

- **GIVEN** a body containing `b = a + 1`, where `a` is an integer parameter
- **WHEN** it is lowered
- **THEN** lowering succeeds and `b` is bound with the IR integer type

#### Scenario: Comparison expression is inferred as boolean

- **GIVEN** a body containing `b = a < 10`, where `a` is an integer parameter
- **WHEN** it is lowered
- **THEN** lowering succeeds and `b` is bound with the IR boolean type

#### Scenario: Negation preserves the operand type

- **GIVEN** a body containing `b = -c`, where `c` is a floating-point local
- **WHEN** it is lowered
- **THEN** lowering succeeds and `b` is bound with the IR floating-point type

#### Scenario: Deeply nested expression is inferred

- **GIVEN** a body containing `b = (a + 1) * 2 - 3`, where `a` is an integer parameter
- **WHEN** it is lowered
- **THEN** lowering succeeds and `b` is bound with the IR integer type

#### Scenario: A call initializer is inferred

- **GIVEN** a source defining `def double(n: int) -> int:` and a second function whose body binds
  `b = double(n)`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `b` is bound with the IR integer type, reversing the previous
  rule that required an annotation here

#### Scenario: A call nested inside an expression is inferred

- **GIVEN** a body containing `b = double(n) + 1`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `b` is bound with the callee's return type combined by the
  operator rules

#### Scenario: Reference to an unbound name is unresolved

- **GIVEN** a body containing `b = q`, where `q` is neither a parameter nor a local
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting `q` as unresolved, not as a missing
  annotation

#### Scenario: Explicit annotation still wins over inference

- **GIVEN** a body containing `b: int = a`, where `a` is an integer parameter
- **WHEN** it is lowered
- **THEN** lowering succeeds using the declared type

### Requirement: Reject unsupported type annotations

Lowering SHALL reject any annotation outside the supported set, and the diagnostic SHALL
report the annotation as written in the source. The supported set is `int`, `float`, `bool`, and
`str`; the parameterised forms `list[T]`, `dict[K, V]`, `set[T]`, and `tuple[T, ...]`, nested to
any depth over supported non-instance types; `None` as a return annotation only; and a class in the
assembled unit as a direct parameter or return annotation of a top-level free function.

Class names SHALL be gathered from every source before direct class-valued annotations are
resolved, so support does not depend on source or decoration order. A bare identifier that could
name a class in another source MAY remain unresolved while one source is validated, but SHALL
resolve when the complete unit is assembled or fail with a diagnostic at the annotation's source
location. This deferral SHALL NOT make built-in unsupported annotations such as `complex`, or
malformed annotations, temporarily valid.

A class-valued type nested beneath a collection in a Python-boundary signature, such as
`list[Tally]`, SHALL be rejected with a located diagnostic before target source is emitted. Direct
class annotations on method parameters or returns other than the implicit `self` receiver remain
outside this initial support and SHALL be rejected on the same terms.

A parameterised annotation SHALL be rejected when its parameters are missing, when their number
does not match the form, or when a parameter is itself unsupported. A bare `list`, `dict`, `set`,
or `tuple` without parameters SHALL be rejected, because an element type that is not written down
is not a type compylr can compile against.

#### Scenario: Floating-point annotation is accepted

- **GIVEN** a function whose parameter is annotated `float`
- **WHEN** it is lowered
- **THEN** lowering succeeds and the parameter has the IR floating-point type

#### Scenario: Collection annotations are accepted

- **GIVEN** a function whose parameters are annotated with each collection form
- **WHEN** it is lowered
- **THEN** lowering succeeds and each parameter has the corresponding IR collection type

#### Scenario: Nested collection annotations are accepted

- **GIVEN** a function whose parameter is annotated `dict[str, list[int]]`
- **WHEN** it is lowered
- **THEN** lowering succeeds and the nesting is preserved in the IR type

#### Scenario: Direct class annotations are accepted on a free function

- **GIVEN** a unit containing class `Tally` and a free function taking a borrow-only `Tally`
- **WHEN** it is lowered
- **THEN** lowering succeeds and the signature carries the `Tally` instance type

#### Scenario: A class annotation resolves across sources

- **GIVEN** a free function lowered before the separate source defining its annotated class
- **WHEN** the complete unit is assembled
- **THEN** the complete unit resolves the annotation regardless of source order

#### Scenario: An unknown class annotation is located

- **GIVEN** a complete unit containing a direct annotation `Taly` but no class of that name
- **WHEN** it is lowered
- **THEN** lowering fails at the annotation's location and reports `Taly` as unknown

#### Scenario: A nested class-valued boundary annotation is rejected

- **GIVEN** a top-level free function with a parameter or return annotated `list[Tally]`
- **WHEN** it is lowered
- **THEN** lowering fails at that annotation before target source is emitted

#### Scenario: A class-valued method boundary is not accepted accidentally

- **GIVEN** an exported method that directly takes or returns `Tally` other than as its receiver
- **WHEN** it is lowered
- **THEN** lowering fails with a located diagnostic explaining that the position is not supported

#### Scenario: An unparameterised collection annotation is rejected

- **GIVEN** a function whose parameter is annotated bare `list`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that an element type is required

#### Scenario: Wrong parameter count is rejected

- **GIVEN** a function whose parameter is annotated `dict[str]`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the annotation as unsupported

#### Scenario: An unsupported parameter is rejected

- **GIVEN** a function whose parameter is annotated `list[complex]`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting `complex` as an unsupported type

#### Scenario: A floating-point mapping key is rejected

- **GIVEN** a function whose parameter is annotated `dict[float, int]`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic explaining that a floating-point value cannot be a key

#### Scenario: A floating-point set element is rejected

- **GIVEN** a function whose parameter is annotated `set[float]`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic explaining that a floating-point value cannot be a set
  element

#### Scenario: Unsupported scalar annotation

- **GIVEN** a function whose parameter is annotated `complex`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting `complex` as an unsupported type

#### Scenario: Unsupported generic annotation

- **GIVEN** a function whose parameter is annotated `frozenset[int]`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the annotation as unsupported

#### Scenario: Type parameters are rejected

- **GIVEN** a function declared as `def f[T](a: T) -> T:`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that type parameters are not yet
  supported

#### Scenario: None is rejected as a parameter type

- **GIVEN** a function whose parameter is annotated `None`
- **WHEN** it is lowered
- **THEN** lowering fails, because `None` is supported only as a return annotation

### Requirement: Reject constructs outside the subset

Lowering SHALL reject any statement or expression outside the supported subset, including
control flow, class and import statements, and top-level statements other than function
definitions. The diagnostic SHALL name the unsupported construct. The single exception is a
leading docstring, defined in "Docstrings are accepted and carry no runtime meaning".

#### Scenario: Control flow is rejected

- **GIVEN** a function body containing an `if` statement
- **WHEN** it is lowered by a frontend whose subset excludes control flow
- **THEN** lowering fails with a diagnostic naming the conditional as unsupported

#### Scenario: Top-level statement is rejected

- **GIVEN** a source containing an `if __name__ == '__main__':` guard
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that only function definitions are
  permitted at top level

#### Scenario: A module-level docstring is still rejected

- **GIVEN** a source whose first statement is a module-level string literal
- **WHEN** it is lowered
- **THEN** lowering fails, because the docstring exception applies only inside a function body

#### Scenario: Import is rejected

- **GIVEN** a source containing an import statement
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming the import as unsupported

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

#### Scenario: Out-of-range integer literal is rejected

- **GIVEN** an integer literal too large to be represented as an `i64`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that the literal exceeds the supported
  integer range, rather than silently truncating it

#### Scenario: Non-finite float literal is not producible

- **GIVEN** any floating-point literal written in source
- **WHEN** it is lowered
- **THEN** lowering succeeds, since Python source cannot spell infinity or NaN as a literal

### Requirement: Operator type rules

Lowering SHALL determine an expression's type from its operator and operand types, and SHALL
reject operand types the operator does not accept. Arithmetic operators SHALL accept numeric
operands and produce a numeric result; true division SHALL always produce a floating-point
result even when both operands are integers; string concatenation with `+` SHALL produce a
string; and every comparison SHALL produce a boolean. Booleans SHALL NOT be usable as numbers,
so that a backend never has to decide what a boolean means in arithmetic.

#### Scenario: Integer arithmetic yields an integer

- **GIVEN** `a + b`, `a - b`, `a * b`, `a // b`, or `a % b` with two integer operands
- **WHEN** it is lowered
- **THEN** the expression's type is the IR integer type

#### Scenario: Floating-point arithmetic yields a float

- **GIVEN** an arithmetic expression with two floating-point operands
- **WHEN** it is lowered
- **THEN** the expression's type is the IR floating-point type

#### Scenario: True division always yields a float

- **GIVEN** the expression `a / b` with two integer operands
- **WHEN** it is lowered
- **THEN** lowering succeeds and the expression's type is the IR floating-point type

#### Scenario: Floor division of integers stays an integer

- **GIVEN** the expression `a // b` with two integer operands
- **WHEN** it is lowered
- **THEN** the expression's type is the IR integer type, distinguishing it from `a / b`

#### Scenario: String concatenation yields a string

- **GIVEN** the expression `a + b` with two string operands
- **WHEN** it is lowered
- **THEN** the expression's type is the IR string type

#### Scenario: Comparison yields a boolean

- **GIVEN** any supported comparison between two operands of compatible type
- **WHEN** it is lowered
- **THEN** the expression's type is the IR boolean type

#### Scenario: Arithmetic on a string and a number is rejected

- **GIVEN** the expression `a + b` where `a` is a string and `b` is an integer
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the operand types

#### Scenario: Arithmetic on booleans is rejected

- **GIVEN** the expression `a + b` where both operands are booleans
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that booleans are not numeric

#### Scenario: Negating a non-numeric value is rejected

- **GIVEN** the expression `-a` where `a` is a string
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the operand type

#### Scenario: Comparing unrelated types is rejected

- **GIVEN** the expression `a < b` where `a` is a string and `b` is an integer
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the operand types

### Requirement: Numeric promotion

When one operand of an arithmetic or comparison expression is an integer and the other is a
floating-point number, lowering SHALL treat the expression as floating-point, matching
Python. The IR SHALL record the promotion explicitly rather than leaving a backend to infer
it from operand types, so that a target whose native operators do not promote emits the
correct conversion.

#### Scenario: Mixed arithmetic promotes to float

- **GIVEN** the expression `a + b` where `a` is an integer and `b` is a floating-point number
- **WHEN** it is lowered
- **THEN** lowering succeeds and the expression's type is the IR floating-point type

#### Scenario: Mixed comparison is permitted

- **GIVEN** the expression `a < b` where `a` is an integer and `b` is a floating-point number
- **WHEN** it is lowered
- **THEN** lowering succeeds and the expression's type is the IR boolean type

#### Scenario: Promotion is visible in the IR

- **GIVEN** an integer operand used where the expression's type is floating-point
- **WHEN** it is lowered
- **THEN** the IR makes the conversion explicit, so a backend does not need to re-derive it

### Requirement: Check declared types against inferred types

Where a binding carries an annotation and its initializer's type is determined, lowering
SHALL reject a disagreement between the two. Likewise, where a returned expression's type is
determined, lowering SHALL reject a disagreement with the function's declared return type.
Catching these here is strictly better than handing a backend IR it cannot render.

#### Scenario: Annotation conflicting with the initializer is rejected

- **GIVEN** a body containing `b: str = 1`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

#### Scenario: Annotation conflicting with an aliased name is rejected

- **GIVEN** a body containing `b: str = a`, where `a` is an integer parameter
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

#### Scenario: Returned value conflicting with the declared return type is rejected

- **GIVEN** `def f() -> int:` whose body returns `"x"`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

#### Scenario: Returning a value from a unit function is rejected

- **GIVEN** `def f() -> None:` whose body returns `1`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that the function returns no value

#### Scenario: Integer initializer for a float annotation is accepted

- **GIVEN** a body containing `c: float = 1`
- **WHEN** it is lowered
- **THEN** lowering succeeds, because numeric promotion makes an integer acceptable where a
  float is declared

#### Scenario: Float initializer for an integer annotation is rejected

- **GIVEN** a body containing `n: int = 1.5`
- **WHEN** it is lowered
- **THEN** lowering fails, because narrowing a float to an integer would silently lose
  information

#### Scenario: Undetermined initializer is not type checked

- **GIVEN** a body containing `b: int = helper(a)`, with `helper` defined nowhere in the source
- **WHEN** it is lowered
- **THEN** lowering succeeds using the declared type, because the initializer's type is not
  determined during lowering

### Requirement: Resolve local names during lowering

Lowering SHALL accept a name reference only when it refers to a parameter of the enclosing
function or to a local bound earlier in that function's body. References that resolve to
nothing SHALL be rejected.

#### Scenario: Parameter and local references resolve

- **GIVEN** a body binding `x: int = a + 1` and then returning `x`
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: Reference to an unbound name

- **GIVEN** a body referencing a name that is neither a parameter nor a local
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the name as unresolved

#### Scenario: Reference before binding

- **GIVEN** a body referencing a local before the statement that binds it
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the name as unresolved

#### Scenario: Rebinding an existing local is rejected

- **GIVEN** a body assigning to a name already bound in that function
- **WHEN** it is lowered by a frontend whose subset excludes reassignment
- **THEN** lowering fails with a diagnostic reporting that reassignment is not yet supported,
  keeping every IR binding a single introduction of a new name

#### Scenario: Binding over a parameter name is rejected

- **GIVEN** a body assigning to a name that is already a parameter
- **WHEN** it is lowered by a frontend whose subset excludes reassignment
- **THEN** lowering fails with a diagnostic reporting that reassignment is not yet supported

### Requirement: Validate calls against the assembled unit

Because functions reach the compiler independently but share one build artifact, call targets
SHALL be resolved against the assembled unit rather than against the source that contained the
call. Validating a unit SHALL reject any call whose target is not a function in that unit, and
any call whose argument count differs from the target's parameter count.

#### Scenario: Call across sources resolves

- **GIVEN** a function in one source calling a function lowered from another
- **WHEN** the unit is validated
- **THEN** validating the unit succeeds

#### Scenario: Call to a function added later resolves

- **GIVEN** a unit whose called function has since been added
- **WHEN** it is validated
- **THEN** validation succeeds regardless of the order the two functions were added

#### Scenario: Call to an unknown function

- **GIVEN** a unit containing a call to a name that is not a function in it
- **WHEN** it is validated
- **THEN** validation fails with a diagnostic reporting the name as unresolved

#### Scenario: Argument count mismatch

- **GIVEN** a unit containing a call passing the wrong number of arguments
- **WHEN** it is validated
- **THEN** validation fails with a diagnostic reporting the expected and actual argument
  counts

### Requirement: Diagnostics are located and non-fatal to the process

Lowering and unit validation MUST NOT panic on any input that the frontend parsed
successfully. Every diagnostic SHALL carry the source position of the offending construct and
render as a human-readable message naming both the problem and its location.

This SHALL hold for **arbitrary** Python, not only for programs written to exercise a rejection
rule. A curated rejection corpus demonstrates that each *known* refusal is located; it cannot
demonstrate that an unanticipated construct is refused rather than crashed on, because every program
in it was written by someone who already knew the answer. The property is therefore established
against Python that was not written for this compiler.

#### Scenario: Rejection does not panic

- **GIVEN** any source violating the subset rules
- **WHEN** it is lowered
- **THEN** lowering returns a failure result and the process continues running

#### Scenario: Diagnostic carries a position

- **GIVEN** a source whose offending construct is at a known position
- **WHEN** it is lowered
- **THEN** the diagnostic carries that source position

#### Scenario: First violation is reported

- **GIVEN** a source containing more than one subset violation
- **WHEN** it is lowered
- **THEN** lowering fails reporting the first violation in source order

#### Scenario: Python written for other purposes is refused rather than crashed on

- **GIVEN** a parsed program not written to exercise a subset rule
- **WHEN** it is lowered
- **THEN** the outcome is a lowered unit or a failure carrying a source position, and never a panic

### Requirement: Docstrings are accepted and carry no runtime meaning

Lowering SHALL accept a **docstring**: a bare string-literal expression statement in the first
position of a function body. It SHALL contribute nothing to the function's behavior, matching
Python, where the interpreter records the docstring from the code object rather than by executing
the statement.

The exception SHALL be exactly this narrow. A bare expression statement anywhere else in a body,
or in first position but not a string literal, SHALL remain rejected: its value is discarded, so
it is either dead code or a side effect the subset cannot express.

A docstring SHALL NOT affect a function's fingerprint. It is prose about the function rather than
part of what the function computes, and a rebuild triggered by editing documentation would break
the existing guarantee that reformatting costs nothing.

#### Scenario: A documented function lowers

- **GIVEN** a function whose first body statement is a string literal
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: The docstring does not become a statement

- **GIVEN** a function with a docstring and a single `return`
- **WHEN** it is lowered
- **THEN** the IR body contains only the return statement

#### Scenario: The docstring is retained on the function

- **GIVEN** a function with a docstring
- **WHEN** it is lowered
- **THEN** the IR function carries the docstring's text

#### Scenario: A function with only a docstring and no return

- **GIVEN** a function annotated `-> None` whose body is just a docstring
- **WHEN** it is lowered
- **THEN** lowering succeeds and the body produces no value

#### Scenario: Editing a docstring does not change the fingerprint

- **GIVEN** one function, written twice with different docstring text
- **WHEN** both are lowered and fingerprinted
- **THEN** both produce the same fingerprint

#### Scenario: Adding a docstring does not change the fingerprint

- **GIVEN** one function, written with and without a docstring and otherwise identical
- **WHEN** both are lowered and fingerprinted
- **THEN** both produce the same fingerprint

#### Scenario: A string statement after the first is rejected

- **GIVEN** a body whose second statement is a bare string literal
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming the unsupported statement

#### Scenario: A non-string expression statement is still rejected

- **GIVEN** a body whose first statement is a bare expression such as `a + 1`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming the unsupported statement

#### Scenario: A bare call statement is still rejected

- **GIVEN** a body whose first statement is a bare call, discarding its result
- **WHEN** it is lowered
- **THEN** lowering fails, because the subset cannot express a call made for its side effect

### Requirement: Signatures are collected before bodies are lowered

Lowering a source SHALL proceed in two passes: first collecting every function's name, parameter
types, and return type, then lowering each body with that table available. A call's type SHALL be
taken from the collected signature.

The passes exist to keep results **independent of definition order**. A function may call one
defined later in the same source, and typing that call from a table built beforehand gives the same
answer either way. Signature collection reads annotations only — which are mandatory on parameters
and returns — so it never depends on inference and cannot itself be order-sensitive.

A call whose callee is **in** the table SHALL be typed from that signature, and its arity and
argument types SHALL be checked against it, with a location.

A call whose callee is **not** in the table SHALL leave the call's type undetermined rather than
being rejected. Lowering sees one source at a time, and a decorated function may legitimately call
one defined in another module that has not been marked yet — rejecting here would make acceptance
depend on decoration order, which is the property the unit's design exists to protect. Such a call
is still caught, by unit validation, once every source has been assembled.

#### Scenario: A function may call one defined later

- **GIVEN** a source where the first function calls the second
- **WHEN** it is lowered
- **THEN** lowering succeeds and the call is typed from the second function's signature

#### Scenario: Definition order does not change the result

- **GIVEN** two mutually-referencing functions
- **WHEN** they are lowered in both definition orders
- **THEN** both produce identical IR

#### Scenario: A callee in another source leaves the type undetermined

- **GIVEN** a body containing a call to a name not defined in this source
- **WHEN** it is lowered
- **THEN** lowering succeeds and the call's type is undetermined, so a binding from it still
  requires an annotation

#### Scenario: A genuinely unknown callee is still caught

- **GIVEN** a unit assembled from every source, with one call resolving to no function anywhere
- **WHEN** it is validated
- **THEN** unit validation reports the unresolved callee

#### Scenario: A call with the wrong arity is rejected

- **GIVEN** a call passing two arguments to a function taking one
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting both counts

#### Scenario: An argument of the wrong type is rejected

- **GIVEN** a call passing a string where the signature declares an integer
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

#### Scenario: Promotion applies to arguments

- **GIVEN** a call passing an integer where the signature declares a float
- **WHEN** it is lowered
- **THEN** lowering succeeds and the argument carries an explicit conversion

#### Scenario: A self-recursive function types

- **GIVEN** a function whose body calls itself
- **WHEN** it is lowered
- **THEN** lowering succeeds, since its own signature is in the table

#### Scenario: Signatures may be supplied from outside the source

- **GIVEN** a source and signatures gathered from other sources
- **WHEN** it is lowered together with them
- **THEN** a call to one of those functions is typed from its signature, exactly as a call within
  the source would be

#### Scenario: A source's own definitions take precedence

- **GIVEN** a source defining a function whose name also appears in the supplied signatures
- **WHEN** it is lowered
- **THEN** the source's own definition is used, so a source is always typed against what it
  actually contains

#### Scenario: Cross-source calls still resolve at the unit

- **GIVEN** two functions in separate sources calling each other
- **WHEN** both are lowered and added to one unit
- **THEN** lowering each source succeeds only if resolution is deferred, and unit validation
  resolves the call across sources

### Requirement: Reject a function that cannot return its declared type

Lowering SHALL reject a function whose declared return type is not the unit type and whose body
cannot produce a value on **every path**. The diagnostic SHALL name the function and report its
location.

With branching, this is no longer the structural question of whether the last statement is a
`return`. A body returns when its final statement returns; a conditional returns only when it has
an alternative **and both branches return**; and a loop SHALL NOT be assumed to return, because its
body may never run. Treating a loop as returning would let a program through whose generated code
does not compile, and the resulting complaint would be about Rust rather than about the user's
function.

This is a program the user wrote incorrectly, so it belongs with every other subset violation. Left
to a backend, it surfaces as an internal code-generation error with no source location, which
describes the compiler's difficulty rather than the user's mistake.

#### Scenario: A body of only pass is rejected

- **GIVEN** `def f() -> int:` whose body is `pass`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming `f` and reporting its location

#### Scenario: A body ending in a binding is rejected

- **GIVEN** a function declaring an integer return whose body binds a local and stops
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming the function

#### Scenario: A conditional returning on both branches is accepted

- **GIVEN** a function whose body is an `if`/`else` where both branches return
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A conditional with no alternative does not return

- **GIVEN** a function whose only `return` is inside an `if` with no `else`
- **WHEN** it is lowered
- **THEN** lowering fails, because the path where the test is false produces no value

#### Scenario: One branch returning is not enough

- **GIVEN** a function whose `if` returns but whose `else` does not
- **WHEN** it is lowered
- **THEN** lowering fails

#### Scenario: A return after a conditional covers it

- **GIVEN** a function with an `if` that returns, followed by a `return`
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A loop is not assumed to run

- **GIVEN** a function whose only `return` is inside a `while`
- **WHEN** it is lowered
- **THEN** lowering fails, because the loop body may never execute

#### Scenario: Nested conditionals are analysed through

- **GIVEN** a function whose branches each contain further conditionals that all return
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A unit-returning function needs no return

- **GIVEN** `def f() -> None:` whose body is `pass`
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A function that does return is unaffected

- **GIVEN** a function whose body ends in a `return`
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: The diagnostic distinguishes this from a type mismatch

- **GIVEN** a function that cannot return its declared type
- **WHEN** it is lowered
- **THEN** the diagnostic reports a missing return rather than a mismatch between two types

### Requirement: Collection literals unify their element types

Lowering SHALL determine a collection literal's type from its elements. Every element of a
sequence or set literal SHALL have the same type, and every key and every value of a mapping
literal SHALL likewise agree. A literal whose elements disagree SHALL be rejected reporting both
types, rather than being widened to a union — the IR has no union type, and inventing one here
would put a decision in the compiler that the user should be making in the annotation.

A tuple literal SHALL take a type per position, so its elements need not agree.

An **empty** literal has no elements to infer from, so its type is undetermined and it SHALL
require an annotation, on the same terms as any other undetermined initializer.

#### Scenario: A sequence literal infers its element type

- **GIVEN** a body containing `xs = [1, 2, 3]`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `xs` is a sequence of the integer type

#### Scenario: A mapping literal infers its key and value types

- **GIVEN** a body containing `d = {"a": 1}`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `d` is a mapping from the string type to the integer type

#### Scenario: A set literal infers its element type

- **GIVEN** a body containing `s = {1, 2}`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `s` is a set of the integer type

#### Scenario: A tuple literal takes a type per position

- **GIVEN** a body containing `t = (1, "a")`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `t` is a two-element tuple of the integer and string types

#### Scenario: Mismatched sequence elements are rejected

- **GIVEN** a body containing `xs = [1, "a"]`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting both element types

#### Scenario: Mismatched mapping values are rejected

- **GIVEN** a body containing `d = {"a": 1, "b": "x"}`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting both value types

#### Scenario: Numeric promotion applies within a literal

- **GIVEN** a sequence literal mixing integer and floating-point elements
- **WHEN** it is lowered
- **THEN** the literal is a sequence of the floating-point type, and each integer element carries
  an explicit conversion, matching promotion elsewhere in the subset

#### Scenario: An empty literal requires an annotation

- **GIVEN** a body containing `xs = []`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming `xs` as requiring an explicit type annotation

#### Scenario: An annotated empty literal is accepted

- **GIVEN** a body containing `xs: list[int] = []`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `xs` is a sequence of the integer type

#### Scenario: A literal conflicting with its annotation is rejected

- **GIVEN** a body containing `xs: list[str] = [1, 2]`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

### Requirement: Subscript typing

Lowering SHALL type a subscript from the type of the expression being subscripted. Subscripting a
sequence with an integer SHALL yield its element type; subscripting a mapping with its key type
SHALL yield its value type; and subscripting a tuple SHALL yield the type at that position.

A tuple index SHALL be a non-negative integer literal within range. Each position of a tuple has
its own type, so a computed index has no single answer that lowering could give.

Subscripting a set, or a scalar, SHALL be rejected. Slicing SHALL be rejected.

#### Scenario: Sequence subscript yields the element type

- **GIVEN** the expression `xs[0]`, where `xs` is a sequence of integers
- **WHEN** it is lowered
- **THEN** the expression's type is the integer type

#### Scenario: Mapping subscript yields the value type

- **GIVEN** the expression `d[k]`, where `d` maps strings to integers and `k` is a string
- **WHEN** it is lowered
- **THEN** the expression's type is the integer type

#### Scenario: A mapping subscript with the wrong key type is rejected

- **GIVEN** the expression `d[1]`, where `d` maps strings to integers
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the key type

#### Scenario: A sequence subscript with a non-integer index is rejected

- **GIVEN** the expression `xs["a"]`, where `xs` is a sequence
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the index type

#### Scenario: Tuple subscript yields the type at that position

- **GIVEN** the expression `t[1]`, where `t` is a tuple of an integer and a string
- **WHEN** it is lowered
- **THEN** the expression's type is the string type

#### Scenario: A computed tuple index is rejected

- **GIVEN** the expression `t[i]`, where `i` is an integer variable
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic explaining that a tuple index must be a literal

#### Scenario: An out-of-range tuple index is rejected

- **GIVEN** the expression `t[5]`, where `t` has two positions
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the index and the tuple's length

#### Scenario: Subscripting a set is rejected

- **GIVEN** the expression `s[0]`, where `s` is a set
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that sets are not subscriptable

#### Scenario: Slicing is rejected

- **GIVEN** the expression `xs[1:3]`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming slicing as unsupported

#### Scenario: A subscript composes

- **GIVEN** the expression `d["a"][0]`, where `d` maps strings to sequences of integers
- **WHEN** it is lowered
- **THEN** the expression's type is the integer type

### Requirement: Length of a collection or string

Lowering SHALL recognise `len` applied to a sequence, mapping, set, tuple, or string and type it
as an integer. Applying `len` to a scalar other than a string SHALL be rejected.

`len` SHALL be reserved: a function in the unit named `len` SHALL be rejected. Without that, whether
`len(x)` meant the builtin or a user's function would depend on what else had been marked for
compilation, which is exactly the order-dependence the unit's design exists to avoid.

#### Scenario: Length of a sequence

- **GIVEN** the expression `len(xs)`, where `xs` is a sequence
- **WHEN** it is lowered
- **THEN** the expression's type is the integer type

#### Scenario: Length of a mapping, set, and tuple

- **GIVEN** `len` applied to a mapping, a set, and a tuple
- **WHEN** each is lowered
- **THEN** each expression's type is the integer type

#### Scenario: Length of a string

- **GIVEN** the expression `len(s)`, where `s` is a string
- **WHEN** it is lowered
- **THEN** the expression's type is the integer type

#### Scenario: Length of a number is rejected

- **GIVEN** the expression `len(n)`, where `n` is an integer
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the operand type

#### Scenario: Length takes exactly one argument

- **GIVEN** the expression `len(a, b)`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting the argument count

#### Scenario: A user function named len is rejected

- **GIVEN** a source defining `def len(x: int) -> int:`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that `len` is reserved

#### Scenario: Length is not treated as a call to be resolved

- **GIVEN** a unit containing `len(xs)` and no function named `len`
- **WHEN** it is validated
- **THEN** validation succeeds, because `len` is a builtin rather than an unresolved callee

### Requirement: Conditionals

Lowering SHALL accept `if`, `elif`, and `else`. The test SHALL be a boolean; any other type SHALL
be rejected reporting the type found.

Python treats many values as truthy, but compylr does not: a subset whose annotations are mandatory
should not then infer that an integer means a condition. Requiring a boolean keeps the meaning of a
test written down rather than inferred.

#### Scenario: A conditional lowers

- **GIVEN** a body containing `if a < b:` with a returning branch
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: An alternative lowers

- **GIVEN** a body containing `if`/`else`
- **WHEN** it is lowered
- **THEN** both branches appear in the IR

#### Scenario: elif lowers as a nested conditional

- **GIVEN** a body containing `if`/`elif`/`else`
- **WHEN** it is lowered
- **THEN** the IR nests the second conditional inside the first one's alternative

#### Scenario: A non-boolean test is rejected

- **GIVEN** the statement `if n:`, where `n` is an integer
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic reporting that a test must be a boolean

#### Scenario: A branch is a scope for reachability but not for names

- **GIVEN** a name bound inside a branch and read after the conditional
- **WHEN** it is lowered
- **THEN** lowering rejects the read, because the binding may not have happened

### Requirement: Loops

Lowering SHALL accept `while` with a boolean test, and `for` binding one name over a range or a
supported collection. It SHALL accept `break` and `continue` inside a loop body and reject them
outside one.

Iterating a sequence SHALL bind its element type; a mapping SHALL bind its **key** type, matching
Python; a set SHALL bind its element type; and a range SHALL bind an integer.

#### Scenario: A while loop lowers

- **GIVEN** a body containing `while a < b:`
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A non-boolean while test is rejected

- **GIVEN** the statement `while n:`, where `n` is an integer
- **WHEN** it is lowered
- **THEN** lowering fails reporting that a test must be a boolean

#### Scenario: Iterating a range binds an integer

- **GIVEN** the loop `for i in range(n):`
- **WHEN** it is lowered
- **THEN** `i` is bound with the integer type

#### Scenario: Iterating a sequence binds its element type

- **GIVEN** the loop `for x in xs:`, where `xs` is a sequence of strings
- **WHEN** it is lowered
- **THEN** `x` is bound with the string type

#### Scenario: Iterating a mapping binds its key type

- **GIVEN** the loop `for k in d:`, where `d` maps strings to integers
- **WHEN** it is lowered
- **THEN** `k` is bound with the string type, matching Python

#### Scenario: Iterating a scalar is rejected

- **GIVEN** the loop `for x in n:`, where `n` is an integer
- **WHEN** it is lowered
- **THEN** lowering fails reporting the type

#### Scenario: The loop variable does not escape

- **GIVEN** a name bound by a `for` and read after the loop
- **WHEN** it is lowered
- **THEN** lowering rejects the read

#### Scenario: Loop control inside a loop

- **GIVEN** a loop body containing `break` and `continue`
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: Loop control outside a loop is rejected

- **GIVEN** a `break` in a function body with no enclosing loop
- **WHEN** it is lowered
- **THEN** lowering fails reporting that it is not inside a loop

#### Scenario: Loop control reaches the nearest enclosing loop

- **GIVEN** a `break` inside a conditional inside a loop
- **WHEN** it is lowered
- **THEN** lowering succeeds

### Requirement: Reassignment keeps a name's type

Lowering SHALL accept assigning to a name already bound in the same function. The name's type is
fixed where it was first bound: a value of a different type SHALL be rejected, with promotion
applying as it does elsewhere.

Rebinding is not re-declaration. Allowing a name to change type would mean the same identifier
denotes different things at different points, which a reader has to simulate the program to follow,
and which every backend would then have to model.

#### Scenario: Reassignment lowers

- **GIVEN** a body binding `i = 0` and then `i = i + 1`
- **WHEN** it is lowered
- **THEN** lowering succeeds and `i` keeps the integer type

#### Scenario: A different type is rejected

- **GIVEN** a body binding `i = 0` and then `i = "x"`
- **WHEN** it is lowered
- **THEN** lowering fails reporting both types

#### Scenario: Promotion applies to a reassignment

- **GIVEN** a body binding `x: float = 1.0` and then `x = 2`
- **WHEN** it is lowered
- **THEN** lowering succeeds and the integer carries an explicit conversion

#### Scenario: An annotation on a rebinding is rejected

- **GIVEN** a body binding `i = 0` and then `i: int = 1`
- **WHEN** it is lowered
- **THEN** lowering fails, because the second annotation re-declares a name that already exists

#### Scenario: A parameter may be reassigned

- **GIVEN** a body assigning to one of its own parameters
- **WHEN** it is lowered
- **THEN** lowering succeeds and the parameter keeps its declared type

#### Scenario: Reassignment inside a loop is the point

- **GIVEN** a `while` whose body increments a counter bound before it
- **WHEN** it is lowered
- **THEN** lowering succeeds

### Requirement: range is reserved

Lowering SHALL recognise `range` with one, two, or three integer arguments and reject any other
arity or argument type. A function in the unit named `range` SHALL be rejected, on the same terms
as `len`: a builtin whose meaning depended on what else had been compiled would be worse than no
builtin at all.

#### Scenario: One argument

- **GIVEN** the expression `range(n)`
- **WHEN** it is lowered
- **THEN** the IR carries a start of zero, a stop of `n`, and a step of one

#### Scenario: Two and three arguments

- **GIVEN** the expressions `range(a, b)` and `range(a, b, c)`
- **WHEN** each is lowered
- **THEN** each component is carried as written

#### Scenario: A non-integer argument is rejected

- **GIVEN** the expression `range(x)`, where `x` is a string
- **WHEN** it is lowered
- **THEN** lowering fails reporting the type

#### Scenario: range with the wrong arity is rejected

- **GIVEN** the expression `range()`, or a call with four arguments
- **WHEN** it is lowered
- **THEN** lowering fails reporting the argument count

#### Scenario: A user function named range is rejected

- **GIVEN** a source defining `def range(n: int) -> int:`
- **WHEN** it is lowered
- **THEN** lowering fails reporting that `range` is reserved

#### Scenario: A range outside a loop is rejected

- **GIVEN** a binding whose initializer is a bare `range(n)`
- **WHEN** it is lowered
- **THEN** lowering fails, because a range is only meaningful as something to iterate

### Requirement: Element assignment

Lowering SHALL accept assigning to an element of a sequence or a mapping. The index SHALL be an
integer for a sequence and the key type for a mapping, and the value SHALL match the element or
value type, with promotion applying as elsewhere. Assigning to an element of a set, a tuple, or a
scalar SHALL be rejected.

#### Scenario: Sequence element assignment

- **GIVEN** the statement `xs[0] = 1`, where `xs` is a local sequence of integers
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: Mapping element assignment

- **GIVEN** the statement `d["a"] = 1`, where `d` is a local mapping from strings to integers
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A wrong value type is rejected

- **GIVEN** the statement `xs[0] = "a"`, where `xs` holds integers
- **WHEN** it is lowered
- **THEN** lowering fails reporting both types

#### Scenario: A wrong index type is rejected

- **GIVEN** the statement `xs["a"] = 1`, where `xs` is a sequence
- **WHEN** it is lowered
- **THEN** lowering fails reporting the index type

#### Scenario: Promotion applies to an assigned element

- **GIVEN** the statement `xs[0] = 1`, where `xs` holds floats
- **WHEN** it is lowered
- **THEN** lowering succeeds and the value carries an explicit conversion

#### Scenario: A tuple is immutable

- **GIVEN** an assignment to a tuple element
- **WHEN** it is lowered
- **THEN** lowering fails, matching Python, where tuples cannot be assigned into

#### Scenario: A set has no elements to assign

- **GIVEN** an assignment to a set element
- **WHEN** it is lowered
- **THEN** lowering fails

### Requirement: Mutation is confined to locals

Lowering SHALL reject mutating a collection that arrived as a **parameter**, whether by element
assignment or by appending, and SHALL reject mutating any local that **aliases** one. A local
aliases a parameter when it is bound directly to that parameter, or to another local that aliases
it; the relation is transitive. The diagnostic SHALL explain that a collection parameter is a copy,
so the mutation could not be observed by the caller, and where an alias is involved SHALL name both
the local and the parameter it came from.

Collections cross the boundary by value. A compiled function mutating a parameter would leave its
caller's collection unchanged, where an interpreted function would have modified it — a wrong
answer with no error.

Aliasing is the same hazard at one remove. In Python, binding a name to a collection does not copy
it, so `copied = xs` leaves both names denoting one object and mutating either is observable to the
caller. Under compylr's value semantics the bind is a copy, so the caller sees nothing. Permitting
it because "the local is the function's own value" is true of the emitted code and false of the
Python it claims to translate.

A collection built locally and returned is unaffected, which is the shape mutation exists to
enable. Copying a parameter's contents explicitly — building a fresh collection and filling it — is
also unaffected, and is the workaround the diagnostic points at.

#### Scenario: A local collection may be mutated

- **GIVEN** a body binding an empty sequence, appending to it, and returning it
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A parameter may not be mutated

- **GIVEN** a body appending to one of its sequence parameters
- **WHEN** it is lowered
- **THEN** lowering fails, explaining that the parameter is a copy and the caller would not see it

#### Scenario: Assigning into a parameter is rejected

- **GIVEN** a body assigning to an element of a mapping parameter
- **WHEN** it is lowered
- **THEN** lowering fails

#### Scenario: Reading a parameter is unaffected

- **GIVEN** a body reading elements of a parameter without mutating it
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A local aliasing a parameter may not be mutated

- **GIVEN** a body binding a local to a collection parameter and then mutating the local
- **WHEN** it is lowered
- **THEN** lowering fails, because in Python the local and the parameter denote one object and the
  caller would have observed the mutation

#### Scenario: The diagnostic names the alias and its origin

- **GIVEN** a body mutating a local that aliases a parameter
- **WHEN** it is lowered
- **THEN** the diagnostic names both the local and the parameter it came from, because a refusal
  pointing only at a local the user just created gives them no reason to look at the signature

#### Scenario: Aliasing is transitive

- **GIVEN** a body binding one local to a parameter, a second to the first, and mutating the
  second
- **WHEN** it is lowered
- **THEN** lowering fails, because otherwise one more binding defeats the rule

#### Scenario: Copying a parameter's contents explicitly may be mutated

- **GIVEN** a body building a fresh collection, filling it from a parameter, and mutating it
- **WHEN** it is lowered
- **THEN** lowering succeeds, because the fresh collection is not the parameter under any semantics

#### Scenario: Aliasing a non-collection is unaffected

- **GIVEN** a body binding a local to a scalar parameter
- **WHEN** it is lowered
- **THEN** lowering succeeds and nothing about it is restricted, because a scalar has no mutation
  to observe

#### Scenario: A local that stops aliasing may be mutated

- **GIVEN** a body binding a local to a parameter, rebinding it to a fresh collection, then
  mutating it
- **WHEN** it is lowered
- **THEN** lowering succeeds, because after the rebinding the local no longer denotes the caller's
  collection

### Requirement: Append

Lowering SHALL accept `append` on a local sequence, with one argument whose type matches the
element type. Any other method SHALL remain rejected, and the diagnostic SHALL name the method.

#### Scenario: Appending lowers

- **GIVEN** the statement `xs.append(1)`, where `xs` is a local sequence of integers
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A wrong element type is rejected

- **GIVEN** the statement `xs.append("a")`, where `xs` holds integers
- **WHEN** it is lowered
- **THEN** lowering fails reporting both types

#### Scenario: append with the wrong arity is rejected

- **GIVEN** the statement `xs.append()`, or `xs.append(1, 2)`
- **WHEN** it is lowered
- **THEN** lowering fails reporting the argument count

#### Scenario: Appending to a non-sequence is rejected

- **GIVEN** the statement `d.append(1)`, where `d` is a mapping
- **WHEN** it is lowered
- **THEN** lowering fails reporting the type

#### Scenario: Another method is rejected by name

- **GIVEN** the statement `xs.pop()`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming `pop` as unsupported

### Requirement: Membership

Lowering SHALL accept `in` and `not in` over a sequence, mapping, set, or string, yielding a
boolean. Membership in a mapping SHALL test its **keys**, matching Python. The value's type SHALL
match what the container holds — its element type, its key type, or a string for a string.

#### Scenario: Membership yields a boolean

- **GIVEN** the expression `x in xs`, where `xs` is a sequence of integers and `x` an integer
- **WHEN** it is lowered
- **THEN** the expression's type is boolean

#### Scenario: Mapping membership tests keys

- **GIVEN** the expression `k in d`, where `d` maps strings to integers
- **WHEN** it is lowered
- **THEN** `k` must be a string, matching Python

#### Scenario: Negated membership

- **GIVEN** the expression `x not in xs`
- **WHEN** it is lowered
- **THEN** the expression's type is boolean

#### Scenario: A mismatched value type is rejected

- **GIVEN** the expression `"a" in xs`, where `xs` holds integers
- **WHEN** it is lowered
- **THEN** lowering fails reporting both types

#### Scenario: Membership in a scalar is rejected

- **GIVEN** the expression `x in n`, where `n` is an integer
- **WHEN** it is lowered
- **THEN** lowering fails reporting the type

#### Scenario: Membership in a string tests substrings

- **GIVEN** the expression `a in s`, where both are strings
- **WHEN** it is lowered
- **THEN** the expression's type is boolean, matching Python's substring test

### Requirement: Class definitions

Lowering SHALL accept a class definition containing an `__init__` and any number of methods, and
SHALL reject a class body containing anything else. Every method SHALL be fully annotated, on the
same terms as a free function, and SHALL take `self` as its first parameter — which SHALL NOT carry
an annotation, since its type is the class being defined.

Inheritance, decorators on methods, class-level attributes, and dunder methods other than `__init__`
SHALL be rejected, each naming what was found.

#### Scenario: A class lowers

- **GIVEN** a class with an `__init__` and one method
- **WHEN** it is lowered
- **THEN** lowering succeeds and the unit contains the class

#### Scenario: A class without __init__ is rejected

- **GIVEN** a class with no `__init__`
- **WHEN** it is lowered
- **THEN** lowering fails, because a class's attributes are declared there and nowhere else

#### Scenario: A method must take self

- **GIVEN** a method whose first parameter is not `self`
- **WHEN** it is lowered
- **THEN** lowering fails naming the method

#### Scenario: self must not be annotated

- **GIVEN** a method annotating `self`
- **WHEN** it is lowered
- **THEN** lowering fails, because its type is the class being defined

#### Scenario: Method parameters and returns are mandatory

- **GIVEN** a method missing a return annotation
- **WHEN** it is lowered
- **THEN** lowering fails naming the method

#### Scenario: Inheritance is rejected

- **GIVEN** a class declaring a base
- **WHEN** it is lowered
- **THEN** lowering fails naming inheritance as unsupported

#### Scenario: A class-level statement is rejected

- **GIVEN** a class whose body contains a statement other than a method definition
- **WHEN** it is lowered
- **THEN** lowering fails naming the construct

#### Scenario: A dunder other than __init__ is rejected

- **GIVEN** a class defining `__eq__`
- **WHEN** it is lowered
- **THEN** lowering fails naming the method

#### Scenario: Two methods of the same name are rejected

- **GIVEN** a class defining the same method twice
- **WHEN** it is lowered
- **THEN** lowering fails reporting the conflict

### Requirement: Attributes are declared in __init__

Every attribute SHALL be declared by an annotated assignment to `self` in `__init__`. An assignment
to an attribute that was not declared there SHALL be rejected, and so SHALL a declaration outside
`__init__`.

Python allows an attribute to appear anywhere, which means an object's shape depends on which
methods have run. A compiled struct's fields cannot depend on that, and requiring the declaration up
front is the same rule the subset already applies to parameters and returns.

#### Scenario: An attribute is declared and typed

- **GIVEN** an `__init__` containing `self.count: int = 0`
- **WHEN** it is lowered
- **THEN** the class carries an attribute `count` of the integer type

#### Scenario: An undeclared attribute is rejected

- **GIVEN** a method assigning to an attribute not declared in `__init__`
- **WHEN** it is lowered
- **THEN** lowering fails naming the attribute

#### Scenario: An unannotated declaration is rejected

- **GIVEN** an `__init__` containing `self.count = 0`
- **WHEN** it is lowered
- **THEN** lowering fails, because an attribute's type must be written down

#### Scenario: A declaration outside __init__ is rejected

- **GIVEN** a method containing an annotated assignment to a new attribute
- **WHEN** it is lowered
- **THEN** lowering fails

#### Scenario: An attribute may hold a collection

- **GIVEN** an `__init__` containing `self._cache: dict[int, int] = {}`
- **WHEN** it is lowered
- **THEN** the class carries an attribute of that mapping type

#### Scenario: Every declared attribute must be initialised

- **GIVEN** an `__init__` declaring an attribute without a value
- **WHEN** it is lowered
- **THEN** lowering fails, because a struct cannot be constructed with a field missing

### Requirement: Attribute access and assignment

Lowering SHALL type an attribute read from the class of the object being read, and SHALL check an
attribute assignment against the declared type, with promotion applying as elsewhere. Reading or
assigning an attribute the class does not declare SHALL be rejected naming it.

Attributes SHALL be mutable: assigning to `self.x` inside a method is permitted, which is what makes
state that outlives a call possible.

#### Scenario: An attribute read is typed

- **GIVEN** the expression `self.count`, where `count` is an integer attribute
- **WHEN** it is lowered
- **THEN** the expression's type is the integer type

#### Scenario: An attribute is assigned

- **GIVEN** the statement `self.count = 1`
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A wrong type is rejected

- **GIVEN** the statement `self.count = "x"`, where `count` is an integer
- **WHEN** it is lowered
- **THEN** lowering fails reporting both types

#### Scenario: An unknown attribute is rejected

- **GIVEN** the expression `self.missing`
- **WHEN** it is lowered
- **THEN** lowering fails naming the attribute and the class

#### Scenario: An attribute is read from another object

- **GIVEN** the expression `obj.count`, where `obj` is an instance parameter
- **WHEN** it is lowered
- **THEN** the expression's type is the attribute's type

#### Scenario: A collection attribute may be mutated

- **GIVEN** a method assigning into a mapping attribute
- **WHEN** it is lowered
- **THEN** lowering succeeds, unlike the same operation on a collection parameter

### Requirement: Methods and construction

Lowering SHALL type a method call from the method's signature, checking arity and argument types
with promotion, and SHALL type a construction as the class's instance type, checking its arguments
against `__init__`.

Methods and classes SHALL be resolvable across sources on the same terms as functions: a class the
current source does not define leaves a construction's type undetermined rather than failing, and
unit validation catches one that exists nowhere.

#### Scenario: A method call is typed

- **GIVEN** the expression `obj.value()`, where `value` returns an integer
- **WHEN** it is lowered
- **THEN** the expression's type is the integer type

#### Scenario: Construction is typed

- **GIVEN** the expression `Counter()`, where `Counter` is a class in the source
- **WHEN** it is lowered
- **THEN** the expression's type is that class's instance type

#### Scenario: Constructor arguments are checked

- **GIVEN** a construction whose arguments do not match `__init__`
- **WHEN** it is lowered
- **THEN** lowering fails reporting the mismatch

#### Scenario: Method arity is checked

- **GIVEN** a method call with the wrong number of arguments
- **WHEN** it is lowered
- **THEN** lowering fails reporting both counts

#### Scenario: An unknown method is rejected

- **GIVEN** a call to a method the class does not define
- **WHEN** it is lowered
- **THEN** lowering fails naming the method and the class

#### Scenario: A method may call another on the same object

- **GIVEN** a method whose body calls `self.other()`
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: A class in another source leaves construction undetermined

- **GIVEN** a construction of a class this source does not define
- **WHEN** it is lowered
- **THEN** lowering succeeds with an undetermined type, and unit validation resolves it

### Requirement: Lowering takes a resolved behavior and sets every mode from it

Lowering SHALL accept a resolved behavior and SHALL set every declared mode on every node it
produces from that behavior. No mode SHALL be set from a constant belonging to one language, and no
node SHALL be left to acquire a mode later.

Lowering SHALL be a pure function of the parsed source and the resolved behavior together: lowering
the same source twice under the same behavior SHALL produce identical IR, and under two different
behaviors SHALL produce IR that differs in exactly the modes the two behaviors differ on.

#### Scenario: Every mode comes from the behavior

- **GIVEN** a source containing division, remainder, subscripting, length, and arithmetic
- **WHEN** it is lowered under a resolved behavior
- **THEN** each resulting node's declared modes match what the resolved behavior says for that axis

#### Scenario: Two behaviors differ only where the behaviors differ

- **GIVEN** one source and two behaviors differing on one axis
- **WHEN** it is lowered under each
- **THEN** the two units differ only in the modes that axis governs, and are otherwise identical

#### Scenario: A behavior is required

- **GIVEN** a source about to be lowered
- **WHEN** lowering is invoked
- **THEN** a resolved behavior is supplied, and there is no lowering path that supplies its own

### Requirement: Behavior does not change what source is accepted

The set of Python programs lowering accepts SHALL NOT depend on the resolved behavior. A behavior
selects what an accepted operation *means*; it SHALL NOT make a rejected program acceptable or an
acceptable program rejected.

Type rules SHALL likewise be unaffected. In particular, `/` SHALL yield a float under every
behavior, so that the same annotated source type-checks identically whichever behavior compiles it;
what the behavior selects for `/` is what happens when the divisor is zero, not what type the
result has.

#### Scenario: Acceptance is behavior-independent

- **GIVEN** every accepted fixture and every behavior
- **WHEN** each is lowered under each
- **THEN** all of them lower successfully under all of them

#### Scenario: Rejection is behavior-independent

- **GIVEN** every rejected fixture and every behavior
- **WHEN** each is lowered under each
- **THEN** all of them are rejected under all of them, with the same diagnostic code

#### Scenario: Division's result type does not move

- **GIVEN** the expression `a / b` with integer operands
- **WHEN** it is lowered under a behavior selecting the target's stance on exact division
- **THEN** the result is still typed as a float, and the operands are still promoted

#### Scenario: A negative index is not rejected statically

- **GIVEN** the expression `xs[-1]`
- **WHEN** it is lowered under a behavior in which a negative index is out of range
- **THEN** lowering succeeds, because the index is a runtime value and refusing a literal one would
  reject only the cases that are visible

### Requirement: Borrowed instance parameters do not escape

A direct instance parameter of a top-level free function SHALL be borrow-only for the duration of
the call. Lowering SHALL permit it to be read, mutated directly or through a mutating method, or
passed as an argument to another direct instance parameter whose use is likewise borrow-compatible.
Lowering SHALL reject any use that would require owning, copying, moving, or storing that borrowed
instance.

Rejected ownership uses SHALL include returning the parameter, binding or assigning it into another
storage slot, placing it in a collection or another instance's attribute, rebinding the parameter
name to an owned instance, or passing it to a position that consumes an owned value. The rejection
SHALL carry a stable diagnostic category and the source location of the consuming use, and SHALL
occur before target source is emitted. Lowering SHALL follow direct aliases when necessary so an
escape cannot be hidden behind another local name.

The borrow SHALL extend to instances reached *through* the parameter. An attribute of a borrowed
instance whose declared type is a class, and an element of a collection held in one, SHALL be
rejected in the same ownership positions and with the same category, because the caller still holds
the container and would otherwise receive a detached copy of an instance CPython would return by
identity. Reading such an instance, and passing it to a position that borrows it, SHALL remain
permitted.

A free function with an instance return type SHALL remain valid when its result is newly owned by
the call, including an instance constructed in the function or returned by another function that
produces an owned instance. The compiler SHALL NOT make a borrowed parameter satisfy such a return
by cloning it.

#### Scenario: A borrowed parameter may be read

- **GIVEN** a free function reading an attribute of a direct instance parameter
- **WHEN** it is lowered
- **THEN** lowering succeeds without transferring ownership of the parameter

#### Scenario: A borrowed parameter may be mutated

- **GIVEN** a free function assigning an attribute or invoking a mutating method on a direct
  instance parameter
- **WHEN** it is lowered
- **THEN** lowering succeeds and the use remains a borrow of the caller's instance

#### Scenario: A borrowed parameter may be forwarded compatibly

- **GIVEN** a free function passing its direct instance parameter to another whose parameter is
  compatible
- **WHEN** it is lowered
- **THEN** lowering succeeds without cloning or moving the instance

#### Scenario: Returning a borrowed parameter is rejected

- **GIVEN** the function `def identity(t: Tally) -> Tally: return t`
- **WHEN** it is lowered
- **THEN** lowering fails at the returned `t` with the borrowed-instance-escape category before
  target source is emitted

#### Scenario: An alias cannot hide a borrowed return

- **GIVEN** a function binding `same = t` from a direct instance parameter and later returning
  `same`
- **WHEN** it is lowered
- **THEN** lowering fails with a located borrowed-instance-escape diagnostic rather than cloning
  the instance

#### Scenario: An instance reached through a borrow cannot be consumed

- **GIVEN** a function returning `holder.item` or `holder.items[0]`, where `holder` is a direct
  instance parameter
- **WHEN** it is lowered
- **THEN** lowering fails at that expression with the borrowed-instance-escape category, rather
  than emitting a clone whose later mutation the caller never observes

#### Scenario: An instance reached through a borrow may still be read and forwarded

- **GIVEN** a function reading `holder.item.value`, or passing `holder.item` to a function that
  borrows it
- **WHEN** it is lowered
- **THEN** lowering succeeds and the instance is borrowed rather than copied

#### Scenario: Storing a borrowed parameter is rejected

- **GIVEN** a function placing a direct instance parameter in a collection or another instance's
  attribute
- **WHEN** it is lowered
- **THEN** lowering fails at the storing use before target source is emitted

#### Scenario: Rebinding a borrowed parameter is rejected

- **GIVEN** a function assigning a newly constructed instance to the name of a direct instance
  parameter
- **WHEN** it is lowered
- **THEN** lowering fails at the assignment because that parameter binding is borrow-only

#### Scenario: A newly constructed return is owned

- **GIVEN** a function annotated `-> Tally` returning `Tally(start)`
- **WHEN** it is lowered
- **THEN** lowering succeeds because the returned instance is newly owned by the call

#### Scenario: An owned callee result may be returned

- **GIVEN** a function annotated `-> Tally` returning the result of another function that
  produces a new one
- **WHEN** it is lowered
- **THEN** lowering succeeds
