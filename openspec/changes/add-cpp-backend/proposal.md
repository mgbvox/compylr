## Why

[`bridge.rs`](../../../crates/compylr-core/src/bridge.rs#L1-L21) states the one place compylr's
modularity does not come for free: frontends and backends compose N + M because they meet at the
IR and never see each other, but a bridge is keyed by the `(source, target)` **pair** and therefore
costs N × M. It also records the escape hatch, and defers it:

> The trait is shaped so that a canonical-C-ABI hub — one bridge registered for many pairs — could
> be implemented *behind* it later, collapsing N × M back to N + M at the cost of a marshalling
> layer. That trade is deferred, not foreclosed.

Two frontends and two backends make four pairs and we have written two bridges. A third target
written the old way costs two more hand-written bridges, and a third frontend would then cost
three. The deferral comes due now, and **C++ is the target that makes cashing it in honest**: it
is the only supported target whose idiomatic export surface already *is* a C ABI, and both host
runtimes we have can consume one with what they ship — `ctypes` in CPython, `node:ffi` /
`process.dlopen` in Node — rather than through a third-party binding library that would have to be
chosen per pair.

So this change is two things that only make sense together. A third backend, which is the second
*statically compiled* one and the first with move semantics, RAII, and no garbage collector — the
combination [`conformance.rs`](../../../crates/compylr-host-python/tests/conformance.rs#L971)
enumerates from the registry precisely so a backend added tomorrow is covered today. And the first
bridge that is not written twice: one crate emits the export surface, and each frontend adds a
loader. Adding a fourth frontend after this costs a loader, not a bridge.

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

- **`crates/compylr-bridge-cpp-abi`, the shared half.** Emits the `extern "C"` export surface and
  the marshalling layer for every generated member: one exported symbol per member, opaque handles
  for instances, and an out-parameter error channel. It is not registered as a bridge and
  implements no pair; it is what the pair bridges are built from.

- **`crates/compylr-bridge-python-cpp` and `crates/compylr-bridge-typescript-cpp`, the thin
  halves.** Each implements [`HostBridge`](../../../crates/compylr-core/src/bridge.rs#L83) for its
  pair, adds only its own loader — a `ctypes` module and a `.pyi` for Python, an FFI loader and an
  `index.d.ts` for TypeScript — and delegates everything else. Both register in
  [`bridges`](../../../crates/compylr-registry/src/bridges.rs#L22). The registry stays keyed by
  pair: the hub sits *behind* the trait, exactly as the deferral described.

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
  [`demo/demo-python-rust`](../../../demo/demo-python-rust/README.md) sets: the same algorithm
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
- `cpp-abi-bridge`: The shared `extern "C"` export surface and marshalling layer for generated C++,
  and the contract a per-frontend loader implements against it — including how the two registered
  pair bridges are composed from it without breaking pair-keyed resolution.

### Modified Capabilities

- `pipeline-architecture`: `cpp` becomes an implemented backend; `(python, cpp)` and
  `(typescript, cpp)` become registered bridges; and a bridge may be composed from a shared
  target-side ABI plus a source-side loader while still resolving by pair.
- `semantic-behavior`: C++ declares a complete stance on all six axes.
- `cli`: `--backend cpp` emits C++ source, and the whole-crate form writes a buildable CMake tree.
- `demo`: the demo requirement covers a set of projects, one per bridged pair, rather than one
  project; `demo-python-cpp` and `demo-ts-cpp` join at the existing standard.
- `fixture-corpus`: the differential tiers run each accepted fixture over every bridged pair the
  registry reports, rather than over one.
- `build-pipeline`: the toolchain a build requires depends on the target, and a missing one is
  named per target rather than as Rust's.
- `python-api`: `backend="cpp"` is selectable from the Python host.
- `typescript-api`: `backend: "cpp"` is selectable from the TypeScript host.

## Impact

- **New crates**: `compylr-backend-cpp`, `compylr-bridge-cpp-abi`, `compylr-bridge-python-cpp`,
  `compylr-bridge-typescript-cpp` — taking the workspace from 13 to 17.
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
