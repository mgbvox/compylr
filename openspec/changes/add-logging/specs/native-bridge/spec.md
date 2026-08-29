## ADDED Requirements

### Requirement: The bridge forwards records into the host's logging system

The Python bridge SHALL install a logging implementation that forwards records produced by compiled
code into the host's logging system, mapping levels in both directions, and SHALL install it before
any compiled function can run. Installing it is a bridge concern because
[`crate_boundaries.rs`](../../../../../crates/compylr-host-python/tests/crate_boundaries.rs) forbids
the backend from naming the host language.

#### Scenario: The implementation is installed with the module

- **GIVEN** a generated extension module containing records
- **WHEN** the module is imported
- **THEN** the forwarding implementation is installed
- **AND** no compiled record can be produced before it is

#### Scenario: Host handlers receive forwarded records

- **GIVEN** a host that has configured a handler
- **WHEN** compiled code records
- **THEN** the handler receives the record with its level, message, and logger name

#### Scenario: The host's effective level is what suppresses

- **GIVEN** a host that raises its effective level above a record's level
- **WHEN** the compiled function runs
- **THEN** compiled code stops emitting that record without being rebuilt

#### Scenario: Installation is idempotent

- **GIVEN** more than one generated module imported into one process
- **WHEN** each is imported
- **THEN** the implementation is installed once
- **AND** records from every module are forwarded

#### Scenario: An implementation the host already installed is not displaced

- **GIVEN** a host application that has already installed its own logging implementation
- **WHEN** a generated module is imported
- **THEN** the bridge does not displace it

#### Scenario: A failure while forwarding does not abort

- **GIVEN** a host whose logging raises while a record is being forwarded
- **WHEN** compiled code records
- **THEN** the failure is contained and the compiled function continues or reports
- **BUT** the process does not abort
