## Context

compylr is designed around a modular compiler pipeline where source language frontends and target language backends meet at a language-agnostic Intermediate Representation (IR), and host bridges negotiate calling conventions between runtimes ($N \times M$).

```
source text ──frontend──> tree ──lower──> IR ──verify──> passes ──backend──> target code ──bridge──> host extension
```

Currently, compylr implements Python as a source frontend, Rust as a target backend, PyO3 as the host bridge, and Python host bindings. Adding TypeScript as a source frontend and Golang as a target backend validates the universality of this architecture across different language paradigms:
- Dynamically/gradually typed source: Python (CPython) and TypeScript (Node.js/V8).
- Statically typed target: Rust (LLVM native, manual memory management) and Go (native, garbage-collected).

## Goals / Non-Goals

**Goals:**
- Implement `compylr-frontend-typescript`: A Rust crate that parses a strict, fully annotated TypeScript subset using `oxc_parser` and lowers it to `compylr_ir::Unit`.
- Implement `compylr-backend-golang`: A Rust crate that renders `compylr_ir::Unit` into clean, idiomatic, standalone Go code (`.go` files and `go.mod`), runtime helpers (`compat.go`), and type mappings.
- Implement `compylr-bridge-typescript-golang`: A host bridge that generates C-shared library bindings (`//export`), TypeScript declaration files (`.d.ts`), and JavaScript/Node.js FFI wrappers.
- Implement `compylr-host-typescript`: A Node-API native extension built with `napi-rs` exposing compylr compiler operations to TypeScript/Node.js.
- Implement `typescript/compylr`: A TypeScript client library providing the `@compyle` decorator, JIT build orchestration via `go build -buildmode=c-shared`, caching under `.compylr/`, and dynamic module replacement.
- Register `"typescript"`, `"go"`, and `("typescript", "go")` in `compylr-registry`.
- Maintain strict crate boundaries: frontends never know backends, backends never know frontends, and parsers never leak into IR or core.

**Non-Goals:**
- Implementing unbridged combinations such as `("python", "go")` or `("typescript", "rust")` in this change (they remain unbridged per the $N \times M$ modularity design).
- Full TypeScript type-checker emulation (we parse static type annotations and enforce compylr's strict typing rules during IR lowering).
- Supporting asynchronous functions (`async`/`await`), generator functions, or untyped `any`/`unknown` constructs.

## Decisions

### Decision 1: TypeScript AST Parsing via `oxc_parser`
- **Choice**: Use `oxc_parser` inside `compylr-frontend-typescript`.
- **Rationale**: `oxc_parser` is written in Rust, extremely fast, actively maintained, has 100% ECMAScript & TypeScript conformance, and provides precise 1-based source locations for diagnostics.
- **Alternatives Considered**:
  - `swc_ecma_parser`: Viable and mature, but heavier dependency tree.
  - `tree-sitter-typescript`: C-based, heavier build configuration.
  - Shelling out to Node.js / `tsc`: Replaced because frontend lowering must remain pure in-process Rust with zero external process overhead.

### Decision 2: Go Backend Code Structure
- **Choice**: The Go backend emits a multi-file package matching the layout design established by the Rust backend:
  - `go.mod`: Module definition.
  - `generated.go`: The pure translated functions and class structs/methods.
  - `compat.go`: Runtime semantic helpers (division by zero handling, remainder conventions, bounds-checked negative slicing, rune counting).
- **Rationale**: Keeps translated code readable for humans inspecting output, while keeping runtime helpers identical across compilations.
- **Alternatives Considered**:
  - Single monolithic file: Buries user functions beneath dozens of helper lines; rejected.

### Decision 3: Host Calling Convention via Go C-Shared Library (`-buildmode=c-shared`)
- **Choice**: The `("typescript", "go")` host bridge emits CGo export annotations (`//export`) on entrypoints, compiling to a dynamic shared library (`.so` / `.dylib` / `.dll`) via `go build -buildmode=c-shared`. The TypeScript package loads the library using a high-speed FFI layer (such as Node-API FFI or `koffi`).
- **Rationale**: Provides in-process native execution with sub-microsecond invocation overhead (~30-50ns for primitive types), avoiding IPC serialization bottlenecks.
- **Alternatives Considered**:
  - CLI Subprocess IPC (stdin/stdout): Process spawning and IPC overhead is ~10-20ms per call, unviable for tight computational loops.
  - WebAssembly (Wasm): Go Wasm binaries are large (2MB+ minimum runtime) and Node.js Wasm FFI has memory-copy overhead for structured data.

### Decision 4: Compiler Engine Host Bindings via `napi-rs`
- **Choice**: Build `compylr-host-typescript` with `napi-rs` to export the Rust compiler core as `@compylr/core`.
- **Rationale**: `napi-rs` is the standard, battle-tested tool for exposing Rust to Node.js via ABI-stable Node-API, mirroring PyO3's role in `compylr-host-python`.

### Decision 5: TypeScript Decorator & Source Inspection
- **Choice**: The TypeScript package uses standard decorators and wrapper functions `compyle(fn)`. It inspects function text via `fn.toString()` and validates IR lowering immediately upon registration. On first execution, it compiles all registered functions into a single Go artifact, loads it, and replaces the JS implementation.
- **Rationale**: Matches the Python developer experience (`@compylr.compyle`) exactly without requiring a mandatory custom build step or bundler plugin.

### Decision 6: Caching and Fingerprint Rebuild Logic
- **Choice**: Key the build on `Unit::fingerprint()` and pass configuration (`BuildKey`). The compiled binary and metadata are cached under `.compylr/` with a state file `.compylr/build.json`.
- **Rationale**: Guarantees byte-reproducible rebuild decisions, ensuring comments or formatting edits in TypeScript do not trigger recompilations.

## Risks / Trade-offs

- **[Risk] Go Toolchain Dependency at Runtime** → *Mitigation*: Like the Rust backend requiring `cargo` and `maturin`, the Go backend requires `go` on the `PATH`. If missing, the TypeScript manager produces a clear diagnostic directing the user to install Go or use precompiled artifacts.
- **[Risk] FFI Data Conversion Overhead for Complex Types** → *Mitigation*: Primitive types cross the C ABI directly by value with negligible overhead (~30ns). Slices and strings pass direct pointers and lengths. Complex nested structures use compact flat serialization.
- **[Risk] Node.js Version Compatibility with Native Addons** → *Mitigation*: Using Node-API (N-API) guarantees cross-version ABI stability across Node.js v18, v20, v22+, and Bun.
