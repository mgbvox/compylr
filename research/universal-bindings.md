# Research: universal/N-language binding generators (for the C++-backend / bridge planning)

Scope: for each project — what the intermediate description is, which host languages it reaches,
what it costs (perf/expressiveness), how it handles object identity/mutation/errors across the
boundary, and whether a canonical-C-ABI hub is actually how the successful ones solve the N×M
problem. All claims below are either a direct quote/paraphrase from a fetched primary source
(cited inline) or explicitly marked UNVERIFIED. WebSearch ran out of budget partway through this
session (200/200 used); the remainder relied on WebFetch against sources already identified.

---

## 1. Mozilla UniFFI (uniffi-rs)

**What it is / IR.** A Rust crate you annotate; the interface can be described two ways: a
dedicated IDL file (UDL — "UniFFI Definition Language") or inline proc-macro attributes on the
Rust source itself. Both compile down to the same internal component-interface model that the
per-language backends consume.
Source: https://github.com/mozilla/uniffi-rs/blob/main/README.md

**Languages reached.** Primary/first-party bindings: **Kotlin, Swift, Python, Ruby**. Third-party
(externally maintained) backends: **Go, C#, Dart, Java, Node.js, Haskell**, plus a
JS/WASM/React-Native variant.
Source: https://github.com/mozilla/uniffi-rs/blob/main/README.md

**Production use.** "UniFFI is currently used extensively by Mozilla in Firefox mobile and
desktop browsers; written once in Rust, auto-generated bindings allow that functionality to be
called from both Kotlin (for Android apps) and Swift (for iOS apps)."
Source: https://firefox-source-docs.mozilla.org/rust-components/developing-rust-components/uniffi.html
(quoted via the README fetch)

**Object identity.** Every exposed Rust struct/object instance is wrapped in `Arc<T>`. Crossing
into foreign code is the "arc-to-pointer dance": `Arc::into_raw` turns the `Arc` into a `u64`
handle ("Interfaces are lowered as `u64` handles. `0` is reserved as an invalid value.") that is
handed to the foreign side as an opaque number — the foreign language never sees Rust struct
layout. Foreign code clones a handle by calling back into an FFI function that runs
`Arc::increment_strong_count`; freeing calls a function that runs `Arc::decrement_strong_count`,
transferring ownership back to Rust, which drops the value via `Arc::from_raw`.
Source: https://mozilla.github.io/uniffi-rs/latest/internals/object_references.html

**Mutation.** Because calls can arrive from any foreign thread outside Rust's own ownership
system, **every exposed object must be `Send + Sync` and usable without `&mut self`** — i.e.
mutability has to be pushed inside the type via `Mutex`/`RwLock` (interior mutability), not
expressed as an ordinary Rust `&mut` receiver at the boundary.
Source: https://mozilla.github.io/uniffi-rs/latest/internals/object_references.html

**Errors.** UDL lets you declare `[Error] enum` (a "flat" error exposed to foreign code without
its associated data) or `[Error] interface` (a "rich" error whose fields *are* exposed), and mark
a fallible function `[Throws=ErrorType]`.
Source: https://mozilla.github.io/uniffi-rs/latest/udl/errors.html
UNVERIFIED (not found in the fetched pages): the exact runtime mechanism that turns a Rust `Err`
into a Kotlin/Swift/Python native exception — the errors.html page documents only the UDL
declaration syntax, not the lifting/lowering internals. The likely mechanism (documented
elsewhere in the manual under "Lifting, Lowering and Serialization," not fetched here) is that
each FFI call has an out-parameter for a `RustCallStatus` code plus buffer, and each language
backend's generated wrapper raises its native exception type when that status is non-zero — but
this session did not fetch a page that states it directly, so treat it as plausible, not verified.

**Cost / expressiveness.** No performance numbers were found in any fetched UniFFI page in this
session (README, interfaces.html, errors.html, object_references.html all omit benchmarks).
UNVERIFIED beyond that omission.

**Canonical-C-ABI-hub or not?** UniFFI is **not** structured as a shared hub crate that other
tools link against. Each foreign-language backend is generated code that talks to raw
`extern "C"` FFI functions exported directly by the user's own Rust crate (via the `uniffi`
proc-macro / scaffolding) — there's no separate, versioned "ABI crate" in between. The `u64`
handle scheme above *is* effectively a minimal, purpose-built ABI, but it's inline in the
generated Rust, not an independent hub artifact.

