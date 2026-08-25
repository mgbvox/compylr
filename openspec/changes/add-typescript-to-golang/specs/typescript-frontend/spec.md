## Purpose

The source language frontend for TypeScript: turns TypeScript source text into language-agnostic compylr IR, validates static type annotations, declares what TypeScript means by each operator and what guarantees it requires a target to preserve, and owns how types and operators are spelled in compiler diagnostics.

## ADDED Requirements

### Requirement: Parse TypeScript source text into an AST
The TypeScript frontend SHALL accept TypeScript source text (or a file path) and produce a parsed syntax tree using a Rust-native TypeScript parser. The parser dependency SHALL remain strictly confined to `compylr-frontend-typescript` and SHALL NOT be visible to `compylr-ir`, `compylr-core`, or any backend.

#### Scenario: Valid TypeScript function is parsed
- **WHEN** valid TypeScript source text defining a typed function is supplied
- **THEN** parsing succeeds and yields an AST

#### Scenario: Syntax error reports location
- **WHEN** syntactically malformed TypeScript source is supplied
- **THEN** parsing fails with a `LoweringError::Syntax` containing 1-based line and column locations

### Requirement: Lower supported TypeScript subset to compylr IR
The frontend SHALL lower a strict, fully annotated TypeScript subset into `compylr_ir::Unit`. Supported constructs include top-level functions, typed parameters, explicit return types, variable declarations (`const`, `let`), assignments, control flow (`if`/`else`, `while`, `for (let i = 0; i < n; i++)`, `for (const x of xs)`, `break`, `continue`), expressions (arithmetic, comparisons, boolean ops, indexing, calls), collections (`Array<T>`, `Map<K, V>`, `Set<T>`, `[T1, T2]`), and classes with constructor property initialization and methods.

#### Scenario: Function with primitive annotations lowers to IR
- **WHEN** `function add(a: number, b: number): number { return a + b; }` is lowered
- **THEN** it produces an IR function with integer/float parameters and return type, returning a binary addition node

#### Scenario: Missing return on path is rejected
- **WHEN** a function declaring a non-void return type has a control flow path without a return
- **THEN** lowering fails with a located `LoweringError::Unsupported` diagnostic

#### Scenario: Parameter mutation is rejected
- **WHEN** a function mutates an array parameter via `xs.push(v)` or `xs[i] = v`
- **THEN** lowering fails explaining that parameters cross the boundary by value and cannot be mutated in place

### Requirement: TypeScript type validation and inference
The frontend SHALL validate type consistency across initializers, assignments, and expressions, and infer local variable types from initializers when omitted.

#### Scenario: Local type inference
- **WHEN** `const x = 42;` is lowered
- **THEN** `x` is inferred as an integer binding

#### Scenario: Incompatible type assignment is rejected
- **WHEN** a variable initialized as a string is assigned a number
- **THEN** lowering fails with a diagnostic stating the type mismatch

### Requirement: TypeScript frontend owns TypeScript diagnostics and spellings
The frontend SHALL format types and operators in diagnostics using canonical TypeScript spellings (`number`, `string`, `boolean`, `void`, `Array<T>`, `Map<K, V>`, `Set<T>`, `[A, B]`).

#### Scenario: Diagnostic reports TypeScript type spelling
- **WHEN** a type mismatch occurs involving a map from strings to numbers
- **THEN** the error message spells the type as `Map<string, number>`

### Requirement: Declare TypeScript behavior profile and required guarantees
The frontend SHALL declare TypeScript's stance on all six semantic axes (floating-point division `/`, truncating/integer division semantics, modulo sign convention `%`, 0-based indexing, UTF-16/character string length) and declare required guarantees (division by zero, integer overflow reporting where applicable, float ordering).

#### Scenario: Frontend registers behavior declaration
- **WHEN** `Frontend::behavior()` is queried on the TypeScript frontend
- **THEN** it returns a complete `LanguageBehavior` covering all behavior axes without referencing any target language
