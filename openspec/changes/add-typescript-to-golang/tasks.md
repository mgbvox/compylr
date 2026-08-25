## 1. Crate Scaffolding & Registry Updates

- [ ] 1.1 Scaffold `compylr-frontend-typescript` crate with `oxc_parser` dependency
- [ ] 1.2 Scaffold `compylr-backend-golang` crate with Go code generator modules
- [ ] 1.3 Scaffold `compylr-bridge-typescript-golang` crate implementing `HostBridge`
- [ ] 1.4 Scaffold `compylr-host-typescript` crate using `napi-rs` for Node-API bindings
- [ ] 1.5 Update `compylr-registry` to register `"typescript"`, `"go"`, and `("typescript", "go")`
- [ ] 1.6 Update `tests/crate_boundaries.rs` to enforce crate boundary isolation for new crates

## 2. TypeScript Frontend (`compylr-frontend-typescript`)

- [ ] 2.1 Implement TypeScript parser wrapper and AST extraction with 1-based source location tracking
- [ ] 2.2 Implement primitive, collection, and tuple type annotation lowering (`number`, `string`, `boolean`, `void`, `Array<T>`, `Map<K, V>`, `Set<T>`, `[T1, T2]`)
- [ ] 2.3 Implement function signature extraction, cross-source signature collection, and local type inference
- [ ] 2.4 Implement statement lowering (returns, variable declarations, assignments, `if`/`else`, `while`, `for`, `break`, `continue`)
- [ ] 2.5 Implement expression lowering (literals, binary ops, comparisons, unary ops, calls, indexing, member access)
- [ ] 2.6 Implement class and method lowering with constructor field extraction and receiver mutability inference
- [ ] 2.7 Implement TypeScript-specific type and operator spellings for compiler diagnostics
- [ ] 2.8 Implement `Frontend` trait: declare TypeScript semantic behavior profile and required guarantees
- [ ] 2.9 Add unit tests for TypeScript frontend lowering, valid programs, and rejected subset violations

## 3. Go Backend (`compylr-backend-golang`)

- [ ] 3.1 Implement Go type spelling mapping from language-agnostic IR types
- [ ] 3.2 Implement Go function, struct, constructor, and method receiver code emission
- [ ] 3.3 Implement Go statement and control flow emission (`var`, `:=`, `if`/`else`, `for`, `return`, `break`, `continue`)
- [ ] 3.4 Implement Go expression and operator emission
- [ ] 3.5 Implement `compat.go` runtime helpers (division by zero, negative slicing, remainder sign conventions, rune length)
- [ ] 3.6 Implement `go.mod` package manifest generation and pure deterministic `GeneratedFiles` emission
- [ ] 3.7 Implement `Backend::post_process` cosmetic formatting using `gofmt`
- [ ] 3.8 Implement `Backend` trait: declare Go semantic behavior profile and preserved guarantees
- [ ] 3.9 Add unit tests and conformance tests rendering the shared IR corpus to Go

## 4. TypeScript to Go Bridge (`compylr-bridge-typescript-golang`)

- [ ] 4.1 Implement `HostBridge` trait for `source() == "typescript"` and `target() == "go"`
- [ ] 4.2 Implement CGo export wrapper emission (`bindings.go` with `//export` annotations)
- [ ] 4.3 Implement TypeScript type declaration generator (`index.d.ts`)
- [ ] 4.4 Implement JavaScript runtime FFI loader module (`index.js`)
- [ ] 4.5 Implement primitive and collection type marshalling across the C ABI boundary
- [ ] 4.6 Implement runtime error propagation translating Go errors to JavaScript `Error` exceptions
- [ ] 4.7 Implement unique loadable module naming incorporating `BuildKey` fingerprint and variant tag
- [ ] 4.8 Add bridge unit tests verifying emitted `HostArtifact` files and build manifests

## 5. TypeScript Host Bindings (`compylr-host-typescript`)

- [ ] 5.1 Implement Node-API native bindings in Rust using `napi-rs`
- [ ] 5.2 Expose core compiler operations to JS/TS (`lowerTypeScript`, `validateUnit`, `fingerprint`, `emitBridge`)
- [ ] 5.3 Implement structured error translation from Rust compiler errors to JavaScript errors with location metadata
- [ ] 5.4 Add native integration tests verifying Node-API addon loading and execution

## 6. TypeScript Client Package (`typescript/compylr`)

- [ ] 6.1 Initialize TypeScript package structure (`package.json`, `tsconfig.json`)
- [ ] 6.2 Implement `compylr.initialize` configuration manager
- [ ] 6.3 Implement `@compyle` decorator and `compyle(fn)` wrapper with source code inspection
- [ ] 6.4 Implement JIT build manager invoking `go build -buildmode=c-shared` under `.compylr/`
- [ ] 6.5 Implement build state caching in `.compylr/build.json` keyed by IR fingerprint and compylr version
- [ ] 6.6 Implement dynamic runtime replacement swapping JS functions with compiled Go implementations
- [ ] 6.7 Implement `COMPYLR_DISABLE=1` bypass mechanism for running interpreted code

## 7. CLI & End-to-End Integration

- [ ] 7.1 Wire `--frontend typescript` and `--backend go` flags into `compylr-cli`
- [ ] 7.2 Create TypeScript accepted and rejected fixture corpora (`typescript/fixtures/accepted/`, `typescript/fixtures/rejected/`)
- [ ] 7.3 Create differential test drivers running accepted TypeScript fixtures against both Node.js and compiled Go
- [ ] 7.4 Run workspace tests (`cargo test --workspace`), clippy lints, formatting, and verify all crate boundaries
