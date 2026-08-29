## ADDED Requirements

### Requirement: Lowering assigns every parameter a passing mode

Lowering SHALL assign each parameter a passing mode before the unit is complete, using the escape
analysis defined by the parameter-passing capability, and SHALL default to owned. Assigning a mode
SHALL NOT change which programs are accepted and SHALL add no diagnostic.

#### Scenario: Every parameter has a mode

- **GIVEN** a unit that has finished lowering
- **WHEN** its functions and methods are inspected
- **THEN** every parameter carries a mode

#### Scenario: The accepted subset is unchanged

- **GIVEN** the whole accepted corpus
- **WHEN** it is lowered before and after this change
- **THEN** the same programs are accepted
- **AND** the same programs are refused

#### Scenario: No diagnostic mentions a mode

- **GIVEN** any program in [`rejected/`](../../../../../frontends/python/fixtures/rejected/)
- **WHEN** it is lowered
- **THEN** its diagnostic is unchanged
- **AND** it mentions no passing mode

#### Scenario: A cross-source callee forces ownership

- **GIVEN** a function passing a parameter to a callee this compilation cannot see
- **WHEN** the unit is lowered
- **THEN** the parameter is owned, because a borrow cannot be proven safe against an unknown
  signature
