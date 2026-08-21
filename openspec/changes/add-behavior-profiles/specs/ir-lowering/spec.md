## ADDED Requirements

### Requirement: Lowering takes a resolved behavior and sets every mode from it

Lowering SHALL accept a resolved behavior and SHALL set every declared mode on every node it
produces from that behavior. No mode SHALL be set from a constant belonging to one language, and no
node SHALL be left to acquire a mode later.

Lowering SHALL be a pure function of the parsed source and the resolved behavior together: lowering
the same source twice under the same behavior SHALL produce identical IR, and under two different
behaviors SHALL produce IR that differs in exactly the modes the two behaviors differ on.

#### Scenario: Every mode comes from the behavior

- **WHEN** a source containing division, remainder, subscripting, length, and arithmetic is lowered
- **THEN** each resulting node's declared modes match what the resolved behavior says for that axis

#### Scenario: Two behaviors differ only where the behaviors differ

- **WHEN** the same source is lowered under two behaviors that differ on one axis
- **THEN** the two units differ only in the modes that axis governs, and are otherwise identical

#### Scenario: A behavior is required

- **WHEN** lowering is invoked
- **THEN** a resolved behavior is supplied, and there is no lowering path that supplies its own

### Requirement: Behavior does not change what source is accepted

The set of Python programs lowering accepts SHALL NOT depend on the resolved behavior. A behavior
selects what an accepted operation *means*; it SHALL NOT make a rejected program acceptable or an
acceptable program rejected.

Type rules SHALL likewise be unaffected. In particular, `/` SHALL yield a float under every
behavior, so that the same annotated source type-checks identically whichever behavior compiles it;
what the behavior selects for `/` is what happens when the divisor is zero, not what type the
result has.

#### Scenario: Acceptance is behavior-independent

- **WHEN** every accepted fixture is lowered under each behavior
- **THEN** all of them lower successfully under all of them

#### Scenario: Rejection is behavior-independent

- **WHEN** every rejected fixture is lowered under each behavior
- **THEN** all of them are rejected under all of them, with the same diagnostic code

#### Scenario: Division's result type does not move

- **WHEN** `a / b` with integer operands is lowered under a behavior that selects the target's
  meaning for exact division
- **THEN** the result is still typed as a float, and the operands are still promoted

#### Scenario: A negative index is not rejected statically

- **WHEN** `xs[-1]` is lowered under a behavior in which a negative index is out of range
- **THEN** lowering succeeds, because the index is a runtime value and refusing a literal one would
  reject only the cases that are visible

## MODIFIED Requirements

### Requirement: Lower a parsed source to IR functions

Lowering SHALL accept a parsed Python source and a resolved behavior, and produce one IR function
per top-level function definition the source contains, preserving the structure of each body.
Lowering a source SHALL NOT require knowledge of any other source.

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

#### Scenario: The behavior travels with the source

- **WHEN** two sources are lowered under different behaviors into one unit
- **THEN** each resulting function carries the modes of the behavior its own source was lowered
  under
