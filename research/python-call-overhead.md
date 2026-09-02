# Research: Python native-call overhead — ctypes / cffi / PyO3 / pybind11 / nanobind / hand-written C extension

Assignment: get hard, sourced numbers on per-call overhead crossing from Python into native code,
across the mechanisms compylr's bridge crates could plausibly use, to inform whether a
C-ABI-everywhere bridge design is viable. This is written to be read alongside the C-ABI collapse
finding (Node has no core FFI; nanobind/node-addon-api are now the plan) — the question here is
purely: **how much does crossing the Python boundary cost, per mechanism, and does it change
whether pairwise (PyO3-for-Rust, nanobind-for-C++) bridges are the right call.**

Everything below is either a quote from a primary source with its URL, or a number pulled directly
from a primary source's table. Where I could not find a number, I say so rather than estimate one.

---

## 1. The decisive primary source: Basso do Amaral, Ferreira & Goldman (2025), arXiv:2507.00264

**"Rust vs. C for Python Libraries: Evaluating Rust-Compatible Bindings Toolchains"** — University of
São Paulo. PDF fetched and read directly: https://arxiv.org/pdf/2507.00264 (abstract page:
https://arxiv.org/abs/2507.00264). This is the single most relevant source for this question — it
is the only one found that benchmarks **ctypes vs cffi vs PyO3 head-to-head on the same Rust code**,
with a stated methodology, hardware, and reproducible repo
(https://github.com/isinyaaa/python-ffi).

### Methodology (quoted/paraphrased from the paper)

- Hardware/software: **Linux 6.14.19, Ryzen 7 5800X, Rust 1.87.0, CPython 3.12.8 (single-threaded
  build), NumPy 2.2.0, cffi 1.17.1, maturin 1.8.6.**
- Workload: `mean()` and population `stddev()` over an array of `f64`, implemented once in Rust and
  exposed three ways.
- Two binding *shapes* per method, matching the exact axis this research task asked about
  (marshalling cost of a collection argument):
  - **M1 "in-situ conversion"** — the array is converted from Python to native on *every call*
    (this is the shape compylr's own bridge uses: a `list[T]` parameter converts every call).
  - **M2 "specialized constructor"** — an opaque native object is constructed once and reused, so
    marshalling happens once, not per call.
- Timing: `time.perf_counter_ns()` around the call, `gc.disable()`'d, 10 runs × 3 random samples,
  executed under BenchExec/runexec for isolation. Code quoted directly from the paper (Listing 13):

  ```python
  gc.disable()
  timer = time.perf_counter_ns
  def benchmark(fp, expected, *args, tolerance=0.01):
      start = timer()
      actual = fp(*args)
      end = timer()
      assert not math.isnan(actual) and abs(actual - expected) < tolerance
      return end - start
  ```

### Headline numbers

**Table IV — serial runs, M1 (convert-every-call), total time across the whole run (ms, mean ± stddev of the mean-workload):**

| Method | mean (ms) |
|---|---:|
| ctypes | 1.978e+05 ± 1.399e+03 |
| cffi (setuptools) | 7.347e+03 ± 216.20 |
| cffi (Maturin) | 7.262e+03 ± 60.99 |
| PyO3 | 6.423e+03 ± 34.70 |
| NumPy | 2.561e+04 ± 639.30 |

**Table V — serial runs, M2 (pre-converted, marshal once):**

| Method | mean (ms) |
|---|---:|
| ctypes | 1.369e+03 ± 43.84 |
| cffi (setuptools) | 633.1 ± 5.638 |
| cffi (Maturin) | 638.2 ± 4.451 |
| PyO3 | 634.7 ± 3.036 |
| NumPy | 262.5 ± 14.35 |

**Reading these two tables together is the actual answer to "how much does argument marshalling
cost":** for the *same* PyO3/cffi bindings, forcing per-call conversion of the array (M1) instead of
converting once (M2) costs **~10x** (PyO3: 634.7ms → 6,423ms; cffi/Maturin: 638.2ms → 7,262ms).
ctypes is far worse in both regimes, but catastrophically so under M1: **~145x slower than PyO3 at
M1** (1.978e5 ms vs 6.423e3 ms), because — per the paper — "ctypes has shown the most lacking
alternative, requiring manual API redefinitions and expensive type constructions due to `libffi`
outweighing any benefits" (§VII, Analysis). At M2 it closes to "only" **~2.2x** slower than PyO3
(1,369ms vs 634.7ms), i.e. the ctypes tax is overwhelmingly a per-call marshalling tax, exactly the
question this task was asked to answer.

### Per-call overhead regression (Table VIII, the paper's headline "cost per call" number)

They ran a second experiment: fix the total element count, vary the chunk size (hence the number of
calls), and linear-regress total time against call count to separate "cost that scales with element
count" from "cost that scales with call count." Quoted directly:

| Method | mean: per-call (ms) | mean: base (ms) | stddev: per-call (ms) | stddev: base (ms) |
|---|---:|---:|---:|---:|
| PyO3 | 0.1408 ± 0.0015 | 472.7 ± 0.32 | 0.1017 ± 0.0024 | 1,095 ± 0.55 |
| NumPy | 3.562 ± 0.082 | 14.36 ± 9.94 | 8.878 ± 0.060 | 560.9 ± 8.44 |

Their own reading: "the interpreter overhead caused by [NumPy's Python-level dispatch] contrasts
with the higher base overhead, but lower per-call overhead in the custom implementations" (§VII).
In other words PyO3's **fixed per-call dispatch cost is ~0.14ms (140μs)** for this array-argument
signature (this bundles PyO3's function-call dispatch *and* whatever share of Vec<f64> extraction
isn't captured by the "base"/element-count term) — small next to NumPy's ~3.6ms/call, and the
"base" term is where the O(n) element-conversion cost actually lives (it does not vary with call
count by construction of the regression).

### Their stated conclusion (§X, quoted)

> "PyO3 offers higher-level tooling to bindings developers, and allows for Python-native interfaces
> with minimal per-call overhead. To optimize their bindings, developers should always focus on
> separating large type conversions and delineating memory boundaries."

And on ctypes specifically (§X / Table IX): **"Poor"** performance rating, driven entirely by
`libffi`'s dynamic-symbol-resolution path plus manual, Python-side type (re)construction on every
call.

**Confidence: high** — directly fetched, read the full PDF, methodology and numbers quoted verbatim
from the tables.

### One caveat on external validity, quoted from the paper itself (§IX)

> "External validity. This paper purposefully limited its scope to comparing only easily
> implementable mathematical functions... in favor of a complete optimization of specific
> algorithms."

So this is `f64` arrays specifically — not strings, not dicts, not Python objects. Treat the ratio
(cffi/PyO3 comparable; ctypes far worse under per-call conversion) as the transferable finding, not
the absolute millisecond figures for other data shapes.

---

## 2. PyO3's own stated per-call dispatch overhead (no argument marshalling)

**Source:** https://github.com/PyO3/pyo3/issues/3827 (fetched directly). This isolates *just* the
call-dispatch mechanism — no collection to marshal — by comparing PyO3's `#[pyfunction]` macro
against a hand-rolled implementation using `_PyCFunctionFastWithKeywords` (the CPython fast-call
convention PyO3 doesn't use by default).

- **PyO3 function (`slow_len`):** ~33.5 ns/call
- Hand-written "baremetal" `_PyCFunctionFastWithKeywords` implementation (`fast_len`): ~12.6 ns/call
- Gap: **~20 ns of PyO3-macro overhead per call**, on top of a floor that itself isn't free.
- The issue notes PyO3 still doesn't use the fastest calling convention available
  (`_PyCFunctionFast`), and was **closed as "not planned"** — i.e. this is an accepted, not a
  transient, cost of the ergonomic macro layer.

**Confidence: high** for the ~33.5ns/~12.6ns pair (quoted from the issue thread); **medium** for
generalizing "20-40ns" as a universal PyO3 tax, since it is signature-dependent.

---

## 3. Pure-Python vs Cython vs Rust/PyO3 call floor

**Source:** https://pythonspeed.com/articles/python-extension-performance/ (fetched directly,
"The hidden performance overhead of Python C extensions", Itamar Turner-Trauring / pythonspeed.com).
Uses IPython `%timeit`.

- Pure Python function call: **62.5 ns ± 0.11 ns**
- Cython function call: **30 ns ± 0.04 ns**
- Rust (PyO3) function call: **165 ns ± 0.04 ns**

This is a *trivial* function (their point is call overhead, not the work done), and it's a single
data point with a different function shape than PyO3/pyo3#3827's `slow_len`, which is presumably why
Rust/PyO3 comes out ~5x *slower* than pure Python here rather than the ~30-40ns PyO3 overhead cited
elsewhere — the articles are not measuring the identical call shape, and the pythonspeed number
should be read as "yes, a trivial PyO3 call floor is on the order of 100-200ns, not single-digit ns,"
not as a precise, reproducible constant. **Confidence: medium** (single source, single run, no
methodology detail on what the Rust function's signature was beyond "a Rust function call").

---

## 4. cffi: ABI mode vs API mode — the split this research task specifically asked about

**Source:** https://cffi.readthedocs.io/en/stable/overview.html (official CFFI docs, fetched
directly). This is the authoritative, primary statement — CFFI's own maintainers describing their
own two modes:

> "The most immediate drawback of the ABI level is that calling functions needs to go through the
> very general *libffi* library, which is slow."

> "The API mode instead compiles a CPython C wrapper that directly invokes the target function. It
> can be massively faster."

> "Note also that at runtime, the API mode is faster than the ABI mode."

> "If using a C compiler to install your module is an option, it is highly recommended to use the
> API mode instead."

So: **ABI mode == ctypes-style, libffi-backed, slow** (this is the mode compylr's own README-Bash
CLI-adjacent "C-ABI escape hatch" language would land in if it meant *dynamic* symbol resolution
rather than a compiled wrapper). **API mode == compile-time generated CPython wrapper, comparable to
PyO3/pybind11's approach**, and is what the arXiv paper's `cffi (Maturin)`/`cffi (setuptools)` rows
above actually measured (they used `ffibuilder.set_source(...)` + `.emit_c_code()`, i.e. API mode —
confirmed by reading Listing 6 in the paper). I could not find a source with an isolated ABI-vs-API
nanosecond comparison on identical code — the closest hard numbers are ctypes (which is
libffi/ABI-shaped) vs cffi-API in the arXiv paper's Tables IV/V above, which is a reasonable proxy
given cffi's own docs say ABI mode is dominated by the same libffi cost ctypes pays.
**Confidence: high** on the qualitative claim (official docs, quoted verbatim); **no source found**
for an isolated ABI-vs-API-mode-only nanosecond delta.

---

## 5. pybind11 vs raw CPython C API (isolates the "convenience macro" tax, same shape as #2)

**Source:** https://ashvardanian.com/posts/pybind11-cpython-tutorial/ ("Our CPython bindings got 5x
faster without PyBind11", Ash Vardanian, StringZilla; fetched directly, dated Oct 10, 2023).

- Native Python `str.find()`: **1 μs**
- Same operation via **pybind11** binding: **15 μs**
- Same operation via **raw CPython C API** binding: **3 μs**
- **→ 5x latency reduction** switching pybind11 → hand-written C API, attributed specifically to
  `PyArg_ParseTupleAndKeywords`, which the author describes as building "substantial stack frames"
  and running "three separate loops for parsing" per call, vs. hand-rolled parsing or the
  `METH_FASTCALL` convention.
- Author's own caveat on precision, quoted: "The accuracy of this value might be off... The
  challenge lies in measuring such short durations."

**Confidence: medium-high** — concrete numbers from a named, technical author with a real library
(StringZilla) behind it, but self-published (not peer-reviewed) and the author flags his own
measurement uncertainty at these short durations.

---

## 6. nanobind vs pybind11 (the two most likely candidates for the C++ side)

Three independent sources, converging on similar multiples but not identical absolute numbers —
worth citing all three since none gives a full methodology + numbers combination on its own.

**a) nanobind's own docs** — https://nanobind.readthedocs.io/en/latest/benchmark.html (fetched
directly):
> "a ~3x improvement for simple functions, and an ~10x improvement when classes are being passed
> around" vs pybind11; "~1.6-2.1x improvement" vs cppyy.
Methodology: 720 synthetic function declarations + struct bindings, median of 5 runs, AMD Ryzen 9
7950X, Ubuntu 22.04.2, Clang++ 15.0.7, `-Os` build. **No absolute ns numbers given** in what I could
extract — it's presented as relative charts.

**b) matecdev.com, "Nanobind vs Pybind11: Calling C++ from Python in 2026"** —
https://www.matecdev.com/posts/nanobind-vs-pybind11-cpp-python.html (fetched directly, "Last
updated: April 6, 2026"):
- `add(1.0, 2.0)` called 1,000,000 times: **pybind11 0.079s total (79 ns/call); nanobind 0.061s
  total (61 ns/call)** → nanobind **~1.3x** faster.
