# nanobind (primary source, fetched 2026-09-01)

https://nanobind.readthedocs.io/en/latest/why.html and /benchmark.html

## Measured against pybind11
Benchmark: 720 trivial function declarations and struct bindings, AMD Ryzen 9, clang++ 15.0.7,
debug and `-Os` builds, medians of five runs.

| axis | vs pybind11 | vs others |
| --- | --- | --- |
| compile time | **2.7–4.4×** faster | 1.6–4.4× vs Cython |
| binary size (`-Os`) | **3–5×** smaller | ~11× vs Boost.Python, 3–12× vs Cython |
| runtime, simple functions | **~3×** faster | — |
| runtime, **classes passed around** | **~10×** faster | ~1.6–2.1× vs cppyy |
| per-instance wrapper overhead | 56 B → **24 B** (2.3×) | — |

## Requirements
- Targets a smaller C++ subset than pybind11; leans on C++17 and Python 3.10 improvements.
- **Stable ABI (Py_LIMITED_API) from Python 3.12**; 3.10 in "split mode."

## Consequences for `add-cpp-backend`
- D3's choice of nanobind over pybind11 is supported on every axis that matters here. Compile time
  is the one a user feels — compylr compiles on first call — and **~10× on class passing** is the
  one that matters most, since instances crossing by handle is the contrast the subset draws.
- The 3.12 stable-ABI floor is real: below it, the built artifact is pinned to the interpreter's
  minor version, so the `.compylr/` cache key must include it. Node-API has no such constraint,
  which makes this an asymmetry between the two new bridges rather than a shared rule.

## Absolute figures
The docs report multipliers; absolute seconds/KB live in plots not extractable as text.
