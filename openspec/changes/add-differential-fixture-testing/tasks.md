## 1. The driver format and the shared runner

- [x] 1.1 Write tests for the driver format in `python/tests/test_drivers.py`: a driver declares its
      calls as literal data readable with `ast.literal_eval`; a free-function call names a member and
      its arguments; a class call names constructor arguments and an ordered sequence of method
      calls; a malformed driver fails with a message naming the driver and what was wrong
- [x] 1.2 Add `python/fixtures/drivers/_runner.py`: read a driver's declaration, invoke the calls in
      order against a supplied module, and return the results. It returns *values*, not text — D2 —
      so the boundary tier can compare objects
- [x] 1.3 Add the canonical transcript renderer beside it: JSON, mapping keys sorted, sets as sorted
      arrays, tuples as arrays, one fixed float representation. Write the tests first, one per `Ty`,
      including a mapping whose insertion order differs from its sorted order
- [x] 1.4 Add the float tolerance as one named constant, read by both the runner and the renderer,
      and assert it matches the one `demo/src/algorithms/__main__.py` already uses — D4
- [x] 1.5 Add `python/fixtures/drivers/` to the linted and type-checked paths in `pyproject.toml`,
      and confirm `python/fixtures/accepted/` and `rejected/` stay excluded
- [x] 1.6 `ruff check`, `ruff format --check`, `ty check`, `pytest`; commit

## 2. A driver for every accepted fixture

- [x] 2.1 Write the check first, in `crates/compylr-host-python/tests/fixtures.rs`: every fixture in
      `python/fixtures/accepted/` has exactly one driver, and the suite fails naming any that does
      not. Derive both lists from their directories, never from a literal list
- [x] 2.2 Add the coverage check beside it: every function and every class a fixture defines is named
      by its driver. Read the members from the lowered unit, so the check cannot drift from what the
      fixture actually declares
- [x] 2.3 Write one driver per accepted fixture. Choose inputs that reach the boundary values each
      fixture's constructs distinguish — negative operands for `division.py` and any indexing, an
      empty collection, a non-ASCII string wherever length or text is involved, and the recursive
      cases in `calls.py` and `call_inference.py`
- [x] 2.4 Confirm every driver produces at least one line of output under CPython, and that running
      it twice produces the same transcript
- [x] 2.5 `ruff check`, `pytest`, `cargo test --workspace`; commit

## 3. The translation tier

- [x] 3.1 Write the tier's tests first in `crates/compylr-host-python/tests/differential.rs`: for one
      fixture, the transcript from generated Rust equals the transcript CPython produced from the
      same driver; a deliberately corrupted expected transcript fails and the failure names the
      fixture and shows both
- [x] 3.2 Add the `Ty`-directed JSON renderer used to build the harness `main` — D3. Table-driven
      tests over every `Ty`, asserting the Rust rendering of a value matches the Python rendering of
      the same value, in the shape of the existing test that keeps `runtime.rs`'s mirrored
      `IndexOrigin` in step with the IR's
- [x] 3.3 Build the harness on `execution.rs`'s existing pattern: emit the whole crate into
      `$CARGO_TARGET_TMPDIR`, write `src/main.rs` around it, `rustc`, run, capture stdout. Deny
      warnings, and on failure print the generated source as `execution.rs` already does
- [x] 3.4 Drive it over the whole accepted corpus, enumerated from the directory. Group fixtures the
      way `emit_quality.rs` already groups them so cross-source calls resolve
- [x] 3.5 Make a missing `rustc` the only skip, and make that skip name the missing tool. A fixture
      that fails to translate, build, run, or agree is a failure
- [x] 3.6 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 4. The boundary tier

- [ ] 4.1 Write the tier's tests first in `python/tests/test_differential.py`, marked `slow` and
      toolchain-gated, following `test_end_to_end.py`'s existing `pytestmark`
- [ ] 4.2 Build the whole accepted corpus as **one** unit and one extension, as a real project is
      built — not one build per fixture. Resolve any name collision by renaming inside the fixture
      and note it in the fixture
