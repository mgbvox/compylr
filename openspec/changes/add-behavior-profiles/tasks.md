## 1. The behavior model

- [x] 1.1 Write tests for the axis set in `compylr-ir`: exactly six axes, each with a stable
      identifier distinct from its prose, and a test that a stance bundle covering fewer than six
      cannot be constructed
- [x] 1.2 Add `compylr-ir/src/behavior.rs` defining `Axis`, the per-axis stance types, and
      `LanguageBehavior` (one stance per axis, complete by construction). Place it beside
      `guarantee.rs` and for the reason recorded there — a unit holds the modes, and the IR cannot
      depend on the crate that consumes it
- [x] 1.3 Add the resolved `Behavior` type: one stance per axis, plus the accessors lowering will
      use to read a mode for an operation. Re-export both from `compylr-core` the way `Guarantee`
      is re-exported
- [x] 1.4 Write tests for resolution in `compylr-core`: a bare language name sets every axis; an
      unnamed axis inherits the enclosing default rather than a fixed language; a bare name and a
      full per-axis selection of the same language resolve identically; a resolved behavior is
      total
- [x] 1.5 Write tests for resolution failures, branching on a stable category and not on prose: an
      unknown language, a registered-or-reserved language that is not one of this pair, and an
      unknown axis. Assert the pair's two language names appear in each message
- [x] 1.6 Add `compylr-core/src/behavior.rs` with the request type and `resolve`, returning the
      three-way error. Assert `compylr-core` still names no concrete language
- [x] 1.7 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 2. The `Checked` mode on the IR

- [x] 2.1 Write tests: the mode is readable off `Add`/`Sub`/`Mul`/`Div`/`Rem`/`Neg`/`Subscript`;
      two nodes differing only in it are distinguishable; it composes with `DivMode` and `RemSign`
      independently; it survives a round trip; two units differing only in it fingerprint
      differently
- [x] 2.2 Add `Checked::{Reported, Unchecked}` to `compylr-ir::ir` and put it on those seven forms.
      Document `Unchecked` as a statement about the *program* — the program declines to define the
      result — and not about any target, per design D3
- [x] 2.3 Update every construction site in the workspace. Where a match binds an operator, bind
      the mode rather than wildcarding it, so a future backend that ignores it fails to compile
      rather than being silently wrong
- [x] 2.4 Write a test that an artifact at the previous format version is refused, naming both the
      version found and the version expected
- [x] 2.5 Advance the artifact format to version 4. Do not write a v3 reader — design D12
- [x] 2.6 Update `tests/serialization.rs` and any snapshot the shape change moves; confirm
      `demo_coverage.rs`'s `variants_of` scan still reads `BinOp` correctly now that more variants
      carry fields
- [x] 2.7 `cargo fmt --all`, clippy, `cargo test --workspace`; commit

## 3. Each language declares its stance

- [x] 3.1 Write tests that both `Frontend` and `Backend` answer for every axis, and that neither
      declaration names the other language
- [x] 3.2 Add `fn behavior(&self) -> &'static LanguageBehavior` to the `Frontend` and `Backend`
      traits in `compylr-core`. Required rather than defaulted, for the reason `preserves()` is:
      a default of either kind makes the declaration meaningless
- [x] 3.3 Declare Python's stance in `compylr-frontend-python::component`, replacing the five
      constants at the top of `lower.rs` as the source of truth. Assert the new declaration
      reproduces each old constant exactly
- [x] 3.4 Declare Rust's stance in `compylr-backend-rust`: unchecked overflow, truncating division,
      IEEE exact division, remainder taking the sign of the dividend, indexing from the start and
      unchecked, UTF-8 bytes
- [x] 3.5 Extend `tests/crate_boundaries.rs` if needed so a stance declaration cannot become a
      route for one crate to name another language
- [x] 3.6 fmt, clippy, test; commit

## 4. Lowering takes a behavior

- [ ] 4.1 Write tests: the same source under two behaviors differing on one axis produces units
      that differ only in the modes that axis governs; under Python's stance the unit is byte-equal
      to what lowering produced before this change
- [ ] 4.2 Write tests that acceptance is behavior-independent — every fixture in
      `python/fixtures/accepted/` lowers under each behavior, and every fixture in `rejected/` is
      rejected under each with the same diagnostic code
- [ ] 4.3 Write a test that `a / b` on integers is typed float under every behavior, and that
      `xs[-1]` lowers successfully under `index` taking Rust's stance (design D10)
