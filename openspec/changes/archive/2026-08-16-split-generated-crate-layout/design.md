## Context

See proposal.md — Why. What the current code does:

* `RustBackend::emit` builds one `String`: a header, the runtime inlined inside `pub mod runtime`,
  then `pub mod generated`. `bindings::emit_extension` appends the PyO3 layer to it.
* `_build.py` writes that string to `crate/src/lib.rs`, alongside `Cargo.toml` and
  `.cargo/config.toml`. Nothing else in `src/` is written, and nothing there is hand-authored.
* `Backend::emit` is documented as a **pure function of the unit** — no I/O, no environment. That
  is what makes output byte-reproducible and safe to key a rebuild on, and it must survive this
  change: emitting a set of files must not become emitting files.
* Four test files locate generated functions by searching one string for a marker.

## Goals / Non-Goals

**Goals:**

* Make `generated.rs` the file a person opens, and make it hold only what they came for.
* Keep `lib.rs` constant-size, so it stays lean at fifty functions and not just at one.
* Change arrangement only. Byte-for-byte, the same code should be produced.

**Non-Goals:**

* Changing what is generated, how it is compiled, or how it is installed.
* Splitting `compat.rs` — invited by the request, but see D3.

## Decisions

### D1. `emit` returns an ordered map, and stays pure

```rust
fn emit(&self, unit: &Unit) -> Result<BTreeMap<String, String>, BackendError>;
```

Keys are relative paths (`src/generated.rs`). `BTreeMap` rather than `HashMap` so iteration order
is deterministic — the same reason `Unit` holds functions in one. Emission still touches nothing
outside itself; writing remains the build pipeline's job, which is what keeps the determinism
guarantee meaningful.

*Alternative considered:* a `GeneratedCrate` struct with a named field per file. Rejected — it
fixes the file set in the type, so a backend that wants a different arrangement (or `compat/`
split into several files later, D3) would have to change the type every other backend shares.

### D2. Four files, with `bindings.rs` separate from `lib.rs`

```
src/lib.rs        mod declarations + #[pymodule], delegating registration
src/generated.rs  the translated functions
src/bindings.rs   #[pyfunction] wrappers, error mapping, and register()
src/compat.rs     Python semantics in Rust
```

`lib.rs`:

```rust
#![allow(unused_parens, non_snake_case, /* ... */)]
mod bindings;
mod compat;
mod generated;

#[pymodule]
fn compylr_generated_<fingerprint>(m: &Bound<'_, PyModule>) -> PyResult<()> {
    bindings::register(m)
}
```

Constant size: the wrappers, which grow two items per compiled function, live in `bindings.rs`
behind a single `register`. Putting them in `lib.rs` — the three-file reading of the request —
would mean the file described as "lean" is the one that grows fastest.

The lint allowances stay as crate-root inner attributes. Lint attributes are inherited by items in
nested modules, so one declaration covers every file and `generated.rs` needs none of its own —
which is what lets it contain the functions and nothing else.

### D3. `compat.rs` stays one file, for now

The request invites splitting it "if it makes sense". It does not yet. The helpers are about two
hundred lines on one topic — reproducing Python's arithmetic — and are read start to finish by
anyone checking whether `//` really floors. Splitting them into `arith.rs`, `compare.rs`, and
`error.rs` would replace one file that answers the question with three that require navigating
between them, which is the problem this change exists to remove, reintroduced one level down.

The condition for revisiting: when `compat.rs` covers concerns that are not read together — string
methods, collection operations, iteration protocols. Collections in particular will add helpers
that have nothing to do with arithmetic, and that is the point to split by concern rather than by
line count.

### D4. `src/` is rewritten wholesale on every build

Nothing in `src/` is hand-authored, so the pipeline clears it and writes the current file set,
rather than diffing against a record of what it wrote last time.

This is the simplest thing that cannot leave a stale file behind. A file a previous build wrote and
this one did not would still compile, and could still be reachable if a `mod` declaration outlived
it — a failure that presents as "my change had no effect", which is expensive to diagnose.

Pruning is scoped to `src/`. `Cargo.toml`, `.cargo/config.toml`, and `target/` sit outside it and
are left alone; `target/` in particular must survive, or every build would be a cold build.

### D5. `--emit rust` prints `generated.rs`; `--emit crate --out DIR` writes the tree

The two questions "what did my function become?" and "give me something I can compile" have
different answers, and one flag serving both serves neither: a concatenated stream cannot be
redirected into a `.rs` that compiles, so `> out.rs` would silently stop working.

`--out` is required for the crate form rather than defaulting to the working directory. Writing
four files somewhere the user did not name is a side effect a command should not have.

### D6. Tests locate code by file rather than by string surgery

`tests/emit.rs`, `tests/emit_quality.rs`, and `tests/docstrings.rs` currently find the generated
functions by searching for `"pub mod generated {"` and slicing. That becomes
`files["src/generated.rs"]` — the change touches every emission test, but each one gets simpler.

`tests/execution.rs` compiles emitted code with a single `rustc` invocation. It will write the
files into a directory and compile `lib.rs` as the crate root, which is what the build pipeline
does — closer to what ships than concatenating for the test's convenience would be.

The emission snapshots currently strip the embedded runtime by string surgery, so that editing a
comment in `runtime.rs` does not force a snapshot review. With the runtime in its own file, the
snapshot becomes `generated.rs` alone and the workaround disappears.

## Risks / Trade-offs

* **`wrap_pyfunction!` across a module boundary** → The macro takes a path, so the wrappers need
  to be visible to `lib.rs`. Keeping `register` in `bindings.rs` avoids naming each wrapper from
  another module at all; only `register` needs to be `pub(crate)`. Worth confirming early, since
  it shapes the emitted code.
* **Four files where there was one, for a one-function project** → More files to open, but each is
  the answer to one question, and three of the four are identical in every project. The current
  arrangement optimises for the tool writing it rather than the person reading it.
* **Internal API break with no user-facing change** → `Backend::emit` and `compile_unit` both
  change shape. Nothing outside this repository consumes either, and both are compile-time errors
  rather than silent behavior changes.
* **Test churn is broad but shallow** → Every emission test changes, which makes the diff large
  and easy to skim past. Landing the `emit` signature change as its own commit keeps the
  mechanical part separable from the arrangement it enables.
* **Ordering dependency is not enforced by tooling** → `openspec validate` accepts a MODIFIED
  block for a capability that does not exist yet, so nothing will stop this being applied before
  `add-deferred-quick-wins`. It is stated in the proposal; the check is a human one.

## Migration Plan

Existing `.compylr/` directories hold a `crate/src/lib.rs` from the old layout.

Left alone, they would stay that way: fingerprints are computed over the IR, not the output, so an
unchanged project skips the build and never rewrites `src/`. The old single file would sit there
being read by someone who then wonders why the documented layout does not match what they see. It
would never be *compiled* against — a skipped build compiles nothing — so this is a
confusing-artifact problem rather than a correctness one, which makes it easy to dismiss and
annoying to hit.

The pipeline already has the mechanism for this: `_STATE_VERSION`, whose stated purpose is that
"a future layout change is detected rather than misread". **Bump it.** Old state is then ignored,
the next run rebuilds once, and `src/` is rewritten in the new shape. One cold build per project,
paid once, in exchange for never showing anyone a layout that no longer exists.
