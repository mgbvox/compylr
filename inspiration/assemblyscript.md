# AssemblyScript

A TypeScript-to-WebAssembly compiler. https://github.com/AssemblyScript/assemblyscript — Apache-2.0

## Why it matters to compylr

It is the closest prior art to the problem compylr's TypeScript frontend gets wrong: **JS `number`
is IEEE-754 double and there is no integer type**, so any TS-to-native compiler must decide what a
`number` annotation means before it can emit anything.

## What it does

Introduces explicit machine types into the language — `i32`, `u32`, `i64`, `u64`, `f32`, `f64` —
and requires them. A bare `number` maps **contextually**: `i32` by default, reconsidered to `i64`
if a value does not fit, or `f64` for floats.

Inference is deliberately weak, and the docs say why: *"type inference in AssemblyScript is limited
because the type of each expression must be known in advance."* Consequently **annotations are
mandatory** — variable and parameter declarations must be annotated or have an initializer, and
functions must declare a return type.

## What compylr should steal

The shape of the answer. compylr's frontend maps `number` to `Ty::Int` **unconditionally**
(`crates/compylr-frontend-typescript/src/lower.rs:184`), which is neither JS-correct (should be
float) nor AssemblyScript-like (should be explicit). That single decision is upstream of issue #37
(division compiles as integer division) and issue #43 (float literals truncated to `i64`).

`lower.rs:215` already accepts `"int"` and `"float"` as named type references — the pattern is
half-built. Finishing it means making bare `number` an error where a concrete type is required,
which is consistent with a subset that already refuses to guess ("a test must be a `bool`").

## What to avoid

AssemblyScript's contextual widening — defaulting to `i32` and reconsidering — is exactly the kind
of inference compylr's subset has deliberately rejected elsewhere. Take the explicit types and the
mandatory annotations; do not take the guessing.
