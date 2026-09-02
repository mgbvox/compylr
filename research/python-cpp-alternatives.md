# Research: Python↔C++ binding approaches (as of 2026-09-01)

Scope: pybind11, nanobind, cppyy, SWIG, Cython, Boost.Python, raw CPython C API. For each:
maintenance status, build-time cost, runtime call overhead, ABI stability, and fitness for
**code-generated** (compiler-emitted) bindings rather than hand-written ones — which is
compylr's actual use case (a backend emits binding *source text*, nothing hand-authors it).

All version/date facts below were pulled live via the GitHub API (`mcp__github__list_releases`,
`list_tags`, `get_commit`) or WebFetch against primary docs, not from training-data recollection —
each has an inline source and a confidence mark. Search-snippet-only claims are marked accordingly.

---

## 1. Maintenance status (verified via GitHub API, live as of 2026-09-01)

| Project | Repo | Latest release | Date | Status |
|---|---|---|---|---|
| pybind11 | pybind/pybind11 | v3.1.0 | 2026-08-06 | Actively maintained, frequent releases |
| Cython | cython/cython | 3.3.0 | 2026-08-22 | Actively maintained, frequent releases |
| SWIG | swig/swig | v4.5.0 | 2026-08-05 | Actively maintained |
| nanobind | wjakob/nanobind | v3.0.1 | 2026-08-27 | Actively maintained, single maintainer |
| Boost.Python | boostorg/python | boost-1.92.0 | latest commit 2026-08-24 | Maintained as part of monolithic Boost release cycle; low feature-commit volume |
| cppyy (original) | wlav/cppyy | cppyy-3.5.0 tag | 2024-12-17 | **Repo frozen** as of 2026 — see below |
| cppyy-backend (research fork) | compiler-research/cppyy-backend | — | archived Aug 2026 | **Archived**; superseded by CppJIT |

Evidence:
- pybind11 releases via `mcp__github__list_releases(pybind, pybind11)`: v3.1.0 published
  `2026-08-06T23:30:36Z`; v3.0.4 `2026-04-19`; v3.0.3 `2026-03-31`; v3.0.2 `2026-02-17`; v3.0.1
  `2025-08-22`; v3.0.0 `2025-07-10`.
- Cython releases via `mcp__github__list_releases(cython, cython)`: 3.3.0 published
  `2026-08-22T05:14:59Z`, preceded by 3.2.9 (`2026-07-24`), 3.2.8 (`2026-06-30`) — roughly monthly
  cadence.
- SWIG has no GitHub "releases" but tags: v4.5.0 commit `d598176` authored by wsfulton
  ("swig-4.5.0 release notes and date") dated `2026-08-05T21:56:41Z` (`get_commit`).
