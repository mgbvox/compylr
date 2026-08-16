## Why

Everything the backend generates goes into one `lib.rs`, so reading it means hunting. For a
project containing a **single one-line function**, that file is 238 lines, and the translated code
is lines 200–212:

```
  1–3     header and lint allowances
  4–199   pub mod runtime { ... }        the embedded Python-semantics helpers
200–212   pub mod generated { ... }      ← the 13 lines you came to read
213–238   PyO3 wrappers and #[pymodule]
```

Writing both intermediates to disk was justified on the grounds that a transpiler whose stages are
invisible cannot be trusted. That argument only pays off if the output can actually be read. A
196-line preamble in front of every answer is the difference between an artifact someone inspects
and one they give up on — and the preamble is byte-identical in every project, so it is exactly
the part nobody needs to look at twice.

## What Changes

- **Emit a crate of files rather than one string.** The generated crate becomes:

  ```
  crate/src/
    lib.rs        module declarations and the module registration; constant size
    generated.rs  ONLY the translated functions
    bindings.rs   the PyO3 wrappers and the error mapping
    compat.rs     Python semantics in Rust: py_add, py_floordiv, and friends
  ```

- `bindings.rs` is a **fourth** file beyond the three the request named. The binding layer grows by
  two items per compiled function, so folding it into `lib.rs` would contradict the same request's
  "keep lean" — `lib.rs` stays constant-size regardless of how many functions a project compiles.
- `compat.rs` stays a **single** file. Splitting it by concern is invited but not yet warranted:
  it is one coherent topic at around two hundred lines, and dividing it would trade one file that
  is read start to finish for four that require navigation. The condition for revisiting is
  recorded in design.
- **BREAKING (internal API)**: `Backend::emit` and `emit_python_extension` return a *set of named
  files* instead of a `String`, and `compylr._core.compile_unit` reports `target_sources` — a
  mapping of relative path to contents — in place of `target_source`. Nothing user-facing changes;
  both are internal seams, and no Python code outside the package reads them.
- **`--emit rust` prints only the translated code.** That is the part worth reading, and it stays
  pipeable into a pager or `grep`. **`--emit crate --out DIR`** writes the whole tree when
  something compilable is wanted. One mode doing both would do neither well: a concatenated stream
  cannot be redirected into a single `.rs` that compiles.
- The build pipeline writes every file of the crate, and **removes stale ones**, so a rename or a
  removal in the emitter cannot leave a file behind that still compiles.

Explicitly **not** in this change: any change to what is generated. The emitted code is the same
code, in the same order, arranged into files — so the compiled artifact behaves identically and
fingerprints do not move.

## Capabilities

### New Capabilities

None — this restructures output that four existing capabilities already describe.

### Modified Capabilities

- `rust-backend`: emission produces a named set of files rather than a single source string, with
  a stated division of concerns and the determinism guarantee extended to cover every file.
- `native-bridge`: the compile entry point reports the generated files as a mapping rather than
  one string.
- `build-pipeline`: the artifact requirement covers writing a multi-file crate and pruning files
  a previous build wrote that this one did not.
- `cli`: `--emit rust` narrows to the translated code, and a new form writes the whole crate to a
  directory.

## Impact

- **Ordering: this must land after `add-deferred-quick-wins`.** That change introduces the `cli`
  capability, which does not exist in `openspec/specs/` yet; the delta here modifies a requirement
  it creates. Applying this first would leave a delta with nothing to modify.
- **Code**: `src/backend/rust.rs` and `src/backend/bindings.rs` (assemble files instead of
  concatenating), `src/backend/mod.rs` (the trait's return type), `src/bridge.rs` (the reported
  shape), `src/main.rs` (the new emit form), `python/compylr/_build.py` (write and prune).
- **Every test that greps emitted text changes shape.** `tests/emit.rs`, `tests/emit_quality.rs`,
  `tests/docstrings.rs`, and `tests/execution.rs` all locate the generated functions by searching
  for a marker inside one string. With files, that search becomes a lookup — simpler, but it
  touches every emission test.
- **`tests/execution.rs` compiles emitted code with a single `rustc` invocation.** A multi-file
  crate needs the files written into a directory and `lib.rs` passed as the crate root, or the
  files concatenated for that purpose. The former is more faithful to what is shipped.
- **The build stays a maturin build of a crate directory**, so nothing about compilation changes —
  only how many files `src/` holds.
- **Snapshots shrink usefully.** The emission snapshots currently exclude the embedded runtime by
  string surgery to avoid a comment edit in `runtime.rs` forcing a snapshot review. With the
  runtime in its own file, snapshotting `generated.rs` alone is the natural thing to do rather
  than a workaround.
- **The README's artifact listing changes**: `crate/src/lib.rs` becomes the four-file layout, and
  the drift tests check every referenced path exists.
