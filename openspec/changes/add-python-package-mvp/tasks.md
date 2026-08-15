## 1. Crate shape and dependencies

- [x] 1.1 Archive `add-local-type-inference` so `openspec/specs/` describes `float`, `/`, and the promotion node before a backend is written against them (design.md — Migration Plan)
- [x] 1.2 Add `serde` (derive) and `serde_json` to `Cargo.toml`; confirm `cargo test` still passes untouched
- [x] 1.3 Add `pyo3` at a version supporting CPython 3.14, with an `abi3` floor, resolving the first Open Question; record the chosen version and floor in design.md
- [x] 1.4 Set `[lib] crate-type = ["cdylib", "rlib"]` and verify the existing binary and both integration test targets still link
- [x] 1.5 Add `.compylr/` and generated build output to `.gitignore`

## 2. IR serialization

- [x] 2.1 Write a round-trip test asserting a unit covering every type, statement form, and expression form deserializes structurally equal to the original
- [x] 2.2 Write a test asserting the fingerprint is identical before and after a round trip
- [x] 2.3 Write a test asserting float literals round-trip bit-exactly, including `-0.0` staying distinguishable from `0.0`
- [x] 2.4 Write a test asserting serializing the same unit twice is byte-identical, and that two units built in different addition orders serialize identically
- [x] 2.5 Write a test asserting units lowered from sources differing only in comments and indentation serialize byte-identically — this is the test that fails if spans are serialized
- [x] 2.6 Write a test asserting the artifact contains no Rust spellings (`i64`, `f64`, `String`)
- [x] 2.7 Derive `Serialize`/`Deserialize` across the IR, skipping `Span` per design.md D7, and implement unit serialization to and from JSON
- [x] 2.8 Confirm every test in this group passes and that `Unit`'s existing ordering guarantees carry the determinism rather than a sort added here

## 3. Backend registry

- [x] 3.1 Write tests for the three-way lookup: `rust` resolves to an implemented backend, `typescript`/`go`/`cpp` resolve as reserved, an unrecognized name fails listing the available names
- [x] 3.2 Write a test asserting the reserved-backend error says the backend is not implemented yet, and is distinguishable from the unknown-name error
- [x] 3.3 Create `src/backend/mod.rs` with the `Backend` trait and the registry from design.md D6

## 4. Rust backend — structure

- [x] 4.1 Write tests asserting each IR type emits its Rust spelling, and that a function returning unit emits no return type
- [x] 4.2 Write a test asserting emission leaves the IR unchanged and that no Rust spelling appears anywhere in `src/ir.rs`
- [x] 4.3 Write tests for function emission: parameters in source order with spelled types, and all functions of a unit present in the unit's deterministic order
- [x] 4.4 Write tests for statement emission: return of an expression, a `pass` body under a unit return type, and a local binding stating its type explicitly
- [x] 4.5 Write tests for expression emission: literals of every type, name references, negation, the promotion node, and a call to another function in the unit
- [x] 4.6 Write a test asserting a string literal containing a double quote, a backslash, and a newline emits a Rust literal denoting exactly those characters
- [x] 4.7 Write a test asserting nesting is preserved regardless of Rust precedence — arithmetic inside a comparison inside a call argument
- [x] 4.8 Implement `src/backend/rust.rs` covering types, functions, statements, and expressions, emitting fully parenthesized binary expressions per design.md D5
- [x] 4.9 Confirm the backend never re-derives promotion: assert `TrueDiv` on two integers emits a plain division because lowering already wrapped both operands in the promotion node

## 5. Rust backend — Python operator semantics

