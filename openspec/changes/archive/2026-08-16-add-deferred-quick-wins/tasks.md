## 1. Signature collection

- [x] 1.1 Write tests asserting a signature table is built from a source, carrying each function's parameter types and return type
- [x] 1.2 Write a test asserting collection reads annotations only, so it succeeds for a source whose bodies would not lower
- [x] 1.3 Write a test asserting duplicate function names in one source are reported during collection
- [x] 1.4 Implement `collect_signatures` over a parsed module, per design.md D1
- [x] 1.5 Thread the table into `lower_function` and `lower_expr` without changing any existing behavior yet; confirm `cargo test` is unchanged

## 2. Calls type their bindings

- [x] 2.1 Write a test asserting `b = double(n)` lowers with no annotation and binds the callee's return type
- [x] 2.2 Write a test asserting a call nested in an expression infers, so `b = double(n) + 1` works
- [x] 2.3 Write a test asserting a function may call one defined later in the same source
- [x] 2.4 Write a test asserting both definition orders of a mutually-referencing pair produce identical IR — the order-independence property, asserted directly rather than assumed, per design.md D1
- [x] 2.5 Write a test asserting a self-recursive function lowers
- [x] 2.6 Write tests asserting an unknown callee, wrong arity, and a wrong argument type are each rejected with a location
- [x] 2.7 Write a test asserting promotion applies to arguments, so an integer passed where a float is declared carries an explicit conversion
- [x] 2.8 Write a test asserting a declared annotation still wins, so `b: float = double(n)` is accepted via promotion
- [x] 2.9 Change call lowering to return a determined type from the signature table
- [x] 2.10 Verify `Unit::validate` still resolves cross-source calls, and add a test asserting a cross-source call still lowers per source and resolves at the unit, per design.md D2

## 3. Reject a function that cannot return

- [x] 3.1 Write tests asserting `def f() -> int: pass` and a body ending in a binding are both rejected, naming the function and its location
- [x] 3.2 Write tests asserting a unit-returning function needs no return, and that a function ending in `return` is unaffected
- [x] 3.3 Write a test asserting the diagnostic reports a missing return rather than a type mismatch
- [x] 3.4 Implement the structural check per design.md D3
- [x] 3.5 Confirm the backend's equivalent `Unsupported` error is now unreachable from lowered input, and leave it as a defensive guard rather than deleting it

## 4. CLI

- [x] 4.1 Write tests asserting the default output reports the fingerprint and each function's signature, and that no arguments prints usage and exits unsuccessfully
- [x] 4.2 Write tests asserting `--emit ir` writes the IR artifact and `--emit rust` writes generated source, both without performing a build
- [x] 4.3 Write a test asserting emitted output goes to the output stream and diagnostics to the error stream, so redirection produces a usable file
- [x] 4.4 Write tests asserting a missing file, a syntax error, and a subset violation each exit unsuccessfully with a located message
- [x] 4.5 Write tests asserting `--backend` selects a backend, and that reserved and unknown names are reported distinctly
- [x] 4.6 Write a test asserting an unrecognized `--emit` value lists the accepted forms
- [x] 4.7 Implement flag parsing and the output forms in `src/main.rs` per design.md D4, keeping it a thin wrapper over the library

## 5. Project root discovery

- [x] 5.1 Write tests asserting an existing `.compylr/` is found from a subdirectory, and that `pyproject.toml` is found when there is none
- [x] 5.2 Write a test asserting an existing artifact directory wins over a `pyproject.toml` higher up
- [x] 5.3 Write a test asserting the search falls back to the working directory when it reaches the filesystem root with no marker
- [x] 5.4 Write a test asserting an explicitly supplied root skips discovery entirely
- [x] 5.5 Write a test asserting a project built from its root and then run from a subdirectory reuses the artifacts and does not invoke the toolchain
- [x] 5.6 Implement discovery in `python/compylr/_build.py` per design.md D5

## 6. Fixtures and migration

- [x] 6.1 Move `python/fixtures/rejected/unannotated_from_call.py` to `accepted/`, since it asserts the behavior this change reverses
- [x] 6.2 Add accepted fixtures for a call-typed binding, a forward reference, and a self-recursive function
- [x] 6.3 Add rejected fixtures for an unknown callee, wrong arity, and a function that cannot return
- [x] 6.4 Update the rejection table and the fixture-count guard in `tests/fixtures.rs`
- [x] 6.5 Review the regenerated IR snapshots, confirming call-typed bindings carry the callee's return type

## 7. Verification

- [x] 7.1 Run `cargo fmt`, `cargo clippy -p compylr --all-targets -- -D warnings`, and `cargo test`
- [x] 7.2 Run `pytest`, `ruff check python/`, and `mypy python/compylr`
- [x] 7.3 Confirm Rust coverage over `src/` still exceeds 80%, including the new CLI paths
- [x] 7.4 Update the README: the subset section no longer needs its "a call still needs an annotation" caveat, and the CLI section gains `--emit`
- [x] 7.5 Note the artifact-root change in the README, so a one-off rebuild after upgrading is not mistaken for a cache bug
- [x] 7.6 Update `CLAUDE.md`'s current state and commands
- [x] 7.7 Run `openspec validate add-deferred-quick-wins --strict` and confirm every scenario in all three delta specs has a passing test
