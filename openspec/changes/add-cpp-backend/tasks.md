## 1. The boundary rules, before the crates they govern

- [ ] 1.1 Write the failing rules first in [`crate_boundaries.rs`](../../../crates/compylr-host-python/tests/crate_boundaries.rs):
      a `compylr-bridge-*` crate links no host runtime and parses no source language — restate
      [`the_typescript_golang_bridge_neither_parses_ts_nor_links_napi`](../../../crates/compylr-host-python/tests/crate_boundaries.rs#L293)
      over the prefix so the new crates are covered without a fourth copy
- [ ] 1.2 Write the failing rule that the C++ backend knows no host language, alongside
      [`the_golang_backend_knows_no_host_language`](../../../crates/compylr-host-python/tests/crate_boundaries.rs#L256)
- [ ] 1.3 Add the C++ entry to the stance table
      [`a_stance_declaration_names_only_its_own_language`](../../../crates/compylr-host-python/tests/crate_boundaries.rs#L341) —
      own language `cpp`, every other language foreign
- [ ] 1.4 `cargo test --workspace` fails on exactly these three and nothing else; commit the tests

## 2. The C++ backend: skeleton, spellings, registration

- [ ] 2.1 Write the type-spelling tests first — every scalar, every collection, nesting, and that
      the same IR lowered from Python and from TypeScript emits byte-identical source
      (`cpp-backend`: *Concrete C++ type spellings*)
- [ ] 2.2 Create `crates/compylr-backend-cpp/` depending on `compylr-ir` and `compylr-core` only,
      and add it to [`Cargo.toml`](../../../Cargo.toml)'s workspace dependencies
- [ ] 2.3 Implement the type spellings from the table in `specs/cpp-backend/spec.md`
- [ ] 2.4 Implement identifier escaping over C++'s keyword set; test with a member named `template`
      built as IR, since no accepted source can produce it
- [ ] 2.5 Declare `CPP_BEHAVIOR` (the native stance) and `PRESERVES` (all three) as **separate**
      decisions — design D5. Test that a unit requiring `IntegerOverflowReported` negotiates
      successfully against `cpp`
- [ ] 2.6 Emit `CMakeLists.txt` selecting C++26 with `CMAKE_CXX_STANDARD_REQUIRED ON` — design D1
- [ ] 2.7 Register `cpp` as implemented in [`backends`](../../../crates/compylr-registry/src/backends.rs#L37),
      leaving it reserved in [`frontends`](../../../crates/compylr-registry/src/frontends.rs#L39)
- [ ] 2.8 `cargo test --workspace`; the boundary rules from group 1 now pass. Commit

## 3. `compat.hpp`: the behavior axes, unit-tested before they are emitted

- [ ] 3.1 Write the helper tests first, one per axis and per mode, mirroring the shape of
      [`runtime.rs`](../../../crates/compylr-backend-rust/src/runtime.rs)'s own suite — including
      the `Scenario Outline` rows for `-7 // 2` under each rounding
- [ ] 3.2 Write `compat.hpp` self-contained: no include of anything from this project, so it
      compiles once pasted into somebody else's tree — design D6
- [ ] 3.3 Implement checked integer overflow with the compiler's overflow builtins, so
      `IntegerOverflowReported` is genuinely preserved and not merely declared
- [ ] 3.4 Implement division rounding and checking, remainder sign and checking, subscript origin
      and checking, and text length units — dispatching on the node's **modes**, never on the
      operation's name
- [ ] 3.5 Embed it with `include_str!`, and assert the emitted file is byte-identical to the crate's
      own copy so the two lives cannot drift
- [ ] 3.6 `cargo test --workspace`; commit

## 4. Emission: statements, expressions, classes, and the conformance corpus

- [ ] 4.1 Write the fallible-signature tests first: a function containing a checked operation returns
      `std::expected`, one containing none returns its type directly, and a failure propagates out of
      a caller rather than being dropped (`cpp-backend`: *A fallible operation returns a value*)
- [ ] 4.2 Emit every statement form in every position it is legal in — function body, constructor,
      shared receiver, mutable receiver, loop body. [`conformance.rs`](../../../crates/compylr-host-python/tests/conformance.rs#L1033)
      enumerates backends from the registry, so this group is done when it passes with no edit to it
- [ ] 4.3 Emit classes: one member per attribute, a constructor assigning all of them, methods whose
      mutation is observable to the next call
- [ ] 4.4 Emit `mut`-equivalent places rather than values for mutation targets, and borrows for
      nested reads — the two directions `CLAUDE.md` records as live defects the demo found
- [ ] 4.5 Implement `post_process` shelling out to `clang-format`, falling back to the unformatted
      text; assert a machine with no formatter still gets buildable files
- [ ] 4.6 Assert emission reads and writes nothing —
      [`emission_reads_and_writes_nothing`](../../../crates/compylr-host-python/tests/crate_boundaries.rs#L396)
      already states it over every backend; confirm it covers the new crate
- [ ] 4.7 `cargo test --workspace`; commit

## 5. The shared C-ABI surface

- [ ] 5.1 Write the surface tests first: the generated export surface contains no source-language
      spelling, and two units holding the same IR from different frontends emit byte-identical
      shared files (`cpp-abi-bridge`: *One target-side export surface serves every source language*)
- [ ] 5.2 Create `crates/compylr-bridge-cpp-abi/`, depending on `compylr-ir`, `compylr-core`, and
      `compylr-backend-cpp`. It implements no pair and is **not** registered as a bridge
- [ ] 5.3 Emit `bindings.cpp`: one flat `extern "C"` symbol per member, arguments by C-compatible
      value, result through an out-parameter, integer status returned — design D4
- [ ] 5.4 Emit the allocation and release entry points, so every buffer is freed by the surface that
      allocated it, and opaque handles for instances
- [ ] 5.5 Derive `loaded_as` from the [`BuildKey`](../../../crates/compylr-core/src/bridge.rs#L44)
      fingerprint **and** variant tag; test that two pass configurations of one unit do not collide
- [ ] 5.6 `cargo test --workspace`; commit

## 6. The Python to C++ loader

- [ ] 6.1 Write the loader tests first: a scalar crosses and returns, text crosses as UTF-8, a
      collection parameter is a copy the caller does not see mutated, an instance handle keeps its
      state across two calls, and a reported failure raises `ZeroDivisionError`
- [ ] 6.2 Create `crates/compylr-bridge-python-cpp/`, implementing `HostBridge` for `("python", "cpp")`
      by delegating to the shared crate and adding only the `ctypes` loader and its stubs — design D3
- [ ] 6.3 Register the pair in [`bridges`](../../../crates/compylr-registry/src/bridges.rs#L22)
- [ ] 6.4 Assert a handle is released exactly once, and run this group's tests under a leak
      sanitizer — the failure mode here does not surface as a wrong answer
- [ ] 6.5 `cargo test --workspace`; commit

## 7. The TypeScript to C++ loader

- [ ] 7.1 Write the same boundary tests from the TypeScript side, including that a reported failure
      is thrown as an `Error` carrying the message
- [ ] 7.2 Create `crates/compylr-bridge-typescript-cpp/` on the same shape, adding only the FFI
      loader and `index.d.ts`, and register `("typescript", "cpp")`
- [ ] 7.3 Assert the two artifacts' shared files are byte-identical and only the loader differs —
      the claim design D3 is making, and the one that stops the split becoming a copy
- [ ] 7.4 `cargo test --workspace`; commit

## 8. Toolchain preflight, per target

- [ ] 8.1 Write the failing tests first: a project targeting C++ on a machine with no Rust does not
      report cargo missing, and a compiler present but below the version floor is diagnosed naming
      the standard required (`build-pipeline`: *A missing toolchain is diagnosed clearly*)
- [ ] 8.2 Move the unconditional check in [`_build.py`](../../../frontends/python/compylr/_build.py#L161)
      behind a per-target requirement list, including version floors — design D7
- [ ] 8.3 Add the C++ build driver: configure and build the emitted CMake tree, carrying the
      toolchain's own diagnostics through the way the maturin path does
- [ ] 8.4 Accept `backend="cpp"` end to end from [`_config.py`](../../../frontends/python/compylr/_config.py),
      leaving `DEFAULT_BACKEND` unchanged
- [ ] 8.5 Do the same for the TypeScript host: `backend: "cpp"` selectable, default unchanged
- [ ] 8.6 `make python` and `make ts`; commit

## 9. The corpus, over every bridged pair

- [ ] 9.1 Move the proposal's worked example into `frontends/python/fixtures/accepted/` with its
      driver in `frontends/python/fixtures/drivers/`. Check the member name is unique across the
      whole accepted corpus first — [`Unit::add_function`](../../../crates/compylr-ir/src/ir.rs#L1216)
      refuses a duplicate and the boundary tier builds every fixture into one unit
- [ ] 9.2 Add the TypeScript sibling of the same program to `frontends/typescript/fixtures/`, with
      its driver
- [ ] 9.3 Change the differential tiers in [`differential.rs`](../../../crates/compylr-host-python/tests/differential.rs)
      to enumerate pairs from the bridge registry rather than from a list, and to report a missing
      target toolchain as skipped **naming the tool** rather than as a pass
- [ ] 9.4 Confirm the fixture lists are still read from the directory rather than hardcoded —
      [`fixtures.rs`](../../../crates/compylr-host-python/tests/fixtures.rs) and
      [`emit_quality.rs`](../../../crates/compylr-host-python/tests/emit_quality.rs)
- [ ] 9.5 `cargo test --workspace` with the C++ toolchain present, then again with it hidden, and
      confirm the second run reports skips rather than passes; commit

## 10. `demo/demo-python-cpp`

- [ ] 10.1 Write the demo's own coverage test first, so the README's claim is an assertion before
      there is a README to make it
- [ ] 10.2 Lift the coverage walker and the benchmark harness into a place both new demos import
      rather than copying them a third and fourth time — design D8
- [ ] 10.3 Port the algorithm breadth — sorting, arithmetic, stats, text, graphs, dynamic, matrices,
      structures — and the nth-prime depth, at the standard
      [`demo-python-rust`](../../../demo/demo-python-rust/README.md) sets
- [ ] 10.4 Write the README with `<!-- benchmark:NAME -->` markers, leaving the tables to be
      generated
- [ ] 10.5 `cd demo/demo-python-cpp && uv run pytest && uv run ruff check . && uv run ty check src`;
      commit

## 11. `demo/demo-ts-cpp`

- [ ] 11.1 Write its coverage and agreement tests first
- [ ] 11.2 Port the same breadth and depth in TypeScript, importing the shared harness from 10.2
- [ ] 11.3 Write the README with its own benchmark markers
- [ ] 11.4 Make the demo set derived from the bridge registry, and add the failing check that a
      registered pair without a demo fails the suite (`demo`: *Every bridged pair has a demo*)
- [ ] 11.5 Group both new demo checks with the existing slow tests, skipping with a named reason
      when the C++ toolchain is absent
- [ ] 11.6 `npm test` in the new demo, then `cargo test --workspace -- --ignored`; commit

## 12. Documentation, generated and prose

- [ ] 12.1 Teach [`update_benchmarks.py`](../../../scripts/update_benchmarks.py) the two new demos'
      markers, and run it to fill the tables from a real run — never by hand
- [ ] 12.2 Run [`update_subset.py`](../../../scripts/update_subset.py); the subset is unchanged, so
      confirm it rewrites nothing
- [ ] 12.3 Update [`README.md`](../../../README.md)'s capability list, module layout, and every
      referenced path, in this change and not after it —
      [`readme.rs`](../../../crates/compylr-host-python/tests/readme.rs) enforces the mechanical half
- [ ] 12.4 Update [`CLAUDE.md`](../../../CLAUDE.md): the crate count, the third backend, the two new
      bridges, the shared-ABI split, and the C++ toolchain requirement
- [ ] 12.5 Add `cpp` targets to the [`Makefile`](../../../Makefile), the CI workflow, and
      [`.pre-commit-config.yaml`](../../../.pre-commit-config.yaml) — all three, or it is a check
      people discover in a pull request
- [ ] 12.6 `./scripts/update_benchmarks.py --check && ./scripts/update_subset.py --check`; commit

## 13. Checks

- [ ] 13.1 `make check` — the whole of what CI runs
- [ ] 13.2 `make clean-artifacts && make demo` and `make demo-ts`, confirming the existing two pairs'
      numbers have not moved; the rebuild key is the IR fingerprint and the compiler's version does
      not move during development, so the removal is not optional
- [ ] 13.3 Run the new demos the same way and record their numbers
- [ ] 13.4 `make coverage` with the venv deactivated, confirming the new crates are above the
      threshold
- [ ] 13.5 `openspec validate add-cpp-backend --strict`
