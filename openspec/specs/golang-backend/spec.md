# golang-backend Specification

## Purpose
Translates IR into standalone, deterministic Go source. It owns Go's type spellings, the emission
of every statement and expression form, the runtime shim implementing what the IR's operations
declared, and Go's own declaration of what it means and what it preserves.

It is also where the IR's target-neutrality stops being a claim: it consumes the same tree the
Rust backend does, unchanged.

## Requirements

### Requirement: Concrete Go type spellings
The backend SHALL map each IR type to a Go type. The mapping SHALL live in this backend alone,
because a concrete spelling is a target's business and never the IR's:

| IR type | Go type |
| --- | --- |
| integer | `int64` |
| float | `float64` |
| bool | `bool` |
| string | `string` |
| unit | no value, with the failure channel alone |
| sequence of `T` | `[]T` |
| mapping from `K` to `V` | `map[K]V` |
| set of `T` | `map[T]struct{}` |
| tuple of `T1..Tn` | a positional struct |
| instance of a class | a pointer to that struct |

#### Scenario: Every scalar is spelled the Go way
- **GIVEN** a unit whose functions take integer, float, bool, and text parameters
- **WHEN** the unit is emitted for the `go` backend
- **THEN** those parameters are spelled `int64`, `float64`, `bool`, and `string`

#### Scenario: A nested collection is spelled recursively
- **GIVEN** a unit with a parameter that is a sequence of mappings from text to integers
- **WHEN** the unit is emitted for the `go` backend
- **THEN** that parameter is spelled `[]map[string]int64`

### Requirement: Function, class, and method emission
The backend SHALL emit each function so that a recoverable failure can be reported alongside its
answer, since Go reports failure as a value. A class SHALL emit a struct, a constructor that
establishes every attribute, and methods taking a pointer receiver.

#### Scenario: A fallible function can report its failure
- **GIVEN** a unit with a function whose body can fail at runtime
- **WHEN** the unit is emitted for the `go` backend
- **THEN** the emitted function returns its answer and a failure channel alongside it

#### Scenario: A class emits as a struct with a constructor
- **GIVEN** a unit with a class carrying attributes and methods
- **WHEN** the unit is emitted for the `go` backend
- **THEN** a struct with a field per attribute is emitted
- **AND** a constructor establishing every field is emitted
- **AND** each method takes a pointer receiver

### Requirement: The runtime shim implements what the IR declared
The backend SHALL emit a runtime shim implementing the semantics the IR's nodes declared — the
rounding a division carries, the sign a remainder carries, the origin an index counts from, the
units text is measured in. Emission SHALL read the mode a node carries and never the operation's
name.

#### Scenario: An index counted from the end resolves through the shim
- **GIVEN** a unit whose subscript declares indexing from the end
- **WHEN** the unit is emitted for the `go` backend
- **THEN** the emitted code resolves the index through the shim
- **AND** an index outside the sequence reports rather than reading out of bounds

#### Scenario: A checked division reports instead of panicking
- **GIVEN** a unit whose integer division declares that it reports failure
- **WHEN** the emitted code divides by zero
- **THEN** the failure is reported through the failure channel
- **BUT** the process does not panic

#### Scenario: One artifact holds two stances on the same operation
- **GIVEN** a unit with two divisions declaring different rounding
- **WHEN** the unit is emitted for the `go` backend
- **THEN** each division emits the rounding its own node declared

### Requirement: Emission is a pure function of the unit
The backend SHALL produce the generated files as a value: the module definition, the translated
source, and the runtime shim. Emission SHALL perform no I/O, consult no environment, and invoke no
toolchain, which is what makes its output byte-reproducible and therefore safe to key a cache on.

#### Scenario: The same unit emits the same bytes
- **GIVEN** one unit
- **WHEN** it is emitted for the `go` backend twice in different environments
- **THEN** the emitted files are byte-identical

### Requirement: Formatting is a separate step
The backend SHALL offer formatting as a post-processing step applied by whoever writes the files
out, and SHALL fall back to unformatted source when the formatter is unavailable. Keeping it out
of emission is what leaves emission pure.

#### Scenario: Generated source is formatted when the toolchain is present
- **GIVEN** generated Go source and an available formatter
- **WHEN** the post-processing step is applied
- **THEN** the formatted source is returned

#### Scenario: A missing formatter does not fail the build
- **GIVEN** generated Go source and no available formatter
- **WHEN** the post-processing step is applied
- **THEN** the unformatted source is returned
- **BUT** emission does not fail
