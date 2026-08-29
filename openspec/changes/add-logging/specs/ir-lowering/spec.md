## ADDED Requirements

### Requirement: Logging resolves to module-level operations only

Lowering SHALL resolve the supported logging functions as effectful operations of the logging
module, and SHALL reject obtaining a logger object with a located diagnostic naming the supported
module-level functions. This follows from the rule that a module is a namespace and not a value.

#### Scenario: A module-level record lowers

- **GIVEN** a function whose body contains

  ```python
  logging.info(low)
  ```

- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering succeeds and produces an effect statement carrying that level

#### Scenario: Obtaining a logger is refused

- **GIVEN** a source whose body contains

  ```python
  log = logging.getLogger(__name__)
  ```

- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming the supported module-level functions,
  because a logger would be a value and a module is not one

#### Scenario: A logger is not bindable

- **GIVEN** a source binding a local to a logging module attribute
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails through the existing rule that a module is not a value

#### Scenario: Configuring logging from compiled code is refused

- **GIVEN** a source whose body calls a logging configuration function
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic, because configuration belongs to the host and
  compiled code that reconfigured it would silently override the caller

#### Scenario: An unsupported logging operation names itself

- **GIVEN** a source naming a logging attribute the registry does not list
- **WHEN** the source is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming the module and the attribute
