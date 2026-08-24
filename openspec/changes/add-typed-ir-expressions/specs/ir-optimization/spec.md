## ADDED Requirements

### Requirement: Verification rejects a unit whose types are inconsistent

Verification SHALL reject a unit in which an expression's declared type contradicts its form, its
operands, or the declaration it is used against.

At minimum it SHALL reject: an operation whose result type does not follow from its operands and its
declared modes; an argument whose type does not match the parameter it is passed to; a returned
expression whose type does not match the function's declared return type; and a bound name read at a
type other than the one it was bound at.

The reason is the one verification already exists for: an inconsistently typed unit produces target
source that does not build, and the failure arrives as a complaint about generated code rather than
about the program.

#### Scenario: A consistently typed unit passes

- **WHEN** a unit produced by lowering an accepted program is verified
- **THEN** verification succeeds and the unit is unchanged

#### Scenario: A result type that does not follow is rejected

- **WHEN** a unit contains an addition of two integers declaring a string result
- **THEN** verification fails, naming the operation

#### Scenario: A mismatched argument is rejected

- **WHEN** a unit passes an expression typed as a string to a parameter declared as an integer
- **THEN** verification fails, naming the call and the parameter

#### Scenario: A mismatched return is rejected

- **WHEN** a unit returns an expression whose type is not the function's declared return type
- **THEN** verification fails, naming the function

#### Scenario: The check does not know the source language

- **WHEN** the same inconsistently typed unit is presented as though produced by any frontend
- **THEN** verification reports the same failure

### Requirement: A pass leaves the unit consistently typed

A pass SHALL preserve the type of every expression it replaces: a replacement SHALL carry the type
the replaced expression carried.

A unit that verified before a pass SHALL verify after it. A pass that cannot establish this for a
transformation SHALL leave the unit unchanged, which is what the pass contract already requires.

#### Scenario: A folded expression keeps its type

- **WHEN** constant folding replaces an operation with a literal
- **THEN** the literal carries the type the operation carried

#### Scenario: The unit still verifies

- **WHEN** the pass pipeline runs over a unit that verified
- **THEN** the resulting unit verifies
