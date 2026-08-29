## ADDED Requirements

### Requirement: Lowering assigns every parameter a passing mode

Lowering SHALL assign each parameter a passing mode before the unit is complete, using the escape
analysis defined by the parameter-passing capability, and SHALL default to owned. Assigning a mode
SHALL NOT change which programs are accepted.

#### Scenario: Every parameter has a mode

- **WHEN** a unit finishes lowering
- **THEN** every parameter of every function and method carries a mode

#### Scenario: The accepted subset is unchanged

- **WHEN** the whole accepted corpus is lowered before and after this change
- **THEN** the same programs are accepted, and the same programs are refused

#### Scenario: No diagnostic mentions a mode

- **WHEN** any program in the rejected corpus is lowered
- **THEN** its diagnostic is unchanged and mentions no passing mode

#### Scenario: A cross-source callee forces ownership

- **WHEN** a function passes a parameter to a callee this compilation cannot see
- **THEN** the parameter is owned, because a borrow cannot be proven safe against an unknown
  signature
