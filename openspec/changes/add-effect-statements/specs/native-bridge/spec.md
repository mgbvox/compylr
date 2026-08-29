## ADDED Requirements

### Requirement: The host installs the output sink

The Python bridge SHALL install an output sink that writes through the host's own output stream,
and SHALL install it before any compiled function can run. Output written by compiled code SHALL be
visible to host-level redirection and SHALL appear in the order it was produced relative to output
written by the host.

#### Scenario: The sink is installed with the module

- **WHEN** the generated extension module is imported
- **THEN** the output sink is installed, and no compiled function can run before it is

#### Scenario: Ordering is preserved across the boundary

- **WHEN** the host prints, calls a compiled function that prints, and prints again
- **THEN** the three lines appear in that order on the stream, including when it is redirected to a
  pipe or a file

#### Scenario: Host-level capture sees compiled output

- **WHEN** the host captures its output stream and calls a compiled function that prints
- **THEN** the captured text contains the compiled output

#### Scenario: A failure to write is reported, not swallowed

- **WHEN** the host stream rejects a write while compiled code is printing
- **THEN** the failure surfaces as an exception rather than being silently discarded

#### Scenario: Disabling compilation does not change what is printed

- **WHEN** the same program runs with compilation disabled and again compiled
- **THEN** both produce the same output text in the same order
