## 1. Workspace split (behavior-preserving)

- [x] 1.1 Convert the repo root to a Cargo workspace with a shared `[workspace.dependencies]`
      section, keeping `compylr` as the member that builds the `cdylib` so `maturin develop` and
      `compylr._core` are unaffected
- [x] 1.2 Create `compylr-diagnostics` and move `src/span.rs` into it verbatim, re-exporting `Span`
      and `LineColumn`; keep `LowerError`'s span behavior identical
- [x] 1.3 Create `compylr-ir` and move `src/ir.rs` into it verbatim, including `returns_on_all_paths`
      and the fingerprint; verify `tests/serialization.rs` passes with only import-path edits
- [x] 1.4 Create `compylr-core` and move `src/backend/mod.rs`'s `Backend` trait, registry, and
      `BackendError` into it; `format_source` moves with it for now
- [x] 1.5 Create `compylr-frontend-python` and move `src/frontend.rs`, `src/lower.rs`, and the
      lowering half of `src/error.rs` into it; move the four vendored ruff path dependencies onto
      this crate alone
- [x] 1.6 Create `compylr-backend-rust` and move `src/backend/rust.rs` and `src/backend/runtime.rs`
      into it
- [x] 1.7 Create `compylr-bridge-python-rust` and move `src/backend/bindings.rs` into it, depending
      on `compylr-backend-rust` for type spellings
- [x] 1.8 Create `compylr-cli` and move `src/main.rs` into it; keep the binary name `compylr` and
      every `--emit` mode working
- [x] 1.9 Leave `src/bridge.rs` in the `compylr` cdylib crate, re-exporting the workspace crates so
      `compylr::{ir, lower, backend}` paths used by tests still resolve or are updated in place
- [x] 1.10 Confirm `cargo build -p compylr-backend-rust` does not compile ruff or PyO3, and assert
      this as a test or a documented check rather than a one-time observation
- [x] 1.11 Run the full suite unchanged (`cargo test`, `cargo clippy --workspace --all-targets -- -D
      warnings`, `pytest`) and confirm no fixture, snapshot, or diagnostic text moved

## 2. Frontend as a component

- [x] 2.1 Write tests for frontend-name resolution covering implemented, reserved, and unknown, and
      for branching on the failure kind without matching message text
- [x] 2.2 Define the `Frontend` trait in `compylr-core`: source text in, `Unit` out, plus the
      frontend's name and its required guarantees
- [x] 2.3 Add the frontend registry with `python` implemented, mirroring the backend registry's
      `Entry { name, impl: Option<..> }` shape. `typescript`, `go`, and `cpp` are reserved here
      too, rather than "no reserved names yet" as first written: the spec requires the reserved
      answer to be one of three, and with no reserved name it could not be exercised
- [x] 2.4 Implement `Frontend` for the Python frontend and route `compylr::compile` and the CLI
      through the registry instead of calling lowering directly
- [x] 2.5 Assert that `compylr-core`, `compylr-ir`, and `compylr-backend-rust` contain no Python
      keyword, spelling, or parser dependency

## 3. Host bridge as a pair

- [ ] 3.1 Write tests for pair resolution: a bridged pair succeeds, an unbridged pair fails naming
      both languages, and the failure is distinguishable by kind from an unimplemented target
- [ ] 3.2 Define the `HostBridge` trait in `compylr-core`, keyed by `(source, target)`, and a
      registry for it
- [ ] 3.3 Move `Backend::emit_python_extension` off the `Backend` trait and re-home it as the
      `(python, rust)` bridge implementation
- [ ] 3.4 Update `compylr::compile`, `src/bridge.rs`, and the CLI's crate emission to resolve the
      bridge by pair; confirm `--emit rust` still works with no bridge involved
- [ ] 3.5 Assert the bridge crate does not depend on a Python parser, and that generating a binding
      layer from a deserialized unit is identical to generating it from one in memory

## 4. Declared semantics on IR nodes (fingerprint-changing)

- [ ] 4.1 Write tests that two integer-division nodes declaring different rounding modes are
      distinguishable, survive serialization, and produce different fingerprints
- [ ] 4.2 Replace `BinOp::FloorDiv` with integer division carrying `Rounding::{TowardNegInf,
      TowardZero}`, `Mod` with remainder carrying `RemSign::{Divisor, Dividend}`, and `TrueDiv` with
      division carrying an explicit promotion
- [ ] 4.3 Update the Python frontend to declare Python's meanings on every operator it lowers, with
      tests asserting the declaration rather than the operator name
- [ ] 4.4 Update the Rust backend to emit from the declared mode, with execution tests for both
      rounding modes and both remainder conventions, including the negative-operand cases
- [ ] 4.5 Move `Ty::python_name` and `BinOp::python_symbol` out of `compylr-ir` into the Python
      frontend; give the IR a neutral `Display`; confirm every diagnostic still quotes Python
      spellings unchanged
