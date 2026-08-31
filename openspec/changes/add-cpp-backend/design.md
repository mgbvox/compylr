## Context

See [`proposal.md`](proposal.md) — Why. The facts that shape the approach:

* [`bridge.rs`](../../../crates/compylr-core/src/bridge.rs#L18) records the canonical-C-ABI hub as
  deferred rather than foreclosed, and the trait is already shaped for it:
  [`HostBridge::emit`](../../../crates/compylr-core/src/bridge.rs#L95) returns a whole
  [`HostArtifact`](../../../crates/compylr-core/src/bridge.rs#L69), so a bridge is free to obtain
  most of that artifact from somewhere shared.
* [`bridges`](../../../crates/compylr-registry/src/bridges.rs#L22) keys by pair and holds
  `&'static dyn HostBridge` — two entries, no `Option`. Nothing prevents two entries delegating to
  one implementation; the registry never asks where the bytes came from.
* [`conformance.rs`](../../../crates/compylr-host-python/tests/conformance.rs#L971) enumerates
  backends from the registry, and the corpus is authored as IR rather than as any source language,
  so a third backend is covered the moment it is registered. There is nothing to add there.
* The Rust backend's own stance
  ([`RUST_BEHAVIOR`](../../../crates/compylr-backend-rust/src/rust.rs#L226)) is `Unchecked` on
  every axis, and its [`PRESERVES`](../../../crates/compylr-backend-rust/src/rust.rs#L189) names
  all three guarantees. The two are decided separately, and
  [`runtime.rs`](../../../crates/compylr-backend-rust/src/runtime.rs) is why: it implements each
  checked mode, is embedded into generated crates by
  [`RUNTIME_SOURCE`](../../../crates/compylr-backend-rust/src/rust.rs#L71), and is deliberately
  self-contained so it compiles once pasted into somebody else's project.
* [`Unit::add_function`](../../../crates/compylr-ir/src/ir.rs#L1216) refuses a duplicate member
  name across a whole unit, which is what makes a flat `extern "C"` symbol namespace viable at all.
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

* Pay the N × M bridge cost once for this target rather than once per pair, without changing how a
  bridge is resolved.
* A third backend that tests target-neutrality against a language with no garbage collector, where
  ownership of every value crossing the boundary must be decided rather than assumed.
* Parity at the demo level, so the two new pairs are demonstrated the way the first two are.

**Non-Goals:**

* A C++ **frontend**. `cpp` stays reserved on the frontend side, which also keeps a live example
  for the reserved-name scenarios elsewhere.
* Retrofitting the shared-ABI split onto the Rust or Go bridges. Both work; neither's target
  presents a C ABI as its idiomatic surface.
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
set(CMAKE_CXX_STANDARD_REQUIRED ON)
add_library(compylr_generated SHARED generated.cpp bindings.cpp)
```

The features generated code actually relies on are `std::expected`, `std::vector`,
`std::unordered_map`, `std::unordered_set`, `std::tuple`, and the compiler's overflow builtins —
every one of them available well before C++26. The rule is written down in the backend's module doc
so a later contributor does not reach for a half-implemented library feature and make the backend
unbuildable in practice.

**Why:** the standard requested and the features used are separable, and separating them is what
makes "latest standard" a real answer rather than a bet. `CMAKE_CXX_STANDARD_REQUIRED ON` means a
compiler that cannot give C++26 fails at configure time with a message about the standard, which is
actionable, rather than mid-compile with a message about a missing header, which is not. When
contracts and reflection are implemented, the emitted set widens without the manifest moving.

**Alternatives considered:** *Select C++23 and call it latest* — rejected, it is not what was asked
and it forecloses the contract option in D6. *Select C++26 and use it freely* — rejected; support is
partial and uneven across GCC 15 and Clang 20, so the backend's buildability would depend on which
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

**Why:** three reasons that agree. An exception cannot cross the `extern "C"` boundary D3 builds on
— it is undefined behavior — so the error channel has to be a value at the edge regardless, and
having two error mechanisms inside would mean converting between them at every export. The
propagation is visible in the generated source, which is the source a user reads under `.compylr/`
to answer "what did compylr understand?". And it matches what the Rust backend already emits, so the
two targets' generated code reads the same way and a reviewer moving between them is not switching
mental models.

**Alternatives considered:** *Exceptions internally, caught at each export* — rejected; the catch
blocks are per-export boilerplate, and a `noexcept` violation anywhere terminates the process
rather than reporting. *An out-parameter for the error and a plain return* — rejected inside
generated code, because a caller that forgets to check it gets a garbage value silently; it is used
only at the C boundary in D4, where the language offers nothing better. *`std::optional`* —
rejected, it cannot carry which failure occurred, and the diagnostics are the point.

### 3. The bridge splits into one shared ABI crate and two thin loader crates

**Decision:** `compylr-bridge-cpp-abi` emits everything that does not depend on the caller;
`compylr-bridge-python-cpp` and `compylr-bridge-typescript-cpp` add only a loader and register the
pair.

```rust
// compylr-bridge-python-cpp/src/bridge.rs
impl HostBridge for PythonCppBridge {
    fn source(&self) -> &'static str { "python" }
    fn target(&self) -> &'static str { "cpp" }

    fn emit(&self, unit: &Unit, key: &BuildKey) -> Result<HostArtifact, BackendError> {
        // Everything target-side: generated.cpp, compat.hpp, bindings.cpp, CMakeLists.txt.
        let mut artifact = compylr_bridge_cpp_abi::emit_shared(unit, key)?;
        // Everything source-side: the ctypes loader and its type stubs.
        artifact.files.insert("__init__.py".into(), emit_ctypes_loader(unit, &artifact.loaded_as));
        artifact.files.insert("__init__.pyi".into(), emit_stubs(unit));
        Ok(artifact)
    }
}
```

**Why:** the registry keys by pair because a calling convention is a negotiation between two
runtimes — that reasoning is sound and stays. What it does not imply is that every *byte* of the
artifact depends on both, and for a C++ target most of them depend only on the target. Splitting
where the dependence actually falls means the second pair cost a loader, and the third will too. The
composition is invisible to resolution: `bridges::lookup` sees two entries and cannot tell.

The cost is real and worth stating: a shared surface is a shared failure mode, so a defect in
`emit_shared` breaks both pairs at once. That is why the differential tier in the specs runs *per
pair* rather than once — a shared implementation still has to be checked from each side.

**Alternatives considered:** *Two full bridges, as Go and Rust have* — rejected; it is the N × M
cost paid deliberately when this target does not require it, and the two would drift. *One bridge
registered for many pairs* — rejected; the loader genuinely differs, `HostBridge::source` returns
one `&'static str`, and making a bridge answer "several" would change the trait for every existing
implementation to serve one target. *A `ctypes` loader for both* — rejected, Node cannot use it.

### 4. The C surface is one flat symbol per member, with handles for instances

**Decision:** each member exports one `extern "C"` symbol taking its arguments by C-compatible
value, returning an integer status, and writing its result through an out-parameter. An instance is
an opaque handle.

```cpp
extern "C" {
    // Scalars in, result out, status returned. 0 is success.
    int32_t compylr_divide(int64_t a, int64_t b, int64_t* out, compylr_err* err);
    // A sequence crosses as pointer plus length; the callee copies.
    int32_t compylr_running_totals(const int64_t* values, size_t n, int64_t divisor,
                                   int64_t** out, size_t* out_n, compylr_err* err);
    // An instance is a handle the caller owns and releases.
    int32_t compylr_PrimeCache_new(compylr_handle* out, compylr_err* err);
    void    compylr_free_i64(int64_t* p, size_t n);
    void    compylr_PrimeCache_free(compylr_handle h);
}
```

**Why:** flat symbols are viable because
[`Unit::add_function`](../../../crates/compylr-ir/src/ir.rs#L1216) already refuses a duplicate name
across the whole unit — the property four demo fixtures carry a header about — so `extern "C"`
losing overloading costs nothing. Every buffer is freed by the surface that allocated it, which is
the one ownership rule a language without a collector cannot leave implicit; a loader that called
its own runtime's free on a C++ allocation is the defect this shape exists to make impossible.

The handle is what preserves the contrast the subset already draws and that people get wrong: a
collection **parameter** crosses by value and cannot be mutated observably, while an **instance** is
not converted at all — so a mutated attribute is what the caller sees next call, and an attribute can
be a cache. Copying an instance across would silently break every memoized demo variant.

**Alternatives considered:** *Serialize arguments to a buffer and pass one pointer* — rejected; it
adds a format to version and a copy per call on top of the per-element price already measured.
*Return a struct by value* — rejected; struct return across a C ABI is where calling conventions
differ most between platforms. *Reference-counted handles* — rejected as unneeded: the calling
runtime already has a lifetime for the object it wraps, and a second count is a second thing to get
wrong.

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
concern because "GCC 14 is present" and "GCC 15 is required" is exactly the case D1's configure-time
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

## Risks / Trade-offs

**C++26 support is partial and uneven, so a machine that builds everything else may not build this.**
→ D1 confines the emitted feature set to what ships, and `CMAKE_CXX_STANDARD_REQUIRED ON` fails at
configure time naming the standard. The demo and differential specs both require a missing toolchain
to be reported as *skipped* naming the tool, never as a pass — a green suite that silently never
compiled C++ is the failure mode worth spending a requirement on.

**A shared ABI surface is a shared failure mode: one defect breaks both pairs.** → The differential
tier runs per registered pair rather than once, so a defect that only manifests through one loader
is still caught; and the "two source languages receive the same target-side artifact" scenario pins
that the shared half really is shared, so the split cannot quietly become a copy.

**Manual memory management at the boundary is the one place this target can leak or double-free,
and neither shows up as a wrong answer.** → D4 makes the allocating side the freeing side, and the
ABI spec requires a handle to be released exactly once. This is where the implementation should
expect to spend its debugging time; running the boundary tier under a sanitizer is the cheap check
and belongs in tasks.

**The per-element boundary price is paid again, possibly at a different rate.** → The measured Python
price is recorded in `CLAUDE.md` for the Rust path; nothing here assumes it carries over. The demos
are what report it, which is the point of building them rather than asserting speedups.

**Four demos roughly double the slow suite.** → Each is grouped with the existing slow tests and
skipped with a named reason when its toolchain is absent, so the fast suite is unchanged and CI
opts in per pair.

**`std::unordered_map` iteration order differs from every other target's.** → Not a risk to manage,
a rule already held: the subset promises neither mapping nor set iteration order, and a test that
distinguishes them by iteration order is asserting behavior the language deliberately does not
provide. Worth restating here because a new target is exactly when someone writes that test.

**Two more crates named for a language raises the "which of these is which" question again.** →
The existing convention answers it: a crate is named for the *job*. `compylr-backend-cpp` writes
C++, `compylr-bridge-cpp-abi` is the target-side half of calling it, and
`compylr-bridge-<source>-cpp` is a source-side half. The boundary tests are extended to state it
rather than leaving it to the names.

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
  scenario in the ABI spec, the choice is bounded to one file, and it should be made against the
  Node version the TypeScript host actually targets rather than in advance.
