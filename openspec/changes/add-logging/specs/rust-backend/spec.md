## ADDED Requirements

### Requirement: Records emit through the target's logging facade

The Rust backend SHALL emit a record as a call to the target's standard logging facade at the
mapped level, and SHALL NOT emit a direct write to any stream. The generated crate SHALL depend on
the facade only, never on a logging implementation, so the host decides what writes. Level mapping
SHALL be total, with no default arm.

#### Scenario: A record emits a facade call

- **GIVEN** a record at a given level
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted Rust calls the logging facade at the corresponding level

#### Scenario: No implementation is pulled in

- **GIVEN** a generated crate containing records
- **WHEN** its manifest is inspected
- **THEN** it declares the logging facade
- **BUT** it declares no logging implementation

#### Scenario: A record is emitted against the root logger

- **GIVEN** a record produced by a module-level logging function
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted call targets the root logger, matching where the interpreted program records
- **BUT** it does not target a logger named for the source module

#### Scenario Outline: Records are emitted in every position they are legal in

- **GIVEN** a record in a <position>
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** it emits correctly
- **AND** the conformance check covers the pair

**Examples:**

| position           |
| ------------------ |
| free function body |
| method body        |
| constructor body   |
| loop body          |

### Requirement: The level test precedes rendering

The Rust backend SHALL emit a level test that guards evaluation and rendering of a record's
argument, so that a record at a disabled level performs no rendering and no allocation. The guard
SHALL NOT change the generated function's signature.

#### Scenario: Rendering is inside the guard

- **GIVEN** a record whose argument requires rendering
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the rendering appears inside the level test in the emitted source
- **BUT** it does not appear before it

#### Scenario: A disabled record allocates nothing

- **GIVEN** a compiled loop recording at a disabled level
- **WHEN** the loop runs
- **THEN** no allocation attributable to the record occurs across the loop

#### Scenario: The guard does not change the signature

- **GIVEN** one function with records and the same function without them
- **WHEN** both are emitted for the `rust` backend
- **THEN** their signatures are identical
