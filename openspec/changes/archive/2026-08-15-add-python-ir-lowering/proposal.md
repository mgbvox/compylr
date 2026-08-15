## Why

The end state for `compylr` (see `CLAUDE.md`) is a Python package where `@compylr.compyle`
transpiles a decorated function to Rust with PyO3 bindings, builds it via maturin on first
run, and swaps the compiled implementation in on subsequent runs. Every one of those stages —
codegen, bindings, build, runtime swap — needs a stable, typed answer to the question "what
does this Python function actually say?"

Today the repo cannot answer it: `src/main.rs` is a non-compiling scaffold whose `to_ast` has
unbridged error types and whose `compyle` returns `None`. There is no representation between
"ruff AST" and "Rust source". Emitting Rust straight from the ruff AST would hard-code
Python's shape into the code generator and make every later stage a rewrite.

This change builds the middle of the pipeline first — a typed intermediate representation
(IR) and the lowering pass that produces it — so the backend, the bindings, and the decorator
runtime are all written against one already-tested contract.

## What Changes

- Replace the current single-file `src/main.rs` scaffold with a library crate (`src/lib.rs`)
  plus a thin binary, so the pipeline is testable without any user-facing entry point.
  **BREAKING** for anything importing the current `compyle`/`to_ast` free functions (nothing
  does yet).
- Add a **frontend** that parses Python **source text** into a ruff syntax tree, reporting
  syntax failures through one structured error type. Source text is the primary input because
  the eventual decorator obtains its input from `inspect.getsource(fn)`, not from a path; a
  thin read-from-path helper exists for fixtures and tests. This finishes the work the
  existing `ToAstError` enum was reaching for.
- Add the **compylr IR**: an owned tree of functions, statements, and expressions that is
  independent of *both* Python and any target language, together with a semantic type model
  (`Ty`) covering a 64-bit signed integer, a boolean, a UTF-8 string, and a unit type. The IR
  deliberately does not spell types in Rust — Rust is the first backend compylr will
  implement, but Go, C++, and TypeScript backends should be able to consume the same IR, so
  the `int` → `i64` / `str` → `String` mapping belongs to a future `rust-codegen` capability
  rather than to the IR. Operators likewise carry *Python* semantics (floor division rounds
  toward negative infinity; remainder takes the divisor's sign), leaving each backend
  responsible for preserving them rather than mapping to a same-named native operator that
  behaves differently on negative operands. The type model is concrete-only; the PEP 695
  generics in the target-state example are rejected for now and get their own change.
- Make the IR **unit an aggregate**: because every `@compylr.compyle` function in a project
  is exposed by one shared maturin crate, a unit is assembled from many independently-parsed
  function sources rather than from one file. The IR therefore supports incremental assembly,
  enforces unique function names across the whole unit, and resolves calls against the
  assembled unit — the shape the decorator needs, established before any backend depends on
  it.
- Give each IR function a **stable structural fingerprint**, and order the assembled unit
  deterministically by function name. Adding a fourth function to a project that already has
  three, or editing one of the three, must trigger exactly one rebuild of the shared crate —
  so the rebuild decision needs a cache key that changes when meaning changes and stays put
  otherwise. Keying on IR rather than on source text means reformatting, renamed locals'
  whitespace, comments, and decoration order do not force a recompile, while a real edit
  does. The rebuild machinery itself is a later change; this change supplies the key it
  needs.
- Add the **lowering pass** that walks the ruff AST and produces IR, enforcing the strict
  subset: every parameter and every return must carry a type annotation, and every local
  binding must be an annotated assignment — with one narrow exception. A binding whose
  initializer is a bare reference to an already-typed name (`b = a`) infers its type from that
  name, because aliasing has exactly one possible answer. Every other unannotated initializer
  (literal, arithmetic, comparison, call) is still rejected, so this buys the common alias case
  without becoming a general inference engine. Unannotated or unsupported code produces a
  precise diagnostic (construct + source span) rather than a panic or a silent guess.
- Establish the supported subset for this slice: module-level `def`s only; bodies of
  `return`, `pass`, and annotated assignment; expressions of literals, names, unary minus,
  arithmetic (`+ - * // %`), comparisons (`== != < <= > >=`), and calls to functions defined
  in the same unit.
- Add Python fixture files and snapshot tests covering both the accepted subset and each
  rejection case.

Explicitly **not** in this change: Rust code emission, PyO3 bindings, the maturin build step,
the rebuild/caching machinery, the Python package and `@compyle` decorator, type inference,
generics, control flow (`if`/`while`/`for`), classes, imports, collections, and floats. Also
not in this change: a backend abstraction (trait, plugin registry, or visitor) — with zero
backends implemented, an abstraction would be shaped by guesswork. Keeping the IR free of
target-language details is what preserves that option; the abstraction itself can be
extracted once the Rust backend exists and a second target is real. Each of these is a
natural follow-up change against the IR contract this one defines.

## Capabilities

### New Capabilities

- `python-frontend`: Parsing Python source text into a ruff syntax tree, with structured,
  non-panicking errors for syntax failures and for reading source off disk.
- `ir`: The compylr intermediate representation — the function/statement/expression node
  model and the `Ty` type model that fixes how supported Python annotations correspond to
  Rust types.
- `ir-lowering`: Translating a parsed Python unit into IR, enforcing the strict annotated
  subset and emitting located diagnostics for anything outside it.

### Modified Capabilities

None — this is the first change in the repo and `openspec/specs/` is empty.

## Impact

- **Code**: `src/main.rs` is reduced to a binary entry point; new modules `src/lib.rs`,
  `src/frontend.rs`, `src/ir.rs`, `src/lower.rs`, `src/error.rs`, `src/span.rs`. The existing
  teaching comments and `todo!()` in `main.rs` are replaced by working implementations.
- **Tests**: New `python/fixtures/` directory holding accepted and rejected `.py` samples;
  the existing `test_basic_python_compilation` test is superseded. `python/entrypoint.py`
  keeps its `__main__` guard and therefore becomes a *rejected* fixture in this slice, which
  is the correct behavior for the declared subset.
- **Dependencies**: adds path dependencies on the already-vendored `ruff_text_size` and
  `ruff_source_file` crates (spans and line/column rendering), and `insta` as a
  dev-dependency for IR snapshot tests — matching the vendored ruff workspace's own testing
  approach. No new runtime dependencies from outside the vendored tree; error types are
  hand-written rather than pulling in `thiserror`.
- **Downstream**: the IR node and `Ty` definitions become the contract that a future
  `rust-codegen` capability consumes, which `pyo3-bindings` and the maturin build step then
  build on, and which the Python-side `@compyle` decorator ultimately drives. The unit
  fingerprint becomes the cache key that decides when that shared artifact is rebuilt.
  Backends for other targets attach at the same seam without changing the IR.