- [x] 5.1 Write executable tests for floor division: `-7 // 2 == -4`, `7 // -2 == -4`, `-6 // 2 == -3`, and `-7.0 // 2.0 == -4.0`
- [x] 5.2 Write executable tests for remainder: `-7 % 2 == 1`, `7 % -2 == -1`, and the identity `(a // b) * b + (a % b) == a` over a table of signed operand pairs
- [x] 5.3 Write executable tests for true division: `7 / 2 == 3.5`, and a function returning `/` on two integers has Rust return type `f64`
- [x] 5.4 Write tests for the remaining operators: string concatenation, and each of the six comparisons yielding `bool`
- [x] 5.5 Write tests asserting division and remainder by zero produce a recoverable error rather than a panic, for integer and float operands alike — Python raises where IEEE would return infinity
- [x] 5.6 Write tests asserting overflow produces a recoverable error rather than wrapping, including `i64::MIN / -1`
- [x] 5.7 Write a test asserting a failure inside a called generated function propagates to the outermost caller
- [x] 5.8 Implement the emitted runtime helpers from a single `const` per design.md D4, with inner functions uniformly returning `Result<T, RuntimeError>` per D3
- [x] 5.9 Verify each semantics test executes emitted code rather than comparing emitted strings, so a helper that is wrong in a way the string still looks right cannot pass

## 6. Emission quality

- [x] 6.1 Write a test asserting the same unit emits byte-identically twice, and that addition order does not change output
- [x] 6.2 Write a test that lowers and emits every accepted fixture and compiles the result, asserting no errors and no warnings under the project's lint settings
- [x] 6.3 Pipe emitted source through `rustfmt` on a best-effort basis, falling back to unformatted output when it is unavailable
- [x] 6.4 Snapshot the emitted Rust for the accepted fixtures so an unintended change in shape shows up as a diff

## 7. PyO3 binding emission

- [ ] 7.1 Write tests asserting a built unit exposes every function as a module attribute, and exposes nothing else beyond standard module attributes
- [ ] 7.2 Write tests asserting parameters are accepted both positionally and by keyword under their Python names
- [ ] 7.3 Write tests asserting each type round-trips across the boundary, that a unit return is `None`, and that a `bool` return is a Python `bool` rather than an `int`
- [ ] 7.4 Write tests asserting a wrong argument type and a wrong argument count each raise `TypeError`
- [ ] 7.5 Write tests asserting division by zero raises `ZeroDivisionError`, overflow raises `OverflowError`, the process survives both, and a failure in a nested compiled call propagates
- [ ] 7.6 Implement binding emission as a layer over the pure-Rust functions per design.md D2, mapping `RuntimeError` onto the Python exception types
- [ ] 7.7 Emit the module under a fingerprinted name per design.md D13

## 8. Native bridge

- [ ] 8.1 Write tests for the compile entry point: one source compiles and returns target source, IR artifact, and fingerprint; an empty collection succeeds with an empty unit
- [ ] 8.2 Write tests asserting sources are assembled into one unit — a cross-source call resolves, both supply orders give the same fingerprint, and duplicate names fail naming the conflict
- [ ] 8.3 Write tests asserting compilation accepts source text with no file behind it
- [ ] 8.4 Write tests asserting a syntax error and a subset rejection raise distinguishable exceptions, both carrying message and `line:column`, and both catchable through one base type
- [ ] 8.5 Write tests asserting the backend registry's three-way behavior surfaces through the bridge
- [ ] 8.6 Write tests asserting the reported fingerprint is unchanged by comments and reformatting, and changes when a body changes
- [ ] 8.7 Implement `src/bridge.rs` as the `compylr._core` module with the exception taxonomy from design.md D15
- [ ] 8.8 Add the root `pyproject.toml` with the maturin backend and the layout from design.md D14, and confirm the module imports from Python

## 9. Python package — API surface