- Binary size: pybind11 318 KB vs nanobind 125 KB (**2.5x** smaller).
- 100,000-element NumPy array pass/return: "nearly identical" between the two — i.e. for bulk
  array marshalling specifically, nanobind's advantage over pybind11 largely disappears; the
  per-call dispatch win doesn't carry over once the cost is dominated by element-count-scaled work.
- No version numbers or exact test dates given beyond the "last updated" stamp; **treat with medium
  confidence** — no listed methodology beyond the one function tested.

**c) pybind11's own official benchmark page** —
https://pybind11.readthedocs.io/en/stable/benchmark.html (fetched directly): compares itself against
**Boost.Python only** (not nanobind, not ctypes/cffi/PyO3) — compile time and binary size, **no
runtime-overhead numbers at all**. Useful only to confirm pybind11 doesn't publish its own
call-overhead figures.

**Reconciling (a) and (b):** the ~3x nanobind/pybind11 gap in nanobind's own benchmark and the ~1.3x
gap in matecdev's are for different workloads (720 synthetic declarations vs one `add(f64,f64)`
called a million times) and different hardware/compiler — they are not in tension, they're measuring
different things. The safe summary: **nanobind is reliably faster than pybind11 for pure call
dispatch, by something in the 1.3x-3x range depending on signature complexity, converging toward
parity once bulk array data dominates the call.**

