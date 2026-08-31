# typescript-bindings Specification

## Purpose
The native Node-API host extension module (`compylr-host-typescript`), built with `napi-rs`, which
exposes the compylr compiler core — parsing, lowering, validation, IR optimization, backend
emission, host bridge generation, and fingerprinting — directly to Node.js and TypeScript host
runtimes.

## Requirements

### Requirement: Node-API native extension for compylr core
The `compylr-host-typescript` crate SHALL compile to a Node-API native addon providing JavaScript
bindings to compylr's core operations. It SHALL be the only crate in the workspace linking `napi`
or `napi-derive`.

#### Scenario: The compiler is reachable from a Node process
- **GIVEN** a Node.js process with the built addon installed
- **WHEN** `@compylr/core` is required
- **THEN** the addon loads
- **AND** the compiler operations are exported from it

#### Scenario: Only a host crate links the host runtime
- **GIVEN** the workspace manifests
- **WHEN** the crate boundaries are checked
- **THEN** `compylr-host-typescript` is the only crate naming `napi` or `napi-derive`

### Requirement: Expose compiler pipeline operations
The native addon SHALL expose operations for lowering TypeScript source text into an IR unit under
a named behavior, for validating and fingerprinting units, for resolving frontends, backends, and
host bridges from the registry, and for emitting target source and a complete host artifact for a
`(source, target)` pair.

#### Scenario: TypeScript source lowers to a unit
- **GIVEN** TypeScript source text inside the supported subset
- **WHEN** it is lowered through the addon under a named behavior
- **THEN** an IR unit is returned
- **AND** the unit carries the behavior it was lowered under

#### Scenario: A pair with a bridge emits a complete artifact
- **GIVEN** a validated unit and the `(typescript, go)` pair
- **WHEN** a host artifact is requested through the addon
- **THEN** the artifact carries the generated Go source and the loader that calls it

#### Scenario: A pair without a bridge is refused by name
- **GIVEN** a validated unit and a target compylr can emit but not yet call back
- **WHEN** a host artifact is requested for that pair
- **THEN** the request fails
- **AND** the failure names both languages rather than reporting an unavailable target

### Requirement: Structured error translation
Errors from the compiler engine SHALL cross into JavaScript as `Error` instances carrying
structured fields — the kind of failure, its location, its code, and its message — rather than as
a formatted string a caller would have to parse.

#### Scenario: A lowering failure keeps its location
- **GIVEN** TypeScript source text with a construct outside the supported subset
- **WHEN** it is lowered through the addon
- **THEN** a JavaScript error is thrown
- **AND** the error carries the line and column of the offending construct as fields
- **BUT** the caller does not have to read them out of the message text
