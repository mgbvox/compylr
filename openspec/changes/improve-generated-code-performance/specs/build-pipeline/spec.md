## ADDED Requirements

### Requirement: The generated crate is built under an explicit release profile

The generated crate's manifest SHALL declare its own release profile rather than inheriting
Cargo's defaults. The artifact is built once per fingerprint and imported on every subsequent run,
so build time is the cheap side of that trade and run time is the expensive one.

The profile SHALL at minimum enable link-time optimization and a single codegen unit. This is not a
generic "make it faster" setting: the runtime helpers are emitted into a different module from the
code that calls them, and at Cargo's default of sixteen codegen units they are frequently not
inlined — which matters here in particular because every arithmetic operation in the supported
subset is emitted as a trait call by design.

The profile SHALL NOT select a target CPU. A generated crate may be copied to another machine, and
an artifact that faults on an unsupported instruction is a worse outcome than a slower one.

#### Scenario: The manifest declares a release profile

- **WHEN** the generated crate's manifest is written
- **THEN** it contains a release profile section declaring link-time optimization and a single
  codegen unit

#### Scenario: The build still succeeds end to end

- **WHEN** a project is compiled with the profile in place
- **THEN** the crate builds and the resulting module imports and runs as before

#### Scenario: The artifact stays portable

- **WHEN** the generated crate's manifest and cargo configuration are written
- **THEN** neither pins a target CPU, so the built artifact does not depend on the machine that
  built it

#### Scenario: Panics still reach Python as exceptions

- **WHEN** the release profile is chosen
- **THEN** it preserves unwinding, because the bridge converts a panic into a Python exception and
  aborting would terminate the interpreter instead
