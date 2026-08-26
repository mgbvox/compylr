## ADDED Requirements

### Requirement: Every expression carries its type

Every expression in the IR SHALL carry the type of the value it produces, readable from the
expression itself without reference to where it appears.

The type SHALL be one of the IR's own types, so it names no source language and no target language.

An expression's form and its type SHALL be constructed together, so that an expression whose type
contradicts its form is not representable. There SHALL be no type meaning *undetermined*: a value a
consumer cannot be told the type of is a value every consumer must invent a rule for, which is the
condition this requirement removes.

#### Scenario: A type is readable from any expression

- **WHEN** any expression in a lowered unit is inspected
- **THEN** its type is readable from the expression, without consulting the statement or function
  that contains it

#### Scenario: The type is an IR type

- **WHEN** an expression's type is inspected
- **THEN** it is one of the IR's own types, carrying no source-language or target-language spelling

#### Scenario: A nested expression carries its own type

- **WHEN** a subscript of a mapping from strings to sequences of integers is inspected
- **THEN** the subscript carries the sequence type and the mapping it reads carries the mapping type

#### Scenario: A comparison carries the boolean it produces

- **WHEN** a comparison between two integers is inspected
- **THEN** its type is boolean, not the type of its operands

#### Scenario: A form and a type cannot disagree

- **WHEN** an expression is constructed
- **THEN** its type is supplied with its form, and the two cannot be set independently

## MODIFIED Requirements

### Requirement: Stable structural fingerprint

Every IR function SHALL expose a fingerprint derived solely from its structure — name,
parameter names and types, return type, and body, including the type every expression in that body
carries. Two functions with identical structure SHALL produce identical fingerprints, and a change
to any of those components SHALL produce a different fingerprint. A unit SHALL expose a fingerprint
derived from the fingerprints of the functions it contains. Fingerprints SHALL NOT depend on source
formatting, comments, or the order in which functions were added to the unit.

#### Scenario: Identical functions fingerprint identically

- **WHEN** the same function is lowered from two sources that differ only in comments,
  blank lines, and indentation width
- **THEN** both IR functions produce the same fingerprint

#### Scenario: Changed body changes the fingerprint

- **WHEN** a function's body is edited so it computes something different
- **THEN** its fingerprint differs from the fingerprint before the edit

#### Scenario: Changed signature changes the fingerprint

- **WHEN** a function's parameter type or return type is changed
- **THEN** its fingerprint differs from the fingerprint before the change

#### Scenario: Adding a function changes the unit fingerprint

- **WHEN** a fourth function is added to a unit containing three
- **THEN** the unit fingerprint differs from its previous value
- **AND** the fingerprints of the three original functions are unchanged

#### Scenario: Unit fingerprint ignores addition order

- **WHEN** the same set of functions is assembled into two units in different orders
- **THEN** both units produce the same fingerprint

#### Scenario: A body differing only in an expression's type fingerprints differently

- **WHEN** two bodies have identical expression forms and one expression differs in its type
- **THEN** their fingerprints differ

### Requirement: A unit serializes to a durable artifact

The IR SHALL be serializable to a durable, self-describing artifact and SHALL be reconstructible
from it. This belongs to the IR rather than to any one backend: the IR is the stage every
backend consumes, so an on-disk form of it is what makes the pipeline inspectable between
lowering and code generation regardless of which target is being emitted.

The type each expression carries SHALL be part of the serialized form. Storing it is redundant with
the tree, and the redundancy is the point: it is recomputed and checked, so a stored type cannot
quietly diverge from the expression it describes, and an artifact whose types were edited is refused
rather than trusted.

#### Scenario: A unit is written and read back

- **WHEN** a unit is serialized and then deserialized
- **THEN** the result compares structurally equal to the original

#### Scenario: The artifact describes every construct

- **WHEN** a unit containing every supported type, statement form, and expression form is
  serialized
- **THEN** each construct is represented in the artifact and survives a round trip

#### Scenario: Fingerprint survives a round trip

- **WHEN** a unit is serialized, deserialized, and its fingerprint recomputed
- **THEN** the fingerprint equals that of the original unit

#### Scenario: Float literals survive exactly

- **WHEN** a unit containing float literals, including negative zero, is round-tripped
- **THEN** each literal is bit-for-bit identical to the original, consistent with the IR's rule
  that float literals compare by bit pattern

#### Scenario: The artifact carries no target-language information

- **WHEN** an artifact is inspected
- **THEN** it names IR types and operators only, containing no Rust or other target spellings

#### Scenario: Every expression's type survives a round trip

- **WHEN** a unit is serialized and read back
- **THEN** every expression carries the same type it carried before

#### Scenario: An artifact whose declared form is not this one is refused

- **WHEN** an artifact written in an earlier form of the IR is read
- **THEN** reading fails, and does not attempt to reinterpret it