---

## 2. Diplomat (rust-diplomat/diplomat)

**What it is / IR.** A `#[diplomat::bridge]` proc macro marks Rust modules to expose. Internally
diplomat-tool uses a two-layer IR: a lightweight AST (built by simplifying `syn` output, "designed
to not need whole-program information") that is then lowered into an HIR ("higher-level
intermediate representation... _much_ nicer to work with") used for actual codegen. Language
support is a plugin interface over that HIR.
Sources:
https://github.com/rust-diplomat/diplomat/blob/main/README.md ,
https://manishearth.github.io/blog/2026/06/14/diplomat-multi-language-ffi-for-rust-libraries/

**Directionality.** Explicitly **unidirectional**: "for when foreign code wishes to call into a
Rust library, but not vice versa." It also deliberately does *not* do cross-crate whole-program
analysis — only specially tagged (`#[diplomat::bridge]`) modules are scanned, so a change in an
unrelated dependency struct cannot silently change the generated API.
Source: https://github.com/rust-diplomat/diplomat/blob/main/README.md

**Languages reached.** C, C++, Dart, JavaScript/TypeScript, .NET (C#), Kotlin (via JNA), and
Python (via **nanobind**).
Source: https://github.com/rust-diplomat/diplomat/blob/main/README.md
(Note: this is the same Python↔C++ binder — nanobind — the user has already decided compylr's own
Python↔C++ bridge should use, per the assignment brief; Diplomat independently converged on the
same choice.)

**Origin / Google connection.** Built as the supplemental FFI tool for **ICU4X**, Google's Rust
internationalization library, originally as a Google-internship-adjacent project.
Source: search snippet citing Google's ICU4X blog (WebSearch result, not independently re-fetched
from google's own blog in this session) — confidence **medium**, not re-verified against a primary
Google page.

**Design rationale / why a C layer, but not a "hub crate."** From the design doc: "the bindings
should all go through an underlying C layer" — Diplomat generates a stable, auto-derived C
interface *per Rust crate* as the lingua franca, and each language plugin consumes that shape to
emit idiomatic bindings, explicitly to avoid the "manual wrappers... go out of sync from the
actual C API" problem that hand-written FFI has. It is explicitly framed as the generalization of
`cxx` (Rust↔C++ only) to a pluggable target language: "cxx, but if the target language were
pluggable."
Source: https://github.com/rust-diplomat/diplomat/blob/main/docs/design_doc.md

Important nuance: this is **not** a single shared hub *artifact* that multiple independent tools
link against (unlike, say, a canonical shared library). It's a per-project, per-build C shape that
diplomat-tool itself both emits and immediately consumes (via the HIR, not by re-parsing the
generated C text) to drive every language backend from one source of truth. So it's "one logical
ABI shape," generated fresh per project, not a persistent shared-hub crate/service.

**Object identity.** Opaque Rust types are marked `#[diplomat::opaque]` and boxed
(`Box<T>` treated as FFI-compatible with `*mut T`); the design doc flags this as still slightly
uncertain in its own words: "(We may have to assume `Box<T>` is layout-compatible with `*mut T`
for this.)"
Source: https://github.com/rust-diplomat/diplomat/blob/main/docs/design_doc.md

**Lifetimes / borrowing (the part Diplomat spends the most design ink on).** Borrowed references
(`&Foo`) can't be exposed directly to garbage-collected host languages, so Diplomat introduces
"lifetime edges" — internal references purely to keep the parent object alive from the GC's
perspective for as long as a borrowed child is reachable. The author's own assessment: "This is
pretty straightforward for this API, but gets complicated pretty quickly when you start having
multiple lifetimes, structs with lifetimes, or strings."
Source: https://manishearth.github.io/blog/2026/06/14/diplomat-multi-language-ffi-for-rust-libraries/

**Mutation.** UNVERIFIED beyond what the design doc implies (opaque `&mut self` methods are
listed as a supported bridge-module construct) — no fetched page in this session gives Diplomat's
mutation semantics the same explicit treatment UniFFI's docs give (i.e. no stated
Send+Sync-and-interior-mutability requirement was found for Diplomat; it may simply allow
`&mut self` directly since it's unidirectional and doesn't need to defend against arbitrary
foreign threads calling back asynchronously the way a callback-capable system like UniFFI does).

