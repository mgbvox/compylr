## Context

See `proposal.md` for motivation and `specs/workspace-documentation/spec.md` for the contract.
The handoff records a workspace-only rustdoc failure on `plan/py2many-adoption`: current dependents
could not see behavior-profile items in `compylr-ir`, while ordinary builds and package-scoped docs
passed. That branch did not add a Cargo-manifest change relative to its base.

The failure is not present at current `main` (`cf06649`). Both the exact command and `make doc`
succeed. A second run with an empty isolated target and Cargo `-vv` shows one `compylr-ir` package
in metadata and the same freshly built `libcompylr_ir-f57a04404494c192.rmeta` passed to
`compylr-core`, `compylr-backend-rust`, `compylr-frontend-python`, the bridge, registry, and host.
The host manifest does name the local workspace crates as both normal and dev dependencies, but
that fact alone does not establish the reported failure's cause.

The repository currently spells the same Cargo doc command in the Makefile and CI. The CI setup
restores a shared Cargo target cache, and pre-commit has no rustdoc hook. Any solution must preserve
the crate-boundary constraints and must not add dependencies to `compylr-ir` or
`compylr-diagnostics` merely to influence Cargo scheduling.

## Goals / Non-Goals

**Goals:**

- Identify the affected branch's actual rustdoc dependency inputs before changing manifests.
- Make a clean workspace-wide documentation build the authoritative regression path.
- Give local, pre-commit, and CI checks one command definition that cannot drift silently.
- Apply the smallest cause-based Cargo fix if, and only if, the comparison demonstrates one.

**Non-Goals:**

- Changing compiler behavior, generated Rust, the public Python API, or supported syntax.
- Treating package-scoped documentation as adequate coverage.
- Pinning a stale `.rmeta` filename, hash, or other toolchain-private artifact detail.
- Removing legitimate test dependencies solely because duplicate dependency kinds look suspicious.

## Decisions

### D1: Put an evidence gate before the Cargo fix

Implementation begins by checking out the exact affected commit in an isolated worktree or
equivalent source snapshot and running the reported command with an empty `CARGO_TARGET_DIR`.
Record `rustc`, `rustdoc`, and Cargo versions; `cargo metadata`; relevant manifest dependency kinds
and features; and filtered `cargo doc -vv` rustdoc invocations. Run the same capture on the
main-based implementation branch.

The comparison must identify which `compylr-ir` metadata path/configuration each failing dependent
receives and why it differs before a Cargo manifest, feature, or dependency edge is changed. If the
affected state no longer reproduces, implementation records that result and makes no speculative
Cargo graph edit; the clean-workspace regression and check alignment still make the currently
working state enforceable.

Alternatives considered:

- **Immediately remove the host's duplicate dev-dependencies.** Rejected because the suite imports
  workspace crates directly for a documented reason, and current main succeeds with those entries.
- **Call the failure stale cache corruption.** Rejected because the handoff reports that
  `cargo clean --doc` did not fix it, while current clean output gives no evidence for that cause.
- **Choose a Cargo feature workaround up front.** Rejected until verbose invocations demonstrate a
  feature/configuration collision.

### D2: Use the real workspace build as the metadata regression

The regression executes the full workspace library documentation build in an isolated empty Cargo
target. It does not fabricate or inject `.rmeta` files: those are rustc-private, versioned artifacts,
and a synthetic fixture could test a condition different from Cargo's workspace scheduling. A
clean whole-workspace invocation directly covers the failure mode that package-scoped commands
missed.

The implementation adds the regression assertion before the fix. When the affected state can be
reproduced, the assertion or its focused fixture must fail there and pass only after the selected
fix. When it cannot be reproduced, the initial current-main success is the recorded baseline and
the clean CI job is the durable regression guard.

Alternatives considered:

- **Parse and pin the current hashed `--extern` filename.** Rejected because hashes legitimately
  vary with toolchain, target, features, and source.
- **Test only `cargo doc -p compylr-ir` or `-p compylr-core`.** Rejected because both were reported
  to pass in the broken state.

### D3: Make `make doc` the single repository entry point

The Makefile retains the exact command:

```text
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --lib
```

CI and the new Rust/Cargo-gated pre-commit hook invoke `make doc` rather than copying the Cargo
flags. CI supplies a dedicated empty `CARGO_TARGET_DIR` outside the shared restored target cache,
so its result cannot depend on a previous job's local metadata. Pre-commit reuses the normal target
for reasonable latency; it enforces command equivalence, while CI enforces the clean-target case.

A lightweight repository-configuration test checks that the CI documentation job and the
applicable pre-commit hook continue to route through `make doc`, matching the existing convention
that CI, Makefile, and hooks stay aligned.

Alternatives considered:

- **Copy the exact Cargo command into all three files.** Rejected because the copies can drift.
- **Force every local `make doc` to rebuild in a temporary target.** Rejected because it would
  discard useful incremental work on every invocation; clean isolation is required in CI and in
  explicit regression verification, not for every developer run.

### D4: Keep any Cargo remedy narrow and architecture-neutral

If D1 demonstrates a dependency-kind, feature-unification, target-name, or artifact-selection
cause, the fix changes only the manifest/configuration responsible for that cause and adds a test
that observes the workspace result. It must preserve direct crate testing and the established
dependency boundaries. The change must not couple shared IR or diagnostics crates to a host,
frontend, backend, or documentation-only dependency.

## Risks / Trade-offs

- **[The old branch may no longer reproduce under the installed toolchain]** → Preserve the exact
  command, versions, metadata, and verbose comparison; do not claim a root cause or make a Cargo
  edit unsupported by those observations.
- **[A clean CI rustdoc target costs more build time]** → Isolate only the documentation job;
  developer and pre-commit runs retain incremental artifacts.
- **[A whole-workspace command detects but does not localize a future Cargo regression]** → On
  failure, retain filtered `-vv` output showing each local `--extern` path as the diagnostic step.
- **[Calling `make doc` from CI and hooks could hide accidental Makefile drift]** → Keep the exact
  command visible in the Makefile and cover the routing mechanically.

## Migration Plan

1. Capture affected-branch and current-main evidence in isolated targets, and document whether the
   reported failure reproduces.
2. Add the clean workspace regression and configuration-alignment test first; demonstrate the
   regression against the affected state when reproducible.
3. Apply the narrow, evidence-supported Cargo fix, or explicitly record that no Cargo change is
   justified when the failure cannot be reproduced.
4. Route CI and the applicable pre-commit hook through `make doc`; isolate the CI Cargo target.
5. Verify the exact Cargo command and `make doc` from empty targets, repeat after other workspace
   builds, then run the repository's standard format, lint, test, and configuration checks.

Rollback consists of reverting the manifest/configuration fix and check routing together. There is
no user data or public API migration.

## Open Questions

- Which affected-branch input caused rustdoc to select API-incompatible `compylr-ir` metadata?
  D1 resolves this during implementation; the answer selects the narrow D4 remedy without changing
  the capability contract or task sequence.
