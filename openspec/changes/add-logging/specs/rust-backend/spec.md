## ADDED Requirements

### Requirement: Records emit through the target's logging facade

The Rust backend SHALL emit a record as a call to the target's standard logging facade at the
mapped level, and SHALL NOT emit a direct write to any stream. The generated crate SHALL depend on
the facade only, never on a logging implementation, so the host decides what writes.

#### Scenario: A record emits a facade call

- **WHEN** emitting a record at a given level
- **THEN** the emitted Rust calls the logging facade at the corresponding level

#### Scenario: No implementation is pulled in

- **WHEN** the generated crate's manifest is inspected
- **THEN** it declares the logging facade and no logging implementation

#### Scenario: The origin is emitted as the record's target

- **WHEN** emitting a record
- **THEN** the emitted call carries an origin derived from the source module

#### Scenario: Records are emitted in every position they are legal in

- **WHEN** a record appears in a free function body, a method body, a constructor body, or a loop
  body
- **THEN** each emits correctly, and the conformance check covers all four pairs

### Requirement: The level test precedes rendering

The Rust backend SHALL emit a level test that guards evaluation and rendering of a record's
argument, so that a record at a disabled level performs no rendering and no allocation.

#### Scenario: Rendering is inside the guard

- **WHEN** emitting a record whose argument requires rendering
- **THEN** the rendering appears inside the level test in the emitted source, not before it

#### Scenario: A disabled record allocates nothing

- **WHEN** a compiled loop records at a disabled level
- **THEN** no allocation attributable to the record occurs across the loop

#### Scenario: The guard does not change the signature

- **WHEN** a function containing records is emitted
- **THEN** its signature is identical to the same function without them
