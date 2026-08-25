## Purpose

Generates the host bridge artifacts necessary to build, wrap, and invoke compiled Go functions and structures from a TypeScript/JavaScript runtime (Node.js/Bun). Implements the `HostBridge` trait for the `("typescript", "go")` language pair.

## ADDED Requirements

### Requirement: Implement HostBridge for TypeScript and Go
The `compylr-bridge-typescript-golang` crate SHALL implement `compylr_core::bridge::HostBridge` returning `source() == "typescript"` and `target() == "go"`.

#### Scenario: Bridge resolution
- **WHEN** the bridge for `("typescript", "go")` is requested from the registry
- **THEN** resolution succeeds and returns the TypeScript-Go bridge implementation

### Requirement: HostArtifact emission for Go shared library and JS wrapper
The bridge SHALL emit a complete `HostArtifact` containing the Go backend source files, C-shared export wrappers (`bindings.go` with `//export` annotations and CGo headers), a TypeScript declaration file (`index.d.ts`), and a JavaScript/TypeScript loader (`index.js`) configuring the runtime FFI binding.

#### Scenario: HostArtifact contents
- **WHEN** `HostBridge::emit(unit, build_key)` is called
- **THEN** the emitted `HostArtifact.files` includes `go.mod`, `generated.go`, `compat.go`, `bindings.go`, `index.js`, and `index.d.ts`

### Requirement: Boundary type marshalling and error propagation
The bridge SHALL generate marshalling code crossing the runtime boundary:
- Primitive numbers, booleans, and UTF-8 strings are converted between JS and Go types.
- Array, Map, and Struct collections are serialized/marshalled across the boundary.
- When an emitted Go function returns a non-nil `error`, the JavaScript wrapper translates the error into a thrown JavaScript `Error` with the failure message.

#### Scenario: Successful call across boundary
- **WHEN** a JavaScript caller invokes a compiled Go function `add(2, 3)`
- **THEN** arguments cross the bridge and the returned integer `5` is received in JavaScript

#### Scenario: Runtime error becomes thrown exception
- **WHEN** a compiled Go function returns an error (e.g. division by zero)
- **THEN** the JavaScript wrapper throws an `Error("division by zero")`

### Requirement: Loadable name uniqueness
The bridge SHALL encode the `BuildKey` variant tag and fingerprint into the artifact module name (`compylr_generated_<fingerprint>_<variant>`), ensuring that recompilations or different pass configurations load as distinct modules without cache collision in the host runtime.

#### Scenario: BuildKey reflected in loaded module
- **WHEN** a unit is emitted with a given `BuildKey`
- **THEN** `HostArtifact.loaded_as` contains the fingerprint and variant tag
