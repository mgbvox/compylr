## 1. Reproduce and Compare Before Editing Cargo

- [ ] 1.1 Check out the exact reported `plan/py2many-adoption` state in an isolated worktree or
  source snapshot, allocate an empty `CARGO_TARGET_DIR`, and record the result of
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --lib` together with the Cargo,
  rustc, and rustdoc versions.
- [ ] 1.2 Repeat the clean-target command on the main-based implementation branch, and run the
  package-scoped `compylr-ir` and `compylr-core` doc commands on both states as controls.
- [ ] 1.3 Capture and compare `cargo metadata`, relevant manifest dependency kinds/features, and
  filtered `cargo doc --workspace --no-deps --lib -vv` invocations, including every
  `--extern compylr_ir` path/configuration received by the reported failing crates.
- [ ] 1.4 Update the design's open question with the observed reproduction result and evidence; name
  a Cargo remedy only if the comparison identifies a cause, otherwise record that no speculative
  manifest or feature change is justified.

## 2. Add the Regression First

- [ ] 2.1 Add a failing repository-configuration test that requires `make doc` to retain the exact
  warnings-denied workspace command and requires both the Rust CI documentation job and the
  Rust/Cargo-gated pre-commit hook to route through `make doc`.
- [ ] 2.2 Add the clean-target workspace documentation regression path before changing Cargo or
  check configuration; demonstrate that it detects the affected state when task 1 reproduces the
  failure, or record current main's clean success as the baseline when it does not.
- [ ] 2.3 Commit the regression checkpoint separately so the expected pre-fix failure (or the
  non-reproducing baseline) remains reviewable.

## 3. Apply Only the Evidence-Supported Fix

- [ ] 3.1 If task 1 identifies a dependency-kind, feature-unification, target-name, or artifact
  selection defect, make the smallest Cargo manifest/configuration change that corrects it while
  preserving direct crate tests and all crate-boundary constraints; if no cause reproduces, make no
  Cargo graph change.
- [ ] 3.2 Run the focused clean and warm workspace-doc comparisons and confirm every dependent uses
  current, compatible `compylr-ir` metadata without pinning an artifact hash.
- [ ] 3.3 Commit the Cargo-fix checkpoint separately when task 3.1 produces a change.

## 4. Align Developer and Automation Entry Points

- [ ] 4.1 Keep `make doc` as the single definition of
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --lib`.
- [ ] 4.2 Change the Rust documentation CI job to invoke `make doc` with a dedicated empty
  `CARGO_TARGET_DIR` that is not satisfied by the shared restored target cache.
- [ ] 4.3 Add an applicable pre-commit hook that invokes `make doc`, passes no filenames, and is
  triggered by Rust source or Cargo manifest changes without reaching vendored, inspiration, or
  worktree content.
- [ ] 4.4 Run the repository-configuration test from task 2.1 and commit the aligned Makefile, CI,
  pre-commit, and test changes as one checkpoint.

## 5. Verify the Contract

- [ ] 5.1 From one empty target, run
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --lib`; from a second empty target,
  run `make doc`, and confirm both document every workspace library successfully.
- [ ] 5.2 Populate a target with other workspace builds, rerun the exact Cargo command and
  `make doc`, and use filtered verbose output to confirm the result and local `compylr-ir` metadata
  selection match the clean runs.
- [ ] 5.3 Run `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `make precommit`, and `openspec validate fix-workspace-rustdoc`; resolve every failure.
- [ ] 5.4 Review README and contributor guidance for command drift, update only prose made inaccurate
  by the implementation, and commit the final verification/documentation checkpoint.