**Errors.** The design doc shows Rust `Result<T, E>` compiled to a C-compatible struct shape
(example: `struct PluralNewResult { ... }` standing in for `Result<PluralRules, PluralError>`),
which each language plugin then turns into its native error mechanism (the doc's own example:
"Java throws clauses").
Source: https://github.com/rust-diplomat/diplomat/blob/main/docs/design_doc.md

**Cost.** No performance numbers found in any fetched Diplomat page.

---

## 3. cbindgen (mozilla/cbindgen)

**What it is.** **Not** a bindings generator — a **C/C++ header generator only**. It scans a Rust
crate for `#[no_mangle] pub extern "C" fn`, `#[no_mangle] pub static`, and `pub const` items plus
`#[repr(C)]` types, and emits a matching `.h` (or `.hpp`, or Cython `.pxd` with `--lang cython`)
declaration file.
Source: https://github.com/mozilla/cbindgen/blob/main/docs.md

**What it explicitly does not do.** No marshalling/ownership/borrowing wrapper code is generated
— you still hand-write the `extern "C"` functions on the Rust side and hand-write (or hand-call)
the matching C/C++/Cython usage. The docs candidly admit gaps: "may randomly fail to support some
particular situation simply because no one has put in the effort to handle it yet," cannot
disambiguate same-named types across modules, and cannot handle anonymous tuples, wide pointers
(`&dyn Trait`, `&[T]`), or zero-sized function arguments.
Source: https://github.com/mozilla/cbindgen/blob/main/docs.md

**Relevance to the "canonical-C-ABI-hub" question.** cbindgen is the closest thing on this list to
literally *being* "generate a C header and let everyone consume it" — and it is instructive that
even Mozilla's own tool that does exactly that is deliberately scoped to headers only. It solves
zero of the N×M glue-writing cost by itself: every consuming language still needs its own
hand-written or separately-tool-generated marshalling layer on top of the header. In this
ecosystem it functions as a low-level *component* other tools (Diplomat's C backend, hand-rolled
FFI) build on, not as an N×M solution in its own right.

---

## 4. flapigen-rs (Dushistov/flapigen-rs, formerly rust_swig)

**What it is / IR.** No separate IDL file — bridge/foreign-class declarations are written
**inline as Rust macros** (`foreign_class!`, `foreign_enum!`, `foreign_callback!`,
`foreign_typemap!`) inside the crate's own `build.rs`-driven macro expansion; flapigen "expands
rust macroses and generates not rust code" as part of the Cargo build script mechanism.
Source: https://dushistov.github.io/flapigen-rs/about.html ,
https://github.com/Dushistov/flapigen-rs/blob/master/README.md

**Name history.** Formerly `rust_swig`, renamed to flapigen ("**f**oreign **l**anguage **api**
**gen**erator") specifically "to not confuse with swig" — i.e. it is a spiritually-SWIG-like tool
built specifically for Rust, not a Rust backend bolted onto SWIG itself.
Source: WebSearch snippet of https://github.com/Dushistov/flapigen-rs (top-of-README text) —
confidence medium (not independently re-fetched verbatim in a WebFetch call).

**Languages reached.** Currently implemented: **Java and C++**. The project states extensibility
is a goal ("you can write support for any language of your choice") but only those two are
shipped.
Source: https://dushistov.github.io/flapigen-rs/about.html

**Mechanism.** Goes "via C API, so generated Rust's code is wrapper around your code to provide
[a] C API" — i.e. like Diplomat, it manufactures a C-shaped layer as an intermediate step, then
generates JNI wrappers + Java source for the Java target, or C++ wrappers for the C++ target, from
that shape.
Source: https://dushistov.github.io/flapigen-rs/about.html

**Object identity / mutation / errors.** UNVERIFIED at any useful level of detail — the fetched
`about.html` describes only the mechanism above; it did not state whether Java objects hold raw
pointers, a handle-table, or something else, nor how `&mut self` or `Result` are surfaced. Not
re-fetched from deeper docs pages in this session due to time/budget.

**Cost.** No performance numbers found.

---

## 5. SWIG (Simplified Wrapper and Interface Generator)

**What it is.** Parses C/C++ header/interface declarations directly (no separate proc-macro or
Rust-specific step — this is the pre-Rust-era universal binder) and "generates the 'glue code'
required for the... target languages to call into the C/C++ code."
Source: https://www.swig.org/ (fetched)

**Languages reached.** 20+ targets across scripting and compiled languages: **Python, Perl, Tcl,
Ruby, PHP, Java, C#, D, Go, Lua, Octave, R, Guile/Scheme, Scilab, OCaml**, plus historically
ALLEGROCL, CHICKEN, CLISP/CFFI, Modula-3, MzScheme, Pike, and XML output.
Sources: https://www.swig.org/ and WebSearch snippet of the SWIG GitHub README/Wikipedia summary.

**Current version.** **SWIG 4.5.0, released August 6, 2026** — adds C++20 support, drops Python 2
(now targets Python 3.5+), plus assorted per-language enhancements.
Source: https://www.swig.org/ (fetched directly — confidence high for the version/date pairing,
since it came from the fetched front page rather than a search snippet).

