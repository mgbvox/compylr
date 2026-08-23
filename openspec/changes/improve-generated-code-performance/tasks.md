## 0. The instrument, before anything is measured

- [x] 0.1 Write tests for the benchmark harness: a timing carries a spread, a stated noise floor is
      reported, and a difference inside the noise floor is reported as not resolvable rather than
      as a ratio
- [x] 0.2 Change `demo/src/algorithms/benchmark.py` to report run-to-run spread alongside each
      timing instead of a single best-of figure, and to print the noise floor derived from the
      never-compiled `reference` row
- [x] 0.3 Run `make demo SCALE=4` several times and record the observed spread per workload.
      `sorting.merge_sort` is known to range 160–277us across byte-identical builds; confirm it is
      now visible as unstable rather than reported as a clean number
- [x] 0.4 Record the noise floor in `demo/README.md` as the figure every other row is read against
- [x] 0.5 `cd demo && uv run pytest && uv run ruff check . && uv run mypy src`; commit

## 1. The release profile

- [x] 1.1 Write a test in `compylr-bridge-python-rust` that the generated manifest declares a
      release profile with link-time optimization and one codegen unit, and that it pins no target
      CPU
- [x] 1.2 Add the profile to `cargo_manifest` in `crates/compylr-bridge-python-rust/src/bindings.rs`
- [x] 1.3 Confirm the `.cargo/config.toml` the build writes still pins no target CPU, and add a test
      asserting it — `target-cpu=native` was measured and rejected (design D7), and the assertion is
      what stops it being re-added
- [x] 1.4 `rm -rf .compylr demo/.compylr`, run `make demo SCALE=4`, and record before/after against
      the noise floor from 0.3
- [x] 1.5 Record the build-time cost (roughly 7s to 10s on the demo) alongside the gain
- [x] 1.6 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 2. In-place accumulation

- [x] 2.1 Write emission tests: `x = x + y` on a `str` local appends in place; on an `int` local it
      still performs a checked addition; **`x = y + x` is left alone** — the mirrored form is the
      one that looks like it should work and would produce wrong text
- [x] 2.2 Write execution tests asserting on values, not emitted text: string accumulation in a
      loop produces the same result as today, and integer accumulation still reports overflow
- [x] 2.3 Add a `PyAddAssign` trait to `crates/compylr-backend-rust/src/runtime.rs` with
      implementations for the types `PyAdd` covers, so the choice stays type-directed and the
      backend still never derives an expression's type
- [x] 2.4 Recognise the accumulator shape in `Stmt::Assign` in `rust.rs` and emit through it
- [x] 2.5 `rm -rf .compylr demo/.compylr`, `make demo SCALE=4`; `text.joined` is expected to move
      from roughly 0.9x to roughly 3.8x. Record it
- [x] 2.6 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 3. The hasher becomes a choice

- [x] 3.1 Write tests that the runtime's mapping and set implementations accept a container built
      with a non-default hasher — these fail today, which is the point
- [x] 3.2 Make all ten mapping/set trait implementations in `runtime.rs` generic over the hasher,
      plus the free helper that reads a mapping entry. Assert the runtime stays self-contained: no
      external crate, nothing named that a generated crate could not compile
- [x] 3.3 Add a self-contained non-cryptographic hasher to `runtime.rs` and the container aliases
      generated code uses
- [x] 3.4 Change `rust_ty` in `rust.rs` to emit the aliases, and change dict and set literal
      emission to build through the general construction path — the array conversion exists only
      for the default hasher (design D4)
- [x] 3.5 Confirm no test asserts on mapping or set iteration order; CLAUDE.md forbids it, and this
      is the change that would expose one
- [x] 3.6 `rm -rf .compylr demo/.compylr`, `make demo SCALE=4`; `graphs.bfs_distances` is expected
      to move from roughly 0.7x to roughly 1.4x. Record it
- [x] 3.7 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 4. Moved returns

- [x] 4.1 Write emission tests: a bare local returned in tail position is moved; a `return` of a
      local anywhere else is unchanged; a returned attribute is still copied
- [x] 4.2 Write an execution test that a function returning a collection built in a loop still
      returns the right value
- [x] 4.3 Change the tail branch of `emit_body` in `rust.rs` to move a bare local
- [x] 4.4 Verify against the demo's generated source that all 25 sites moved and the crate still
      compiles
- [x] 4.5 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 5. Borrowed loop variables

- [x] 5.1 Write tests that the runtime's traits accept a reference wherever they accept an owned
      value — these fail today and are what makes 5.3 legal (design D5)
- [x] 5.2 Add blanket implementations over references in `runtime.rs`, delegating to the owned
      implementations
- [x] 5.3 Write emission tests: a read-only loop variable is bound by reference; a loop variable the
      body assigns to is still owned
