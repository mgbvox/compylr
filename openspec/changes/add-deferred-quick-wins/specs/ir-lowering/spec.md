## MODIFIED Requirements

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

## ADDED Requirements

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
