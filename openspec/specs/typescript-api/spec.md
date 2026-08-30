# typescript-api Specification

## Purpose
The user-facing TypeScript package (`compylr` / `@compylr/compylr`): provides the `@compyle` decorator and wrapper functions, project manager initialization, automated JIT/AOT build orchestration with the Go toolchain, `.compylr/` build caching, dynamic module replacement at runtime, and environment control.

## Requirements

### Requirement: Configuration manager initialization
The package SHALL provide an `initialize(config)` entrypoint returning a configured compilation manager. Configuration options SHALL include `backend` (defaulting to `"go"`), `behavior`, and `llmAssist` (accepted but rejected when enabled).

#### Scenario: Default initialization
- **WHEN** `compylr.initialize()` is called
- **THEN** it returns a manager targeting the Go backend with default semantic behavior

### Requirement: @compyle decorator and function wrapper
The manager SHALL provide a `@c.compyle` decorator for methods/classes and a `c.compyle(fn)` higher-order function wrapper for functions. On declaration, it SHALL extract the function's source text (via `fn.toString()` or source map resolution) and validate it against the TypeScript frontend immediately.

#### Scenario: Decorating a TypeScript function
- **WHEN** a typed function is wrapped with `compyle(fn)`
- **THEN** its source is validated immediately against `compylr-frontend-typescript`

### Requirement: Single shared artifact and first-call compilation
All decorated functions in a project SHALL be compiled into a single shared Go package. Compilation SHALL occur on the first call to any decorated function in the project. The manager SHALL invoke the Go toolchain (`go build -buildmode=c-shared -o ...`) to produce the native artifact under `.compylr/`, load it into the process via the bridge FFI loader, and replace the function body.

#### Scenario: First call triggers build and invocation
- **WHEN** a decorated function is called for the first time
- **THEN** the shared Go artifact is built, loaded, and the call executes the compiled Go implementation

#### Scenario: Subsequent calls run compiled code directly
- **WHEN** a decorated function is called a second time
- **THEN** it executes the compiled implementation with zero build overhead

### Requirement: Cache validation and fingerprinting
The manager SHALL record the IR fingerprint, backend, and compylr version in `.compylr/build.json`. If the fingerprint and version match on subsequent runs, building SHALL be skipped entirely.

#### Scenario: Cached build is reused
- **WHEN** a process starts with an unchanged project and calls a decorated function
- **THEN** the existing built artifact is loaded immediately without invoking `go build`

### Requirement: Environment disable switch
When `COMPYLR_DISABLE=1` is set in the environment, the manager SHALL bypass all validation and compilation, returning the original TypeScript functions unaltered.

#### Scenario: Disabled via environment variable
- **WHEN** `COMPYLR_DISABLE=1` is set
- **THEN** decorated functions run interpreted in JavaScript/Node.js without compiling
