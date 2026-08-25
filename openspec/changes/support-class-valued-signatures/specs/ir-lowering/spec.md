## MODIFIED Requirements

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

- **WHEN** lowering a function whose parameter is annotated `float`
- **THEN** lowering succeeds and the parameter has the IR floating-point type

#### Scenario: Collection annotations are accepted

- **WHEN** lowering a function whose parameters are annotated `list[int]`, `dict[str, int]`,
  `set[int]`, and `tuple[int, str]`
- **THEN** lowering succeeds and each parameter has the corresponding IR collection type

#### Scenario: Nested collection annotations are accepted

- **WHEN** lowering a function whose parameter is annotated `dict[str, list[int]]`
- **THEN** lowering succeeds and the nesting is preserved in the IR type

#### Scenario: Direct class annotations are accepted on a free function

- **WHEN** a unit contains class `Tally` and a top-level free function takes a borrow-only `Tally`
  parameter or returns a newly owned `Tally`
- **THEN** lowering succeeds and the signature carries the `Tally` instance type

#### Scenario: A class annotation resolves across sources

- **WHEN** a free function is lowered before the separate source defining its annotated class
- **THEN** the complete unit resolves the annotation regardless of source order

#### Scenario: An unknown class annotation is located

- **WHEN** a complete unit contains a direct annotation `Taly` but defines no class of that name
- **THEN** lowering fails at the annotation's location and reports `Taly` as unknown

#### Scenario: A nested class-valued boundary annotation is rejected

- **WHEN** a top-level free function has a parameter or return annotated `list[Tally]`
- **THEN** lowering fails at that annotation before target source is emitted

#### Scenario: A class-valued method boundary is not accepted accidentally

- **WHEN** an exported method other than its implicit receiver directly takes or returns `Tally`
- **THEN** lowering fails with a located diagnostic explaining that the position is not supported

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

A free function with an instance return type SHALL remain valid when its result is newly owned by
the call, including an instance constructed in the function or returned by another function that
produces an owned instance. The compiler SHALL NOT make a borrowed parameter satisfy such a return
by cloning it.

#### Scenario: A borrowed parameter may be read

- **WHEN** a free function reads an attribute of a direct instance parameter
- **THEN** lowering succeeds without transferring ownership of the parameter

#### Scenario: A borrowed parameter may be mutated

- **WHEN** a free function assigns an attribute or invokes a mutating method on a direct instance
  parameter
- **THEN** lowering succeeds and the use remains a borrow of the caller's instance

#### Scenario: A borrowed parameter may be forwarded compatibly

- **WHEN** a free function passes its direct instance parameter to another free function whose
  corresponding direct instance parameter is borrow-only
- **THEN** lowering succeeds without cloning or moving the instance

#### Scenario: Returning a borrowed parameter is rejected

- **WHEN** lowering `def identity(t: Tally) -> Tally: return t`
- **THEN** lowering fails at the returned `t` with the borrowed-instance-escape category before
  target source is emitted

#### Scenario: An alias cannot hide a borrowed return

- **WHEN** a function binds `same = t` from a direct instance parameter and later returns `same`
- **THEN** lowering fails with a located borrowed-instance-escape diagnostic rather than cloning
  the instance

#### Scenario: Storing a borrowed parameter is rejected

- **WHEN** a function places a direct instance parameter in a collection or another instance's
  attribute
- **THEN** lowering fails at the storing use before target source is emitted

#### Scenario: Rebinding a borrowed parameter is rejected

- **WHEN** a function assigns a newly constructed instance to the name of a direct instance
  parameter
- **THEN** lowering fails at the assignment because that parameter binding is borrow-only

#### Scenario: A newly constructed return is owned

- **WHEN** a function annotated `-> Tally` returns `Tally(start)`
- **THEN** lowering succeeds because the returned instance is newly owned by the call

#### Scenario: An owned callee result may be returned

- **WHEN** a function annotated `-> Tally` returns the result of another function that produces a
  newly owned `Tally`
- **THEN** lowering succeeds
