## 1. Registry entries

- [ ] 1.1 Write the failing tests: each level resolves; a gated operation reports its level; an
      ungated effectful operation is unaffected.
- [ ] 1.2 Add a declared level to registry entries for effectful operations.
- [ ] 1.3 Add the `logging` entries for `debug`, `info`, `warning`, `error`, and `critical`, each
      effectful, gated, and taking one renderable argument.
- [ ] 1.4 Confirm no IR form is added and `ARTIFACT_VERSION` is unchanged.

## 2. Lowering

- [ ] 2.1 Write the failing tests: a record lowers; obtaining a logger is refused; a second argument
      is refused naming deferred formatting; a configuration call is refused; a mapping argument is
      refused.
- [ ] 2.2 Resolve the logging functions as effectful operations carrying their level.
- [ ] 2.3 Refuse obtaining a logger with a diagnostic naming the supported module-level functions.
- [ ] 2.4 Refuse a second positional argument with a diagnostic naming placeholder-style formatting
      as deferred.
- [ ] 2.5 Refuse logging configuration calls, explaining that configuration belongs to the host.
- [ ] 2.6 Reuse the renderable-type check from output, so mappings, sets, and instances are refused
      by the same rule and not a second copy of it.

## 3. Rust emission

- [ ] 3.1 Write the failing tests: a record emits a facade call; the guard wraps the rendering; a
      disabled record allocates nothing; the signature is unchanged.
- [ ] 3.2 Map the five levels onto the facade's levels exhaustively, with no default arm.
- [ ] 3.3 Emit the level test around the argument's evaluation and rendering.
- [ ] 3.4 Emit the origin derived from the source module as the record's target.
- [ ] 3.5 Declare the facade — and no implementation — in the generated manifest.
- [ ] 3.6 Extend `tests/conformance.rs` to cover a record in all four positions.
- [ ] 3.7 Add the allocation test that fails if rendering escapes the guard.

## 4. The bridge

- [ ] 4.1 Write the failing tests: handlers receive records; a host level change suppresses without
      a rebuild; installation is idempotent; a raising handler does not abort.
- [ ] 4.2 Install a forwarding logging implementation when the extension module is imported.
- [ ] 4.3 Map levels in both directions.
- [ ] 4.4 Carry the origin through so name-keyed host configuration applies.
- [ ] 4.5 Make installation idempotent across several generated modules in one process.
- [ ] 4.6 Contain a failure raised while forwarding; never abort the process.
- [ ] 4.7 Cache the host's effective level on the compiled side and invalidate it when the host
      changes, keeping correctness ahead of the saving.
- [ ] 4.8 Confirm `crate_boundaries.rs` still passes.

## 5. Corpus

- [ ] 5.1 Add an accepted fixture recording at every supported level, with unique member names.
- [ ] 5.2 Add its driver, carrying no expected records.
- [ ] 5.3 Extend the differential harness to capture records structurally — level, message, order —
      from both tiers, never comparing formatted lines.
- [ ] 5.4 Add a case where the effective level suppresses a record, and confirm the absence is
      asserted rather than ignored.
- [ ] 5.5 Add rejected fixtures: obtaining a logger, a second positional argument, a configuration
      call, and recording a mapping.
- [ ] 5.6 Add the derived check that fails when a supported level has no fixture.

## 6. Documentation and checks

- [ ] 6.1 Regenerate the README subset matrix; confirm `--check` passes.
- [ ] 6.2 Update README prose: logging, the one-argument limit, and that host configuration governs.
- [ ] 6.3 Update `CLAUDE.md`: logging added without an IR change, the guard placement, the facade
      versus implementation split, and the deferred formatting.
- [ ] 6.4 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and
      `cargo test --workspace`.
- [ ] 6.5 Run `make check`.
- [ ] 6.6 Run `make demo` and confirm a disabled record in a hot loop costs no measurable time.
