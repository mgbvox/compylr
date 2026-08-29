## Purpose

Defines what a compiled program may record to a log, how a level suppresses a record before its
cost is paid, and how records reach the host application's logging configuration so that compiled
and interpreted code produce one stream.

## ADDED Requirements

### Requirement: A record carries a level and one message

A logging operation SHALL carry one of the levels debug, information, warning, error, or critical,
and SHALL take exactly one argument of a renderable type. Additional positional arguments SHALL be
rejected with a located diagnostic naming placeholder-style formatting as not yet supported.

#### Scenario: Each level is available

- **WHEN** a program records at debug, information, warning, error, or critical level
- **THEN** each lowers to an effectful operation carrying that level

#### Scenario: A single argument of any renderable type is accepted

- **WHEN** a program records an integer, float, boolean, string, sequence, or tuple
- **THEN** lowering succeeds and the value is rendered by the same convention output uses

#### Scenario: Additional arguments are refused with their reason

- **WHEN** a program records a message with a second positional argument
- **THEN** lowering fails with a located diagnostic reporting that placeholder-style formatting is
  not yet supported, rather than silently joining the arguments

#### Scenario: An unordered container is refused

- **WHEN** a program records a mapping or a set
- **THEN** lowering fails for the reason it fails for output: iteration order is not guaranteed

### Requirement: A suppressed record costs nothing

Where a record's level is not enabled, the program SHALL NOT evaluate or render the record's
argument. The level test SHALL precede any work the record would require.

#### Scenario: A disabled record does not render its argument

- **WHEN** a record at a disabled level names an argument whose rendering is observable
- **THEN** the rendering does not happen

#### Scenario: A disabled record in a loop is a level test per iteration

- **WHEN** a loop body records at a disabled level
- **THEN** the per-iteration cost is a level test, and no rendering or allocation occurs

#### Scenario: An enabled record renders once

- **WHEN** a record at an enabled level is reached
- **THEN** its argument is rendered exactly once

### Requirement: The host's logging configuration governs compiled records

Records produced by compiled code SHALL be delivered to the host's logging system, so that the
host's handlers, formatters, and effective levels determine what is written and how. Changing the
host's level SHALL change what compiled code emits without rebuilding it.

#### Scenario: Host handlers receive compiled records

- **WHEN** the host configures a handler and calls a compiled function that records
- **THEN** the handler receives the record

#### Scenario: The host's level suppresses compiled records

- **WHEN** the host sets its effective level above a record's level and calls a compiled function
- **THEN** the record is not written, and no rebuild was required to achieve that

#### Scenario: Levels map both ways

- **WHEN** a record is produced at each supported level
- **THEN** it arrives at the host at the corresponding level, and a host level maps back to the
  same one

#### Scenario: A record carries an origin

- **WHEN** a compiled record reaches the host
- **THEN** it carries an origin derived from the source module, so configuration keyed by logger
  name applies to it

#### Scenario: Compiled and interpreted records share one stream

- **WHEN** the same program runs with compilation disabled and again compiled
- **THEN** both produce records at the same levels with the same messages, through the same host
  configuration
