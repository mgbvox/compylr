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

#### Scenario: Alias of a previously bound local is inferred

- **WHEN** lowering a body that binds `x: str = "hi"` and then contains `y = x`
- **THEN** lowering succeeds and the IR binds `y` with the IR string type

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

#### Scenario: A call initializer is inferred

- **WHEN** lowering a source defining `def double(n: int) -> int:` and a second function whose
  body contains `b = double(n)`
- **THEN** lowering succeeds and `b` is bound with the IR integer type, reversing the previous
  rule that required an annotation here

#### Scenario: A call nested inside an expression is inferred

- **WHEN** lowering a body containing `b = double(n) + 1`
- **THEN** lowering succeeds and `b` is bound with the callee's return type combined by the
  operator rules

#### Scenario: Reference to an unbound name is unresolved

- **WHEN** lowering a body containing `b = q` where `q` is neither a parameter nor a
  previously bound local
- **THEN** lowering fails with a diagnostic reporting `q` as unresolved, not as a missing
  annotation

#### Scenario: Explicit annotation still wins over inference

- **WHEN** lowering a body containing `b: int = a` where `a` is an integer parameter
- **THEN** lowering succeeds using the declared type

### Requirement: Reject unsupported type annotations

Lowering SHALL reject any annotation outside the supported set, and the diagnostic SHALL
report the annotation as written in the source. The supported set is `int`, `float`, `bool`, and
`str`; the parameterised forms `list[T]`, `dict[K, V]`, `set[T]`, and `tuple[T, ...]`, nested to
any depth; plus `None` as a return annotation only.

A parameterised annotation SHALL be rejected when its parameters are missing, when their number
does not match the form, or when a parameter is itself unsupported. A bare `list`, `dict`, `set`,
or `tuple` without parameters SHALL be rejected, because an element type that is not written down
is not a type compylr can compile against.

#### Scenario: Floating-point annotation is accepted

- **WHEN** lowering a function whose parameter is annotated `float`
- **THEN** lowering succeeds and the parameter has the IR floating-point type

#### Scenario: Collection annotations are accepted

- **WHEN** lowering a function whose parameters are annotated `list[int]`, `dict[str, int]`,
  `set[int]`, and `tuple[int, str]`
- **THEN** lowering succeeds and each parameter has the corresponding IR collection type

#### Scenario: Nested collection annotations are accepted

- **WHEN** lowering a function whose parameter is annotated `dict[str, list[int]]`
- **THEN** lowering succeeds and the nesting is preserved in the IR type

#### Scenario: An unparameterised collection annotation is rejected

- **WHEN** lowering a function whose parameter is annotated bare `list`
- **THEN** lowering fails with a diagnostic reporting that an element type is required

#### Scenario: Wrong parameter count is rejected

- **WHEN** lowering a function whose parameter is annotated `dict[str]`
- **THEN** lowering fails with a diagnostic reporting the annotation as unsupported

#### Scenario: An unsupported parameter is rejected

- **WHEN** lowering a function whose parameter is annotated `list[complex]`
- **THEN** lowering fails with a diagnostic reporting `complex` as an unsupported type

#### Scenario: A floating-point mapping key is rejected

- **WHEN** lowering a function whose parameter is annotated `dict[float, int]`
- **THEN** lowering fails with a diagnostic explaining that a floating-point value cannot be a key

#### Scenario: A floating-point set element is rejected

- **WHEN** lowering a function whose parameter is annotated `set[float]`
- **THEN** lowering fails with a diagnostic explaining that a floating-point value cannot be a set
  element

#### Scenario: Unsupported scalar annotation

- **WHEN** lowering a function whose parameter is annotated `complex`
- **THEN** lowering fails with a diagnostic reporting `complex` as an unsupported type

#### Scenario: Unsupported generic annotation

- **WHEN** lowering a function whose parameter is annotated `frozenset[int]`
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
definitions. The diagnostic SHALL name the unsupported construct. The single exception is a
leading docstring, defined in "Docstrings are accepted and carry no runtime meaning".

#### Scenario: Control flow is rejected

- **WHEN** lowering a function body containing an `if` statement
- **THEN** lowering fails with a diagnostic naming the conditional as unsupported

#### Scenario: Top-level statement is rejected

- **WHEN** lowering a source containing an `if __name__ == '__main__':` guard
- **THEN** lowering fails with a diagnostic reporting that only function definitions are
  permitted at top level

#### Scenario: A module-level docstring is still rejected

- **WHEN** lowering a source whose first statement is a module-level string literal
- **THEN** lowering fails, because the docstring exception applies only inside a function body

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

- **WHEN** lowering a function whose first body statement is a string literal
- **THEN** lowering succeeds

#### Scenario: The docstring does not become a statement

- **WHEN** a function with a docstring and a single `return` is lowered
- **THEN** the IR body contains only the return statement

#### Scenario: The docstring is retained on the function

- **WHEN** a function with a docstring is lowered
- **THEN** the IR function carries the docstring's text

#### Scenario: A function with only a docstring and no return

