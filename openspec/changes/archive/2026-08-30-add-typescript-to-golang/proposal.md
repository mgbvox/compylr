## Why

compylr's core thesis is that a compiler pipeline cleanly separated by an intermediate representation (IR) allows any source language frontend to compile to any target language backend, with language-pair bridges negotiating the calling conventions between runtimes ($N \times M$ made explicit and modular). Today, compylr fully supports Python transpiling to Rust with PyO3 host bindings.

Adding support for **TypeScript to Golang** demonstrates the generality of this architecture across a completely different runtime pair: a dynamically/gradually typed source language (TypeScript on Node.js/V8) compiling to a statically typed, garbage-collected target language (Go). This expands compylr beyond the Python/Rust ecosystem while strictly preserving the existing crate boundaries, semantic behavior negotiation, IR independence, and decorator-driven developer experience.

## What Changes

- **Add `compylr-frontend-typescript` crate**: A source frontend that parses a strict, fully annotated subset of TypeScript/JavaScript (using a Rust-based TS parser like `oxc_parser` or `swc_ecma_parser`) and lowers it into `compylr_ir::Unit`. Implements the `Frontend` trait, owns TypeScript spellings and diagnostics, and declares TypeScript's semantic behavior and required guarantees.
- **Add `compylr-backend-golang` crate**: A target backend that translates `compylr_ir::Unit` into clean, deterministic, standalone Go code (`.go` files and `go.mod`). Implements the `Backend` trait, provides Go runtime semantic helpers (handling division, remainder, overflow, slicing, and unicode length according to resolved behavior), and maps IR types to Go types (`int64`, `float64`, `bool`, `string`, slices, maps, structs).
- **Add `compylr-bridge-typescript-golang` crate**: A host bridge implementing `HostBridge` for the `("typescript", "go")` pair. Generates the necessary C-shared library (`-buildmode=c-shared`) or Node-API/CGo FFI wrappers, TypeScript type definitions (`.d.ts`), and build manifests to allow seamless in-process calling of compiled Go code from Node.js / TypeScript.
- **Add `compylr-host-typescript` crate**: Native Node-API extension module (using `napi-rs`) exposing the compylr compiler engine (parsing, lowering, verification, emission, and fingerprinting) to Node.js/TypeScript runtimes, matching the role `compylr-host-python` plays for Python.
- **Add TypeScript Host Package (`packages/compylr` / `typescript/compylr`)**: A TypeScript library providing the `@compyle` decorator and function wrapper, runtime manager, JIT/AOT build orchestration invoking the Go toolchain, artifact caching under `.compylr/`, and dynamic module replacement.
- **Update `compylr-registry`**: Move `"typescript"` from reserved to implemented in `frontends`, move `"go"` from reserved to implemented in `backends`, and register the `("typescript", "go")` host bridge in `bridges`.
- **Update CLI (`compylr-cli`)**: Allow `--frontend typescript` and `--backend go` in the CLI for inspection, IR emission, and code generation.
- **Add TypeScript and Go fixture corpora**: Accepted, rejected, and differential test drivers for TypeScript source programs and Go emission.

## Capabilities

### New Capabilities
- `typescript-frontend`: Parse and lower a typed TypeScript subset into compylr IR, validate static type annotations, provide TS-specific diagnostics, and declare TypeScript behavior and guarantee requirements.
- `golang-backend`: Translate compylr IR into deterministic, idiomatic Go source code and `go.mod`, map abstract IR types to Go types, and provide Go semantic runtime helpers.
- `typescript-go-bridge`: Host bridge generating the C-shared/Node-API FFI wrappers, TypeScript declarations, and build manifests for calling compiled Go functions and structs from Node.js/TypeScript.
- `typescript-bindings`: Node-API native host extension (`compylr-host-typescript`) built with `napi-rs` that exposes compylr's compiler operations to TypeScript/Node.js.
- `typescript-api`: User-facing TypeScript package providing the `@compyle` decorator, runtime manager, cache management under `.compylr/`, and toolchain build orchestration for Go.

### Modified Capabilities
- `pipeline-architecture`: Register `"typescript"` as an implemented frontend, `"go"` as an implemented backend, and `("typescript", "go")` as a registered host bridge.
- `semantic-behavior`: Declare TypeScript and Go stances across the 6 semantic behavior axes (integer overflow, integer division, exact division, remainder, sequence indexing, text length).
- `cli`: Support TypeScript frontend and Go backend flags in `compylr-cli`.

## Impact

- **New Crates**:
  - `crates/compylr-frontend-typescript`
  - `crates/compylr-backend-golang`
  - `crates/compylr-bridge-typescript-golang`
  - `crates/compylr-host-typescript`
- **New Directory / Package**:
  - `typescript/` (TypeScript client package, test suites, and fixture corpora).
- **Modified Crates**:
  - `crates/compylr-registry` (enabling frontend, backend, and bridge entries).
  - `crates/compylr-cli` (wiring up frontend/backend flags and tests).
- **Toolchain Requirements**:
  - Building and running TypeScript to Go compilations at runtime requires the Go toolchain (`go`) and Node.js / npm installed on the host.
- **Breaking Changes**: None. Existing Python to Rust transpilation, CLI defaults, and IR structures remain fully backwards-compatible.
