## Why

compylr claims a frontend → IR → backend split, but only the *names* are split: one crate holds
everything, and Python's semantics are welded into three places that a second frontend or backend
would have to fight rather than extend. `BinOp::FloorDiv`/`Mod`/`TrueDiv` **are** Python's
operators — a Go frontend emitting `/` would mean truncation and get flooring; `Ty::python_name`
and `BinOp::python_symbol` put Python spellings on the IR that every backend's diagnostics inherit;
`backend/runtime.rs` and `backend/bindings.rs` are specific to *both* endpoints at once, so they
cannot be reused by either a second target or a second source. The registry already advertises
`typescript`, `go`, and `cpp` as reserved names, which is a promise the current shape cannot keep.

Doing this now is cheap and later is not: the subset is still small, and every requirement added
against a Python-flavoured IR is another one to re-derive when the seam finally has to be cut.

## What Changes

- **BREAKING (IR shape).** Operators stop *being* Python's and start *declaring* what they mean.
  `BinOp::FloorDiv` becomes integer division carrying an explicit rounding mode; `Mod` carries an
  explicit sign convention; `TrueDiv` carries an explicit promotion rule. A frontend states the
  semantics it needs; a backend reproduces exactly those semantics; neither names the other's
  language. This changes the serialized IR, so every existing `.compylr` cache rebuilds once.
- **BREAKING (Rust API).** The single `compylr` crate becomes a Cargo workspace: `compylr-ir`,
  `compylr-diagnostics`, `compylr-core`, `compylr-frontend-python`, `compylr-backend-rust`,
  `compylr-bridge-python-rust`, `compylr-cli`, and the `compylr` cdylib that stays the Python
  extension. Adding a language becomes adding a crate and a registry entry, with no edit to core.
- Python spellings (`Ty::python_name`, `BinOp::python_symbol`) move off the IR and onto the Python
  frontend, which owns how a Python programmer's own syntax is quoted back at them.
- A **frontend registry** appears alongside the backend registry, with the same three-way answer
  (implemented / reserved / unknown) that already serves backends well.
- Host bindings — Python calling generated Rust — are recognised as a property of the **pair**
  `(source, target)`, not of either side, and are registered as their own kind of component. A
  missing pair is a fourth honest answer: compylr can generate the target but cannot call it back.
- A **pass pipeline** over the IR is introduced: a verifier that rejects an ill-formed tree from any
  frontend, plus configurable target-agnostic passes and pair-directed passes. Constant folding
  ships as the one real pass, because folding `7 // -2` is only correct if the pass reads the
  rounding mode off the node — it proves the semantics carrier works.
- Post-generation, target-specific optimization becomes an explicit, *negotiated* backend hook: a
  frontend declares the guarantees its source language needs preserved (integer overflow must trap,
  float arithmetic must not be reassociated), a backend declares what its post-processing preserves,
  and core refuses a combination that would silently break the source language's meaning.
- Behaviour of the supported Python subset does not change. The accepted and rejected fixtures, the
  diagnostics, and the observable behaviour of generated Rust stay as they are; a backend
  conformance harness is added that runs the same IR corpus through every implemented backend.

## Capabilities

### New Capabilities

- `pipeline-architecture`: the workspace's component model — the `Frontend`, `Backend`, and
  `HostBridge` traits, their registries and the three-way (plus missing-pair) resolution, the
  dependency rules that keep languages out of core, and the guarantee negotiation that gates
  target-specific post-processing.
- `ir-optimization`: the pass model over the IR — verification, target-agnostic passes, pair-directed
  passes, pass configuration and ordering, and the requirement that a pass preserve declared
  semantics rather than assume a source language.

### Modified Capabilities

- `ir`: operators carry explicit, declared semantics instead of implicitly carrying Python's; the IR
  no longer renders Python spellings; a unit records which frontend produced it.
- `python-frontend`: becomes a registered frontend implementing the shared trait, owning Python
  syntax spellings and declaring the semantics and guarantees Python requires.
- `rust-backend`: becomes a registered backend that reproduces *declared* semantics rather than
  Python's by name, and declares what its post-processing preserves.
- `python-bindings`: PyO3 generation is re-framed as the `(python, rust)` host bridge, selected by
  the pair, so that a second target or second source resolves through the same mechanism.

## Impact

- **Code.** `src/` is redistributed across workspace crates: `ir.rs` → `compylr-ir`; `span.rs` and
  `error.rs` → `compylr-diagnostics` plus per-crate error types; `frontend.rs` + `lower.rs` (3406
  lines, the largest single file) → `compylr-frontend-python`; `backend/rust.rs` + `backend/runtime.rs`
  → `compylr-backend-rust`; `backend/bindings.rs` → `compylr-bridge-python-rust`; `main.rs` →
  `compylr-cli`; `bridge.rs` stays in the `compylr` cdylib. The vendored ruff path dependencies move
  to the Python frontend crate only, so a Go backend does not build a Python parser.
- **Caches.** The IR fingerprint changes shape, so every project rebuilds once on upgrade. The state
  file already records the compiler version, so this is handled rather than silent.
- **Python package.** No user-visible API change: `compylr.initialize`, `@c.compyle`,
  `COMPYLR_DISABLE`, and the `compylr compyle` console script keep their behaviour. `compylr._core`
  keeps its name and its function signatures; only what it links against moves.
- **Build.** `cargo test`, `cargo clippy`, and `maturin develop` all move to workspace-wide
  invocations; `cargo llvm-cov`'s ignore regex and the README's module-layout table (enforced by
  `tests/readme.rs`) both need updating in the same change.
- **Not in scope.** No second frontend and no second backend is implemented here. `typescript`, `go`,
  and `cpp` stay reserved names; this change is what makes filling them in a self-contained piece of
  work instead of a rewrite.