- [ ] 4.4 Wrap `Names<'a>` in a `Copy` carrier holding the behavior alongside it, and thread it
      through `lower_expr` and its callers. Mechanical — no logic moves (design D14)
- [ ] 4.5 Add the behavior to `Ctx<'a>`; delete `PY_TRUE_DIV`, `PY_FLOOR_DIV`, `PY_MOD`,
      `PY_INDEX_ORIGIN`, and `PY_TEXT_UNITS`; set every mode from the behavior
- [ ] 4.6 Add the behavior parameter to `Frontend::lower` and to the Python frontend's
      implementation, per source rather than per call, so one unit can hold members lowered under
      different behaviors
- [ ] 4.7 fmt, clippy, test; commit

## 5. Guarantees become a property of the program

- [ ] 5.1 Write tests: a unit whose arithmetic is unchecked does not require overflow reporting; a
      unit under Python's stance requires exactly what it required before; every unit requires
      float ordering regardless of behavior; a hand-built corpus unit still requires nothing
- [ ] 5.2 Write a test that a target option whose broken guarantee a unit has waived is no longer
      reported by `withheld_by_default` for that unit
- [ ] 5.3 Derive `Origin.requires` by walking the unit's operations rather than copying
      `Frontend::requires()`. Redefine `Frontend::requires()` as what the language requires under
      its own stance and keep it — it is what the negotiation's message names (design D8)
- [ ] 5.4 fmt, clippy, test; commit

## 6. Folding reads the mode

- [ ] 6.1 Write tests: a constant expression that would overflow folds to a reported error under
      `Reported` and is left unfolded under `Unchecked`; the same for a zero divisor; an
      `Unchecked` fold that cannot fail still folds
- [ ] 6.2 Update `compylr-core::folding` to read `Checked` alongside the rounding mode it already
      reads (design D11)
- [ ] 6.3 fmt, clippy, test; commit

## 7. Native emission in the Rust backend

- [ ] 7.1 Write emission tests asserting the emitted *form*, which is legitimate here because the
      form is the property: an unchecked integer add with a known expected type emits a bare `+`
      with no `?`; a reported one emits the helper unchanged; unchecked truncating division emits
      `/`; unchecked remainder taking the sign of the dividend emits `%`; unchecked indexing from
      the start emits native indexing; UTF-8 length emits Rust's own length
- [ ] 7.2 Write a test that a flooring division declaring `Unchecked` still emits a flooring
      helper, since Rust's `/` does not floor — the combination is reachable and is the likeliest
      thing to get wrong
- [ ] 7.3 Write execution tests over a hand-built unit (no Python involved) covering both stances
      of every axis, so a backend defect cannot hide behind the Python frontend's choices
- [ ] 7.4 Add the infallible `NativeAdd`/`NativeNum` shims to `runtime.rs`, with unit tests in
      `tests/runtime.rs`. Keep the file self-contained — it is embedded verbatim into every
      generated crate
- [ ] 7.5 Emit natively in `emit_binop`, `emit_expr`'s `Neg`, `Subscript`, and `Len` where the
      node's modes are Rust's own; fall back to the infallible shim where the expected type is
      `Ty::Unit` (design D6)
- [ ] 7.6 Write a test that an all-unchecked function's signature is the same fallible one it would
      have had under the default behavior, and that its body contains no `?` (design D7)
- [ ] 7.7 Extend the `(form, position)` matrix in `tests/conformance.rs` with a stance dimension,
      scoped as design D15 states: every form carrying a mode, in every position it is legal in,
      under both stances of its axis
- [ ] 7.8 fmt, clippy, test; commit

## 8. Host bindings and CLI

- [ ] 8.1 Write tests that `_core` accepts a behavior per source, that an omitted one is the source
      language's stance, that a cross-behavior call resolves, and that the same source under two
      behaviors reports two different fingerprints
- [ ] 8.2 Change `compile_unit` to take `(source, behavior)` pairs. Leave `validate_source`
      unchanged — acceptance does not depend on behavior (design D9)
- [ ] 8.3 Add `check_behavior(frontend, backend, mapping)`, mirroring `check_backend`, carrying a
      stable failure category so the Python side branches without matching prose
- [ ] 8.4 Update `_core.pyi`
- [ ] 8.5 Write CLI tests: no behavior means the source language's stance; a language name sets
      every axis; per-axis assignments work; an invalid language and an unknown axis each exit
      unsuccessfully before parsing; the IR and the Rust emitted for one file under a non-default
      behavior agree with each other
