## 1. Discovery

- [x] 1.1 Write tests asserting every marked function across several modules is found, and that marked classes are found alongside them
- [x] 1.2 Write a test asserting only modules beneath the given root are imported
- [x] 1.3 Write a test asserting virtual environments, caches, version-control directories, and build output are skipped, so precompiling does not import a dependency tree
- [x] 1.4 Write a test asserting a module that raises on import is reported and the others still processed
- [x] 1.5 Implement discovery by importing beneath the root, per design.md D2

## 2. The programmatic entry point

- [x] 2.1 Write tests asserting a root is compiled, and that the report names modules imported, functions and classes found, and whether a build occurred
- [x] 2.2 Write a test asserting a project with nothing marked is not an error and says so
- [x] 2.3 Write a test asserting a build failure raises the same error a call-triggered build raises, carrying the toolchain output
- [x] 2.4 Implement `precompile(root) -> Report`, returning facts rather than formatted text, per design.md D3

## 3. Building ahead of a call

- [x] 3.1 Write a test asserting a project builds with no marked function having been called
- [x] 3.2 Write a test asserting building ahead and building by calling record the **same fingerprint**, so a later run reuses rather than rebuilds
- [x] 3.3 Write a test asserting building ahead from a different working directory lands in the project's own artifact directory
- [x] 3.4 Write a test asserting an already-current project is not rebuilt
- [x] 3.5 Write a test asserting a missing toolchain reports the same diagnostic as a call-triggered build
- [x] 3.6 Implement building without a call

## 4. The command

- [x] 4.1 Declare a `compylr` console script in `pyproject.toml`, per design.md D1
- [x] 4.2 Write tests asserting the command compiles a project and exits successfully, and that a later run does not invoke the toolchain
- [x] 4.3 Write tests asserting an unchanged project is not rebuilt, an edit is picked up, and reformatting is not
- [x] 4.4 Write a test asserting the help states that discovery imports the project's modules
- [x] 4.5 Write a test asserting a missing root is reported and exits unsuccessfully
- [x] 4.6 Implement the command as a thin wrapper that only formats the report

## 5. Reporting and exit status

- [x] 5.1 Write a test asserting a successful build names the counts found and reports that it built
- [x] 5.2 Write a test asserting finding nothing says so rather than reporting success with nothing done
- [x] 5.3 Write a test asserting reuse is distinguished from building
- [x] 5.4 Write a test asserting a build failure carries the toolchain's diagnostics
- [x] 5.5 Write tests asserting success, build failure, and nothing-found are each distinguishable from the exit status alone, per design.md D4
- [x] 5.6 Write a test asserting the count of import failures appears in the summary, not only in the detail

## 6. End to end

- [x] 6.1 Build a scratch project, precompile it, and assert a subsequent run performs no build
- [x] 6.2 Measure and record cold-precompile and warm-run timings, so the claim that first-run cost is removed is a number
- [x] 6.3 Assert that precompiling and then editing one function rebuilds only once

## 7. Verification

- [x] 7.1 Run `cargo fmt`, `cargo clippy -p compylr --all-targets -- -D warnings`, and `cargo test`
- [x] 7.2 Run `pytest`, `ruff check python/`, and `mypy python/compylr`; coverage with the venv deactivated
- [x] 7.3 Confirm Python coverage still exceeds the project threshold
- [x] 7.4 Update the README: the precompile command, that it imports the project, and **that `compylr` now names the Python console script while the Rust binary is reached through `cargo run`**
- [x] 7.5 Update `CLAUDE.md`'s commands and current state
- [x] 7.6 Run `openspec validate add-cli-precompile --strict` and confirm every scenario in all three delta specs has a passing test
