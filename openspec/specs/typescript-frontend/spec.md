# typescript-frontend Specification

## Purpose
The source language frontend for TypeScript: it turns TypeScript source text into
language-neutral IR, enforces the supported subset, declares what TypeScript means by each
operator and what it requires a target to preserve, and owns how types and operators are spelled
back to the programmer in diagnostics.

## Requirements

### Requirement: Parse TypeScript source text into a syntax tree
The frontend SHALL accept TypeScript source text and produce a parsed syntax tree. The parser
dependency SHALL remain confined to this frontend and SHALL NOT be reachable from the IR, the
component model, or any backend — which is what makes "a backend cannot name TypeScript" a
property of the build rather than a convention.

#### Scenario: A typed function parses
- **GIVEN** TypeScript source text defining a fully annotated function
- **WHEN** it is parsed by the `typescript` frontend
- **THEN** parsing succeeds

#### Scenario: A syntax error is located
- **GIVEN** TypeScript source text that is not syntactically valid
- **WHEN** it is parsed by the `typescript` frontend
- **THEN** parsing fails
- **AND** the diagnostic carries the 1-based line and column of the offending text

#### Scenario: No other crate can reach the parser
- **GIVEN** the workspace manifests
- **WHEN** the crate boundaries are checked
- **THEN** this frontend is the only crate depending on a TypeScript parser

### Requirement: Lower the supported TypeScript subset to IR
The frontend SHALL lower a strict, fully annotated TypeScript subset into an IR unit: top-level
functions with annotated parameters and an explicit return type, `const` and `let` declarations,
assignment, `if`/`else`, `while`, counted and iterating `for`, `break`, `continue`, arithmetic,
comparison and boolean expressions, indexing, calls, the collection types, and classes whose
attributes are established in the constructor.

#### Scenario: An annotated function lowers to the operation it wrote
- **GIVEN** a module whose only function is

  ```typescript
  function add(a: number, b: number): number {
      return a + b;
  }
  ```

- **WHEN** the module is lowered by the `typescript` frontend
- **THEN** lowering succeeds
- **AND** the unit carries one function whose body returns an addition of its two parameters

#### Scenario: A path that does not return is refused
- **GIVEN** a function declaring a return type with a path that reaches the end without returning
- **WHEN** the module is lowered by the `typescript` frontend
- **THEN** lowering fails
- **AND** the diagnostic points at the function rather than reporting a failure from the backend

#### Scenario: Mutating a parameter is refused with its reason
- **GIVEN** a function that pushes onto, or assigns into, one of its own array parameters
- **WHEN** the module is lowered by the `typescript` frontend
- **THEN** lowering fails
- **AND** the diagnostic says the parameter crossed by value, so the caller could not observe the
  mutation

### Requirement: Type validation and local inference
The frontend SHALL check that initializers, assignments, and expressions agree on type, and SHALL
infer a local binding's type from its initializer when the initializer determines it.

#### Scenario: A local takes the type of its initializer
- **GIVEN** a function binding a local to an integer literal with no annotation
- **WHEN** the module is lowered by the `typescript` frontend
- **THEN** the binding carries the integer type

#### Scenario: A binding keeps the type it was first bound at
- **GIVEN** a function that binds a local to text and later assigns a number to it
- **WHEN** the module is lowered by the `typescript` frontend
- **THEN** lowering fails
- **AND** the diagnostic names both types

### Requirement: The frontend owns TypeScript's spellings
The frontend SHALL spell types and operators in diagnostics the way TypeScript writes them, so
that a programmer is answered in the language they wrote. How a construct is spelled back to the
programmer belongs to the frontend that read it, never to the IR or to a backend.

#### Scenario: A diagnostic answers in TypeScript's spelling
- **GIVEN** a program whose types disagree, one of them a mapping from text to numbers
- **WHEN** the module is lowered by the `typescript` frontend
- **THEN** lowering fails
- **AND** the diagnostic spells that type `Map<string, number>`
- **BUT** it does not use the IR's own rendering or a target language's

### Requirement: Declare TypeScript's behavior and required guarantees
The frontend SHALL declare TypeScript's stance on every semantic axis, and SHALL declare the
guarantees it requires a target to preserve. The declaration SHALL name no target language.

#### Scenario: The declaration is complete and target-neutral
- **GIVEN** the `typescript` frontend
- **WHEN** its behavior is requested
- **THEN** it answers for every axis
- **BUT** the answer names no target language
