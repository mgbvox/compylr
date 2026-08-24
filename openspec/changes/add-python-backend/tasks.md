## 1. The language crate

- [ ] 1.1 Write the boundary tests first in `crates/compylr-host-python/tests/crate_boundaries.rs`:
      a `compylr-lang-*` crate depends on `compylr-ir` and nothing else; it parses nothing; and it
      joins the stance table so it names only its own language
- [ ] 1.2 Create `crates/compylr-lang-python/` with `compylr-ir` as its only dependency, and add it
      to the workspace
- [ ] 1.3 Move `PYTHON_BEHAVIOR` out of `crates/compylr-frontend-python/src/component.rs` into it.
      **Move, not copy** — leave no re-export, so there is one way to reach the declaration
- [ ] 1.4 Move `crates/compylr-frontend-python/src/spelling.rs` into it unchanged, and repoint the
      frontend's uses. A pure move: no message text changes
- [ ] 1.5 Commit this move **on its own**, with no other edit in it, so `cargo test --workspace`
      before and after is a clean comparison
- [ ] 1.6 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 2. A backend names its translated-source file

- [ ] 2.1 Write the test first: for every implemented backend, asking it for its translated-source
      file returns a path that is present in what it emitted. Enumerate backends from the registry
- [ ] 2.2 Add the declaration to the `Backend` trait in `crates/compylr-core/src/backend.rs`, with
      **no default implementation** — D6
- [ ] 2.3 Implement it in `compylr-backend-rust`, returning `GENERATED_PATH`
- [ ] 2.4 Change `crates/compylr-cli/src/main.rs` so the target-code form asks the backend instead of
      naming `compylr_backend_rust::rust::GENERATED_PATH`. Confirm `crates/compylr-cli/tests/cli.rs`
      still passes unchanged
- [ ] 2.5 `cargo test --workspace`; commit

## 3. The backend skeleton

- [ ] 3.1 Write tests first for type spellings: every scalar, every collection, nesting, and that
      spelling does not depend on the producing frontend
- [ ] 3.2 Create `crates/compylr-backend-python/` depending on `compylr-ir`, `compylr-core`, and
      `compylr-lang-python`. Assert in `crate_boundaries.rs` that it depends on no parser and names
      only Python
- [ ] 3.3 Implement `Backend`: the name, `behavior()` reading `compylr-lang-python`, `preserves()`
      declaring all three guarantees, and the file set `generated.py` / `compat.py` / `__init__.py`
- [ ] 3.4 Implement identifier escaping over Python's keyword set — D5. Test with a member named
      `lambda`, built as IR, since no Python source can produce it
- [ ] 3.5 Register it in `crates/compylr-registry/src/backends.rs`
- [ ] 3.6 Write the negotiation test: `(python, python)` resolves with no guarantee withheld
- [ ] 3.7 `cargo test --workspace`; commit

## 4. Emission

- [ ] 4.1 Write tests first for function emission: name, parameters in order with annotations,
      return annotation, and a docstring in first position when the function carries one
- [ ] 4.2 Emit functions, then every statement form. Work from `tests/conformance.rs`'s
      `Position::admits` table so each form is covered in every position it is legal in — free
      function, constructor, method, and loop body
- [ ] 4.3 Emit every expression form, including the ones with no Python syntax: `Expr::ToFloat`,
      `Expr::TupleIndex`, `Expr::Range` in iterable position, `Expr::Len`, `Expr::Contains`
- [ ] 4.4 Emit classes: `__init__` with annotated attribute declarations, then methods
- [ ] 4.5 Write the determinism tests: emitting the same unit twice is byte-identical, and emission
      reads and writes nothing. Extend `emission_reads_and_writes_nothing` in `crate_boundaries.rs`
      to cover the new crate
- [ ] 4.6 Add `ruff format` as `post_process`, and the test that formatting does not change results
- [ ] 4.7 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 5. Modes that are not Python's

- [ ] 5.1 Write tests first, as hand-built IR — Python has no syntax for these, which is the point:
      rounding toward zero, remainder signed by the dividend, index origin from the start, text
      length in UTF-8 bytes, and each unchecked arithmetic mode
- [ ] 5.2 Write `compat.py`: one helper per non-Python mode, embedded as a constant the way
      `runtime.rs` is — D2
- [ ] 5.3 Emit a direct operator where the declared mode is Python's own, and a helper call where it
      is not. Match on the mode, never on the operation's name
- [ ] 5.4 Assert that a unit declaring only Python's modes emits **no** import, and one declaring any
      other emits exactly the imports it uses
- [ ] 5.5 Run the emitted Python for each non-Python mode and assert the *answer*, not the text — a
      helper that adjusts in the wrong direction looks correct in a string comparison
- [ ] 5.6 `cargo test --workspace`; commit

## 6. Conformance

- [ ] 6.1 Confirm `every_implemented_backend_renders_the_whole_corpus` now covers two backends
      without being edited. If it needed editing, that is a finding — record it in this change's
      notes
- [ ] 6.2 Add the Python sibling of `every_corpus_entry_compiles_for_the_rust_backend`: hand the
      emitted source to an interpreter to parse, and skip naming the missing tool when there is none
- [ ] 6.3 Fix whatever the corpus finds. Every fix is the change earning its keep; record each one in
      the notes, because they are the evidence for whether the IR was neutral
- [ ] 6.4 `cargo test --workspace`; commit

## 7. Round trip and the driver oracle

- [ ] 7.1 Write the round-trip test first: for each accepted fixture, lower it, emit Python, lower
      the result, and compare **fingerprints** — not text, D3
- [ ] 7.2 Name the excluded case explicitly: a unit declaring a mode that is not Python's emits an
      import and does not round-trip. Assert that it still emits valid Python, so the exclusion
      cannot quietly widen
- [ ] 7.3 Write the oracle test: for each accepted fixture, run its driver from
      `python/fixtures/drivers/` against the fixture and against the emitted Python, and require the
      transcripts to match — D4
- [ ] 7.4 Confirm this tier needs no toolchain beyond an interpreter, and say so where it is defined
- [ ] 7.5 `pytest`, `cargo test --workspace`; commit

## 8. Close out

- [ ] 8.1 Add `--backend python` to the CLI's documented backends and confirm
      `compylr --backend python --emit rust f.py` prints the translated Python, and
      `--emit crate` reports the pair unbridged
- [ ] 8.2 Update `README.md`: two backends, the crate layout gains two crates, and what the Python
      backend is for. `tests/readme.rs` enforces the mechanical half — make it pass
- [ ] 8.3 Update `CLAUDE.md` **and its identical `AGENTS.md` copy**: a language's declared semantics
      live in `compylr-lang-*`; adding an IR
      form now costs emission in two backends; `(python, python)` is deliberately unbridged
- [ ] 8.4 Record in this change's notes what the second backend actually found about the IR's
      neutrality — including "nothing", if that is the answer. That finding is the input to
      `add-typed-ir-expressions`
- [ ] 8.5 Run `make demo` and confirm nothing moved: the default backend is unchanged, so any
      movement is a defect in this change
- [ ] 8.6 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`, `make check`; commit
