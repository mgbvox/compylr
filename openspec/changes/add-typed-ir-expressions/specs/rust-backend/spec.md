## ADDED Requirements

### Requirement: Emission asks an expression its type

The backend SHALL determine an expression's type by reading it from the expression, and SHALL NOT
infer it from the declaration the expression appears under, from the operation's name, or from any
default.

An emission decision that depends on an operand's type SHALL be made from that operand's own type.
Where the target can resolve the decision itself, the backend MAY leave it to the target; where the
backend chooses among target constructs, it SHALL choose from the type it read.

#### Scenario: An operand's type is read, not inferred from context

- **GIVEN** an expression in a position whose declared type differs from its own
- **WHEN** it is emitted
- **THEN** emission uses the expression's own type

#### Scenario: A comparison's operands are still typed

- **GIVEN** an arithmetic operation appearing as an operand of a comparison
- **WHEN** it is emitted
- **THEN** emission knows the operand's type, and does not fall back to a type-agnostic form

#### Scenario: Length is emitted from the type

- **GIVEN** the length of a value whose type is a collection
- **WHEN** it is emitted
- **THEN** emission produces the target's direct length operation rather than a type-agnostic
  dispatch

#### Scenario: Text length still honors its declared units

- **GIVEN** the length of a value whose type is a string
- **WHEN** it is emitted
- **THEN** emission produces code counting in the units the node declares

#### Scenario: A value is copied only when its own type requires it

- **GIVEN** a bound name whose type needs no copy
- **WHEN** it is read
- **THEN** emission does not copy it

#### Scenario: The result is unchanged

- **GIVEN** any accepted program
- **WHEN** it is compiled and run before and after this change
- **THEN** it produces the same results
