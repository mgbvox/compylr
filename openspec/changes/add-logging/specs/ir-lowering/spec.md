## ADDED Requirements

### Requirement: Logging resolves to module-level operations only

Lowering SHALL resolve the supported logging functions as effectful operations of the logging
module, and SHALL reject obtaining a logger object with a located diagnostic naming the supported
module-level functions.

#### Scenario: A module-level record lowers

- **WHEN** lowering a statement recording at a supported level
- **THEN** lowering succeeds and produces an effect statement carrying that level

#### Scenario: Obtaining a logger is refused

- **WHEN** lowering a call to obtain a logger object
- **THEN** lowering fails with a located diagnostic naming the supported module-level functions,
  because a logger would be a value and a module is not one

#### Scenario: A logger is not bindable

- **WHEN** lowering a binding whose initializer is a logging module attribute
- **THEN** lowering fails through the existing rule that a module is not a value

#### Scenario: Configuring logging from compiled code is refused

- **WHEN** lowering a call to a logging configuration function
- **THEN** lowering fails with a located diagnostic, because configuration belongs to the host and
  compiled code that reconfigured it would silently override the caller

#### Scenario: An unsupported logging operation names itself

- **WHEN** lowering a logging attribute the registry does not list
- **THEN** lowering fails with a located diagnostic naming the module and the attribute
