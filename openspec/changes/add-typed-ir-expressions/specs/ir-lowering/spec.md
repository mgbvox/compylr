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

- **GIVEN** an expression whose type inference determines
- **WHEN** it is lowered
- **THEN** the emitted expression carries that type

#### Scenario: An annotation supplies what inference cannot

- **GIVEN** a binding annotated with a type, initialized by a call this compilation cannot
  resolve
- **WHEN** it is lowered
- **THEN** the emitted expression carries the annotated type and lowering succeeds

#### Scenario: Neither source of a type is a diagnostic, not a placeholder

- **GIVEN** an expression whose type neither inference nor an annotation determines
- **WHEN** it is lowered
- **THEN** lowering fails with the undetermined-binding diagnostic, and emits no expression

#### Scenario: Numeric promotion is visible in the types

- **GIVEN** an integer operand promoted to floating point
- **WHEN** it is lowered
- **THEN** the promoting expression carries the floating-point type and its operand carries the
  integer type