- [ ] 4.3 Produce the interpreted results in a separate process with `COMPYLR_DISABLE=1`, so a
      marked member calling another marked member is interpreted all the way down
- [ ] 4.4 Compare *values*, not text — D2: `==` for everything but floats, the named tolerance for
      floats. Assert explicitly that a mapping and a set compare equal regardless of iteration order,
      so nobody later "fixes" this into a text comparison
- [ ] 4.5 Report a disagreement naming the fixture, the call, and both values
- [ ] 4.6 Measure what the tier adds to `make check` and record the number in this change's notes. If
      it is intolerable, give it its own Makefile target rather than dropping it — the decision and
      the measurement both go in the notes
- [ ] 4.7 `pytest`, `make check`; commit

## 5. The inverted guard over the rejection corpus

- [ ] 5.1 Write the test first: a program in `python/fixtures/rejected/` that lowers successfully
      fails the suite, naming the program and the rejection the corpus recorded for it
- [ ] 5.2 Add it to `fixtures.rs` beside the existing completeness guard, deriving the list from the
      directory
- [ ] 5.3 Document in the fixture directory that clearing this failure means moving the program into
      `accepted/` with a driver — never adding an allowance
- [ ] 5.4 `cargo test --workspace`; commit

## 6. The robustness walk

- [ ] 6.1 Write the test first in `crates/compylr-host-python/tests/corpus.rs`: over a corpus of
      Python not written for this compiler, every outcome is a lowered unit or a diagnostic carrying
      a source position; a panic fails; a failure without a position fails
- [ ] 6.2 Assemble the corpus per D5: `python/compylr/`, `demo/src/`, `scripts/`, and the running
      interpreter's standard library located by asking it. Skip a file that does not parse, and count
      it as a parse failure rather than a lowering outcome
- [ ] 6.3 Report the proportion of top-level members that lowered, out of how many. **Do not assert a
      threshold** — D5 and the design's non-goals record why
- [ ] 6.4 Skip cleanly, naming what was missing, when no interpreter can be located
- [ ] 6.5 Fix whatever panics or unlocated errors this finds. If one is large, narrow the corpus
      naming the specific module and record a follow-up rather than weakening the check
- [ ] 6.6 `cargo test --workspace`; commit

## 7. The generated subset matrix

- [ ] 7.1 Extract `Region`, `find_region`, and `replace_region` out of
      `scripts/update_benchmarks.py` into a shared module, leaving that script's behavior unchanged.
      Confirm `./scripts/update_benchmarks.py --check` still passes
- [ ] 7.2 Write tests for the generator first: regeneration is idempotent; a construct appears only
      when a fixture exercising it agreed with CPython; `--check` fails on drift naming what differs
      and measures nothing
- [ ] 7.3 Add `scripts/update_subset.py` in the shape of `py2many`'s `scripts/lang_table.py`: derive
      the matrix from the corpus and the differential results, not from a hand-kept list
- [ ] 7.4 Add the markers to `README.md` and generate the first table
- [ ] 7.5 Extend `crates/compylr-host-python/tests/readme.rs` to cover the new region, so the matrix
      joins what already cannot drift
- [ ] 7.6 Wire `--check` into the Makefile, `.pre-commit-config.yaml`, and the CI workflows — all
      three, so it is not a check people discover in a pull request
- [ ] 7.7 `make check`, `make precommit`; commit

## 8. Close out

- [ ] 8.1 Update `README.md`'s prose where it describes how the repository verifies itself, so the
      two tiers and the robustness walk are named alongside `conformance.rs` and `crate_boundaries.rs`
- [ ] 8.2 Update `CLAUDE.md` **and its identical `AGENTS.md` copy**: drivers are required for new
      fixtures; the rejection corpus has an
      inverted guard; the subset matrix is generated and editing it by hand is editing output
- [ ] 8.3 Run `make demo` and confirm nothing moved — this change alters no answer, so any movement
      is a defect in it
- [ ] 8.4 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`, `make check`; commit