- **WHEN** lowering a function annotated `-> None` whose body is just a docstring
- **THEN** lowering succeeds and the body produces no value

#### Scenario: Editing a docstring does not change the fingerprint

- **WHEN** the same function is lowered twice with different docstring text
- **THEN** both produce the same fingerprint

#### Scenario: Adding a docstring does not change the fingerprint

- **WHEN** a function is lowered with and without a docstring, its code otherwise identical
- **THEN** both produce the same fingerprint

#### Scenario: A string statement after the first is rejected

- **WHEN** lowering a body whose second statement is a bare string literal
- **THEN** lowering fails with a diagnostic naming the unsupported statement

#### Scenario: A non-string expression statement is still rejected

- **WHEN** lowering a body whose first statement is a bare expression such as `a + 1`
- **THEN** lowering fails with a diagnostic naming the unsupported statement

#### Scenario: A bare call statement is still rejected

- **WHEN** lowering a body whose first statement is a bare call, discarding its result
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

- **WHEN** lowering a source where the first function calls the second
- **THEN** lowering succeeds and the call is typed from the second function's signature

#### Scenario: Definition order does not change the result

- **WHEN** the same two mutually-referencing functions are lowered in both definition orders
- **THEN** both produce identical IR

#### Scenario: A callee in another source leaves the type undetermined

- **WHEN** lowering a body containing a call to a name not defined in this source
- **THEN** lowering succeeds and the call's type is undetermined, so a binding from it still
  requires an annotation

#### Scenario: A genuinely unknown callee is still caught

- **WHEN** a unit is assembled from every source and one call resolves to no function anywhere
- **THEN** unit validation reports the unresolved callee

#### Scenario: Wrong arity is rejected

- **WHEN** lowering a call passing two arguments to a function taking one
- **THEN** lowering fails with a diagnostic reporting both counts

#### Scenario: An argument of the wrong type is rejected

- **WHEN** lowering a call passing a string where the signature declares an integer
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

#### Scenario: Promotion applies to arguments

- **WHEN** lowering a call passing an integer where the signature declares a float
- **THEN** lowering succeeds and the argument carries an explicit conversion

#### Scenario: A self-recursive function types

- **WHEN** lowering a function whose body calls itself
- **THEN** lowering succeeds, since its own signature is in the table

#### Scenario: Signatures may be supplied from outside the source

- **WHEN** a source is lowered together with signatures gathered from other sources
- **THEN** a call to one of those functions is typed from its signature, exactly as a call within
  the source would be

#### Scenario: A source's own definitions take precedence

- **WHEN** a source defines a function whose name also appears in the supplied signatures
- **THEN** the source's own definition is used, so a source is always typed against what it
  actually contains

#### Scenario: Cross-source calls still resolve at the unit

- **WHEN** two functions in separate sources call each other and both are added to one unit
- **THEN** lowering each source succeeds only if resolution is deferred, and unit validation
  resolves the call across sources

### Requirement: Reject a function that cannot return its declared type

Lowering SHALL reject a function whose declared return type is not the unit type and whose body
cannot produce a value — because it is empty, because it is only `pass`, or because it ends without
returning. The diagnostic SHALL name the function and report its location.

This is a program the user wrote incorrectly, so it belongs with every other subset violation. Left
to a backend, it surfaces as an internal code-generation error with no source location, which
describes the compiler's difficulty rather than the user's mistake.

#### Scenario: A body of only pass is rejected

- **WHEN** lowering `def f() -> int:` whose body is `pass`
- **THEN** lowering fails with a diagnostic naming `f` and reporting its location

#### Scenario: A body ending in a binding is rejected

- **WHEN** lowering a function declaring an integer return whose body binds a local and stops
- **THEN** lowering fails with a diagnostic naming the function

#### Scenario: A unit-returning function needs no return

- **WHEN** lowering `def f() -> None:` whose body is `pass`
- **THEN** lowering succeeds

#### Scenario: A function that does return is unaffected

- **WHEN** lowering a function whose body ends in a `return`
- **THEN** lowering succeeds

#### Scenario: The diagnostic distinguishes this from a type mismatch

- **WHEN** a function that cannot return is rejected
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

- **WHEN** lowering a body containing `xs = [1, 2, 3]`
- **THEN** lowering succeeds and `xs` is a sequence of the integer type

#### Scenario: A mapping literal infers its key and value types

- **WHEN** lowering a body containing `d = {"a": 1}`
- **THEN** lowering succeeds and `d` is a mapping from the string type to the integer type

#### Scenario: A set literal infers its element type

- **WHEN** lowering a body containing `s = {1, 2}`
- **THEN** lowering succeeds and `s` is a set of the integer type

#### Scenario: A tuple literal takes a type per position

- **WHEN** lowering a body containing `t = (1, "a")`
- **THEN** lowering succeeds and `t` is a two-element tuple of the integer and string types

#### Scenario: Mismatched sequence elements are rejected

- **WHEN** lowering a body containing `xs = [1, "a"]`
- **THEN** lowering fails with a diagnostic reporting both element types

