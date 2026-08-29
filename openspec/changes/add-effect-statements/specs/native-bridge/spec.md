## ADDED Requirements

### Requirement: The host installs the output sink

The Python bridge SHALL install an output sink that writes through the host's own output stream,
and SHALL install it before any compiled function can run. Output written by compiled code SHALL be
visible to host-level redirection and SHALL appear in the order it was produced relative to output
written by the host. Installing the sink is a bridge concern because
[`crate_boundaries.rs`](../../../../../crates/compylr-host-python/tests/crate_boundaries.rs) forbids
the backend from naming the host language.

#### Scenario: The sink is installed with the module

- **GIVEN** a generated extension module containing a function that prints
- **WHEN** the module is imported
- **THEN** the output sink is installed
- **AND** no compiled function can run before it is

#### Scenario: Ordering is preserved across the boundary

- **GIVEN** a host that prints, calls a compiled function that prints, and prints again
- **WHEN** the stream is redirected to a pipe or a file
- **THEN** the three lines appear in the order they were produced

#### Scenario: Host-level capture sees compiled output

- **GIVEN** a host that has captured its output stream
- **WHEN** it calls a compiled function that prints
- **THEN** the captured text contains the compiled output

#### Scenario: A failure to write is reported, not swallowed

- **GIVEN** a host stream that rejects a write
- **WHEN** compiled code prints to it
- **THEN** the failure surfaces as an exception
- **BUT** it is not silently discarded

#### Scenario: Disabling compilation does not change what is printed

- **GIVEN** one program run with `COMPYLR_DISABLE=1` and again compiled
- **WHEN** the two outputs are compared
- **THEN** both produce the same text in the same order
