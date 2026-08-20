## ADDED Requirements

### Requirement: Operators carry declared semantics

Every arithmetic operator in the IR that admits more than one reasonable interpretation SHALL carry
its interpretation explicitly on the node, rather than relying on a convention inherited from one
source language. Specifically: integer division SHALL carry a rounding mode, remainder SHALL carry a
sign convention, and division that promotes its operands SHALL say so. A frontend SHALL set these to
whatever its source language means; a backend SHALL reproduce exactly what the node declares,
without knowing which frontend produced it.

The two rounding modes SHALL be *toward negative infinity* and *toward zero*. The two remainder sign
conventions SHALL be *sign of the divisor* and *sign of the dividend*. These pairs cover the
behavior of the languages in compylr's supported list; a source language needing a third SHALL add
it to the IR rather than encode it in its frontend.

#### Scenario: Rounding mode is explicit

- **WHEN** an integer division node is inspected
- **THEN** its rounding mode is readable from the node itself

#### Scenario: The same operator can mean either rounding

- **WHEN** two integer division nodes declare different rounding modes
- **THEN** they are distinguishable, and a backend renders each differently

#### Scenario: Remainder sign convention is explicit

- **WHEN** a remainder node is inspected
- **THEN** its sign convention is readable from the node itself

#### Scenario: Promotion is explicit

- **WHEN** a division node that yields a floating-point result from integer operands is inspected
- **THEN** the promotion is declared on the node rather than implied by the operator's name

#### Scenario: No node's meaning depends on the source language

- **WHEN** a unit is interpreted without knowing which frontend produced it
- **THEN** every operator's meaning is fully determined by the unit

### Requirement: A unit records the frontend that produced it

A unit SHALL record the name of the frontend that lowered it, and the semantic guarantees that
frontend requires be preserved. This is what allows a pair-directed pass to be selected and a
backend's post-processing to be gated without any component re-deriving the source language from the
shape of the tree.

#### Scenario: The producing frontend is recorded

- **WHEN** a unit is produced by lowering source with a named frontend
- **THEN** the unit reports that frontend's name

#### Scenario: Required guarantees travel with the unit

- **WHEN** a unit is inspected
- **THEN** the guarantees its frontend requires preserved are readable from it

#### Scenario: The record survives the artifact

- **WHEN** a unit is serialized and read back
- **THEN** the producing frontend and its required guarantees are unchanged

## MODIFIED Requirements

### Requirement: Target-language independence

The IR SHALL NOT name, spell, or otherwise encode constructs specific to any single target
language, **nor to any single source language**. Choosing the concrete type spelling, operator
syntax, and value representation for a target is the responsibility of that target's backend,
defined by its own capability; choosing how a construct is spelled back to a programmer in
diagnostics is the responsibility of the frontend that read it. Rust is the first backend compylr
implements and Python the first frontend, but the IR SHALL remain producible by a frontend for
another imperative source language and expressible by a backend for another imperative target, such
as Go, C++, or TypeScript.

#### Scenario: No target syntax in the IR

- **WHEN** the IR type model and node definitions are inspected
- **THEN** no target language's type spellings or syntax appear in them

#### Scenario: No source syntax in the IR

- **WHEN** the IR type model and node definitions are inspected
- **THEN** no source language's type spellings, operator spellings, or keywords appear in them

#### Scenario: Backend supplies the mapping

- **WHEN** a backend renders an IR function for a specific target
- **THEN** it derives every concrete type spelling from the IR's semantic types, without
  reading the original source

#### Scenario: Frontend supplies the spelling in diagnostics

- **WHEN** a diagnostic needs to quote a type or operator in the programmer's own language
- **THEN** the spelling comes from the frontend that read the source, not from the IR

## REMOVED Requirements

### Requirement: Operators carry Python semantics

**Reason**: The IR cannot be produced by a second frontend while its operators silently *are*
Python's — a Go frontend lowering `/` would mean truncation and would get flooring, with no place to
say otherwise. The semantics are not being dropped; they are being made explicit on the node so that
any frontend can state them and any backend can reproduce them.

**Migration**: Replaced by "Operators carry declared semantics". Python's meanings are unchanged and
are now set by the Python frontend: integer division declares rounding toward negative infinity,
remainder declares the sign of the divisor, and true division declares float promotion. Backends
that hardcoded these read them off the node instead. Serialized IR changes shape, so unit
fingerprints change and every cached build is rebuilt once.
