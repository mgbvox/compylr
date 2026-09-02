## Why

[`bridge.rs`](../../../crates/compylr-core/src/bridge.rs#L1-L21) states the one place compylr's
modularity does not come for free: frontends and backends compose N + M because they meet at the
IR and never see each other, but a bridge is keyed by the `(source, target)` **pair** and therefore
costs N × M. It also records the escape hatch, and defers it:

> The trait is shaped so that a canonical-C-ABI hub — one bridge registered for many pairs — could
> be implemented *behind* it later, collapsing N × M back to N + M at the cost of a marshalling
> layer. That trade is deferred, not foreclosed.

**The deferral stays deferred, and C++ is what shows why.** An earlier draft of this proposal
argued the opposite — that C++ is where the hub could finally be cashed in, because its idiomatic
export surface already is a C ABI and both hosts could consume one with what they ship. The second
half is false: **Node has no core FFI**, and `process.dlopen` loads only Node-API addons, requiring
`napi_register_module_v1` rather than arbitrary C symbols. A hub would mean writing a Node-API addon
anyway, behind an extra indirection, with hand-rolled marshalling that `Napi::` already provides.

The inversion is the point: **C++ is the target that needs a hub least**, because both hosts'
first-class binding libraries — nanobind and node-addon-api — are already C++ header libraries.
Every other target reaches across a language gap to bind; C++ is the one where the host's own
tooling meets you where you are. So this change adds a third backend and two ordinary pairwise
bridges, and the N x M cost is paid, visibly, as `bridge.rs` always said it would be.

The backend itself is the second *statically compiled* target and the first with move semantics,
RAII, and no garbage collector — which is a real test of the IR's neutrality. It is a weaker test
than it should be, because the shared corpus check turns out to render without compiling; closing
that is part of this change rather than an assumption it inherits.

The subset, its rules, and their reasoning are in [`CLAUDE.md`](../../../CLAUDE.md); this change
grows none of them.

## What Changes

- **`crates/compylr-backend-cpp`, targeting C++26** (`-std=c++26`). Implements
  [`Backend`](../../../crates/compylr-core/src/backend.rs#L40): C++ type spellings, a `compat.hpp`
  implementing the six behavior axes, and `clang-format` as
  [`post_process`](../../../crates/compylr-core/src/backend.rs#L81). A fallible operation returns
  `std::expected<T, compylr::Error>`; nothing throws across a generated function boundary. The
  manifest is a CMake project. `cpp` moves from reserved to implemented in
  [`backends`](../../../crates/compylr-registry/src/backends.rs#L37) — it stays reserved as a
  *frontend*, so the reserved-name scenarios elsewhere keep a live example.

- **`crates/compylr-bridge-python-cpp`**, implementing
  [`HostBridge`](../../../crates/compylr-core/src/bridge.rs#L83) for `(python, cpp)` by generating a
  **nanobind** module: boundary marshalling, `nb::class_` instance binding, and a returned failure
  translated into the Python exception the source operation would have raised. nanobind rather than
  pybind11 for compile time, binary size, and stable ABI on 3.12+.

- **`crates/compylr-bridge-typescript-cpp`**, the same for `(typescript, cpp)` via
  **node-addon-api**, built with cmake-js — which fits because the backend already emits
  `CMakeLists.txt`, where node-gyp would want a `binding.gyp` and a Python interpreter.
  `Napi::ObjectWrap` carries instances; a returned failure becomes a thrown `Error`.

- **A conformance tier that compiles and runs what it renders.** The existing corpus check is
  render-only for a non-Rust backend, so "every implemented backend renders the corpus" has never
  meant the output builds. This change closes that before the C++ backend relies on it.

- **C++ declares its own stance on all six axes**, and only its own — the rule
  [`a_stance_declaration_names_only_its_own_language`](../../../crates/compylr-host-python/tests/crate_boundaries.rs#L341)
  already enforces. Signed overflow is `Checked::Unchecked`; division truncates toward zero; `%`
  takes the dividend's sign; indexing is from the start and unchecked; `.size()` counts UTF-8
  bytes. It nonetheless **preserves all three guarantees**, because `compat.hpp` implements each
  checked mode — the same separation
  [`RUST_BEHAVIOR`](../../../crates/compylr-backend-rust/src/rust.rs#L226) already draws, where a
  stance that is unchecked on every axis sits beside a
  [`PRESERVES`](../../../crates/compylr-backend-rust/src/rust.rs#L189) naming all three. Deriving
  one from the other would refuse every default Python program, which is the parity this change
  exists to keep.

- **A `cpp26-contracts` target option, declared and not implemented**, the way the Rust backend
  declares [`unchecked-arithmetic`](../../../crates/compylr-backend-rust/src/rust.rs#L202): C++26's
  contracts would let a checked mode be expressed as a precondition rather than a branch, and
  permitting it fails saying it is reserved rather than silently doing nothing.

- **Two demo projects, `demo/demo-python-cpp` and `demo/demo-ts-cpp`**, at the standard
  [`demo/demo-python-rust`](../../../demo/demo-python-rust/README.md) sets — **only** that one, since
  `demo-ts-go`'s coverage report is a hardcoded stub and its benchmark table is fabricated (#38): the same algorithm
  breadth, the same nth-prime depth, the IR-coverage table asserted rather than claimed, benchmark
  tables behind `<!-- benchmark:NAME -->` markers, and verification from this repository's suite as
  well as their own.

- **The differential tiers run over every bridged pair, enumerated from the registry** rather than
  from a list — the same discipline `conformance.rs` already uses, and the fix for the drift
  [`CLAUDE.md`](../../../CLAUDE.md) records twice.

- **No IR change, no artifact-format change, no cache invalidation, no subset change.** Existing
  Python → Rust and TypeScript → Go builds are untouched. **Not breaking.**

## Worked Example

The output blocks below are marked `expected:` — nothing here has been run.

### Input

Liftable to [`frontends/python/fixtures/accepted/`](../../../frontends/python/fixtures/accepted/)
unchanged, driver beside it.

```python
def running_totals(values: list[int], divisor: int) -> list[int]:
    totals: list[int] = []
    running: int = 0
    for i in range(len(values)):
        running = running + values[i]
        totals.append(running // divisor)
    return totals
```

One program, because it reaches everything the change touches: a collection parameter crossing by
value, a locally built collection returned by value, `Expr::Subscript` carrying an index origin and
a checking mode, `Expr::Len` carrying text units, and a `BinOp::Div` carrying `Rounding` and
`Checked` — the node whose *modes* the new backend must read, since reading its name would be
silently wrong for the other stance.

### Today

```text
expected:

>>> import compylr
>>> c = compylr.initialize(backend="cpp")
compylr.ConfigurationError: the 'cpp' backend is not implemented yet; it is a planned target
```

### After

`cargo run -p compylr-cli -- --backend cpp --emit source running_totals.py`, showing only the lines
the change puts there — the fallible signature and the mode-driven division:

```cpp
expected:

std::expected<std::vector<int64_t>, compylr::Error>
running_totals(std::vector<int64_t> values, int64_t divisor) {
    std::vector<int64_t> totals{};
    int64_t running = 0;
    for (int64_t i = 0; i < static_cast<int64_t>(values.size()); ++i) {
        running = running + values[static_cast<size_t>(i)];
        auto __d = compylr::floor_div_checked(running, divisor);
        if (!__d) return std::unexpected(__d.error());
        totals.push_back(*__d);
    }
    return totals;
}
```

`floor_div_checked` because the node resolved `Rounding::TowardNegInf` with `Checked::Reported`,
which is Python's stance and not C++'s. Under `behavior="cpp"` the same node resolves
`Rounding::TowardZero` / `Checked::Unchecked` and the backend emits `running / divisor` — the
difference the six axes exist to carry.

### At the boundary

```pycon
expected:

>>> import compylr
>>> c = compylr.initialize(backend="cpp")
>>> running_totals = c.compyle(running_totals)
>>> running_totals([3, 7, 11], 2)
[1, 5, 10]
>>> running_totals([3, 7, 11], 0)
ZeroDivisionError: integer division or modulo by zero
```

The same generated artifact, reached from the other host — the one snippet that cannot be folded
in, because *that only the loader differs* is the claim the change is making:

```ts
expected:

> import { initialize } from "compylr";
> const c = initialize({ backend: "cpp" });
> runningTotals([3, 7, 11], 2)
[ 1n, 5n, 10n ]
```

## Capabilities

### New Capabilities

- `cpp-backend`: Translate compylr IR into deterministic C++26 source and a CMake manifest; own the
  IR-to-C++ type spellings, the `std::expected` fallible-call convention, the `compat.hpp` helpers
  implementing each behavior axis, C++'s own stance declaration and preserved guarantees, and
  `clang-format` post-processing.
- `python-cpp-bridge`: The nanobind module generated onto compiled C++ so it is callable from
  Python — boundary marshalling, instance binding, and translating a returned failure into the
  Python exception the source operation would have raised.
- `typescript-cpp-bridge`: The same for Node, generated as a node-addon-api addon built with
  cmake-js, including `Napi::ObjectWrap` instance binding and `Error` translation.

### Modified Capabilities

- `pipeline-architecture`: `cpp` becomes an implemented backend and `(python, cpp)` /
  `(typescript, cpp)` become registered bridges. The shared-conformance-corpus requirement is
  tightened: rendering is not coverage, and a backend's corpus output must be compiled and run.
- `semantic-behavior`: C++ declares a complete stance on all six axes.
- `cli`: `--backend cpp` emits C++ source, and the whole-crate form writes a buildable CMake tree.
- `demo`: the demo requirement covers a set of projects, one per bridged pair, rather than one
  project. `demo-python-cpp` and `demo-ts-cpp` join — but they **establish** the standard rather
  than joining an existing one, because `demo-ts-go` has been confirmed not to meet it (#38, #39).
- `fixture-corpus`: the differential tiers run each accepted fixture over every bridged pair the
  registry reports, rather than over one.
- `build-pipeline`: the toolchain a build requires depends on the target, and a missing one is
  named per target rather than as Rust's.
- `python-api`: `backend="cpp"` is selectable from the Python host.
- `typescript-api`: `backend: "cpp"` is selectable from the TypeScript host.

## Impact

- **New crates**: `compylr-backend-cpp`, `compylr-bridge-python-cpp`,
  `compylr-bridge-typescript-cpp` — taking the workspace from 13 to 16.
- **New build dependencies for generated trees**: nanobind (Python side) and node-addon-api +
  cmake-js (Node side), each fetched by its own host's package manager.
- **Modified crates**: [`backends`](../../../crates/compylr-registry/src/backends.rs#L23) and
  [`bridges`](../../../crates/compylr-registry/src/bridges.rs#L22) grow entries;
  [`compylr-cli`](../../../crates/compylr-cli/src/main.rs) gains nothing but a passing name.
- **Modified boundary tests**:
  [`crate_boundaries.rs`](../../../crates/compylr-host-python/tests/crate_boundaries.rs) gains the
  rules for the new crates —
  [`the_rust_backend_knows_no_host_language`](../../../crates/compylr-host-python/tests/crate_boundaries.rs#L236)
  restated for C++, the stance table entry, and the rule that a `compylr-bridge-*` crate links no
  host runtime.
- **Modified host packages**: [`_build.py`](../../../frontends/python/compylr/_build.py#L161)
  learns a second toolchain preflight — a C++26 compiler and CMake, where the Rust path checks
  `cargo` and `maturin` — and a second build driver.
- **New directories**: `demo/demo-python-cpp/`, `demo/demo-ts-cpp/`, and
  `frontends/cpp/` for the C++ side of the corpus.
- **Toolchain**: building generated C++ needs a C++26 compiler (GCC 15+ or Clang 20+) and CMake at
  runtime. Support for C++26 is **partial and uneven** across both, so the demos and the
  differential tier are feature-gated on a working compiler rather than assumed — see
  [`design.md`](design.md) D1 for exactly which features are relied on and what happens when one is
  missing.
- **Generated documentation**: [`update_benchmarks.py`](../../../scripts/update_benchmarks.py) and
  [`update_subset.py`](../../../scripts/update_subset.py) grow the two new demos' markers, and
  [`readme.rs`](../../../crates/compylr-host-python/tests/readme.rs) the new paths.
- **Breaking changes**: none.
