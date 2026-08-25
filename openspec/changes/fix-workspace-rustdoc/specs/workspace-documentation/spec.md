## Purpose

Ensures this repository can build complete, warnings-denied Rust API documentation from current
workspace sources and that every contributor-facing check exercises the same contract.

## ADDED Requirements

### Requirement: Every workspace library documents cleanly as one workspace

The repository SHALL build documentation for every workspace package that exposes a library target
with warnings denied, workspace dependency unification enabled, and dependency documentation
disabled. A clean Cargo target SHALL NOT be required to contain metadata from an earlier build for
the documentation build to succeed.

#### Scenario: Exact command succeeds from a clean Cargo target

- **WHEN** a contributor runs `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --lib`
  with an empty Cargo target directory
- **THEN** the command succeeds and documents every workspace library target

#### Scenario: Package-scoped success does not substitute for workspace success

- **WHEN** package-scoped documentation succeeds but the workspace-wide command fails
- **THEN** the repository documentation check fails

### Requirement: Workspace documentation uses current local crate metadata

Every local crate passed to rustdoc as a dependency SHALL resolve to metadata produced from the
current workspace source and the feature/configuration selected for that documentation build.
Workspace documentation SHALL NOT compile a dependent crate against stale or incompatible
`compylr-ir` metadata.

#### Scenario: Current behavior-profile API is visible to dependents

- **WHEN** workspace crates document code that imports the behavior-profile API from `compylr-ir`
- **THEN** rustdoc resolves those imports and trait members from metadata built from the current
  `crates/compylr-ir` source

#### Scenario: Previous metadata cannot mask or create a result

- **WHEN** the same workspace documentation command is run once with a clean Cargo target and once
  after other workspace builds
- **THEN** both runs resolve compatible current local-crate metadata and have the same success or
  failure result

### Requirement: Documentation entry points enforce one command contract

The direct Cargo command, `make doc`, the Rust documentation CI job, and the applicable pre-commit
hook SHALL enforce the same warnings-denied, workspace-wide, library-only, no-dependencies
documentation contract. A change to that contract SHALL update all four entry points together.

#### Scenario: Make target matches the direct command

- **WHEN** `make doc` runs from a clean workspace
- **THEN** it succeeds exactly when
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --lib` succeeds

#### Scenario: CI exercises the local documentation contract

- **WHEN** CI validates Rust documentation
- **THEN** it uses the same workspace scope, target selection, dependency exclusion, and warning
  policy as `make doc`

#### Scenario: Relevant commits exercise the local documentation contract

- **WHEN** a pre-commit run is triggered by Rust source or Cargo manifest changes that can affect
  workspace documentation
- **THEN** it runs the same documentation contract without accepting a package-scoped substitute

### Requirement: Workspace-only metadata regressions are covered

The repository SHALL have automated regression coverage capable of detecting a workspace-wide
documentation failure caused by a local dependency receiving stale, wrong, or incompatible
metadata, even when documenting the dependency or one dependent package alone succeeds.

#### Scenario: Incompatible IR metadata fails the regression

- **WHEN** a workspace documentation build would expose `compylr-ir` metadata that lacks API used
  by current dependent crates
- **THEN** the regression fails before the change can pass the repository checks

#### Scenario: Regression starts without cached build output

- **WHEN** the metadata regression is evaluated
- **THEN** its workspace-wide documentation invocation uses a clean Cargo target so success does
  not depend on pre-existing artifacts
