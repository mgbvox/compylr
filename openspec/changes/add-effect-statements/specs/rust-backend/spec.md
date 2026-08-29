## ADDED Requirements

### Requirement: Output is emitted through the runtime sink

The Rust backend SHALL emit an output statement as a call into the generated runtime's output sink,
and SHALL NOT emit a direct write to the process's standard output. The emitted source SHALL name
no host language, which is what keeps
[`crate_boundaries.rs`](../../../../../crates/compylr-host-python/tests/crate_boundaries.rs) true of
[`compylr-backend-rust`](../../../../../crates/compylr-backend-rust/src/rust.rs).

#### Scenario: Output emits a sink call

- **GIVEN** a unit containing an output statement
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted Rust calls the runtime sink
- **BUT** it does not call a printing macro that writes straight to standard output

#### Scenario: The emitted source names no host

- **GIVEN** an emitted crate containing output
- **WHEN** the crate is inspected
- **THEN** it contains no reference to the calling language
- **AND** the backend crate depends on no host binding

#### Scenario Outline: Output is emitted in every position it is legal in

- **GIVEN** an output statement in a <position>
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** it emits correctly
- **AND** the conformance check covers the pair

**Examples:**

| position             |
| -------------------- |
| free function body   |
| method body          |
| constructor body     |
| loop body            |

#### Scenario: Emission stays byte-reproducible

- **GIVEN** a unit containing output statements
- **WHEN** the unit is emitted twice
- **THEN** the emitted files are byte-identical

### Requirement: Values are rendered by the declared convention

The Rust backend SHALL render each argument using a runtime function in
[`runtime.rs`](../../../../../crates/compylr-backend-rust/src/runtime.rs) selected by the declared
rendering convention, and SHALL NOT rely on the target's own default value formatting where the two
conventions differ. Rendering a sequence SHALL stay linear in its length.

#### Scenario: Booleans and floats use the source convention's renderer

- **GIVEN** an output of a boolean or a floating-point value under the source convention
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted code calls the runtime renderer for that convention
- **BUT** it does not use the target's default formatting

#### Scenario: A sequence renders element by element

- **GIVEN** an output of a sequence
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted code renders each element through the same convention's renderer
- **AND** joins them with the convention's delimiters

#### Scenario: A rendered value is built without an intermediate allocation per element

- **GIVEN** an output of a sequence
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the rendering writes into one buffer
- **BUT** it does not allocate a string per element, so printing a sequence stays linear in its
  length