**Architecture — the key answer to the "canonical hub" question.** SWIG generates **directly to
each target language's own native extension API** (CPython's C API, Perl's XS, etc.) — there is
**no shared intermediate C-ABI hub artifact** in between. Every language backend is its own
independent code generator reading the same parsed interface, which is architecturally exactly
the N+M shape (one parser/IR + M independent per-language backends) rather than routing
everything through one shared runtime ABI.
Source: https://www.swig.org/ (fetched; the "generates directly to each language's C API" framing
was synthesized by WebFetch from the page's own description of its mechanism).

**Object identity / mutation / errors.** UNVERIFIED in this session — not fetched at that level of
detail (SWIG's per-language typemap system is well known from prior knowledge to wrap C++ objects
as opaque pointers held by each language's native object wrapper, and to map C++ exceptions to
each target's native exception type via `%exception` typemaps, but this was not re-verified
against a primary SWIG doc page in this session, so treat as UNVERIFIED / not independently
confirmed here).

---

## 6. WebAssembly Component Model + WIT (wit-bindgen)

**What WIT is.** "WIT (WebAssembly Interface Types)... serves as an Intermediate Description
Language (IDL) for the WebAssembly Component Model... all imports into a WebAssembly binary and
all exports must be described with WIT." Interfaces are grouped into "worlds" (what a component
imports from its host and exports to it) and reusable named interfaces.
Source: https://github.com/bytecodealliance/wit-bindgen (fetched)

**Languages reached.**
- *Guest* (compiled to Wasm and targeted by wit-bindgen codegen): **Rust** (native
  `wasm32-wasip2` support since Rust 1.82), **C/C++** (via WASI SDK), **C#** (native-AOT), **Go**,
  **MoonBit**, and legacy/unmaintained **TinyGo**.
- *Host* (runtimes that execute components and call into them): **Rust** (`wasmtime` crate),
  **JavaScript** (`jco`), **Python** (`wasmtime-py`), **Ruby** (`wasmtime-rb`).
Source: https://github.com/bytecodealliance/wit-bindgen (fetched)

