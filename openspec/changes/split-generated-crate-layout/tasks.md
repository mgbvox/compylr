## 1. Emission returns a file set

- [ ] 1.1 Confirm `add-deferred-quick-wins` has been archived, so the `cli` capability this change modifies actually exists (proposal — Impact)
- [ ] 1.2 Write a test asserting emission returns a map keyed by relative path, and that every key is relative rather than absolute
- [ ] 1.3 Write a test asserting the same unit emits the same file names twice, and that two different units emit the same file names
- [ ] 1.4 Write a test asserting every emitted file is byte-identical across repeated emission and across addition orders
- [ ] 1.5 Change `Backend::emit` and `emit_python_extension` to return `BTreeMap<String, String>` per design.md D1, keeping emission a pure function of the unit
- [ ] 1.6 Land the signature change as its own commit, with the file contents still assembled exactly as today, and confirm `cargo test` compiles

## 2. Split the emitted crate

- [ ] 2.1 Write a test asserting `src/generated.rs` holds the translated functions and nothing else — no helpers, no boundary code, no lint attributes
- [ ] 2.2 Write a test asserting `src/lib.rs` is the same size for a one-function unit and a fifty-function unit, per design.md D2
- [ ] 2.3 Write a test asserting `src/compat.rs` is byte-identical between two unrelated units, since it depends on nothing about the program
- [ ] 2.4 Write a test asserting the boundary wrappers are in `src/bindings.rs` rather than beside the translated functions
- [ ] 2.5 Write a test asserting `src/lib.rs` declares every other emitted file, so none is dead weight on disk
- [ ] 2.6 Emit `src/compat.rs` from the existing runtime source, unwrapped from its enclosing module
- [ ] 2.7 Emit `src/generated.rs` holding only the translated functions
- [ ] 2.8 Emit `src/bindings.rs` with the wrappers, the error mapping, and a `register` entry point
- [ ] 2.9 Emit `src/lib.rs` with the lint allowances, module declarations, and a `#[pymodule]` delegating to `register`
- [ ] 2.10 Confirm `wrap_pyfunction!` resolves with the wrappers behind `register` in another module, per design.md's first risk — check this early, since it shapes the emitted code

## 3. Nothing generated changes

- [ ] 3.1 Write a test asserting the concatenation of the emitted files contains the same helper and function definitions the single file previously did
- [ ] 3.2 Write a test asserting unit fingerprints are unchanged by this change, since they are computed over the IR
- [ ] 3.3 Verify the emitted crate still compiles with warnings denied, for every accepted fixture
- [ ] 3.4 Verify `tests/execution.rs` still reports identical values for the signed-operand table, so semantics are demonstrably untouched

## 4. The bridge reports files

- [ ] 4.1 Write tests asserting `compile_unit` reports each generated file under its own relative path, and that no path is absolute
- [ ] 4.2 Replace `target_source` with `target_sources` on the compiled-unit type and update `_core.pyi`
- [ ] 4.3 Write a pytest asserting the reported mapping contains the four expected files

## 5. The build pipeline writes and prunes

- [ ] 5.1 Write tests asserting every reported file is written under `crate/src/` at its relative path
- [ ] 5.2 Write a test asserting a file written by a previous build and not by this one is removed, per design.md D4
- [ ] 5.3 Write a test asserting `Cargo.toml`, `.cargo/config.toml`, and `target/` survive a rebuild — pruning is scoped to `src/`, and losing `target/` would make every build cold
- [ ] 5.4 Implement writing the file set and clearing `src/` beforehand
- [ ] 5.5 Bump `_STATE_VERSION` so a project built under the old layout rebuilds once rather than leaving a stale single file on disk, per design.md — Migration Plan
- [ ] 5.6 Write a test asserting state recorded under the previous version is ignored, forcing that rebuild

## 6. CLI

- [ ] 6.1 Write a test asserting `--emit rust` prints the translated functions only, without helpers, wrappers, or the crate root
- [ ] 6.2 Write tests asserting `--emit crate --out DIR` writes every file at its relative path, and that the result compiles
- [ ] 6.3 Write a test asserting the crate form requires a destination and reports clearly when one is missing
- [ ] 6.4 Write a test asserting a destination that does not exist is created
- [ ] 6.5 Write a test asserting the crate form writes no source to the output stream, and that it invokes no toolchain
- [ ] 6.6 Implement `--emit crate` and `--out` per design.md D5

## 7. Tests and snapshots

- [ ] 7.1 Replace the marker-and-slice helpers in `tests/emit.rs`, `tests/emit_quality.rs`, and `tests/docstrings.rs` with a lookup by file name
- [ ] 7.2 Change `tests/execution.rs` to write the files to a directory and compile `lib.rs` as the crate root, matching what the build pipeline does
- [ ] 7.3 Snapshot `src/generated.rs` directly, removing the string surgery that stripped the embedded runtime
- [ ] 7.4 Review the regenerated snapshots, confirming they now hold only translated functions

## 8. Verification

- [ ] 8.1 Run `cargo fmt`, `cargo clippy -p compylr --all-targets -- -D warnings`, and `cargo test`, twice, confirming the suite is stable across runs
- [ ] 8.2 Run `pytest`, `ruff check python/`, and `mypy python/compylr`
- [ ] 8.3 Confirm Rust coverage over `src/` still exceeds 80%
- [ ] 8.4 Compile and call a decorated function end to end in a scratch project, and read `.compylr/crate/src/generated.rs` to confirm it opens on the translated code
- [ ] 8.5 Record the line count of `generated.rs` for a one-function project against the 238-line `lib.rs` it replaces, so the improvement is a number rather than a claim
- [ ] 8.6 Update the README's artifact listing and layout section to the four-file crate
- [ ] 8.7 Update `CLAUDE.md`'s current state and the `--emit` commands
- [ ] 8.8 Run `openspec validate split-generated-crate-layout --strict` and confirm every scenario in all four delta specs has a passing test
