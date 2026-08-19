## ADDED Requirements

### Requirement: Control-flow statement forms

The IR SHALL support conditional execution, bounded and unbounded repetition, and the two loop
controls: a conditional carrying a test, a body, and an optional alternative; a loop carrying a
test and a body; a loop carrying a bound name, an iterable expression, and a body; and statements
that abandon or restart the enclosing loop.

`elif` SHALL be represented as a conditional nested in the alternative of another, since that is
what it means; the IR gains no separate form for it.

#### Scenario: Conditional with no alternative

- **WHEN** a function body contains an `if` with no `else`
- **THEN** the IR contains a conditional carrying the test and the body, with no alternative

#### Scenario: Conditional with an alternative

- **WHEN** a function body contains an `if`/`else`
- **THEN** the IR contains a conditional carrying both branches

#### Scenario: elif nests

- **WHEN** a function body contains `if`/`elif`/`else`
- **THEN** the IR represents the `elif` as a conditional inside the first one's alternative

#### Scenario: Conditional test is a boolean

- **WHEN** a conditional is represented in the IR
- **THEN** its test is an expression, and the type rules require that expression to be a boolean

#### Scenario: Unbounded loop

- **WHEN** a function body contains a `while`
- **THEN** the IR contains a loop carrying the test and the body

#### Scenario: Iterating loop

- **WHEN** a function body contains a `for`
- **THEN** the IR contains a loop carrying the bound name, the iterable, and the body

#### Scenario: Loop control

- **WHEN** a loop body contains `break` or `continue`
- **THEN** the IR contains the corresponding statement

#### Scenario: Bodies nest

- **WHEN** a loop contains a conditional containing another loop
- **THEN** the IR preserves the nesting

### Requirement: Range expression

The IR SHALL support a range as an expression form carrying a start, a stop, and a step. All three
SHALL be present in the IR even when the source omitted them, so that a backend never has to know
Python's defaulting rules.

A range is a distinct form rather than a call, for the same reason length is: a call is resolved
against the unit, so leaving it as one would make its meaning depend on what else was compiled.

#### Scenario: Range carries all three components

- **WHEN** `range(n)` is represented in the IR
- **THEN** it carries a start of zero, a stop of `n`, and a step of one

#### Scenario: Explicit bounds are preserved

- **WHEN** `range(a, b, c)` is represented in the IR
- **THEN** it carries each component as written

#### Scenario: A range is not a call

- **WHEN** a unit containing a range is validated
- **THEN** validation does not attempt to resolve `range` as a function

### Requirement: Control flow survives the artifact

Every new statement and expression form SHALL serialize to the durable artifact and be
reconstructible from it, deterministically, on the same terms as the existing forms.

#### Scenario: A unit using every control-flow form round-trips

- **WHEN** a unit containing a conditional, both loop forms, both loop controls, and a range is
  serialized and deserialized
- **THEN** the result compares structurally equal to the original

#### Scenario: Nesting survives

- **WHEN** a unit containing a loop inside a conditional inside a loop is round-tripped
- **THEN** the nesting is preserved

#### Scenario: The artifact stays target-neutral

- **WHEN** an artifact describing control flow is inspected
- **THEN** it names IR forms only, containing no target-language loop or branch syntax