**Object identity — "resources."** WIT has a first-class `resource` construct for "entities
existing outside components that shouldn't be copied." Resources cross the boundary as **handles**,
either **owned** (destroyed when dropped) or **borrowed** ("a temporary loan of a resource from
the caller to the callee for the duration of the call"). A resource exposes behavior only through
methods/statics — i.e. it behaves like an opaque object reference, deliberately never a raw
pointer or shared-memory struct.
Source: https://component-model.bytecodealliance.org/design/wit.html (fetched)

**Mutation.** UNVERIFIED at the WIT-spec level — the fetched page describes resource methods
receiving an implicit `self`-like parameter but does not state a mutation policy explicitly (no
`Send+Sync`-style requirement was surfaced the way UniFFI's docs state one).

**Errors.** `result<T, E>` is a first-class WIT type: "may contain a value of type T *or* a value
of type E (but not both)," explicitly analogized to Rust's `Result`/Haskell's `Either`, with
sugared forms `result<u32>` (no error payload), `result<_, u32>` (no success payload), and bare
`result`.
Source: https://component-model.bytecodealliance.org/design/wit.html (fetched)

**Canonical ABI — this is the one project on the list that IS built around an explicit,
standardized, cross-vendor "hub" layer** — but it is a *wire/lifting-lowering discipline over
linear memory*, not a shared native C struct layout. Every crossing lifts values out of a
component's own linear memory into an abstract, language-neutral form, and lowers them back into
the destination's memory — there is no assumption that two components share pointer layouts or
even an address space.
Source: https://component-model.bytecodealliance.org/design/wit.html and search-derived summary
of https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md (the
CanonicalABI.md file itself was not directly WebFetched in this session — the description above is
built from the WIT design page plus the bytecodealliance.org article below; treat the precise
"linear memory + lift/lower" characterization as **medium confidence**, corroborated across two
sources but not read verbatim from CanonicalABI.md itself).

**Performance — concrete, dated numbers.** From the Bytecode Alliance's own June 8, 2026 article
"The Road to Component Model 1.0":
- Explicit stated goal: **"zero overhead on synchronous calls between components."**
- Current reality: **"the current implementation manages async task infrastructure across
  component boundaries via a host call, which adds roughly 3.5x overhead even on purely
  synchronous call paths."** A fix ("lazy" handles, opt-in in a 0.3.x minor release, full 1.0 after
  adoption) is planned post-WASI-P3 to let compilers "largely optimize away" the per-call task
  allocation.
- A cited real-world number in the *opposite* direction (bypassing the model's own JS glue, not
  the Canonical ABI itself): "a DOM mutation-heavy Wasm VDOM reconciliation loop can get close to
  a 2x speedup from direct Wasm-to-browser API calls, bypassing the JavaScript glue layer" (a
  Mozilla experiment cited in the same article).
Source: https://bytecodealliance.org/articles/the-road-to-component-model-1-0 (fetched directly —
confidence **high** for the 3.5x figure and the "zero overhead" goal quote, since both came
verbatim from the fetched primary source).

**Maturity note.** Per the earlier WebSearch summary (not independently re-fetched from a single
citable page, medium confidence): the Component Model is "production-ready for server-side and
edge workloads built on WASI 0.2, but is not yet ready as a browser target, and threading support
remains a real gap for compute-heavy workloads."

---

## 7. GraalVM / Truffle polyglot interop

**Mechanism — deliberately has no IR and no codegen step at all.** Unlike every other project on
this list, GraalVM's polyglot interop is **not** a build-time bindings generator. Languages
implemented on the Truffle framework interoperate **in-process, in the same memory space, through
a standardized runtime message protocol** ("a set of standardized messages that every language
implements and uses for foreign polyglot values"), letting "GraalVM support interoperability
between any combination of languages without requiring them to know of each other."
Source: https://www.graalvm.org/latest/reference-manual/polyglot-programming/ (fetched)

**Languages reached.** JavaScript/Node.js (GraalJS), Python (GraalPy), Ruby, R, Java, and
LLVM-bitcode-based native languages (C, C++, **Rust**) via the Sulong LLVM-bitcode interpreter, plus
WebAssembly via GraalWasm.
Source: https://www.graalvm.org/latest/reference-manual/polyglot-programming/ (fetched;
Truffle-language list) + prior WebSearch summary of the same page family.

**Object identity / mutation.** Because everything runs in one address space under one runtime,
there is **no marshalling boundary at all for object identity** — "values pass between languages
as shared object references within the same memory space — no serialization occurs," so arrays,
objects, and other structures keep identity and are directly mutable across the language boundary
exactly as if within one language.
Source: https://www.graalvm.org/latest/reference-manual/polyglot-programming/ (fetched, WebFetch
synthesis of the page's description of the interop protocol)

**Errors.** UNVERIFIED — not stated in the fetched excerpt of this page. Given the shared-runtime
design, cross-language exception propagation is architecturally plausible to be native (Truffle's
InteropLibrary is known from general Truffle documentation to define an exception-interop message
family) but this was not confirmed against a primary source in this session.

**Cost — the real tradeoff.** This is the "cheat" answer to N×M: because every language must be
re-implemented *on top of* Truffle/the JVM to get this interop for free, the cost is not paid at
the FFI boundary — it's paid once, up front, per language, as a full from-scratch language
implementation effort. That's a fundamentally different shape of cost than every other tool on
this list, which instead re-uses each language's *existing* native runtime/C-API and pays a
smaller, per-crossing marshalling cost. No specific benchmark numbers were found in the fetched
pages in this session; "near-native speed via the Graal JIT" is a general Truffle/GraalVM claim
from prior knowledge, not confirmed with a number in this session — UNVERIFIED as a number.

