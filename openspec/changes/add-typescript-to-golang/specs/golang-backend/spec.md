## Purpose

Translates compylr IR into standalone, deterministic Go (Golang) source files and package manifests (`go.mod`). Owns Go type spellings, emission of Go statements and expressions, runtime compatibility helpers for IR operations, and declarations of Go's semantic behavior and preserved guarantees.

## ADDED Requirements

### Requirement: Concrete Go type spellings
The backend SHALL map each IR type to a Go type. The mapping SHALL live in `compylr-backend-golang` alone and be derived from IR types:

| IR type | Go type |
| --- | --- |
| integer | `int64` |
| float | `float64` |
| bool | `bool` |
| string | `string` |
| unit | `struct{}` (or omitted value with `error`) |
| sequence of `T` | `[]T` |
| mapping from `K` to `V` | `map[K]V` |
| set of `T` | `map[T]struct{}` |
| tuple of `T1..Tn` | `TupleN[T1, .., Tn]` or anonymous struct |
| instance of `Class` | `*ClassName` |

#### Scenario: Scalar types are spelled
- **WHEN** IR functions with integer, float, bool, and string parameters are emitted
- **THEN** Go function signatures use `int64`, `float64`, `bool`, and `string`

#### Scenario: Collection types are spelled recursively
- **WHEN** a sequence of mappings from strings to integers is emitted
- **THEN** the Go parameter type is `[]map[string]int64`

### Requirement: Function and method emission
The backend SHALL emit each function as a Go function returning `(Ret, error)` (or just `error` for unit return) to represent recoverable runtime failures (e.g. division by zero, index out of bounds). Classes SHALL emit a Go `struct`, constructor function `NewClassName(...) (*ClassName, error)`, and method receivers `func (self *ClassName) Method(...) (Ret, error)`.

#### Scenario: Fallible function emission
- **WHEN** a function `divide(a: int, b: int) -> int` is emitted
- **THEN** it generates `func Divide(a int64, b int64) (int64, error)`

#### Scenario: Class emission
- **WHEN** an IR class with attributes and methods is emitted
- **THEN** it generates a Go struct definition, a constructor function initializing fields, and methods with pointer receivers

### Requirement: Go runtime compatibility helpers
The backend SHALL emit a dedicated `compat.go` containing helper functions that implement semantic behavior requested by the IR nodes (e.g., Python/TypeScript floor division, divisor-sign modulo, safe slice indexing, negative indexing resolution, and UTF-8 rune length counting).

#### Scenario: Negative indexing helper is emitted
- **WHEN** an IR subscript node requests negative-from-end indexing
- **THEN** emitted Go code invokes the subscript helper in `compat.go` and returns an `error` if out of bounds

#### Scenario: Division by zero returns error
- **WHEN** an IR integer division node declaring checked reporting is evaluated with zero divisor
- **THEN** the Go helper returns `(0, errors.New("division by zero"))` rather than causing a panic

### Requirement: Emission is pure, deterministic, and outputs a package
The backend SHALL produce a `GeneratedFiles` map containing relative paths (`go.mod`, `generated.go`, `compat.go`). Emission SHALL be a pure function of the `Unit` without performing I/O or invoking the `go` toolchain directly.

#### Scenario: Generated package structure
- **WHEN** a unit is emitted by the Go backend
- **THEN** the result contains `go.mod`, `generated.go`, and `compat.go` with deterministic file contents

### Requirement: Post-process formatting with gofmt
The backend SHALL provide a `post_process` method that applies `gofmt` formatting to generated `.go` source files if the toolchain is present, falling back to unformatted text cleanly.

#### Scenario: gofmt formats output
- **WHEN** `post_process` is called on generated Go files
- **THEN** `gofmt` is executed on the source strings and formatted code is returned
