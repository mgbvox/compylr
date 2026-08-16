## Why

Three changes have built a pipeline that stops one stage short of doing anything:

```
source text ──frontend──> ruff AST ──lower──> compylr IR ──backend──> target code
     ✓                       ✓                    ✓                   not built
```

`import compylr` raises `ModuleNotFoundError`. There is no `pyproject.toml`, no PyO3, no
maturin build — only a Rust crate whose CLI prints a fingerprint and some signatures. Every
invariant the IR was carefully designed around (target-neutral types, Python operator
semantics, fingerprint-keyed rebuilds) is currently *unexercised*, which means it is also
unverified. A backend is the only thing that can prove those decisions were right.

This change closes the loop end to end for the subset that already lowers: decorate a Python
function, get a compiled Rust extension back. It is deliberately the *minimum* that reaches
`import compylr` — no new Python syntax, no second backend, no `llm_assist`.

## What Changes

- Add a **Rust backend**: IR → Rust source. This is where the abstract type model finally
  meets concrete spellings (`int` → `i64`, `str` → `String`) and where the operator
  invariants have to be honored rather than asserted — `//` floors toward negative infinity,
  `%` takes the sign of the divisor, and `/` always yields a float, none of which Rust's
  native `/` and `%` do.
- Add **PyO3 binding emission** so generated functions are callable from Python, including
  mapping Python's failure modes onto the generated code: division by zero raises
  `ZeroDivisionError` rather than panicking, and `i64` overflow raises `OverflowError` rather
  than silently wrapping.
- Add a **native bridge**: build this crate as an extension module (`compylr._core`) so the
  Python package can reach the frontend, lowering, and backend without a subprocess. Frontend
  and lowering diagnostics become Python exceptions that keep their `line:column`.
- **Emit both intermediate artifacts to disk.** The IR is written as JSON and the generated
  Rust as source, so the pipeline is inspectable at every stage instead of being a black box
  between `@compyle` and a `.so`.
- Add a **build pipeline**: one shared maturin crate for every decorated function in a
  project, built and installed into the active venv, with rebuilds keyed off the unit
  fingerprint so reformatting and comments do not trigger a recompile and adding a fourth
  function to three existing ones rebuilds the single shared artifact.
- Add the **Python package** and its API — `compylr.initialize(backend=..., llm_assist=...)`
  returning a manager, and `@c.compyle` in both bare and parameterized forms, with
  per-function overrides of any global setting.
- `llm_assist` is **accepted but not implemented**: setting it fails with a clear error naming
  it as unimplemented. Same for backends other than `rust` — they are valid names in a
  registry that fail explicitly, not unknown-key errors. This is what keeps the API surface
  stable when they land.
- Rejection stays **strict and early**: a decorated function outside the supported subset
  fails at decoration time with its `line:column`, not at first call and not by silently
  falling back to the interpreted function.

Explicitly **not** in this change: any widening of the Python subset (the backend targets
exactly what lowers today); `llm_assist`; TypeScript, Go, or C++ backends; publishing to
PyPI; arbitrary-precision integers; and shipping prebuilt wheels of the *generated* crate.

## Capabilities

### New Capabilities

- `rust-backend`: translating IR into Rust source — concrete type spellings, expression and
  statement emission, and the semantics-preserving helpers that make Python's `//`, `%`, and
  `/` behave like Python's rather than Rust's.
- `python-bindings`: the PyO3 layer generated onto compiled functions — module and function
  registration, argument and return conversion, and mapping arithmetic failures onto Python
  exception types.
- `native-bridge`: `compylr._core`, the extension module built from this crate that exposes
  the compiler to Python and translates frontend and lowering diagnostics into Python
  exceptions.
- `build-pipeline`: the on-disk shape of a project's compilation — the shared crate, the IR
  and Rust artifacts, maturin invocation and install, and the fingerprint-keyed decision to
  rebuild or reuse.
- `python-api`: the user-facing surface — `initialize`, the manager, both decorator forms,
  configuration resolution and override, source capture, registration, and swapping the
  compiled implementation in.

### Modified Capabilities

- `ir`: gains a requirement that a unit can be serialized to a stable, deterministic,
  round-trippable artifact form. The IR is currently in-memory only, and "emit the IR as an
  artifact" is a property of the IR itself rather than of any one backend — a Go backend
  would want the same file.

## Impact

- **Dependencies**: `pyo3`, `serde`, and `serde_json` enter `Cargo.toml`, and the crate gains
  `crate-type = ["cdylib", "rlib"]` — `rlib` so the existing binary and integration tests
  still link against it, `cdylib` for the extension module. A root `pyproject.toml` with a
  maturin build backend appears for the first time.
- **A Rust toolchain and maturin become runtime requirements.** This is the sharpest
  limitation of the MVP and it should be stated plainly rather than discovered: the decorator
  shells out to `cargo` on the user's machine at first call. `uv add compylr` alone is not
  enough to compile anything. Removing that requirement means shipping prebuilt wheels, which
  is a distribution problem, not a compiler problem, and belongs in its own change.
- **First call is slow.** Compiling a crate takes seconds to tens of seconds; every later run
  hits the fingerprint cache and pays nothing. Design covers when the build is triggered so
  the cost lands once rather than once per decorated function.
- **Two distinct PyO3 roles now exist** and conflating them will cause confusion for the
  lifetime of the project: one exposes *the compiler* to Python (`compylr._core`, built from
  this repo), the other is *generated onto the user's functions* (`compylr_generated`, built
  at runtime). They are different crates with different lifecycles.
- **Semantic divergences become real** the moment code executes rather than being lowered.
  Python integers are arbitrary-precision and `i64` is not; Python's `str` is not Rust's
  `String`. The overflow and division-by-zero requirements above are the cases this change
  handles; anything beyond `i64` range is a documented limitation, consistent with the
  existing `LiteralOutOfRange` rejection.
- **Code**: new `src/backend/` (registry plus the Rust backend) and `src/bridge.rs`; `src/ir.rs`
  gains serialization; a new `python/compylr/` package alongside the existing
  `python/fixtures/`. `src/frontend.rs`, `src/lower.rs`, and `src/span.rs` are untouched.
- **README and its drift tests**: `tests/readme.rs::readme_status_matches_reality` deliberately
  only fires while no backend exists, so it will fall silent rather than fail once
  `src/backend/` lands — the prose claiming "not built" must be updated by hand in the same
  change. `readme_layout_covers_every_module` will fail until new top-level modules are listed.
- **`add-local-type-inference` is complete but unarchived**, so `openspec/specs/` does not yet
  describe `float`, `/`, or `ToFloat` — all of which the Rust backend must emit. Archiving it
  before implementation keeps the spec base honest; the delta in this change is written
  against the post-archive state.
