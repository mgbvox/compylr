## 1. Declared container semantics in the IR

- [ ] 1.1 Write tests that two subscript nodes declaring different index origins, and two length
      nodes declaring different text units, are distinguishable, survive serialization, and produce
      different fingerprints
- [ ] 1.2 Add `IndexOrigin::{FromEitherEnd, FromStart}` and `TextUnits::{CodePoints, Utf8Bytes,
      Utf16Units}` to `compylr-ir`; carry them on `Expr::Subscript` and `Expr::Len`, documenting in
      each that the mode is inert for the operand kinds it does not describe
- [ ] 1.3 Split `Expr::Len` out of the `Neg | ToFloat | Not` match arms it currently shares in
      `rust.rs` and `folding.rs`, and bump `ARTIFACT_VERSION` to 3
- [ ] 1.4 Record in the IR's own documentation which container behaviours are deliberately not
      parameterized — missing keys, mapping iteration, string membership — and why each is a
      difference in the shape of an operation rather than a setting on one

## 2. The Python frontend declares its readings

- [ ] 2.1 Write tests asserting the declaration rather than the node's name, in
      `tests/frontends.rs::declared_meanings`
- [ ] 2.2 Declare Python's readings at the lowering sites as named constants beside `PY_TRUE_DIV`
      and its neighbours, each documenting the language it differs from

## 3. The Rust backend reproduces what the node declares

- [ ] 3.1 Add the two mode enums to `runtime.rs`, self-contained, and take them as parameters on
      `py_index`, `PyIndexable::py_get`, and `PyLen::py_len`
- [ ] 3.2 Emit the declared mode as a literal argument at the subscript and length call sites
- [ ] 3.3 Add a test that the IR's mode enums and the emitted runtime's copies have the same
      variants, since they are two spellings of one decision and cannot be coupled directly
- [ ] 3.4 Rewrite the runtime module doc's "not everything here is neutral yet" paragraph to name
      only what remains, with the reason each remaining item is a conclusion rather than a gap

## 4. Native tests for every runtime helper

- [ ] 4.1 Both index origins, including a negative index, an index past either end, and the
      boundary at exactly the length
- [ ] 4.2 All three text unit readings, against a two-byte character and a character outside the
      basic plane, so all three readings disagree
- [ ] 4.3 `py_key`'s missing-key message, `PySetItem` inserting and overwriting, `PyContains` over
      sequence, mapping, set, and string, and `PyIterate` over all three containers
- [ ] 4.4 Confirm no helper in `runtime.rs` is left without a test, and that its coverage clears
      ~90% from 57.95%

## 5. Execution tests for the readings Python cannot write

- [ ] 5.1 Hand-built IR declaring `FromStart` and `Utf8Bytes`, compiled and run, in
      `tests/execution.rs::modes_python_cannot_write`
- [ ] 5.2 A test that the same program under each reading produces different output, so a backend
      that ignored the mode would fail rather than pass silently

## 6. Corpus coverage by emission position

- [ ] 6.1 Extend the corpus with entries placing each context-sensitive statement form in a
      constructor, both method receiver kinds, and both loop kinds
- [ ] 6.2 Replace the string-matching coverage check with a Rust walk recording `(form, position)`
      pairs, against a table of which forms are legal in which position
- [ ] 6.3 Verify by deletion that the new check reports the specific missing pairs, the way the
      current one was verified

## 7. Precompile imports packages the way the runtime does

- [x] 7.1 Write failing tests: a package whose `__init__.py` imports a sibling relatively; a nested
      subpackage; and a subpackage whose name sorts before `__init__.py`
- [x] 7.2 Register a synthetic `_compylr_precompile` root package, and load `__init__.py` with
      `submodule_search_locations` so it becomes a genuine package
- [x] 7.3 Create every missing ancestor on demand, removing the dependency on enumeration order.
      Both halves were needed: on-demand ancestors make the *name* resolve, and `_module_files`
      importing a package's own module before anything below it makes the *contents* exist. The
      ordering test failed with only the first
- [x] 7.4 Assert in `python/tests/test_demo.py` that precompiling the demo reports zero import
      failures, which it currently reports two of with nothing noticing

## 8. Documentation and close-out

- [ ] 8.1 Update the README's subset section for the container semantics, and note the one-time
      rebuild
- [ ] 8.2 Update `CLAUDE.md`'s conventions for what the IR now declares and what it deliberately
      does not
- [ ] 8.3 Rebuild the demo from scratch, re-run the benchmark, and re-commit `demo/.compylr`
- [ ] 8.4 Run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`, and `cargo llvm-cov` with the venv deactivated; confirm the total
      has not fallen below 87.40%
