## ADDED Requirements

### Requirement: The bridge forwards records into the host's logging system

The Python bridge SHALL install a logging implementation that forwards records produced by compiled
code into the host's logging system, mapping levels in both directions, and SHALL install it before
any compiled function can run.

#### Scenario: The implementation is installed with the module

- **WHEN** the generated extension module is imported
- **THEN** the forwarding implementation is installed, and no compiled record can be produced before
  it is

#### Scenario: Host handlers receive forwarded records

- **WHEN** the host has configured a handler and compiled code records
- **THEN** the handler receives the record with its level, message, and origin

#### Scenario: The host's effective level is what suppresses

- **WHEN** the host raises its effective level above a record's level
- **THEN** compiled code stops emitting that record without being rebuilt

#### Scenario: Installation is idempotent

- **WHEN** more than one generated module is imported into one process
- **THEN** the implementation is installed once and records from every module are forwarded

#### Scenario: A failure while forwarding does not abort

- **WHEN** the host's logging raises while a record is being forwarded
- **THEN** the failure is contained and the compiled function continues or reports, rather than
  aborting the process