---

## 7. compylr's own measured numbers — the most directly relevant data point of all

The repository **already has real, measured, in-project numbers** for exactly this question, for the
exact bridge shape (PyO3, per-call collection marshalling) the design decision is about. Quoted
verbatim from the project's own CLAUDE.md (checked into the repo, not something I measured myself —
flagging that this is a repo claim I did not independently re-run, though the surrounding openspec
trail shows it was measured):

> `/Users/mgb/RustRoverProjects/compylr/CLAUDE.md:168-170`
> "The Python boundary has a measurable per-element price on every call. On this machine an integer
> argument costs about 4 ns per element to convert, text about 42 ns, and returning an element about
> 10 ns."

The same figures appear in the archived OpenSpec change that measured them:

> `/Users/mgb/RustRoverProjects/compylr/openspec/changes/archive/2026-08-25-improve-generated-code-performance/proposal.md:66-67`
> "The boundary's per-element cost is confronted. A collection parameter converts element by element
> on every call: ~4 ns for a `list[int]` element, ~42 ns for a `list[str]` element, ~10 ns [returning
> an element]"

And a directly-measured before/after emission-quality number, from the Rust backend spec:

> `/Users/mgb/RustRoverProjects/compylr/openspec/specs/rust-backend/spec.md:1101-1102`
> "an allocation and a copy per element per loop. Measured on `text.total_length`, whose body is a
> single length read per element: 88.52us to 59.43us."