- [x] 5.4 Change `collection_loop` in `rust.rs` to bind by reference when `is_assigned` says the
      body never writes the loop variable
- [x] 5.5 Confirm every accepted fixture still compiles — blanket implementations can make inference
      ambiguous, and `tests/emit_quality.rs` is what catches it
- [x] 5.6 `rm -rf .compylr demo/.compylr`, `make demo SCALE=4`; record `text.total_length`
- [x] 5.7 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 6. The runtime sweep

- [x] 6.1 Write tests pinning current behaviour first: an out-of-range index reports rather than
      panicking, in both directions; text length is unchanged for non-ASCII input under every units
      setting; a missing mapping key still reports and is not created
- [x] 6.2 Resolve a sequence index once rather than validating and then indexing through a second
      check, without `unsafe`
- [x] 6.3 Add an ASCII shortcut to the code-point length reading, exact when it applies
- [x] 6.4 Fuse the mapping read-modify-write so one key is hashed once rather than three times
- [x] 6.5 Use a known capacity when emitting a collection built by a loop with a known trip count
- [x] 6.6 `rm -rf .compylr demo/.compylr`, `make demo SCALE=4`; `text.word_count`'s body is expected
      to improve by roughly 2.7x. Record it
      - Observed end-to-end after the sweep: 137.53us compiled versus 62.36us interpreted (0.5x),
        with 1% row spread against a 31% reference floor. Boundary conversion still dominates.
- [x] 6.7 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 7. The boundary — documentation and visibility

These carry no design risk and land regardless of whether section 8 does.

- [x] 7.1 Add a workload to the demo whose body does asymptotically less work than converting its
      arguments costs — a search over a sorted list is the clearest — so the shape where compiling
      loses is visible in the benchmark
- [x] 7.2 Confirm the demo already covers a collection of text; `text.word_count` does
- [x] 7.3 Document in `demo/README.md` that a collection parameter costs time proportional to its
      length on **every call**, with the measured per-element figures, and that a function doing
      less work than its arguments cost to convert may be slower compiled
- [x] 7.4 Correct any README prose implying compiled is always at least as fast
- [x] 7.5 `cd demo && uv run pytest && uv run ruff check . && uv run mypy src`; commit

## 8. The boundary — borrowed text parameters

Staged last and separable. **Stop and reassess before starting**: this is the only item that
changes generated signatures, and design D6 says it may become its own change.

- [x] 8.1 Confirm the premise still holds — that a text parameter is never mutated in the accepted
      subset, so borrowing it is always legal
- [x] 8.2 Write execution tests over text parameters: measurement, comparison, membership, and
      passing one into a nested call, all with non-ASCII input
- [x] 8.3 Prototype a borrowed text parameter in the generated bindings and confirm the lifetime
      does not force a change to the uniform result type
- [x] 8.4 If it does force one, stop, record what was learned, and split the rest into its own
      change rather than widening this one. It did not: parameters borrow for the call while text
      results remain owned `String` values in the existing `Result<T, RuntimeError>` shape.
- [x] 8.5 `rm -rf .compylr demo/.compylr`, `make demo SCALE=4`; record every text workload.
      Clean run: `text.joined` 75.61us compiled / 421.34us interpreted (5.6x, 1% spread),
      `text.word_count` 101.12us / 69.30us (0.7x, 23% spread), and `text.total_length`
      64.24us / 33.55us (0.5x, 3% spread), against a 12% reference noise floor. Both modes
      returned the same answer for every workload.
- [x] 8.6 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 9. The regression guard

- [ ] 9.1 Decide which workloads carry a recorded performance property, and record their figures
      with the noise floor they were measured against
- [ ] 9.2 Add a check that fails when a guarded workload regresses beyond the noise floor, and
      confirm it passes repeatedly on an unchanged tree before trusting it
- [ ] 9.3 If it proves flaky, move it out of the default suite rather than loosening it until it
      catches nothing (design, Risks)
- [ ] 9.4 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`; commit

## 10. Documentation and final verification

- [ ] 10.1 Update `CLAUDE.md`: the hasher decision and why it is not a behavior axis, the boundary's
      per-element cost, and that emission changes need `rm -rf .compylr` before measuring
- [ ] 10.2 Update `README.md` where it describes what compiling is worth; `tests/readme.rs` enforces
      the mechanical half
- [ ] 10.3 Record the final table — every workload, before and after, against the noise floor — and
      note that `-C target-cpu=native` was measured and rejected
- [ ] 10.4 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`
- [ ] 10.5 `cargo llvm-cov --workspace --ignore-filename-regex '(vendored/|/main\.rs)'
      --summary-only`, with the venv deactivated, and confirm coverage has not regressed
- [ ] 10.6 `pytest` including slow tests, `ruff check python/`, `mypy python/compylr`
