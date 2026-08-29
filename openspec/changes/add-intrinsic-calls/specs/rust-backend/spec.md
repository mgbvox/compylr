## ADDED Requirements

### Requirement: Intrinsic operations emit target-native operations

The Rust backend SHALL emit each supported intrinsic operation as the corresponding Rust operation,
and SHALL select the emission from the operation's identity and its checking mode rather than from
any text carried in the IR. Emission SHALL remain a pure function of the unit, as
[`rust.rs`](../../../../../crates/compylr-backend-rust/src/rust.rs) already guarantees.

#### Scenario: A mathematical operation emits an inherent method

- **GIVEN** an intrinsic naming a mathematical operation over a floating-point argument
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted Rust applies the corresponding `f64` operation directly
- **AND** no helper call and no runtime dispatch appears around it

#### Scenario: A module constant emits a target constant

- **GIVEN** an intrinsic naming a mathematical constant
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted Rust names the corresponding constant from the standard library

#### Scenario: An integer argument is widened before a float operation

- **GIVEN** an operation declared over floating-point receiving an integer expression
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the widening already present in the IR is emitted
- **AND** the operation is applied to the widened value

#### Scenario: Emission stays a pure function of the unit

- **GIVEN** a unit containing intrinsics
- **WHEN** the unit is emitted twice
- **THEN** the emitted files are byte-identical
- **AND** emission performs no I/O and consults no environment

### Requirement: A checked domain failure is recoverable, not a panic

Where an intrinsic carries the reported [`Checked`](../../../../../crates/compylr-ir/src/ir.rs#L268)
mode, the Rust backend SHALL emit a domain test and SHALL surface a failure as the same recoverable
error type that a checked arithmetic failure uses, carrying a message naming the operation. The
generated function's signature SHALL NOT depend on the mode.

#### Scenario: A domain failure returns an error

- **GIVEN** a checked intrinsic in a generated function
- **WHEN** it is evaluated on an input outside its domain
- **THEN** the generated function returns a recoverable error naming the operation
- **AND** the bridge turns that error into an exception
- **BUT** the process does not abort

#### Scenario: An unchecked intrinsic emits no test

- **GIVEN** an intrinsic carrying the unchecked mode
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** no domain test appears in the emitted source
- **AND** the target operation is applied directly

#### Scenario: The signature does not depend on the mode

- **GIVEN** one program containing a fallible intrinsic
- **WHEN** it is emitted under the reported mode and again under the unchecked mode
- **THEN** the generated function's signature is identical in both
- **AND** the mode changes what the body does and never what the caller sees