This is PyO3 specifically (compylr's only shipped Python bridge). It's consistent in *shape* with
the external sources above: an integer element (4ns) is far cheaper than a text element (42ns) —
matching the arXiv paper's finding that per-call/per-element marshalling, not raw dispatch, is where
the real cost sits, and matching PyO3/pyo3#discussions/2968's finding (see §8) that `String`
extraction is one of the more expensive `FromPyObject` paths.

**Confidence: high that these are the project's own recorded numbers** (verified by grep + reading
the exact lines); **the underlying measurement itself is not something I re-ran in this task**, so
treat the *numbers* as compylr's internal record rather than something this research independently
reproduced.

---

## 8. PyO3 `FromPyObject::extract` — where collection/dict extraction cost actually concentrates

**Source:** https://github.com/PyO3/pyo3/discussions/2968 (fetched directly). This is about the
*shape* of PyO3's extraction cost for polymorphic/collection types, which matters for "how much does
a `dict` cost" specifically:

- List extraction, when the input actually is a `PyList`:
  - Direct `.downcast()`: **961 ps** (0.961 ns)
  - `#[derive(FromPyObject)]` enum dispatch: **5.134 ns** (~5.3x slower than downcast)
- Set extraction, when the input actually is a `PySet`:
  - Direct `.downcast()`: **1.28 ns**
  - `#[derive(FromPyObject)]` enum: **650.84 ns** (~507x slower)
- Manual if/else-chain dispatch (avoiding the derive macro): **48.85 ns**