- [ ] 4.6 Add `producing frontend` and `required guarantees` to `Unit`, serialized and round-tripped
- [ ] 4.7 Rename `Expr::Range` documentation to state its own contract (start, stop, non-zero step,
      half-open) with no reference to Python's defaulting rules
- [ ] 4.8 Record in `CLAUDE.md` and the README that this release invalidates every cached build once,
      and confirm the existing compiler-version check in build state handles it

## 5. Verification and the pass pipeline

- [ ] 5.1 Write tests for a well-formed unit passing verification, a unit referencing an unknown
      callee failing it, and the failure being identical regardless of recorded frontend
- [ ] 5.2 Implement the IR verifier in `compylr-core` as an unconditional stage between lowering and
      emission
- [ ] 5.3 Define the `Pass` interface and an ordered, named, configurable pipeline; make the pass
      names that ran reportable to the caller
- [ ] 5.4 Wire pair-directed passes to run after agnostic passes, with a test that a pass registered
      for one pair does not run for another and that an unregistered pair compiles fine
- [ ] 5.5 Assert the fingerprint is taken pre-optimization and is identical with passes enabled and
      disabled

## 6. Constant folding

- [ ] 6.1 Write tests: integer addition folds; a promoting division of two integer literals folds to
      a float literal; `7 // -2` folds differently under each rounding mode; a non-literal operand is
      untouched
- [ ] 6.2 Write tests that division by zero and an overflowing constant are left in place so the
      runtime failure still reaches the caller
- [ ] 6.3 Implement folding for the arithmetic and comparison forms, reading semantics off each node
- [ ] 6.4 Confirm the folded form appears in the written IR artifact, and review the emitted-Rust
      snapshot churn as expected rather than as a regression

## 7. Guarantee negotiation and post-processing

- [ ] 7.1 Write tests for a covered combination compiling and an uncovered one failing before
      emission with the missing guarantee named
- [ ] 7.2 Implement `RequiredGuarantees` / `PreservedGuarantees` with the initial three members and
      the pre-emission check in `compylr-core`
- [ ] 7.3 Declare Python's requirements on the Python frontend and Rust's preservation on the Rust
      backend, asserting the Python/Rust pair is covered by test rather than by inspection
- [ ] 7.4 Move formatting out of emission into an explicit post-processing hook, keeping it
      unconditional, keeping the missing-formatter fallback, and keeping emission free of I/O
- [ ] 7.5 Add a test that a guarantee-violating build setting (wrapping arithmetic) is not applied by
      default and is reportable when withheld

## 8. Conformance corpus

- [ ] 8.1 Build an IR corpus, authored as IR rather than as source, covering every statement form,
      expression form, and type
- [ ] 8.2 Add a harness that runs every corpus entry through every backend the registry reports as
      implemented, enumerated from the registry — never from a hand-maintained list
- [ ] 8.3 Add a test that fails if an IR node form has no corpus entry, so the corpus cannot drift
      behind the IR

## 9. Build state and Python package

- [ ] 9.1 Record the pass configuration in build state alongside the compiler version, and rebuild
      when it differs
- [ ] 9.2 Include the target language and pass configuration in the generated module's name so two
      configurations can be loaded in one process without colliding
- [ ] 9.3 Confirm `compylr.initialize`, `@c.compyle`, `COMPYLR_DISABLE`, `compylr compyle`, and
      `_core.pyi` are unchanged; run `pytest`, `ruff check python/`, and `mypy python/compylr`
- [ ] 9.4 Rebuild and run the demo end to end (`uv sync`, `uv run compylr compyle src`, `uv run
      python -m nth_prime 25`, `uv run pytest`) after `rm -rf demo/.compylr`, and re-run the
      benchmark to confirm no performance regression from the split
- [ ] 9.5 Re-commit the regenerated `demo/.compylr/{ir,crate}` artifacts, keeping `target/` and
      `dist/` excluded

## 10. Documentation and close-out

- [ ] 10.1 Update `README.md`'s module-layout table, capability list, and every referenced path for
      the workspace, until `tests/readme.rs` passes
- [ ] 10.2 Update the README prose for the frontend/backend/bridge model, the pass pipeline, and the
      operator-semantics change
- [ ] 10.3 Update `CLAUDE.md`: the workspace commands (`cargo test`, `cargo clippy --workspace`,
      the `cargo llvm-cov` ignore regex, `maturin develop`), the two-PyO3-roles section, and the
      conventions describing where semantics live
- [ ] 10.4 Hand-edit the Purpose sections of `openspec/specs/rust-backend/spec.md` and
      `openspec/specs/python-frontend/spec.md`, which name Python and parsing respectively and are
      not reachable through a delta
- [ ] 10.5 Run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test`,
      and `cargo llvm-cov` with the venv deactivated; confirm coverage has not dropped
