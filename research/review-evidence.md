# Adversarial review of `research/DECISION.md` — lens: evidence

Reviewer note: this is a second-agent check of a synthesis written by the same agent whose
decisions it defends. Scope, per the assignment: (1) the ~145x ctypes-vs-PyO3 figure and
`research/python-call-overhead.md`, (2) the C++26 matrix (re-fetched), (3) `node:ffi` v26.1.0
(re-fetched), (4) the nanobind multipliers (re-fetched), plus a pass over the three research files
written earlier in this run (`python-native-compilers.md`, `multi-target-transpilers.md`,
`semantics-mismatch.md`).

Method: every number below was either re-derived by hand from the numbers quoted in the document
under review, or checked against a freshly re-fetched primary source (the arXiv PDF was fetched and
read as images/OCR directly, not summarized by an intermediate model, precisely because a table of
numbers is exactly what a lossy summarization step could corrupt).

---

## 1. FATAL — the "~145× slower" ctypes-vs-PyO3 figure is arithmetically wrong

**Claim attacked** (`research/DECISION.md`, line 38, repeated verbatim in
`research/python-call-overhead.md` line 75-76):

> "**M1** — convert on every call | **~145× slower** (1.978e5 ms vs 6.423e3 ms)"

**What I did**: fetched `arxiv.org/pdf/2507.00264` directly (not via the lossy WebFetch
summarizer — the tool returned only a "cannot extract" response for a numeric-table request, so I
read the saved PDF's pages as images instead) and located Table IV, "Benchmark results for serial
runs converting parameters at the call site, as described on in-situ conversion (M1)":

| Method | mean (ms) |
|---|---|
| ctypes | 1.978e+05 ± 1.399e+03 |
| PyO3 | 6.423e+03 ± 34.70 |

These are exactly the two numbers `DECISION.md` and `python-call-overhead.md` both quote. The
transcription from the paper is correct. The problem is the multiplier computed from them:

```
197800 / 6423 = 30.7956...
```

That is **~30.8×**, not **~145×**. I checked every plausible alternative pairing in the same two
tables in case "145×" referred to a different (undisclosed) comparison and the label was merely
wrong:

| numerator | denominator | ratio |
|---|---|---|
| ctypes M1 (197800) | PyO3 M1 (6423) — **the pair the doc actually cites** | **30.8** |
| ctypes M1 (197800) | PyO3 M2 (634.7) | 311.6 |
| ctypes M1 (197800) | cffi-setuptools M1 (7347) | 26.9 |
| ctypes M1 (197800) | cffi-Maturin M1 (7262) | 27.2 |
| ctypes M2 (1369) | PyO3 M2 (634.7) | 2.16 (doc's own "~2.2×" for M2 — correct) |

No pairing anywhere in the paper's Table IV/V produces 145. The value simply does not correspond to
the data the document itself presents as its source. This is not a rounding disagreement (30.8 vs
31 would be immaterial) — it overstates the real effect by a factor of ~4.7, and it is the headline
number under a section titled "**settled, with numbers**" that the rest of the C-ABI-hub argument
leans on rhetorically ("answers *against* the hub far more strongly than the Node argument did").
The real number (~31×) still supports the same directional conclusion — M1 is catastrophically
worse for ctypes than M2 — but "settled, with numbers" is a claim about the numbers being right,
and this one isn't.

**Severity: fatal.** It is a load-bearing, headline figure, it is checkable from data already
quoted in the same two documents, and it fails the check by a wide margin.

**Correction**: replace "~145× slower" with "~31× slower" in both `research/DECISION.md` (line 38)
and `research/python-call-overhead.md` (lines 75-76, and the restated form at line 51 of
`DECISION.md`'s "Secondary" paragraph is unaffected since it cites a different, correctly-derived
number — the ~20ns PyO3 macro tax, see §5 below).

---

## 2. C++26 matrix — re-fetched, holds

**Claims attacked**: "GCC accepts `-std=c++26` from **14**, not 15." / "Clang spells it
`-std=c++2c`, has **no** contracts and **no** reflection at any version."

Re-fetched `gcc.gnu.org/projects/cxx-status.html` and `clang.llvm.org/cxx_status.html` directly.

- GCC: "C++26 features are available since GCC 14. To enable C++26 support, add the command-line
  parameter `-std=c++26`" — **confirms GCC 14**, matching the doc. (GCC's contracts/reflection land
  in GCC 16, gated further behind `-freflection` for reflection — consistent with, and slightly more
  precise than, the doc's framing, which doesn't claim GCC lacks these features, only that Clang
  does.)
- Clang: flag is `-std=c++2c`, confirmed verbatim. Contracts (P2900R14) listed "No" at every
  version. Reflection (P2996R13 and four related papers) all listed "No" at every version.

**Verdict: holds exactly as stated.**

---

## 3. `node:ffi` v26.1.0 — re-fetched, holds

**Claim attacked**: "`node:ffi` **does** exist, added **v26.1.0**... experimental,
`--experimental-ffi`-gated, self-described unsafe, no ABI guarantee."

Re-fetched `nodejs.org/api/ffi.html` directly. Confirmed verbatim: "Added in: v26.1.0",
"Stability: 1 - Experimental", gated behind `--experimental-ffi`, additionally gated behind
`--allow-ffi` under the Permission Model, and the page repeatedly warns the API "can crash the
process or corrupt memory if used incorrectly."

**Verdict: holds exactly as stated.**

---

## 4. nanobind multipliers — re-fetched, holds (with one sourcing nuance)

**Claim attacked**: "2.7–4.4× faster compiles, 3–5× smaller binaries, ~3× faster on simple
functions and **~10× when classes are passed around**; per-instance overhead 56 B → 24 B. ...
Stable ABI from Python 3.12."

Re-fetched `nanobind.readthedocs.io/en/latest/benchmark.html` directly. Confirmed verbatim:
"a ~3× improvement for simple functions, and an ~10× improvement when classes are being passed
around"; "~2.7-4.4× improvement" in compile time vs pybind11; "3-5× improvement" in binary size vs
pybind11 (11× vs Boost.Python).

**Nuance, not an error**: the "56 B → 24 B" per-instance overhead number and the "stable ABI from
3.12" claim are **not on the benchmark page** — they're on nanobind's separate `why.html` page,
which I also fetched and which confirms both verbatim ("the per-instance overhead for wrapping a
C++ type into a Python object shrinks by a factor of 2.3x. (pybind11: 56 bytes, nanobind:
24 bytes.)"; "nanobind can target Python's stable ABI interface starting with Python 3.12"). The
numbers are accurate; `DECISION.md` just doesn't distinguish which nanobind page each number came
from, which matters only if someone tries to re-verify from the single URL the review brief named.

**Verdict: holds.** Not a finding — the assignment's phrasing ("re-fetch
nanobind.readthedocs.io/en/latest/benchmark.html") pointed at one page, but the numbers it doesn't
carry are on an adjacent page of the same project's own docs, not fabricated.

---

## 5. Secondary PyO3-dispatch number in the same DECISION.md section — re-fetched, holds

Not on the assigned checklist, but it sits in the same paragraph as finding #1 and is a similarly
checkable number, so I verified it while the primary source was open: "PyO3 dispatch is ~33.5 ns/
call against ~12.6 ns for a hand-written `_PyCFunctionFastWithKeywords` — a ~20 ns macro tax."
Re-fetched `github.com/PyO3/pyo3/issues/3827` directly: confirms "33.5 ns ± 0.147 ns" (`slow_len`)
vs "12.6 ns ± 0.0159 ns" (`fast_len`), difference ≈ 20.9 ns, and the issue was closed as "not
planned." **Holds exactly as stated** — this is what a correctly-derived number in this document
looks like, which sharpens rather than excuses the error in finding #1 two paragraphs above it.

---

## 6. `python-native-compilers.md` — spot-checked, holds

Re-fetched the two most load-bearing quotes: Cython's `cdivision` docs
(`cython.readthedocs.io/.../source_files_and_compilation.html`) and mypyc's int-operations docs
(`mypyc.readthedocs.io/.../int_operations.html`). Both confirmed verbatim, including the specific
"up to a 35% speed penalty" figure and the exact undefined-on-overflow / wraps-on-u8 / "recommended
... unless performance is critical" language. The file's confidence markers (e.g., "low confidence"
on whether Codon's `-numerics=py` covers overflow) are honestly placed — it flags what it could not
establish rather than rounding up. **No fatal or major problems found in this file.**

## 7. `multi-target-transpilers.md` — spot-checked against the local submodule, holds precisely

This file's evidence is almost entirely local source reads (`inspiration/py2many`), which are
directly re-checkable without any web fetch. I re-read every file/line-range cited and all of the
following matched exactly:

- `py2many/clike.py:64-65` — `ast.Div: "/"`, `ast.FloorDiv: "/"` present as claimed.
- `pycpp/transpiler.py:412` — `raise AstNotImplementedError(...)` present verbatim at that exact
  line.
- `pyrs/transpiler.py:1097-1170` — `self._features.add("generators")` /
  `self._features.add("generator_trait")` at 1097-1098, `self._features.add("try_blocks")` at 1170
  — confirmed exactly (the doc paraphrases these as "emits `#![feature(...)]`," which is accurate in
  substance: those flags accumulate in `self._features` and are rendered as that exact pragma
  string elsewhere in the same file, at line 280).
- `tests/test_cli.py` — `EXPECTED_COMPILE_FAILURES = ["test_dunder.v", "with.v"]` confirmed verbatim
  at line 166-169.
- `AGENTS.md` and `doc/agent/transpilers.md` — both confirmed to say `py2many/transpilers/` "contains
  all transpiler implementations" and to reference `tests/test_transpiler.py`; `py2many/transpilers/`
  does not exist (`ls` fails) and the real test file is `tests/test_cli.py`. The doc-drift claim is
  exactly right.

**No fatal or major problems found in this file** — this is the strongest-sourced of the three,
because it is checkable against files on disk rather than against a web page a fetch tool might
summarize lossily.

## 8. `semantics-mismatch.md` — spot-checked; one misattributed quote (major), everything else holds

Re-read every cited line range against the same local submodule and against compylr's own source:

- `py2many/clike.py:64-65` (Div/FloorDiv), `pyrs/transpiler.py:898-921` (the `as usize` cast),
  `pycpp/transpiler.py:633-645` and `pygo/transpiler.py:738-753` (unguarded `value[index]` emission),
  `pyrs/plugins.py:203` / `pycpp/plugins.py:88` (`len()` dispatch lambdas), and
  `py2many/clike.py:188-202` (`_slice_value`'s `ast.Slice` guard) — **all confirmed verbatim, at the
  cited lines**, including the exact code shown in the document's own quoted snippets.
- compylr's own side: `crates/compylr-backend-rust/src/runtime.rs`'s `resolve_index` and
  `py_str_len`, and `crates/compylr-ir/src/behavior.rs`'s `Axis`/`LanguageBehavior` definitions —
  all confirmed verbatim at the cited line ranges, including the specific comment text quoted
  ("what a target's native indexing would do with it").

**One finding, major, not fatal:**

> **Claim attacked** (`research/semantics-mismatch.md`, lines 100-113): "`py2many/inference.py:
> 439-462`, `_handle_overflow`... It is explicitly heuristic and the authors know it — the code
> carries its own doubt in a comment right above the widening branch:
> ```
> # Does this hold across all languages?
> if left_id == "int":
>     left_id = "c_int32"
> ```"

I confirmed the comment text exists verbatim — but at **line 503**, not inside the cited
`439-462` range, and not inside `_handle_overflow` at all. `_handle_overflow` (which does contain
the actual widening step, `FIXED_WIDTH_INTS_LIST.index(max_idx + 1)` — that part of the doc's
description is accurate) ends around line 460, followed by `def visit_BinOp(self, node):`. The
`# Does this hold across all languages?` comment sits 40+ lines later, inside `visit_BinOp`, and it
precedes a different step: promoting a plain, unannotated Python `int` to the lookup key
`"c_int32"` so the *later* call into `_handle_overflow` has something to match against — not the
widening arithmetic itself. So the comment is genuine author doubt about a related but distinct
assumption (what fixed width an unannotated `int` should be treated as, for widening-lookup
purposes) rather than doubt about "the widening branch" as the document states.

The substantive point the document is making — py2many's overflow handling is a heuristic and its
own author flagged uncertainty about it somewhere in this code path — survives. What doesn't survive
is the specific evidentiary staging: a code block presented as sitting inside a cited line range,
when it is 40-60 lines outside that range and inside a different function answering a different
question.

**Severity: major**, not fatal — it doesn't change the paper's conclusion (py2many's overflow
handling is still, factually, an unchecked, addition/multiply-only, compile-time widening guess with
no per-operation control), but a reader who went to check "the comment right above the widening
branch" at the cited lines would not find it there, which is exactly the kind of small-but-checkable
slip an evidence review exists to catch.

**Correction**: cite the comment at `py2many/inference.py:503-505` (inside `visit_BinOp`), described
as "a comment on a related assumption a few lines before the call into `_handle_overflow`," not as
sitting inside the widening function itself.

---

## What holds

Tried specifically to break, and could not:

- The M1/M2 terminology itself (`in-situ conversion` vs `specialized constructor`) is the paper's
  own, not a reinterpretation — confirmed by reading the paper's Section II.D headings directly.
- The ~2.2× M2 ctypes-vs-PyO3 ratio (1369/634.7 = 2.157, rounds to "~2.2×" correctly).
- The ~20ns PyO3 macro-dispatch tax (33.5ns − 12.6ns ≈ 20.9ns) and its "closed as not planned"
  status.
- Every C++26/Clang/GCC claim, re-fetched fresh from both compiler projects' own status pages.
- The `node:ffi` existence, version, and experimental-gating claims, re-fetched fresh.
- All four nanobind multipliers, re-fetched fresh (with the minor page-attribution nuance in §4,
  which is not an error, just an unstated cross-page source).
- Cython's 35% `cdivision` figure and mypyc's undefined-on-overflow/u8-wraps/tagged-pointer claims,
  both re-fetched fresh from primary docs.
- Every local-source (`inspiration/py2many`, `compylr` source) line-range citation checked in
  `multi-target-transpilers.md` and `semantics-mismatch.md` except the one misattributed comment
  above — a genuinely high hit rate for citations this specific and this numerous, and evidence the
  local-source legs of this research were done by reading files rather than by recollection.

## Summary

One fatal finding (the 145× figure, which should read ~31×, in both `DECISION.md` and
`python-call-overhead.md`), one major finding (a misattributed quote/line-range in
`semantics-mismatch.md` whose substantive claim still holds), and no other problems established
after checking every number the assignment named plus a representative sample from the three new
research files. The overall shape of the argument — M1 is far worse for ctypes than PyO3, nanobind
beats pybind11, the C++26 floor is 14 not 15, `node:ffi` exists but is not a foundation to build on
— is not undermined by either finding. What is undermined is the document's claim to have this
"settled, with numbers": one of the numbers is wrong by ~4.7×, in the one place this review was
specifically sent to check first.