---

## 8. .NET P/Invoke source generation (`[LibraryImport]`)

Included as the N=1 (single host language, C#) case, useful as a performance contrast for
compile-time-generated marshalling vs. runtime-generated marshalling — directly relevant to
compylr's own PyO3-vs-nanobind-vs-node-addon-api tradeoffs, none of which are pure compile-time
source generators the way `[LibraryImport]` is.

**Mechanism, exact and dated.** From Microsoft's own docs (page dated `ms.date: 2022-07-25`,
content confirmed current via `updated_at: 2025-12-04`):
> "**.NET 7** introduces a source generator for P/Invokes that recognizes the
> `LibraryImportAttribute`... When it's not using source generation, the built-in interop system
> in the .NET runtime generates an IL stub — a stream of IL instructions that is JIT-ed — **at
> runtime**... Since this IL stub is generated at runtime, it isn't available for ahead-of-time
> (AOT) compiler or IL trimming scenarios."
> "The P/Invoke source generator... looks for `LibraryImportAttribute` on a `static` and
> `partial` method to trigger **compile-time** source generation of marshalling code, removing the
> need for the generation of an IL stub at runtime and allowing the P/Invoke to be **inlined**."
Source: https://learn.microsoft.com/en-us/dotnet/standard/native-interop/pinvoke-source-generation
(fetched directly, full frontmatter captured — high confidence)

