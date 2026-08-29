## Purpose

Defines what a compiled program may record to a log, how a level suppresses a record before its
cost is paid, and how records reach the host application's logging configuration so that compiled
and interpreted code produce one stream.

## ADDED Requirements

### Requirement: A record carries a level and one message

A logging operation SHALL carry one of the levels debug, information, warning, error, or critical,
and SHALL take exactly one argument of a renderable type. Additional positional arguments SHALL be
rejected with a located diagnostic naming placeholder-style formatting as not yet supported.

#### Scenario Outline: Each level is available

- **GIVEN** a program whose body records at <level>
- **WHEN** the program is lowered by the `python` frontend
- **THEN** it lowers to an effectful operation carrying that level

**Examples:**

| level       |
| ----------- |
| debug       |
| information |
| warning     |
| error       |
| critical    |

#### Scenario: A single argument of any renderable type is accepted

- **GIVEN** a program recording an integer, float, boolean, string, sequence, or tuple
- **WHEN** the program is lowered by the `python` frontend
- **THEN** lowering succeeds
- **AND** the value is rendered by the same convention output uses

#### Scenario: Additional arguments are refused with their reason

- **GIVEN** a program whose body contains

  ```python
  logging.info("count: %s", count)
  ```

- **WHEN** the program is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic reporting that placeholder-style formatting is
  not yet supported
- **BUT** the arguments are not silently joined

#### Scenario: An unordered container is refused

- **GIVEN** a program recording a mapping or a set
- **WHEN** the program is lowered by the `python` frontend
- **THEN** lowering fails for the reason it fails for output: iteration order is not guaranteed

### Requirement: A suppressed record costs nothing

Where a record's level is not enabled, the program SHALL NOT evaluate or render the record's
argument. The level test SHALL precede any work the record would require.

#### Scenario: A disabled record does not render its argument

- **GIVEN** a record at a disabled level naming an argument whose rendering is observable
- **WHEN** the compiled function runs
- **THEN** the rendering does not happen

#### Scenario: A disabled record in a loop is a level test per iteration

- **GIVEN** a loop body recording at a disabled level
- **WHEN** the compiled function runs
- **THEN** the per-iteration cost is a level test
- **BUT** no rendering and no allocation occurs

#### Scenario: An enabled record renders once

- **GIVEN** a record at an enabled level
- **WHEN** the compiled function runs
- **THEN** its argument is rendered exactly once

### Requirement: The host's logging configuration governs compiled records

Records produced by compiled code SHALL be delivered to the host's logging system, so that the
host's handlers, formatters, and effective levels determine what is written and how. Changing the
host's level SHALL change what compiled code emits without rebuilding it.

#### Scenario: Host handlers receive compiled records

- **GIVEN** a host that has configured a handler
- **WHEN** it calls a compiled function that records
- **THEN** the handler receives the record

#### Scenario: The host's level suppresses compiled records

- **GIVEN** a host whose effective level is above a record's level
- **WHEN** it calls a compiled function containing that record
- **THEN** the record is not written
- **AND** no rebuild was required to achieve that

#### Scenario: Levels map both ways

- **GIVEN** a program recording at each supported level
- **WHEN** the records reach the host
- **THEN** each arrives at the corresponding level
- **AND** a host level maps back to the same one

#### Scenario: A record is attributed to the logger the interpreted program uses

- **GIVEN** a program whose body calls the module-level logging functions
- **WHEN** a compiled record reaches the host
- **THEN** it is attributed to the root logger, which is where the module-level functions record
  interpreted
- **BUT** it is not attributed to the source module, which would make the same source produce a
  different logger name in the two modes

#### Scenario: Compiled and interpreted records share one stream

- **GIVEN** one program run with `COMPYLR_DISABLE=1` and again compiled
- **WHEN** the records of both runs are compared
- **THEN** both produce records at the same levels, with the same messages, against the same logger
- **AND** both pass through the same host configuration
