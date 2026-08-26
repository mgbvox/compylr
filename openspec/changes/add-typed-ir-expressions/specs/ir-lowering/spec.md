## ADDED Requirements

### Requirement: Lowering produces a type for every expression

Lowering SHALL determine the type of every expression it emits, and SHALL emit that type on the
expression.

Where inference alone determines the type, lowering SHALL use it. Where it does not — a call to a
function this compilation cannot see — the annotation that the subset already requires in that
position SHALL supply it. Where neither does, lowering SHALL fail with the existing diagnostic for
an undetermined binding, unchanged in category and in message.

Lowering SHALL NOT emit an expression whose type contradicts its form.

#### Scenario: An inferred type reaches the expression

- **WHEN** an expression whose type inference determines is lowered
- **THEN** the emitted expression carries that type

#### Scenario: An annotation supplies what inference cannot

- **WHEN** a binding annotated with a type is initialized by a call this compilation cannot resolve
- **THEN** the emitted expression carries the annotated type and lowering succeeds

#### Scenario: Neither source of a type is a diagnostic, not a placeholder

- **WHEN** neither inference nor an annotation determines an expression's type
- **THEN** lowering fails with the undetermined-binding diagnostic, and emits no expression

#### Scenario: Numeric promotion is visible in the types

- **WHEN** an integer operand is promoted to floating point
- **THEN** the promoting expression carries the floating-point type and its operand carries the
  integer type
