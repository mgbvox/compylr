# Compiling TypeScript to native: how others handle `number` (fetched 2026-09-01)

## AssemblyScript — https://www.assemblyscript.org/types.html
JS `number` is IEEE-754 double and there is no integer type. AssemblyScript's answer is **explicit
integer types in the language**: `i32`, `u32`, `i64`, `u64`, `f32`, `f64`.

- A bare `number` maps **contextually**: `i32` is the default assumption, reconsidered to `i64` if a
  value does not fit, or `f64` for floats.
- Inference is deliberately weak: "type inference in AssemblyScript is limited because the type of
  each expression must be known in advance."
- Therefore **annotations are mandatory**: "variable and parameter declarations must either have
  their type annotated or have an initializer," and "functions must be annotated with a return type."

## Consequences for compylr — issue #43
compylr's TypeScript frontend maps `number` to `Ty::Int` **unconditionally** (`lower.rs:184`), which
is neither JS-correct (should be float) nor AssemblyScript-like (should be contextual, with
annotation required). It is the worst of both: silent, unconditional, and wrong for any non-integral
value — which is exactly the shape of #37 (division) and #43 (float-literal truncation).

`lower.rs:215` already accepts `"int"` and `"float"` as named type references. That is the
AssemblyScript pattern half-built. Two coherent completions:

1. **AssemblyScript-style (recommended).** Keep `int`/`float` as the real annotations; make bare
   `number` an *error* in a position that needs a concrete type. Fits a subset that already demands
   full annotation and refuses to guess — the same reasoning behind "a test must be a `bool`."
2. **JS-correct.** Map bare `number` to `Ty::Float`. Semantically right, but silently makes every
   loop counter and index a double, which is the performance shape compylr exists to avoid.

Option 1 is more consistent with the accepted subset's existing character. Either way, mapping to
`Int` and hoping is not one of the options.
