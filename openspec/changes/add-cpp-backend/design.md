## Context

See [`proposal.md`](proposal.md) — Why. The facts that shape the approach:

* [`bridge.rs`](../../../crates/compylr-core/src/bridge.rs#L18) records the canonical-C-ABI hub as
  deferred rather than foreclosed. It stays deferred — but **not** because Node cannot consume a C
  ABI. It can, as of `node:ffi` in Node v26.1.0. D3 records why an experimental, flag-gated, self-
  described-unsafe module that postdates this project's own Node is still the wrong foundation.
* [`bridges`](../../../crates/compylr-registry/src/bridges.rs#L22) keys by pair and holds
  `&'static dyn HostBridge`. Two more entries join it; nothing about resolution changes.
* [`conformance.rs`](../../../crates/compylr-host-python/tests/conformance.rs#L971) enumerates
  backends from the registry, and the corpus is authored as IR rather than as any source language.
  **An earlier draft of this document claimed the corpus also compiles what it renders, and that a
  third backend was therefore covered the moment it was registered. That is false.** An audit
  confirmed the Go-backend path is *render-only*: the emitted Go for the corpus's own entries is
  never compiled and never run, so the check establishes that a backend produced text, not that the
  text builds or answers correctly. The C++ backend inherits no such safety net, and this change has
  to build one — see tasks group 4a.
* The Rust backend's own stance
  ([`RUST_BEHAVIOR`](../../../crates/compylr-backend-rust/src/rust.rs#L226)) is `Unchecked` on
  every axis, and its [`PRESERVES`](../../../crates/compylr-backend-rust/src/rust.rs#L189) names
  all three guarantees. The two are decided separately, and
  [`runtime.rs`](../../../crates/compylr-backend-rust/src/runtime.rs) is why: it implements each
  checked mode, is embedded into generated crates by
  [`RUNTIME_SOURCE`](../../../crates/compylr-backend-rust/src/rust.rs#L71), and is deliberately
  self-contained so it compiles once pasted into somebody else's project.
* [`Unit::add_function`](../../../crates/compylr-ir/src/ir.rs#L1216) refuses a duplicate member
  name across a whole unit, which is what makes a flat binding namespace viable at all — see D4.
* [`_build.py`](../../../frontends/python/compylr/_build.py#L161) checks `cargo` and `maturin`
  unconditionally, before any target is consulted.
* [`CLAUDE.md`](../../../CLAUDE.md) records the per-element boundary price already measured for the
  Python path, and the three cost defects the demo found that no correctness test saw.

**This change does not touch the IR.** No node gains a field, no form is added, the artifact
`version` does not move, [`Unit::fingerprint`](../../../crates/compylr-ir/src/ir.rs#L1299) covers
exactly what it covered, and no cache is invalidated. A third backend that needed the IR to change
would be evidence the IR was not target-neutral; that it does not is the result this change reports.
The demo coverage claim is therefore untouched too — no form is added for the demos to reach.

## Goals / Non-Goals

**Goals:**

* Two working bridges on each host's own C++ binding library, accepting the N x M cost rather than
  paying for a hub that Node cannot use.
* A third backend that tests target-neutrality against a language with no garbage collector, where
  ownership of every value crossing the boundary must be decided rather than assumed.
* Parity at the demo level, so the two new pairs are demonstrated the way the first two are.

**Non-Goals:**

* A C++ **frontend**. `cpp` stays reserved on the frontend side, which also keeps a live example
  for the reserved-name scenarios elsewhere.
* A canonical-C-ABI hub, here or anywhere. D3 records why the target that looked likeliest to
  justify one is in fact the target that needs one least.
* Making generated C++ idiomatic C++. It is generated code that must *mean* what the unit declares,
  which is sometimes not what a person would write.
* Growing the accepted subset in any direction.
* Beating the Rust backend. The demos measure; nothing here promises a number.

## Decisions

### 1. The manifest selects `-std=c++26`, and the emitted feature set is deliberately narrower

**Decision:** every generated tree asks for C++26, and the backend emits only features that
shipping compilers implement.

```cmake
cmake_minimum_required(VERSION 3.28)
project(compylr_generated LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 26)
set(CMAKE_CXX_STANDARD_REQUIRED OFF)
add_library(compylr_generated SHARED generated.cpp bindings.cpp)
target_compile_features(compylr_generated PRIVATE cxx_std_23)
```

The features generated code actually relies on are `std::expected`, `std::vector`,
`std::unordered_map`, `std::unordered_set`, `std::tuple`, and the compiler's overflow builtins —
every one of them available well before C++26. The rule is written down in the backend's module doc
so a later contributor does not reach for a half-implemented library feature and make the backend
unbuildable in practice.

Verified against the compilers' own status pages rather than recollection:

| | accepts the mode | contracts | reflection |
| --- | --- | --- | --- |
| GCC | **14** (`-std=c++26`, "experimental support") | GCC **16** | GCC 16, `-freflection`, incomplete |
| Clang | spells it **`-std=c++2c`**, support "Partial" | **No** | **No** (P2996, P3394, P3293, P3491, P3096, P3598 all unimplemented) |

Two corrections to an earlier draft fall out. The floor is GCC **14**, not 15. And Clang has neither
contracts nor reflection at any version — so a feature set that reached for C++26's headline
additions would be unbuildable on Clang entirely, which is exactly what confining the emitted set
avoids.

On spelling: Clang's status page documents `-std=c++2c`, but modern Clang (21, including AppleClang)
accepts `-std=c++26` too; only older Clang requires `c++2c`. Emit `CMAKE_CXX_STANDARD` and let CMake
pick — not because one spelling is wrong, but because that keeps the manifest correct across both
without probing, and probing would break the pure-emission rule.

**Why:** the standard requested and the features used are separable, and separating them is what
makes "latest standard" a real answer rather than a bet. The manifest **requests** 26 and
**requires** 23: an adversarial review measured that the whole emitted set builds under
`-std=c++23` on AppleClang 21/libc++, so `CMAKE_CXX_STANDARD_REQUIRED ON` would have refused
compilers that build the tree perfectly well. `target_compile_features(... cxx_std_23)` states the
real floor, and `CMAKE_CXX_STANDARD 26` still asks for the newest. When contracts and reflection are
implemented, the emitted set widens without the manifest moving.

**Alternatives considered:** *Select C++23 and call it latest* — rejected, it is not what was asked
and it forecloses the contract option in D6. *Select C++26 and use it freely* — rejected; support is
partial and uneven across GCC and Clang, so the backend's buildability would depend on which
library features a given release happened to land, which is not a property a compiler should have.
*Probe the compiler and downgrade* — rejected outright: emission is a pure function of the unit, and
a manifest that varies by machine breaks the byte-reproducibility the rebuild cache is keyed on.

### 2. A fallible operation returns `std::expected`; nothing throws across a boundary

**Decision:** a function whose body contains an operation the resolved behavior can report on
returns `std::expected<T, compylr::Error>`, and failures propagate by early return.

```cpp
std::expected<int64_t, compylr::Error> divide(int64_t a, int64_t b) {
    auto q = compylr::floor_div_checked(a, b);
    if (!q) return std::unexpected(q.error());
    return *q;
}
```

**Why:** three reasons that agree. The first was stated wrongly in an earlier draft, which claimed
both binding libraries *forbid* a C++ exception reaching them. They do not — both translate one. The
real argument is stronger: nanobind's default translation table flattens `std::exception` to
`RuntimeError`, so preserving the failure **kind** that `python-api` requires (`ZeroDivisionError`,
`OverflowError`, `KeyError`, `IndexError`) would need a distinct C++ exception type *and* a
registered translator per kind, in **both** bridges. A returned `compylr::Error` carries the kind as
data and each bridge maps it once. The
propagation is visible in the generated source, which is the source a user reads under `.compylr/`
to answer "what did compylr understand?". And it matches what the Rust backend already emits, so the
two targets' generated code reads the same way and a reviewer moving between them is not switching
mental models.

**Alternatives considered:** *Exceptions internally, caught at each export* — rejected; the catch
blocks are per-export boilerplate, and a `noexcept` violation anywhere terminates the process
rather than reporting. *An out-parameter for the error and a plain return* — rejected inside
generated code, because a caller that forgets to check it gets a garbage value silently.
*`std::optional`* —
rejected, it cannot carry which failure occurred, and the diagnostics are the point.

### 3. Two pairwise bridges on each host's own C++ binding library, not a shared C-ABI hub

**Decision.** `compylr-bridge-python-cpp` emits nanobind; `compylr-bridge-typescript-cpp` emits
node-addon-api. There is no shared ABI crate. Bridges stay N x M.

```rust
// compylr-bridge-python-cpp
impl HostBridge for PythonCppBridge {
    fn source(&self) -> &'static str { "python" }
    fn target(&self) -> &'static str { "cpp" }
    fn emit(&self, unit: &Unit, key: &BuildKey) -> Result<HostArtifact, BackendError> {
        let mut files = compylr_backend_cpp::CppBackend.emit(unit)?;   // generated.cpp, compat.hpp
        files.insert("bindings.cpp".into(), emit_nanobind_module(unit, &loaded_as));
        // ...
    }
}
```

**Why.** An earlier draft proposed a shared `compylr-bridge-cpp-abi` crate emitting one
`extern "C"` surface with thin per-frontend loaders, cashing in the canonical-C-ABI hub
[`bridge.rs`](../../../crates/compylr-core/src/bridge.rs#L18) defers. That draft justified itself on
the claim that *Node cannot consume a C ABI with anything it ships*. **That claim is now false and
the correction is worth recording, because the conclusion survives it for different reasons.**

Node **does** have a built-in FFI: [`node:ffi`](https://nodejs.org/api/ffi.html), added in
**v26.1.0**, with `dlopen(path, definitions)` that resolves symbols out of a plain C library without
any addon. So a C-ABI hub is no longer impossible on the Node side. It is still the wrong choice:

* It is **experimental** (Stability 1) and requires `--experimental-ffi` plus FFI support compiled
  into the Node build. A compiler whose generated code only runs behind a flag is not shipping.
* Its own documentation calls it **unsafe**: "incorrect pointer usage, wrong signatures, or
  accessing freed memory can crash the process or corrupt memory." Node-API is ABI-stable across
  major versions by contract; `node:ffi` offers no such guarantee.
* **v26.1.0 is newer than the ground.** This project's own environment runs Node v24.11.0, where
  the module does not exist at all.
* And the argument that actually decides it: once Python->C++ goes through nanobind, "one mechanism
  everywhere" is already gone. A hub would buy `node:ffi` + nanobind — still two mechanisms, one of
  them experimental — rather than node-addon-api + nanobind, which are both first-class.

Once Python->C++ goes through nanobind, the hub's only remaining benefit — one mechanism everywhere
— is gone. The inversion is the interesting part: **C++ is the target that needs a hub least**,
because both hosts' first-class binding libraries are already C++ header libraries. Every other
target has to reach across a language gap to bind; C++ is the one where the host's own tooling meets
you where you are.

`Napi::ObjectWrap<T>` and nanobind's `nb::class_` each give a live native instance the host holds by
identity — which is the contrast the accepted subset already draws between a collection parameter
(crosses by value) and an instance (not converted at all), and the hardest thing to hand-roll over a
C ABI.

**The measured half.** The rejected draft's Python side was a `ctypes` loader. In the
convert-on-every-call regime, ctypes measures **~31×** against PyO3 (arXiv:2507.00264, Table IV;
research/python-call-overhead.md), and compylr's boundary is that regime by construction — every
argument, collections included, crosses by value on every call. That condemns **a ctypes loader**.
It does not condemn hubs as a class, and it is supporting evidence rather than the reason: the
decision stands on node-addon-api's ABI guarantee alone.

**Alternatives considered.** *A shared C-ABI hub* — rejected on the binding-library argument above,
with the ctypes measurement as corroboration. *pybind11
instead of nanobind* — rejected: same author, but nanobind has dramatically faster compile times
(which matter when the first call compiles), smaller binaries, and a real stable-ABI story on 3.12+.
*Keeping the pair count down by skipping the TypeScript side* — rejected; every `(source, target)`
pair owes a working demo, so a target with one bridge is half a target.

### 4. Flat member names make either binding library straightforward

**Decision.** No name mangling scheme, no namespacing: each member binds under its own name.

```cpp
NB_MODULE(compylr_generated_<fp>_<variant>, m) {
    m.def("running_totals", &running_totals);
    nb::class_<PrimeCache>(m, "PrimeCache").def(nb::init<>()).def("nth", &PrimeCache::nth);
}
```

**Why.** [`Unit::add_function`](../../../crates/compylr-ir/src/ir.rs#L1216) already refuses a
duplicate member name across a whole unit — the property four demo fixtures carry a header about — so
a flat binding namespace is safe without further work. This was originally load-bearing for an
`extern "C"` surface, which has no overloading; it survives D3's replacement because both binding
libraries are simpler when names are unique, not because either requires it.

**Alternatives considered.** *Mangling to allow duplicates* — rejected; it would weaken a uniqueness
rule the corpus and the demos already depend on, to buy nothing.

### 5. C++'s stance is unchecked; the backend preserves all three guarantees anyway

**Decision:** the two declarations are decided separately, exactly as the Rust backend decides them.

```rust
pub const CPP_BEHAVIOR: LanguageBehavior = LanguageBehavior {
    // Signed overflow is undefined in C++; the program does not define the failure.
    integer_overflow: Checked::Unchecked,
    // `-7 / 2` is `-3`; a zero divisor is undefined behavior, not a report.
    integer_division: IntegerDivision { rounding: Rounding::TowardZero, checked: Checked::Unchecked },
    // ...
    text_length: TextUnits::Utf8Bytes,
};

// Wider than the stance, because `compat.hpp` implements every checked mode.
const PRESERVES: &[Guarantee] = &[
    Guarantee::IntegerOverflowReported,
    Guarantee::DivisionByZeroReported,
    Guarantee::FloatOrderPreserved,
];
```

**Why:** this is the decision most likely to be got backwards, and getting it backwards has a loud
consequence: Python's stance on overflow is *reported*, so a default Python program requires
`IntegerOverflowReported`, and a C++ backend that derived its preserved set from C++'s native
operator would refuse every one of them by name — including every algorithm in the new demo. A
stance answers "what does `a + b` mean in this language"; a preserved guarantee answers "can you be
made to report it when asked". `compat.hpp` answers yes, using the compiler's overflow builtins, so
the guarantee is preserved and the demos compile with no behavior override.

**Alternatives considered:** *Derive `preserves()` from the stance* — rejected on the above; it also
would have made the Rust backend, whose stance is unchecked on every axis, preserve nothing.
*Preserve overflow reporting only under an opt-in* — rejected; it makes the default target
silently different from the other two, which is precisely the parity this change is for.

### 6. `compat.hpp` is one self-contained header, and `cpp26-contracts` is declared and refused

**Decision:** the helpers live in a single header embedded verbatim, mirroring
[`runtime.rs`](../../../crates/compylr-backend-rust/src/runtime.rs); the contract facility is a
declared, unimplemented option.

```rust
pub const CPP_COMPAT_SOURCE: &str = include_str!("compat.hpp");

const OPTIONS: &[TargetOption] = &[TargetOption {
    name: "cpp26-contracts",
    breaks: &[Guarantee::IntegerOverflowReported, Guarantee::DivisionByZeroReported],
    implemented: false,
}];
```

**Why (header):** embedding it verbatim means the helpers are unit-testable in this workspace *and*
are the same text the user's build compiles — the two-lives property `runtime.rs` documents. A single
header rather than a header plus a source file because the helpers are templates and constexpr
functions, which have to be visible at the point of use anyway. It must stay self-contained: no
include of anything from this project, or it fails to compile once pasted into somebody else's tree.

**Why (option):** contracts would let a checked mode be expressed as a precondition rather than a
branch, which is a real transformation someone will ask for, and the negotiation exists so that "why
is compylr not emitting the fast thing?" has something to point at. Permitting it fails saying it is
reserved — the same three-way honesty the registries use, and the reason
[`unchecked-arithmetic`](../../../crates/compylr-backend-rust/src/rust.rs#L202) is declared and not
implemented.

**Alternatives considered (option):** *Omit it* — rejected; a backend that declares no option is
indistinguishable from one that has nothing to trade, and this one does. *Implement it* — rejected
for this change; contract semantics under the checked modes is its own design, and shipping it
alongside a new backend would mean debugging both at once.

### 7. Toolchain preflight moves behind the backend rather than growing a second branch

**Decision:** what a build requires is asked of the selected target, not chosen by an `if` in the
manager.

```python
    # before - frontends/python/compylr/_build.py, unconditional
    if shutil.which("cargo") is None:
        raise MissingToolchain("The Rust toolchain (cargo)", ...)

    # after - the target says what it needs, including a version floor
    for tool in requirements_for(self.settings.backend):
        tool.check()   # names the tool, the version it needs, and how to install it
```

**Why:** the existing check is unconditional and Rust-specific, so a project targeting C++ on a
machine with no Rust would be told to install cargo — a diagnostic that sends someone to fix
something that is not wrong. A version floor is part of the requirement rather than a separate
concern because "GCC 14 is present" and "a newer GCC is required" is exactly the case D1's configure-time
failure is trying to name early, and a presence-only check would let it through to a compile error
about a missing header.

**Alternatives considered:** *A branch per backend in the manager* — rejected; it puts a table of
target knowledge in the host package, which is where target knowledge is not supposed to be. *Let
the build fail and surface the tool's own error* — rejected; that is exactly what the existing
requirement forbids, and CMake's message about a missing standard is worse than a named one.

### 8. Both demos are new projects, but the coverage and benchmark machinery is shared, not copied

**Decision:** `demo/demo-python-cpp` and `demo/demo-ts-cpp` carry their own algorithms, README, and
tests; what they do not carry is a fourth and fifth copy of the coverage walker and the benchmark
harness. The set of demos is derived from the bridge registry, so a bridge registered without one
fails.

This is the decision with no code face worth showing: it is a placement call, and the snippet would
only be an import.

**Why:** the coverage walker reads `.compylr/ir/unit.json`, which is target-independent by
construction, and the benchmark's job — best-of-N per call, both modes in separate processes,
answers compared — is the same job in every pair. Copying them a third and fourth time is how the
fixture lists in this repository once drifted and hid a real defect. Deriving the demo set from the
registry is the same discipline `conformance.rs` already uses for backends.

The algorithms themselves are genuinely per-project: they are written in the pair's source language,
and demo-ts-cpp's are TypeScript. Sharing those would mean generating them, which is a compiler for
demos.

**Alternatives considered:** *One demo per source language, parameterized by target* — attractive,
and rejected because the benchmark tables are per pair and the READMEs are the document a reader
lands on; a project that compiles two ways has no single set of numbers to show. *No new demos* —
rejected, it is the parity that was asked for.

### 9. Ownership at the boundary: the host owns instances, the callee owns nothing after returning

**Decision.** An instance the host holds lives in `nb::class_`/`Napi::ObjectWrap` storage and is
**borrowed** by generated code, never owned by it. A returned collection is **moved** into the
binding layer, which copies into the host's representation. Generated code holds no reference to any
argument after it returns.

```cpp
// borrowed: the host object owns the value; the method sees a reference
std::expected<int64_t, compylr::Error> PrimeCache_nth(PrimeCache& self, int64_t n);
// returned: moved out, then converted; nothing generated retains a pointer to it
std::expected<std::vector<int64_t>, compylr::Error> sieve(int64_t n);
```

**Why.** C++ has no borrow checker, so the rule the Rust path gets from the compiler has to be a
written decision here. Getting it wrong does not produce a wrong answer — it produces a leak, a
double free, or a dangling reference, none of which a differential test detects. This is the risk
the Risks section names, and until now the change mitigated it nowhere.

**Alternatives considered.** *Copy instances across* — rejected; it breaks the contrast the subset
draws, where a mutated attribute is what the caller sees next call. *Reference counting on the C++
side* — rejected as redundant: the host runtime already owns a lifetime for the wrapper object.

### 10. A mapping read reports, and never inserts

**Decision.** `d[k]` emits through a `compat.hpp` helper over `find()`, returning
`std::expected`. `std::unordered_map::operator[]` is never emitted for a read.

```cpp
// NOT operator[] -- it default-constructs the missing key and is non-const
template <class K, class V>
std::expected<V, compylr::Error> map_get(const std::unordered_map<K,V>& m, const K& k);
```

**Why.** The IR states a missing mapping key **always** reports — it is one of the three container
behaviours deliberately given no mode. `operator[]` would silently insert a default-constructed
value and return it, so `d["absent"]` would answer `0` *and grow the map*. **A mapping read therefore
makes its function fallible**, exactly as a checked division does. This is the most likely
silent-wrong-answer in the whole backend and nothing in the change mentioned it before this pass.

**Alternatives considered.** *`at()`* — closer, but throws, which D2 forbids crossing the boundary.

### 11. Class-valued signatures work on day one

**Decision.** An instance parameter is a `T&` (or `const T&`) over the object the host holds — the
C++ analogue of `PyRef`/`PyRefMut`. A borrowed instance, **and a field read from one**, may not
leave in an owned return.

**Why.** `class_valued_signatures.py` is an accepted fixture, `CLAUDE.md` records that it runs
through **both** differential tiers, and this change's `fixture-corpus` delta requires both tiers over
**every** registered pair. So `(python, cpp)` must handle it immediately; it is not deferrable.

The escape rule is a located diagnostic in the Rust path because the generated code compiles either
way and the caller would get a detached copy. In C++ the same mistake is a **dangling reference**, so
the diagnostic matters more here, not less.

## Risks / Trade-offs

**C++26 support is partial and uneven, so a machine that builds everything else may not build this.**
→ D1 confines the emitted feature set to what ships, and `target_compile_features(cxx_std_23)` fails
at configure time naming the real floor rather than an aspirational one. The demo and differential specs both require a missing toolchain
to be reported as *skipped* naming the tool, never as a pass — a green suite that silently never
compiled C++ is the failure mode worth spending a requirement on.

**Two independent bridges is duplicated marshalling logic that can drift.** → Accepted, and it is
the price of D3. What bounds it: both bridges consume the *same* `compylr-backend-cpp` output, so
only the binding layer differs, and the differential tier runs per registered pair so a defect
manifesting through one binding library is still caught by the other's run disagreeing with CPython.

**The corpus check will not catch a C++ backend that emits text which does not build.** → This is
the confirmed defect recorded in Context: the Go path renders without compiling. Tasks group 4a
builds the compile-and-run tier before the C++ backend leans on it, so this change does not inherit
a check that has never worked.

**Ownership at the boundary is the one place this target can leak, double-free, or dangle, and none
of those shows up as a wrong answer.** → D9 decides it; tasks group 5/6 runs the boundary tier under
AddressSanitizer and LeakSanitizer at least once per bridge. An earlier draft pointed this risk at a
"D4" that no longer says anything about ownership and at an ABI spec deleted with the hub, leaving
the risk with no mitigation anywhere in the change.

**The per-element boundary price is paid again, possibly at a different rate.** → The measured Python
price is recorded in `CLAUDE.md` for the Rust path; nothing here assumes it carries over. The demos
are what report it, which is the point of building them rather than asserting speedups.

**Four demos roughly double the slow suite.** → Each is grouped with the existing slow tests and
skipped with a named reason when its toolchain is absent, so the fast suite is unchanged and CI
opts in per pair.

**"Nothing throws" cannot mean what it sounds like.** → `push_back`, `std::string`, and map
insertion can each throw `std::bad_alloc` or `std::length_error` from inside emitted code. The
honest promise is narrower and testable: no *compylr-defined* failure is signalled by throwing —
every one is returned — and each exported entry point terminates or translates anything originating
below it, so nothing reaches the host runtime. The spec scenario is narrowed to match.

**`std::unordered_map` iteration order differs from every other target's.** → Not a risk to manage,
a rule already held: the subset promises neither mapping nor set iteration order, and a test that
distinguishes them by iteration order is asserting behavior the language deliberately does not
provide. Worth restating here because a new target is exactly when someone writes that test.

**Three more crates named for a language raises the "which of these is which" question again.** →
The existing convention answers it: a crate is named for the *job*. `compylr-backend-cpp` writes
C++; `compylr-bridge-python-cpp` and `compylr-bridge-typescript-cpp` each make it callable from one
source language. The boundary tests are extended to state it rather than leaving it to the names.

## Migration Plan

Nothing to migrate. No IR change, no artifact `version` bump, no cache invalidation, and no change
to the Python → Rust or TypeScript → Go paths. `cpp` moves from reserved to implemented on the
backend side only, which is a strictly widening change: every name that resolved before resolves the
same way.

During development in this repository the compylr version does not move, so the rebuild key does not
either — `make clean-artifacts` before every measurement, for the reason `CLAUDE.md` records having
cost real time once already.

## Open Questions

* **Which C++ formatter to shell out to in `post_process`, and with which style.** `clang-format` is
  the obvious answer; whether to emit a style file beside the source or pass a built-in style is a
  detail that changes no requirement, no spec scenario, and no task, and is best decided against
  real generated output.
* **Whether the TypeScript loader uses `process.dlopen` or Node's FFI surface.** Both satisfy every
  scenario in `specs/cpp-abi`… — superseded: with D3 there is no such spec. The remaining question is
  narrower and belongs to node-addon-api, not to a hub: the choice is bounded to one file and should
  be made against the
  Node version the TypeScript host actually targets rather than in advance.
