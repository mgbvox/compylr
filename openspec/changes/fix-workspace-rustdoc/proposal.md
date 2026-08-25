## Why

Workspace-wide rustdoc failed on `plan/py2many-adoption` by compiling dependent crates against
`compylr-ir` metadata that did not expose the current behavior-profile API, even though ordinary
builds and package-scoped documentation succeeded. The exact command passes on current `main`, so
the failure needs a reproducible branch comparison and a regression contract rather than an
assumed Cargo root cause.

## What Changes

- Define a workspace-documentation contract in which a clean, warnings-denied documentation build
  covers every workspace library and resolves each local dependency from the current workspace
  source, including `compylr-ir`.
- Reproduce the affected branch state and compare its verbose Cargo/rustdoc dependency inputs with
  current `main` before choosing any manifest, feature, or dependency-graph fix.
- Add regression coverage that fails if workspace documentation selects stale or incompatible
  `compylr-ir` metadata while package-scoped documentation still appears healthy.
- Keep the direct Cargo command, `make doc`, CI, and the applicable pre-commit check aligned so the
  same workspace-wide failure is exercised locally and remotely.

## Capabilities

### New Capabilities

- `workspace-documentation`: Defines reliable, warnings-denied documentation builds for all local
  workspace libraries and consistent enforcement across developer and CI entry points.

### Modified Capabilities

None.

## Impact

Implementation may affect Cargo workspace/package manifests or documentation-check support after
the branch comparison identifies the smallest justified fix. It will also affect the documentation
entry points in `Makefile`, `.github/workflows/rust.yml`, and `.pre-commit-config.yaml`, plus focused
regression tests or scripts. The compiler's public API, generated artifacts, and supported Python
subset do not change.