- [ ] 8.6 Add `--behavior` to `compylr-cli`, accepting a language name or comma-separated
      `axis=language` assignments
- [ ] 8.7 fmt, clippy, test; commit

## 9. The Python API

- [ ] 9.1 Write tests for `Behavior`: six fields, `None` means inherit, an unknown field is
      rejected listing the valid ones, and a bare language name normalises to every field set
- [ ] 9.2 Write tests for inheritance: a per-member behavior naming one axis merges into the
      manager's rather than replacing it, both when the manager's default is the source language
      and when it is the target's
- [ ] 9.3 Write tests for validation at the decorator and at `initialize`, covering all three
      failure categories, and for `initialize` refusing a second call whose behavior differs
- [ ] 9.4 Write tests that two members with different behaviors build one artifact and both run,
      that a cross-behavior call gives each side its own rounding for `-7 // 2`, and that a mixed
      *backend* is still refused
- [ ] 9.5 Write a test that changing a member's behavior rebuilds on the next run and that an
      unchanged one does not
- [ ] 9.6 Add `Behavior` to `_config.py`; add `behavior` to `Settings` with per-field inheritance in
      `override`; validate in `__post_init__` through `check_behavior` (design D13)
- [ ] 9.7 Add `behavior` to `initialize` and to `Manager.compyle`; pass each member's behavior
      through to `compile_unit`; export `Behavior` from `compylr/__init__.py`
- [ ] 9.8 `ruff check python/`, `mypy python/compylr`, `pytest`; commit

## 10. Prove the default path did not move

- [ ] 10.1 Before the emission changes land, snapshot the emitted Rust for every fixture in
      `python/fixtures/accepted/`; after they land, diff against it and require it byte-identical
      under the default behavior
- [ ] 10.2 Add a permanent test that lowering every accepted fixture under Python's stance produces
      the IR the frontend produced before behavior selection existed, keyed on fingerprints
- [ ] 10.3 Confirm `tests/emit_quality.rs` and `tests/fixtures.rs` still enumerate the fixture
      directory rather than a list, and that neither has grown a hardcoded behavior
- [ ] 10.4 fmt, clippy, test; commit

## 11. Documentation and the demo

- [ ] 11.1 Add the axis table to `README.md` — flag, what the source language means, what the
      target means — beside the existing type and operator tables, and extend `tests/readme.rs` to
      enforce that the table lists exactly the axes the code defines
- [ ] 11.2 Update `README.md` prose: `behavior` on `initialize` and on the decorator, the default,
      the validation rule, and that mixed behavior in one project is allowed where a mixed backend
      is not
- [ ] 11.3 Update `CLAUDE.md`: the axis set, that `Unchecked` is a statement about the program, the
      artifact version at 4, and the standing warning about `rm -rf .compylr` when emission changes
- [ ] 11.4 **Prerequisite for everything below.** Make the benchmark report a measure of
      run-to-run spread alongside each timing instead of a single best-of figure, and record the
      resulting noise floor. `sorting.merge_sort` currently varies from 160us to 277us across
      *byte-identical* builds, so a behavior delta under roughly 30% cannot be told apart from the
      harness and 11.7 is unachievable until this lands. If
      `improve-generated-code-performance` has already done this, verify it and move on
- [ ] 11.5 Add a Rust-behavior build of one demo algorithm and report it in the benchmark alongside
      the interpreted and default-behavior timings
- [ ] 11.6 State in `demo/README.md` what the Rust-behavior build gives up for that algorithm, and
      assert both builds produce the documented answer
- [ ] 11.7 `rm -rf .compylr demo/.compylr`, run `make demo`, and record the three timings in the
      change's notes so the claim is measured rather than asserted — reporting each against the
      noise floor from 11.4, and saying the difference was not resolvable where it falls inside it
- [ ] 11.8 `cd demo && uv run pytest && uv run ruff check . && uv run mypy src`; commit

## 12. Final verification

- [ ] 12.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`
- [ ] 12.2 `cargo llvm-cov --workspace --ignore-filename-regex '(vendored/|/main\.rs)'
      --summary-only`, with the venv deactivated, and confirm coverage has not regressed
- [ ] 12.3 `pytest` including slow tests, `ruff check python/`, `mypy python/compylr`
- [ ] 12.4 Confirm `python/fixtures/` was never linted, and that no fixture was edited by a
      formatter during this change
- [ ] 12.5 Re-read the delta specs against what was built and reconcile any drift before archiving