- nanobind tag `v3.0.1` → commit `db4827f` "v3.0.1 release" by Wenzel Jakob, dated
  `2026-08-27T22:29:44Z` (`get_commit`). Also confirmed via
  [pypi.org/project/nanobind](https://pypi.org/project/nanobind/): "Latest Published Version: 3.0.1
  (Released August 27, 2026)". Single maintainer (Wenzel Jakob, EPFL) — this is a bus-factor risk
  worth naming even though the cadence is currently fast. confidence: high
- Boost.Python: `list_commits(boostorg, python)` top commit `9b2f967` "avoid undefined behavior...
  calling front() on a possibly empty std::vector" dated `2026-08-24T12:53:27Z`. Commit volume is
  low (small bugfixes, occasional feature PRs like `vector_indexing_suite` reverse/remove/count from
  April 2026) — it ships as part of the Boost superproject's release train
  (`boost-1.92.0` tag) rather than having its own independent release cadence. confidence: high
- **cppyy is dead as a standalone project.** `wjakob/nanobind`-adjacent search and direct WebFetch
  of `github.com/wlav/cppyy` show: *"This repo is frozen. For the latest development, go to:
  https://github.com/compiler-research/"* — 514 stars, 57 forks, 124 open issues, frozen.
  Development moved to `compiler-research/cppyy-backend`, which is **itself now archived** (WebFetch
  of that repo's README, 2026-09-01): *"This repository was archived in August 2026. Development
  continues in CppJIT, a new package and the successor project of cppyy."* The chain is:
  `wlav/cppyy` (frozen) → `compiler-research/cppyy-backend` (archived Aug 2026) → `CppJIT`
  (current). So "cppyy" as a name/brand has been retired twice in 2026 alone. confidence: high
  (both statements read directly off the repos' own READMEs).
- Last commit to `wlav/cppyy` before the freeze was `2d28e09`, "send folks to the compiler-research
  project", dated `2026-06-04T18:18:48Z` (`list_commits`). confidence: high

**Implication for compylr:** cppyy is not a safe pick to depend on right now — the project a user
would find by searching "cppyy" no longer receives development under that name, and its true
successor (CppJIT) is a new, differently-scoped tool (see §4) that compylr has not evaluated.

---

## 2. Runtime call overhead and build-time cost (nanobind vs pybind11 vs Cython vs Boost.Python vs cppyy)

Source: nanobind's own benchmark page,
[nanobind.readthedocs.io/en/latest/benchmark.html](https://nanobind.readthedocs.io/en/latest/benchmark.html)
(WebFetch, 2026-09-01). Methodology stated on the page: a microbenchmark of **720 trivial
functions/methods performing only additions**, built and imported, specifically designed to isolate
binding *overhead* from computational work — i.e. it measures exactly the axis that matters for
compiler-emitted glue around already-fast Rust/C++ logic.

| Comparison | Compile time | Binary size (size-optimized) | Runtime call overhead |
|---|---|---|---|
| nanobind vs pybind11 | **~2.7–4.4× faster** | **~3–5× smaller** | **~3× faster** (simple functions), **~10× faster** (classes passed around) |
| nanobind vs Boost.Python | — | **~11× smaller** | — |
| nanobind vs Cython | **1.6–4.4× faster** | **3–12× smaller** | roughly comparable — Cython wins one sub-benchmark, nanobind the other |
| nanobind vs cppyy | — | — | **~1.6–2.1× faster** |

confidence: high (numbers read directly off the primary source's own stated benchmark table/prose).

Per-instance object overhead (from nanobind's "why" rationale page,
[nanobind.readthedocs.io/en/latest/why.html](https://nanobind.readthedocs.io/en/latest/why.html),
WebFetch): pybind11 wraps a bound C++ instance in **56 bytes** of Python-object overhead; nanobind
uses **24 bytes** — a 2.3× reduction. confidence: high

Raw CPython C API vs pybind11 (independent third-party benchmark, not from either project):
Ash Vardanian's post
["Our CPython bindings got 5x faster without PyBind11"](https://ashvardanian.com/posts/pybind11-cpython-tutorial/)
(WebFetch, 2026-09-01) measured a `str.find()`-equivalent call:
- native Python `str.find`: **~1 µs**
- pybind11-wrapped C++ implementation: **~15 µs**
- hand-written raw CPython C API implementation: **~3 µs** (5× faster than pybind11, still 3× slower
  than native Python)

The author is explicit about the cost of that win: raw C API meant **"moving from modern C++17 to
more basic C99"** — over 30 lines of manual argument-parsing boilerplate for one function that took
3 lines to actually compute, with `PyArg_ParseTupleAndKeywords` alone requiring "three separate
for-loops." confidence: high for the numbers (primary author's own measurement); the takeaway
("raw C API is fastest but least automatable") is the load-bearing point for compylr, see §5.

SWIG: no comparable first-party quantitative benchmark was found. Community consensus, e.g. the
LSST/Rubin Observatory community forum thread
[Using pybind11 instead of Swig to wrap C++ code?](https://community.lsst.org/t/using-pybind11-instead-of-swig-to-wrap-c-code/1096)
and TensorFlow's own migration RFC
([tensorflow/community RFC 20190208-pybind11.md](https://github.com/tensorflow/community/blob/master/rfcs/20190208-pybind11.md)):
*"SWIG auto-generated code is not optimal for performance"* and *"it has shortcomings with respect
to supporting modern C++ standards, with little development in this direction and it [is] unlikely
SWIG will ever catch up."* No hard numbers found; treat as qualitative. confidence: medium (repeated
independent qualitative claims, no first-party benchmark located).

---

## 3. ABI stability

- **nanobind**: can target CPython's Stable ABI (`Py_LIMITED_API`) starting at **Python 3.12** in
  its default "linked" mode, or **Python 3.10+** in an optional "split mode" that moves
  version-specific internals into a separate backend package
  ([why.html](https://nanobind.readthedocs.io/en/latest/why.html),
  [GitHub search result on nanobind Discussion #500](https://github.com/wjakob/nanobind/discussions/500)).
  A quoted JAX-team testimonial on the nanobind README: *"nanobind can target the Python Stable ABI
  starting with Python 3.12. This means that we will not need to ship per-Python version CUDA
  plugins starting with Python 3.12."* confidence: high
- **pybind11**: **does not** currently support the stable ABI
  ([search result summarizing GitHub Discussion #4474](https://github.com/pybind/pybind11/discussions/4474)
  — "nanobind supports the stable abi but pybind11 currently doesn't"). Each pybind11 extension is
  built against one specific CPython minor version's full API, matching what compylr already does
  today via PyO3 (which also targets per-version ABI unless `abi3` is explicitly configured).
  confidence: medium (from a search-result summary of a GitHub discussion thread, not the primary
  page text itself — worth a direct re-check before relying on it for a design doc).
- **pybind11 internal ABI versioning**: pybind11 tags its own internal binary layout with an "ABI
  version" integer that increments on breaking internal-layout changes — v3.0.0 is internal ABI
  version 22 (per WebFetch summary of the pybind11 changelog). This is a *pybind11-internal*
  cross-extension compatibility mechanism, unrelated to CPython's own Stable ABI. confidence: medium
  (summarized by the fetch tool from primary content, not independently cross-checked against the
  changelog's raw text).
- **pybind11 C++ standard**: v3.0.0's own changelog text (WebFetch,
  [pybind11 changelog](https://pybind11.readthedocs.io/en/stable/changelog.html)) references C++17
  throughout and drops legacy toolchains (Python 3.7, PyPy 3.8/3.9, CMake <3.15), but the fetched
  excerpt did not contain an unambiguous single "minimum standard is now C++17" sentence — mark this
  UNVERIFIED at the precise-wording level even though directionally it reads as a C++17 baseline for
  the 3.x line (older pybind11 2.x explicitly supported C++11). confidence: low on the exact wording,
  medium on the directional claim.
- **Boost.Python**: no stable-ABI story; ties bindings to a specific Boost + Python build, same as
  classic pybind11.
- **cppyy / CppJIT**: architecturally different — bindings are generated **at runtime** by a
  Clang-based interpreter (Cling, or now Clang-REPL via CppInterOp), so there is no separately
  compiled `.so` per Python version to version at all; the question doesn't apply the same way. See
  §4.
- **Raw CPython C API**: no stable-ABI concern unless the hand-written code explicitly restricts
  itself to `Py_LIMITED_API` symbols, which is possible but adds constraints identical in kind to
  what nanobind's split mode already automates.

---

## 4. cppyy / CppJIT: architecturally different, and not currently a fit

cppyy is not a "generate C++ source, compile it, load it" tool like the others — it embeds a
Clang-based C++ interpreter (originally Cling, now Clang-REPL/CppInterOp under the successor
CppJIT project) and builds bindings **dynamically at Python import/run time** by reflecting over
already-compiled or JIT-compiled C++, rather than emitting static PyO3/pybind11/nanobind-style glue
source ahead of time. From cppyy's own docs
([cppyy.readthedocs.io](https://cppyy.readthedocs.io/en/latest/), WebFetch): it is described as *"an
automatic, run-time, Python-C++ bindings generator"* built on Cling, explicitly built to *"match
Python's dynamism, interactivity, and run-time behavior."* confidence: high on what the docs claim
about the design; UNVERIFIED whether CppJIT (the actual current successor, archived Aug 2026 handoff
per §1) preserves the same properties, since this research did not fetch CppJIT's own docs.

This is a poor structural fit for compylr specifically: compylr's whole pipeline is "IR → backend
emits target source as pure text → that text is compiled ahead of time," and the rebuild/fingerprint
model (`Unit::fingerprint()`, `.compylr/` cached Rust) depends on emission being a pure, cacheable,
ahead-of-time step. A Clang-interpreter-driven, run-time reflection system doesn't produce a static
source artifact to fingerprint or cache the same way, and pulls in an LLVM/Clang runtime dependency
none of the other options need. Given also that it's mid-rename/mid-handoff as of this writing (§1),
cppyy/CppJIT should be **ruled out for now**, not because of a technical defect proven in this
research, but because its current maintenance state and architecture are both moving targets. This
conclusion is this researcher's synthesis, not a quoted source claim — flagged accordingly.

---

## 5. Fitness for CODE-GENERATED (compiler-emitted) bindings specifically

This is the axis that actually matters for compylr's `compylr-bridge-*` crates, which emit binding
*source text* programmatically rather than having a human write `PYBIND11_MODULE` blocks by hand.

**Precedent: this exact pattern already exists and already prefers nanobind/pybind11 as the
*target* of code generation, not SWIG.**
[litgen](https://github.com/pthom/litgen) (Pascal Thomet) is a real, published tool whose whole job
is: parse C++ headers, and **emit either pybind11 or nanobind source code** as its output — i.e. the
same "generate binding glue as text, then compile it" shape compylr uses for Rust/PyO3 today. Per
its own docs ([pthom.github.io/litgen](https://pthom.github.io/litgen/), WebSearch summary):
litgen's *"C++ API to be exposed to Python must be C++14 compatible"* and it *"generates Python
bindings for C++ libraries using pybind11 or nanobind."* confidence: medium (from a search-result
summary of the project's own site, not a direct WebFetch of the page body — recommend a direct fetch
before citing verbatim in the design doc).

Why this matters structurally, reasoning from what was verified above (this paragraph is analysis,
not a quoted claim):
- **pybind11 and nanobind are the two tools designed around "author declares bindings via a small
  C++ embedded DSL"** (`m.def(...)`, `nb::class_<T>(...)`) — a shape a code generator can produce
  as a template-filled string trivially, the same way compylr's Rust backend already emits
  `#[pyfunction]` PyO3 source as text. nanobind is a near-drop-in syntactic subset of pybind11
  ("porting guide" exists;
  [nanobind why.html](https://nanobind.readthedocs.io/en/latest/why.html) states pybind11 "must
  deal with all of C++ to bind legacy codebases, while nanobind targets a smaller C++ subset" and
  explicitly rejects "fringe case" PRs pybind11 would accept) — which matches compylr's own
  constrained-subset philosophy (CLAUDE.md: "a strict, fully annotated source subset"). Given
  compylr only ever emits a bounded, self-authored C++ subset (never arbitrary legacy C++), nanobind's
  narrower-but-faster-and-smaller design is the better fit *in principle*, not pybind11's
  broader-but-slower one built to cover hand-written edge cases compylr's generator will never
  produce. confidence: medium — this is a fit judgment built on verified facts about both projects'
  stated design goals, not itself a sourced claim.
- **SWIG is architecturally backwards for compylr's direction of code generation.** SWIG's model is
  "parse existing C++ **headers** written by someone else, generate a wrapper `.cxx` + Python
  module automatically from that reflection." compylr's backend does the opposite: it already has
  the IR and is the one deciding what C++ to emit in the first place, so there is no pre-existing
  header for SWIG to introspect — the generator would have to hand-emit `.i` interface files anyway,
  which is strictly more indirection than emitting pybind11/nanobind source directly. Combined with
  the sourced claim above that SWIG's own community considers it behind on modern C++ standard
  support and non-optimal-performance auto-generated code, SWIG does not look like a good fit.
  confidence: medium (architectural reasoning verified against SWIG's known design; the "behind on
  modern C++" and "non-optimal performance" parts are sourced quotes, see §2).
- **Cython is a poor fit for compiler-emitted glue because it is a full source language compylr
  would have to generate valid `.pyx`/Cython-dialect syntax for**, not a small binding-declaration
  DSL layered on ordinary C++ — it is closer to "generate a second whole language's source" than
  "generate glue calls around already-emitted C++." It also targets wrapping/optimizing arbitrary
  Python-adjacent code more than bridging a specific IR-native module boundary. No primary source
  was fetched making this comparison explicitly (the intended fetch of Stefan Behnel's
  Cython/pybind11/cffi comparison post at blog.behnel.de failed with a TLS handshake error and was
  not retried); this paragraph is this researcher's own architectural judgment, marked
  **UNVERIFIED / not sourced** and should be independently checked before being treated as settled
  in a design doc.
- **Boost.Python is legacy relative to both pybind11 and nanobind** — it predates modern C++11-and-up
  binding libraries, requires linking Boost itself (a large, heavyweight dependency compylr would
  otherwise avoid entirely), and nanobind's own benchmark page states an ~11× binary-size advantage
  over it (§2, sourced). No evidence found that anyone builds a code generator that emits
  Boost.Python source in 2026; pybind11 was explicitly created (2015) to replace Boost.Python for
  exactly this kind of lighter-weight use. Not a fit for a new project in 2026. confidence: high on
  the sourced benchmark number, medium on "no one code-generates Boost.Python" (absence-of-evidence
  claim, not a positive source).
- **Raw CPython C API is the fastest and least automatable option** — per §2's sourced 5× number,
  it beats pybind11 on call latency, but at the cost (Vardanian's own words) of dropping to
  hand-rolled C99-style argument parsing with triple-nested loops per function. That cost is exactly
  what a *code generator* is well-suited to absorb once, by emitting a small number of stereotyped
  parsing templates rather than a human re-deriving them per function — so raw C API is not
  automatically ruled out for a compiler the way it would be for a human hand-writing bindings. But
  no existing precedent tool that code-generates raw CPython C API bindings (analogous to litgen
  for pybind11/nanobind) was found in this research; this remains a build-it-yourself option with no
  demonstrated third-party track record, versus nanobind/pybind11 which do have that track record
  (litgen, robotpy-build, and the ecosystem's own hand-authored-but-templatable `PYBIND11_MODULE`
  pattern). confidence: medium — the tradeoff is reasoned from sourced facts, but "no precedent
  tool" is an absence claim from search results that did not surface one, not a proof none exists.

**Bottom line for compylr, synthesized from the above (not a single quoted source):** among the
seven, **nanobind** is the strongest candidate for a compiler-emitted C++↔Python bridge — it is
the most actively maintained by release cadence (v3.0.1, 2026-08-27), has the lowest runtime
overhead and smallest binaries of any wrapper-generator option (§2, sourced from its own benchmark
page), explicitly targets exactly the constrained-modern-C++-subset shape compylr's backend would
emit, already has a real precedent of being used as a code-generation *target* (litgen), and
supports the CPython Stable ABI (§3) which would let a compiled C++ extension avoid one axis of
per-Python-version rebuild compylr's Rust/PyO3 path doesn't currently get without `abi3`. **pybind11**
is the credible fallback if nanobind's narrower feature surface turns out to reject something
compylr's C++ backend needs to emit — it's more actively maintained by commit/release volume, has a
much larger existing ecosystem, but costs more at runtime and in binary size and lacks stable-ABI
support. SWIG, Cython, and Boost.Python are all weaker fits for the reasons above. cppyy/CppJIT
should be revisited later, not now, given its live mid-2026 rename/architecture handoff. Raw CPython
C API is a legitimate longer-term option specifically *because* compylr is a compiler (which can
absorb the boilerplate cost a human can't) but has no demonstrated precedent and would be
substantially more implementation work up front.

This synthesis is offered for the PR #36 discussion but was **not** itself validated against
compylr's actual C++ backend plan or code — that cross-check is out of scope for this research task.

---

## Open questions / things this research did NOT verify

- The precise C++ standard pybind11 v3.0.0 now requires as its floor (directionally C++17, exact
  wording unconfirmed — see §3).
- Whether pybind11's lack of Stable ABI support (§3) is still current as of v3.1.0 specifically (the
  claim was sourced from a GitHub Discussion summary about an earlier version line, not re-checked
  against the 3.1.0 changelog).
- litgen's exact current feature completeness and whether it's actively maintained (only searched,
  not fetched directly — TLS/fetch issues prevented deeper verification in this pass).
- Cython's own position on being used as a code-generation target (the Behnel blog post that would
  have covered this failed to fetch — SSL handshake failure — and was not retried).
- CppJIT's own documentation and design goals were not fetched at all; everything about it here is
  inferred from the cppyy-backend archive notice.
- No first-party SWIG benchmark numbers were found; the "SWIG generates suboptimal code" claim rests
  on community/migration-RFC commentary, not a controlled benchmark.
