## ADDED Requirements

### Requirement: Intrinsic operations emit target-native operations

The Rust backend SHALL emit each supported intrinsic operation as the corresponding Rust operation,
and SHALL select the emission from the operation's identity and its checking mode rather than from
any text carried in the IR.

#### Scenario: A mathematical operation emits an inherent method

- **WHEN** emitting an intrinsic naming a mathematical operation over a floating-point argument
- **THEN** the emitted Rust applies the corresponding `f64` operation directly, with no helper call
  and no runtime dispatch

#### Scenario: A module constant emits a target constant

- **WHEN** emitting an intrinsic naming a mathematical constant
- **THEN** the emitted Rust names the corresponding constant from the standard library

#### Scenario: An integer argument is widened before a float operation

- **WHEN** an operation declared over floating-point receives an integer expression
- **THEN** the widening already present in the IR is emitted, and the operation is applied to the
  widened value

#### Scenario: Emission stays a pure function of the unit

- **WHEN** the same unit containing intrinsics is emitted twice
- **THEN** the emitted files are byte-identical, and emission performs no I/O and consults no
  environment

### Requirement: A checked domain failure is recoverable, not a panic

Where an intrinsic carries a reported checking mode, the Rust backend SHALL emit a domain test and
SHALL surface a failure as the same recoverable error type that a checked arithmetic failure uses,
carrying a message naming the operation.

#### Scenario: A domain failure returns an error

- **WHEN** a checked intrinsic is evaluated on an input outside its domain
- **THEN** the generated function returns a recoverable error naming the operation, which the
  bridge turns into an exception, and the process does not abort

#### Scenario: An unchecked intrinsic emits no test

- **WHEN** an intrinsic carrying an unchecked mode is emitted
- **THEN** no domain test appears in the emitted source and the target operation is applied
  directly

#### Scenario: The signature does not depend on the mode

- **WHEN** the same program is emitted under a reported and an unchecked mode
- **THEN** the generated function's signature is identical in both, so the mode changes what the
  body does and never what the caller sees