**Why it matters for compylr.** This is a direct, dated, primary-sourced example of the exact
axis compylr's own design keeps re-deriving: build-time-generated glue beats runtime-generated
glue for (a) AOT/trimming compatibility, (b) inlining, and (c) debuggability ("since the
marshalling is now generated source code, you can actually look at and step through the logic in
a debugger"). `[DllImport]`'s IL-stub approach is architecturally the same *kind* of cost compylr
already avoids by generating Rust/C++ source ahead of time rather than marshalling reflectively at
call time.

**Benchmark reality check.** A search-only source (not independently re-fetched; medium
confidence) states plainly that "LibraryImport is not always faster than DllImport" in practice —
i.e. the compile-time-generation win is real for AOT/inlining/debugging but is not a universal,
unconditional speedup; several 2026-dated blog posts exist specifically benchmarking the two
head-to-head (dotnettips.com, "P/Invoke Showdown," Aug 2 2026), which were found but not
independently WebFetched in this session — flagged here as a pointer for follow-up, not as a
verified number.

**Object identity / mutation / errors.** Out of scope for what was fetched — P/Invoke's marshalling
attributes (`MarshalAs`, `StringMarshalling`, `UnmanagedCallConv`) were confirmed from the primary
page, but SEH/HRESULT/GetLastError-style error propagation specifics were not covered in the
fetched excerpt. UNVERIFIED for this session.

---

## Answering the direct question: is a canonical-C-ABI hub how successful projects solve N×M?

Looking across all seven, the pattern is **no, not as a persistent shared hub artifact** — every
project that reaches genuinely many languages does it as **N+M** (one shared IR/description +
M independent per-language backends), and the "C layer" that shows up in several of them is a
*means* to keep those M backends from drifting from each other, not an end-user-facing shared
binary/crate other tools link against:

- **UniFFI**: N+M via one object model (UDL or proc-macros) → M independent language backends.
  Low-level FFI functions are plain `extern "C"`, generated fresh per project; there's no shared
  hub crate.
- **Diplomat**: N+M via one AST→HIR → M language plugins. Explicitly *does* route every backend
  through "an underlying C layer" as a design discipline, but that C shape is generated (and
  consumed, via the HIR, not by re-parsing text) fresh per project by diplomat-tool itself — it is
  not a separate, independently-versioned hub artifact.
- **cbindgen**: literally *is* "just emit a C header" — and precisely because that's all it does,
  it does **not** solve N×M by itself. It's evidence *against* the hub-crate theory: even Mozilla's
  own from-scratch C-header tool stopped at the header and left every consumer to hand-write their
  own glue on top, which is exactly the N×M cost compylr is trying to avoid.
- **flapigen**: also manufactures a C-shaped layer internally, but only ships two language
  backends (Java, C++) — smaller-scale evidence of the same "C shape as internal discipline, not
  external hub" pattern.
- **SWIG**: explicitly the *opposite* strategy — no C-ABI hub at all. Generates directly to each
  language's own native C API (CPython C API, Perl XS, etc.) from one parsed interface. Also N+M,
  just without even the internal C-shape discipline.
- **WIT/Component Model**: the one case that genuinely *is* a standardized, cross-vendor hub layer
  (the Canonical ABI) — but it is a **lifting/lowering wire discipline over linear memory**,
  deliberately not a shared native struct layout, precisely because components are meant to be
  mutually distrusting and not share an address space. And even this design's own stewards state a
  currently-unmet "zero overhead" goal, with a **measured 3.5x overhead on synchronous calls** as
  of June 2026 that they are actively re-architecting to remove.
- **GraalVM/Truffle**: rejects the ABI-hub idea entirely in the other direction — no marshalling
  boundary at all, because every language is reimplemented on one shared runtime. The "hub" here is
  the JVM/Truffle *runtime*, not an ABI.
- **.NET LibraryImport**: not an N-language answer at all (N=1: C# only) — but it's the clearest
  dated evidence that **compile-time-generated glue beats runtime-generated glue**, independent of
  how many languages are involved.

**Bottom line for compylr's own C-ABI-hub premise (`compylr-bridge-cpp-abi`):** none of the
widely-used, many-language tools researched here actually ship a standing, independently-linked
C-ABI hub *crate* that other tools build on top of. The two projects that most resemble that idea
(Diplomat, flapigen) both generate the C shape **fresh, per project, as an internal implementation
detail consumed by their own codegen** — never as a separately-versioned artifact a third party
links against — and the *only* project with a real standardized cross-vendor hub (the WASM
Canonical ABI) pays a measured, currently-unresolved overhead tax for exactly the boundary-safety
properties (no shared address space, mutual distrust) that a Python-process/Node-process/C++
extension situation does not need. This is independent, converging evidence for the user's already
stated conclusion: drop `compylr-bridge-cpp-abi` and go pairwise (nanobind for Python↔C++,
node-addon-api for Node↔C++, PyO3 stays for Python↔Rust) — every real N+M binder either doesn't use
a hub artifact at all (UniFFI, SWIG, GraalVM) or uses one only as a private, per-build
implementation detail of its own single toolchain (Diplomat, flapigen), never as a thing a second,
independent tool would link against the way `compylr-bridge-cpp-abi` was planned to be shared
across Python and Node loaders.

---

## Gaps / things not verified in this session

- Exact uniffi-rs and diplomat crate version numbers — crates.io renders via JS and could not be
  read through WebFetch in this session (returned only the page title, no body content). Not
  independently found elsewhere before the WebSearch budget (200/200) was exhausted.
- UniFFI's exact runtime mechanism for turning a Rust `Err` into a native exception (which page in
  the manual states it, and the precise `RustCallStatus` shape) — plausible from general PyO3/UniFFI
  knowledge but not confirmed against a primary source fetched in this session.
- Diplomat's and flapigen's mutation semantics at the same level of detail UniFFI's docs give
  (Send+Sync / interior-mutability requirements) — not found in the pages fetched.
- SWIG's object-identity/mutation/error-propagation mechanics at a primary-source level (well known
  generally — opaque pointers wrapped per-language, `%exception` typemaps for errors — but not
  re-verified against a SWIG doc page in this session).
- The WASM `CanonicalABI.md` design document itself was not directly fetched; the "linear memory,
  no shared struct layout" characterization rests on the WIT design page plus the Bytecode Alliance
  article, corroborated but not read verbatim from that specific file.
- GraalVM/Truffle's cross-language exception-propagation mechanism, and any dated overhead numbers
  for polyglot calls (only a general "near-native via JIT" claim from prior knowledge, not
  confirmed with a number this session).
- The 2026-dated LibraryImport-vs-DllImport head-to-head benchmark posts (dotnettips.com,
  dotnetramblings.com) were located but not WebFetched — a follow-up session with search budget
  remaining could pull concrete numbers from those if compylr's own bridge work wants a
  compile-time-vs-runtime-marshalling number to cite.