- [ ] 9.1 Create `python/compylr/` and confirm `python/fixtures/` is untouched and `tests/fixtures.rs` still passes
- [ ] 9.2 Write tests for initialization: explicit settings, defaults with no arguments, a repeat call with identical settings returning the same manager, and a repeat call with different settings raising and naming the conflicting setting
- [ ] 9.3 Write tests for both decorator forms — bare, called with no arguments, and called with settings — asserting the first two are equivalent
- [ ] 9.4 Write tests for setting resolution: an override applies to one function only, and unspecified settings are inherited
- [ ] 9.5 Write tests asserting a reserved backend and an unknown backend each fail at decoration with their distinct messages
- [ ] 9.6 Write tests asserting the assist mode raises when enabled globally or per-function, and is silent when disabled or omitted
- [ ] 9.7 Write tests asserting an unsupported function is rejected at decoration with `line:column`, before any call, with no silent fallback
- [ ] 9.8 Write tests asserting source is captured by introspection, that the decorator line is excluded, and that surrounding indentation does not cause a rejection
- [ ] 9.9 Write tests asserting a marked function preserves name, docstring, module, and annotations, exposes the original through `__wrapped__`, and works anywhere a callable is accepted
- [ ] 9.10 Implement the manager, both decorator forms, settings resolution, and the wrapper object per design.md D10 and D12
- [ ] 9.11 Configure ruff and mypy for `python/compylr/`, and confirm the package is clean under both

## 10. Build pipeline

- [ ] 10.1 Write tests asserting three marked functions produce exactly one build and one module, that a fourth rebuilds the same shared artifact, and that compiled functions can call each other
- [ ] 10.2 Write tests asserting the IR artifact and generated target source are both written on every build, survive a skipped rebuild, and reflect an edited function after a rebuild
- [ ] 10.3 Write tests asserting all generated files share one root and that deleting it causes a clean rebuild with identical behavior
- [ ] 10.4 Write tests for the fingerprint cache: an unchanged project skips the toolchain, reformatting does not rebuild, an edit rebuilds, marking a function rebuilds, and a failed build is not recorded as successful
- [ ] 10.5 Write tests asserting a toolchain failure raises an error carrying the toolchain's output, with no silent fallback to interpreted execution
- [ ] 10.6 Write tests asserting a missing Rust toolchain and a missing build tool are each named with install instructions, and are reported before a build is attempted
- [ ] 10.7 Implement crate assembly, artifact writing, and the fingerprint-keyed rebuild decision using the layout in design.md D8
- [ ] 10.8 Implement the build and install step per design.md D9, including cache invalidation and uninstalling the previous generated distribution
- [ ] 10.9 Write a test asserting a rebuild triggered mid-process loads and is used in that same process, exercising the fingerprinted module name

## 11. End to end

- [ ] 11.1 Write an end-to-end test that marks a function, calls it, and asserts the result equals what the interpreted function returns
- [ ] 11.2 Write an end-to-end test asserting a single build covers several marked functions and that repeated calls do not rebuild
- [ ] 11.3 Write an end-to-end test over the accepted fixtures comparing compiled results against the interpreted originals across a table of inputs including negative operands — the case where Python and Rust semantics diverge
- [ ] 11.4 Verify a second process reuses the built artifact without invoking the toolchain
- [ ] 11.5 Measure and record first-build and cached-run timings so the cost claimed in design.md is a number rather than an assertion

## 12. Documentation and verification

- [ ] 12.1 Rewrite the README: remove the "no Python package yet" note and the `TARGET DESIGN` marker, document the real API, and state plainly that a Rust toolchain and maturin are required at runtime
- [ ] 12.2 Update the README pipeline diagram and capability table, and document `.compylr/` and its artifacts
- [x] 12.3 Extend `tests/readme.rs` so the backend claim is enforced in both directions — the existing check falls silent once a backend exists, which is the moment it stops protecting anything
- [ ] 12.4 Update `CLAUDE.md`: current state, the new commands, and the two distinct PyO3 roles
- [ ] 12.5 Run `cargo fmt`, `cargo clippy -p compylr --all-targets -- -D warnings`, and `cargo test`, resolving all findings
- [ ] 12.6 Run the Python test suite with coverage and confirm it exceeds the project threshold
- [ ] 12.7 Confirm Rust coverage over `src/` still exceeds 80%, adding tests for uncovered branches in the backend
- [ ] 12.8 Run `openspec validate add-python-package-mvp --strict` and confirm every scenario in all six delta specs has a corresponding passing test
- [ ] 12.9 Verify the end state by hand in a scratch project: `uv add` the built wheel, decorate a function, call it, and inspect `.compylr/ir/unit.json` and the generated Rust
