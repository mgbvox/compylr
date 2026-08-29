## ADDED Requirements

### Requirement: Output is emitted through the runtime sink

The Rust backend SHALL emit an output statement as a call into the generated runtime's output sink,
and SHALL NOT emit a direct write to the process's standard output. The emitted source SHALL name
no host language.

#### Scenario: Output emits a sink call

- **WHEN** emitting an output statement
- **THEN** the emitted Rust calls the runtime sink rather than a printing macro that writes
  straight to standard output

#### Scenario: The emitted source names no host

- **WHEN** the emitted crate is inspected
- **THEN** it contains no reference to the calling language, and the backend crate depends on no
  host binding

#### Scenario: Output is emitted in every position it is legal in

- **WHEN** an output statement appears in a free function body, a method body, a constructor body,
  or a loop body
- **THEN** each emits correctly, and the conformance check covers all four pairs

#### Scenario: Emission stays byte-reproducible

- **WHEN** a unit containing output statements is emitted twice
- **THEN** the emitted files are byte-identical

### Requirement: Values are rendered by the declared convention

The Rust backend SHALL render each argument using a runtime function selected by the declared
rendering convention, and SHALL NOT rely on the target's own default value formatting where the two
conventions differ.

#### Scenario: Booleans and floats use the source convention's renderer

- **WHEN** emitting output of a boolean or a floating-point value under the source convention
- **THEN** the emitted code calls the runtime renderer for that convention, not the target's
  default formatting

#### Scenario: A sequence renders element by element

- **WHEN** emitting output of a sequence
- **THEN** the emitted code renders each element through the same convention's renderer and joins
  them with the convention's delimiters

#### Scenario: A rendered value is built without an intermediate allocation per element

- **WHEN** emitting output of a sequence
- **THEN** the rendering writes into one buffer rather than allocating a string per element, so
  printing a sequence stays linear in its length
