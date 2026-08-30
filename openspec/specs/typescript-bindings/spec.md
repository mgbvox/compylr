# typescript-bindings Specification

## Purpose
The native Node-API host extension module (`compylr-host-typescript`) built with `napi-rs`: exposes the compylr compiler core (parsing, lowering, validation, IR optimization, backend emission, host bridge generation, and fingerprinting) directly to Node.js/TypeScript host runtimes.

## Requirements

### Requirement: Node-API native extension for compylr core
The `compylr-host-typescript` crate SHALL compile to a Node-API native addon providing JavaScript bindings to compylr's core operations. It SHALL be the only crate in the workspace linking `napi` / `napi-derive`.

#### Scenario: Compylr core loads in Node.js
- **WHEN** `require("@compylr/core")` is called in a Node.js process
- **THEN** the native addon loads successfully and exports compiler functions

### Requirement: Expose compiler pipeline operations
The native addon SHALL expose functions for:
- Lowering TypeScript source texts into an IR unit with a specified behavior profile.
- Validating and fingerprinting units.
- Resolving frontends, backends, and host bridges from `compylr-registry`.
- Emitting target source files and complete `HostArtifact`s for a `(source, target)` pair.

#### Scenario: Lowering TypeScript to IR from Node.js
- **WHEN** `core.lowerTypeScript(sourceCode, behavior)` is invoked
- **THEN** it returns an IR `Unit` object or throws a structured lowering error

### Requirement: Structured error translation
Errors from the Rust compiler engine (`LoweringError`, `FrontendError`, `BackendError`, `BridgeError`) SHALL be translated into JavaScript `Error` instances containing structured fields (such as `kind`, `line`, `column`, `code`, and `message`).

#### Scenario: Syntax error translated to JS error
- **WHEN** `lowerTypeScript` encounters invalid syntax
- **THEN** it throws a JS error with `line` and `column` numbers populated