Root cause, quoted from a PyO3 maintainer in the thread: "This is a known foot gun of how the
`FromPyObject` trait is defined insofar it has to return a `PyResult` whereas `downcast` returns a
`Result<_, PyDowncastErr>`. Wrapping up the `PyDowncastErr` into a `PyErr` is the cost you are
seeing." I.e. **the dominant cost in the worst case here is PyO3's error-path allocation on a failed
downcast attempt inside the derived enum dispatch, not the extraction of the data itself** — a
distinct failure mode from "marshalling N elements costs O(N)," and one that would matter if
compylr's bridge ever went through a polymorphic/`FromPyObject`-derived path rather than a
known-concrete-type extraction (its current per-element list/dict conversion, per §7, extracts a
concrete `list[T]`/`dict[K,V]` given the IR already knows the type — so this failure mode is
probably avoided, but it's worth flagging as a PyO3 API trap if the bridge crate ever adds
polymorphic dispatch).

**Confidence: high** for the numbers (quoted from the discussion thread); this is a microbenchmark
in a GitHub discussion, not a paper, so treat as illustrative of a real, named cost path rather than
a citable absolute constant.

---

## 9. What I could not find (say-so, not estimate)

- **No source gives a controlled, single-methodology comparison across all six of ctypes, cffi (ABI
  *and* API mode separately), PyO3, pybind11, nanobind, and a hand-written CPython C extension.**
  The arXiv paper (§1) is the closest — three of the six, one methodology — everything else is
  cross-source triangulation across different hardware, compilers, and workloads.
- **No isolated ABI-vs-API-mode cffi benchmark** (only the qualitative, official-docs statement in
  §4, plus the proxy of ctypes-vs-cffi-API in §1's tables).
- **No `dict[K,V]` marshalling-cost number for any tool** other than compylr's own PyO3 bridge
  (§7's implicit per-element figures are for `list`/return values; the CLAUDE.md text says text and
  int list elements specifically, not dict). I did not find an external dict-marshalling benchmark
  for PyO3, pybind11, nanobind, or cffi in this search pass.
- **No hand-written CPython C-extension number in the same units as the others**, beyond the
  Ash Vardanian post's 3μs-for-`find()` data point (§5), which is one operation, not a general
  per-call/per-element constant.
- The nanobind-vs-pybind11 **absolute ns numbers** for anything beyond the single `add(f64,f64)`
  case in §6b were not published with a full breakdown in what I could fetch — nanobind's own docs
  (§6a) give only relative multipliers, not the ns floor.

---

## 10. Implications for compylr's design question

Read together, the sources converge on one structural point that bears directly on the
now-abandoned cpp-abi-bridge premise and on the pairwise-bridge decision generally:

1. **The mechanism matters far less than whether marshalling happens once or every call.** The
   arXiv paper's own M1-vs-M2 comparison (§1) — same PyO3 code, same cffi code — shows a **~10x**
   swing from that one design choice alone, dwarfing the differences *between* PyO3, cffi-API, and
   (at M2) even ctypes. compylr's bridge already pays the M1 cost by construction (every collection
   argument converts every call, per CLAUDE.md's own stated rule that collections cross by value)
   — which is a known, named, deliberate cost (see CLAUDE.md's "Known gaps" section), not a defect,
   but it is the dominant cost, confirmed externally.
2. **ctypes/ABI-mode-style dynamic dispatch (libffi) is not a viable "generic C ABI hub"
   mechanism** if per-call marshalling is in the picture — it is ~145x slower than PyO3 in exactly
   that regime in the one controlled study found (§1, Table IV), and CFFI's own maintainers say the
   same thing about their own ABI mode in general terms (§4). This is independent evidence, from a
   different angle than the Node-FFI-doesn't-exist finding, for why a shared, dynamically-resolved
   C-ABI bridge crate would have been the wrong direction even before the Node-side collapse: the
   dynamic-dispatch path both languages would have needed (libffi on the Python side, something
   equivalent on the Node side) is the specific mechanism every source here identifies as the slow
   one, not merely the unavailable one.
3. **A compiled, per-pair wrapper (PyO3 for Rust, nanobind or a hand-written extension for C++,
   node-addon-api for Node) pays a call-dispatch tax on the order of tens to low-hundreds of
   nanoseconds** (§2's ~33.5ns / §3's ~165ns / §6b's ~61-79ns, all different signatures, same order
   of magnitude), which every source agrees is dwarfed by O(n) marshalling cost as soon as a
   collection argument is involved (§1, §7) — so the *specific* choice between PyO3/pybind11/nanobind
   for a given language pair is a second-order decision next to "does this call convert a collection
   every time," which is architecture, not library choice.
