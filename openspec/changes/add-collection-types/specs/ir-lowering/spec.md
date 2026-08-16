## MODIFIED Requirements

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

## ADDED Requirements

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