#### Scenario: Mismatched mapping values are rejected

- **WHEN** lowering a body containing `d = {"a": 1, "b": "x"}`
- **THEN** lowering fails with a diagnostic reporting both value types

#### Scenario: Numeric promotion applies within a literal

- **WHEN** lowering a sequence literal mixing integer and floating-point elements
- **THEN** the literal is a sequence of the floating-point type, and each integer element carries
  an explicit conversion, matching promotion elsewhere in the subset

#### Scenario: An empty literal requires an annotation

- **WHEN** lowering a body containing `xs = []`
- **THEN** lowering fails with a diagnostic naming `xs` as requiring an explicit type annotation

#### Scenario: An annotated empty literal is accepted

- **WHEN** lowering a body containing `xs: list[int] = []`
- **THEN** lowering succeeds and `xs` is a sequence of the integer type

#### Scenario: A literal conflicting with its annotation is rejected

- **WHEN** lowering a body containing `xs: list[str] = [1, 2]`
- **THEN** lowering fails with a diagnostic reporting the declared and actual types

### Requirement: Subscript typing

Lowering SHALL type a subscript from the type of the expression being subscripted. Subscripting a
sequence with an integer SHALL yield its element type; subscripting a mapping with its key type
SHALL yield its value type; and subscripting a tuple SHALL yield the type at that position.

A tuple index SHALL be a non-negative integer literal within range. Each position of a tuple has
its own type, so a computed index has no single answer that lowering could give.

Subscripting a set, or a scalar, SHALL be rejected. Slicing SHALL be rejected.

#### Scenario: Sequence subscript yields the element type

- **WHEN** lowering `xs[0]` where `xs` is a sequence of integers
- **THEN** the expression's type is the integer type

#### Scenario: Mapping subscript yields the value type

- **WHEN** lowering `d[k]` where `d` maps strings to integers and `k` is a string
- **THEN** the expression's type is the integer type

#### Scenario: A mapping subscript with the wrong key type is rejected

- **WHEN** lowering `d[1]` where `d` maps strings to integers
- **THEN** lowering fails with a diagnostic reporting the key type

#### Scenario: A sequence subscript with a non-integer index is rejected

- **WHEN** lowering `xs["a"]` where `xs` is a sequence
- **THEN** lowering fails with a diagnostic reporting the index type

#### Scenario: Tuple subscript yields the type at that position

- **WHEN** lowering `t[1]` where `t` is a tuple of an integer and a string
- **THEN** the expression's type is the string type

#### Scenario: A computed tuple index is rejected

- **WHEN** lowering `t[i]` where `i` is an integer variable
- **THEN** lowering fails with a diagnostic explaining that a tuple index must be a literal

#### Scenario: An out-of-range tuple index is rejected

- **WHEN** lowering `t[5]` where `t` has two positions
- **THEN** lowering fails with a diagnostic reporting the index and the tuple's length

#### Scenario: Subscripting a set is rejected

- **WHEN** lowering `s[0]` where `s` is a set
- **THEN** lowering fails with a diagnostic reporting that sets are not subscriptable

#### Scenario: Slicing is rejected

- **WHEN** lowering `xs[1:3]`
- **THEN** lowering fails with a diagnostic naming slicing as unsupported

#### Scenario: A subscript composes

- **WHEN** lowering `d["a"][0]` where `d` maps strings to sequences of integers
- **THEN** the expression's type is the integer type

### Requirement: Length of a collection or string

Lowering SHALL recognise `len` applied to a sequence, mapping, set, tuple, or string and type it
as an integer. Applying `len` to a scalar other than a string SHALL be rejected.

`len` SHALL be reserved: a function in the unit named `len` SHALL be rejected. Without that, whether
`len(x)` meant the builtin or a user's function would depend on what else had been marked for
compilation, which is exactly the order-dependence the unit's design exists to avoid.

#### Scenario: Length of a sequence

- **WHEN** lowering `len(xs)` where `xs` is a sequence
- **THEN** the expression's type is the integer type

#### Scenario: Length of a mapping, set, and tuple

- **WHEN** lowering `len` applied to a mapping, a set, and a tuple
- **THEN** each expression's type is the integer type

#### Scenario: Length of a string

- **WHEN** lowering `len(s)` where `s` is a string
- **THEN** the expression's type is the integer type

#### Scenario: Length of a number is rejected

- **WHEN** lowering `len(n)` where `n` is an integer
- **THEN** lowering fails with a diagnostic reporting the operand type

#### Scenario: Length takes exactly one argument

- **WHEN** lowering `len(a, b)`
- **THEN** lowering fails with a diagnostic reporting the argument count

#### Scenario: A user function named len is rejected

- **WHEN** lowering a source defining `def len(x: int) -> int:`
- **THEN** lowering fails with a diagnostic reporting that `len` is reserved

#### Scenario: Length is not treated as a call to be resolved

- **WHEN** a unit containing `len(xs)` and no function named `len` is validated
- **THEN** validation succeeds, because `len` is a builtin rather than an unresolved callee
