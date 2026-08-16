## ADDED Requirements

### Requirement: A unit serializes to a durable artifact

The IR SHALL be serializable to a durable, self-describing artifact and SHALL be reconstructible
from it. This belongs to the IR rather than to any one backend: the IR is the stage every
backend consumes, so an on-disk form of it is what makes the pipeline inspectable between
lowering and code generation regardless of which target is being emitted.

#### Scenario: A unit is written and read back

- **WHEN** a unit is serialized and then deserialized
- **THEN** the result compares structurally equal to the original

#### Scenario: The artifact describes every construct

- **WHEN** a unit containing every supported type, statement form, and expression form is
  serialized
- **THEN** each construct is represented in the artifact and survives a round trip

#### Scenario: Fingerprint survives a round trip

- **WHEN** a unit is serialized, deserialized, and its fingerprint recomputed
- **THEN** the fingerprint equals that of the original unit

#### Scenario: Float literals survive exactly

- **WHEN** a unit containing float literals, including negative zero, is round-tripped
- **THEN** each literal is bit-for-bit identical to the original, consistent with the IR's rule
  that float literals compare by bit pattern

#### Scenario: The artifact carries no target-language information

- **WHEN** an artifact is inspected
- **THEN** it names IR types and operators only, containing no Rust or other target spellings

### Requirement: Serialization is deterministic

Serializing the same unit SHALL produce byte-identical output across runs and regardless of the
order functions were added, so that an artifact can be compared, cached, or checked into version
control without spurious differences.

#### Scenario: Repeated serialization

- **WHEN** the same unit is serialized twice
- **THEN** the two outputs are byte-identical

#### Scenario: Addition order does not affect the artifact

- **WHEN** the same functions are assembled into two units in different orders and both are
  serialized
- **THEN** the two outputs are byte-identical

#### Scenario: Formatting changes do not affect the artifact

- **WHEN** a unit is lowered from sources differing only in comments, blank lines, and
  indentation, and serialized
- **THEN** the output is byte-identical to that of the unit lowered from the original sources
