# add-cpp-backend — adjudication of the three reviews

Written 2026-09-02. I read `research/DECISION.md`, all four `add-cpp-backend` artifacts, and the
nine delta specs myself, and re-derived every claim I rule on below. Where a reviewer's finding
survived my own check I say so; where I grade it differently, or where they missed something, I say
that too.

**Nine findings below are mine, not any reviewer's** — B7, B9, B10, B11, B12, B13, B16, and the
three-instance generalisation of the commit-gate conflict (§3, R-SPEC-1). One of them (B16) is that
the proposal's worked example contradicts the spec the same document introduces.

**Two open questions were closed by experiment during this review** (§4, E3/E6). The measurements
are reported inline and change D1.

---

## 0. What I ran

Compiled on this machine — Apple clang 21.0.0 / libc++, CMake 4.4.2, Node v24.11.0, no GCC:

| test | result |
| --- | --- |
| `std::expected` + `__builtin_add_overflow`, `-std=c++23` | **builds and runs** |
| same, `-std=c++26` | **builds and runs** — Clang accepts the `c++26` spelling |
| same, `-std=c++2c` | builds and runs |
| same, `-std=c++20` | fails (as expected; `std::expected` is C++23) |
| `__builtin_add_overflow_p` (GCC spelling), any std | **`use of undeclared identifier`** on Clang |
| `CMAKE_CXX_STANDARD 26` + `CMAKE_CXX_STANDARD_REQUIRED ON`, CMake 4.4 + AppleClang 21 | configures and builds cleanly |

Also confirmed by direct read: `crates/` holds **13** crates; `conformance.rs` touches
`compylr_registry::bridges::pairs()` only to check registry consistency (line 1531) and never
invokes a bridge; `differential.rs` is Rust-specific end to end (`rust_type`, `crate_attributes`,
`fn main() -> Result<(), RuntimeError>`, cargo) with **zero** occurrences of `go`/`golang`;
`Ty::can_key` is `Int | Str | Bool`; Python's `sequence_index` is `FromEitherEnd` + `Checked::Reported`
(`compylr-frontend-python/src/component.rs:68-70`); `git show 69491a5:…/design.md:136` contains
`emit_ctypes_loader`.

---

## 1. Claims that are BROKEN — retract or narrow

### B1 — the headline number is off by 4.7× (FATAL)

> **M1** — convert on every call | **~145× slower** (1.978e5 ms vs 6.423e3 ms)
> — `DECISION.md` §2

197800 / 6423 = **30.8**. The document's own quoted figures do not produce 145 under any pairing.
The M2 row is right (1369 / 634.7 = 2.157 ≈ "~2.2×"), which is what makes the M1 row look checked.

**Correct to:** `**~31× slower** (1.978e5 ms vs 6.423e3 ms)` in `DECISION.md` line 38 and in
`research/python-call-overhead.md` lines 75-76. Then delete "Real, and irrelevant next to a 145×
marshalling difference" (line 51-52) and re-state it against 31×.

I graded this myself from the transcribed numbers alone, so it holds whether or not the paper was
transcribed correctly. **The direction survives; only the magnitude was wrong.** A ~31× M1 penalty
against a ~2.2× M2 penalty still says exactly what the section wanted it to say.

### B2 — "#42 is the root" is false for five of seven findings (FATAL to the ordering advice)

> **#42 is the root.** Nearly every other finding is something a corpus that compiled its output
> would have caught on the first run. Fix ordering should follow from that, not from severity labels.
> — `DECISION.md` §1

I confirmed this independently of the reasoning reviewer, and the mechanism is simpler than they
put it. Two structural facts settle it:

1. **The corpus is authored as IR** (`conformance.rs` module doc: "Authored as IR, not as Python").
   Nothing a *frontend* does is reachable from it. That excludes **#37** and **#43** outright — both
   are `compylr-frontend-typescript/src/lower.rs` defects that happen before the IR exists.
2. **The corpus never invokes a bridge.** `conformance.rs` names `bridges::pairs()` exactly once, at
   line 1531, and only to assert the registry's languages resolve. That excludes **#39** — a bridge
   exporting 24% of members is invisible to a check that compiles backend output.

**#38** (a demo's hardcoded coverage script) and **#44** (an unbuilt host stub) are in neither layer.
Only **#41** — Go backend defects — is something 4a would catch.

The self-contradiction the reasoning reviewer flagged is real and worth quoting: §6 files #37/#39/
#41/#43/#44 into `fix-typescript-go-pair`, "not a small change." Consequences a compiling corpus
catches on the first run do not need a separate scoped change.

**Correct to:** "#42 is the root of #41 and of nothing else. The remaining findings sit in layers the
corpus does not reach — two in the TypeScript frontend, before the IR exists; one in a bridge, which
the corpus never invokes; two in checks and demos, which are different mechanisms. Fix ordering
follows from the layer a defect lives in, not from a single root."

### B3 — the M1 evidence is real but its billing is not (MAJOR, adjudicated against the reasoning reviewer in part)

> This was the decisive unknown flagged when the hub was first questioned. It is now answered, and it
> answers *against* the hub far more strongly than the Node argument did. — `DECISION.md` §2

`grep -rn "145\|ctypes\|M1\b\|M2\b" openspec/changes/add-cpp-backend/` returns **nothing**. The
argument DECISION.md calls more decisive than the Node argument does not appear anywhere in the
specification it is supposed to have decided. That part of the reasoning reviewer's finding stands.

**Where I disagree with them:** they argue the number is a strawman because "a compiled hub was never
measured, and nanobind-vs-pybind11 is only 2.7–10×." That is not fair to the evidence.
`git show 69491a5:openspec/changes/add-cpp-backend/design.md` line 136 is literally
`artifact.files.insert("__init__.py".into(), emit_ctypes_loader(...))`. The rejected draft *was* a
ctypes loader. A ctypes-vs-PyO3 measurement is exactly on point for it. The reviewer's own citation
of nanobind-vs-pybind11 is the weaker comparison — both are compiled binding generators, neither is
a hub, and the pair measures library quality rather than dispatch mechanism.

**What is actually wrong is the scope of the heading**, not the relevance of the number:

> ### The C-ABI hub would have been catastrophic for Python — settled, with numbers

**Correct to:** "*A ctypes-loaded* C-ABI hub would have been catastrophic for Python — settled, with
numbers", and add one sentence: "This measures libffi dispatch, not hubs as a class. A hub fronted by
a compiled shim was never proposed here and is not addressed by this figure."

### B4 — D2's stated justification is false for both named libraries (MAJOR)

> The error channel has to be a value at the generated-code layer regardless — nanobind and
> node-addon-api each translate a returned failure into their host's idiom, and letting a C++
> exception unwind into either binding layer is the one thing both forbid. — `design.md` D2

Both halves are wrong. nanobind's documented *primary* error mechanism is catching a thrown C++
exception and raising the Python equivalent, with a default translation table. node-addon-api
supports C++ exceptions when `NAPI_CPP_EXCEPTIONS` is defined — opt-in, not forbidden. The
completeness reviewer is right on the facts and I accept their evidence.

**The decision survives; the reasoning must be replaced** — and the correct reasoning is *stronger*
than what is there, which the reviewer did not notice. nanobind's default table maps
`std::exception → RuntimeError`. `specs/python-api/spec.md` requires "the Python exception the
corresponding Python operation would have raised" — `ZeroDivisionError`, `OverflowError`, `KeyError`,
`IndexError`. Relying on exception translation therefore either flattens every failure to
`RuntimeError` or requires registering a distinct C++ exception type per failure kind and a
translator for each, in both bridges, kept in step by hand.

**Correct to:** "Both binding libraries *can* translate a thrown C++ exception, and that is the
problem rather than the reason: nanobind's default table collapses `std::exception` to
`RuntimeError`, so preserving the exception *kind* `python-api` requires would mean a C++ exception
type and a registered translator per failure kind, duplicated across both bridges. A returned
`compylr::Error` carries the kind as data, and each bridge maps it in one place."

### B5 / B6 — two dangling references to documents deleted with the pre-D3 draft (FATAL to review, not to design)

> → D4 makes the allocating side the freeing side, and the ABI spec requires a handle to be released
> exactly once. … running the boundary tier under a sanitizer is the cheap check and belongs in
> tasks. — `design.md` Risks, lines 352-353

> Both satisfy every scenario in the ABI spec — `design.md` Open Questions, line 393

D4 in this file is "Flat member names make either binding library straightforward" and contains no
ownership content. `ls specs/` shows nine directories, none an ABI spec. `grep -i` over the whole of
`tasks.md` for sanitiser/leak/free/valgrind/asan returns **zero**. So the document names manual
memory management as its highest risk, promises a mitigation in "D4", cites a spec that does not
exist, and promises a task that was never written. The completeness reviewer is right and the grade
is right: this is the one place the D3 cleanup left a risk **unmitigated**, not merely mis-cited.

### B7 — the retracted Node claim is still live in `proposal.md` (MAJOR — missed by all three reviewers)

> The second half is false: **Node has no core FFI**, and `process.dlopen` loads only Node-API
> addons, requiring `napi_register_module_v1` rather than arbitrary C symbols.
> — `proposal.md` Why, line 15

This is the exact claim `DECISION.md` §2 retracts ("my stated fact was false") and §5 reports as
fixed ("**D3 corrected again** — the `node:ffi` claim"). It was fixed in `design.md` and nowhere
else. `proposal.md` is the document a reader opens first, and it still asserts the falsehood as the
load-bearing premise for the whole change's shape.

The reasoning reviewer found the GCC drift in `proposal.md` but not this, which is the larger one:
GCC 15-vs-14 is a wrong number, this is a retracted fact still doing argumentative work.

**Correct to:** replace with design.md D3's corrected form — Node *does* have `node:ffi` as of
v26.1.0; it is experimental, `--experimental-ffi`-gated, self-described unsafe, and newer than the
v24.11.0 this repository runs (I confirmed `node --version` = v24.11.0). Then re-derive the
paragraph that follows it, because **"C++ is the target that needs a hub least" currently rests on
the retracted premise in `proposal.md`** while resting on a sound one in `design.md` ("both hosts'
first-class binding libraries are already C++ header libraries"). Carry the sound premise over.

### B8 — the GCC-14 correction landed in one table and three places still say 15 (MAJOR)

The reasoning reviewer found one instance. There are three:

| where | text |
| --- | --- |
| `proposal.md:229` | "a C++26 compiler (**GCC 15+ or Clang 20+**)" |
| `design.md:103` | "support is partial and uneven across **GCC 15** and Clang 20" — inside D1's own Alternatives, contradicting D1's own table eight lines above |
| `design.md:299` | D7's worked example: `"GCC 14 is present" and "**GCC 15 is required**"` |

The third is the subtlest: it is the illustration of the version-floor check, and it illustrates a
floor D1 says is wrong.

### B9 — D1's configure-time gate rejects compilers that would build the output (MAJOR — mine, measured)

> `CMAKE_CXX_STANDARD_REQUIRED ON` means a compiler that cannot give C++26 fails at configure time
> with a message about the standard, which is actionable — `design.md` D1

D1 also says, correctly, that the emitted feature set is narrower: "`std::expected`, `std::vector`,
`std::unordered_map`, `std::unordered_set`, `std::tuple`, and the compiler's overflow builtins —
every one of them available well before C++26."

I measured it: that exact set compiles under **`-std=c++23`**. Not one emitted feature needs C++26.
So `CMAKE_CXX_STANDARD_REQUIRED ON` at 26 converts "we deliberately use nothing from C++26" into a
hard configure-time refusal on GCC 13, on any Clang without the c++26 mode, and on any vendor
toolchain that has libc++'s `std::expected` but not the newest standard flag. The failure message is
actionable about a requirement that is not real.

This is the one place D1's two halves — "ask for the latest standard" and "emit only what ships" —
actually conflict, and the design asserts the conflict is free.

**Correct to:** keep C++26 as the *requested* standard (that is what was asked for) but stop making it
a gate. `set(CMAKE_CXX_STANDARD 26)` with `CMAKE_CXX_STANDARD_REQUIRED OFF`, plus an explicit
`target_compile_features(compylr_generated PRIVATE cxx_std_23)` as the real floor. Then the tree asks
for 26, silently accepts 23, and fails at configure time naming **C++23** — which is a floor the
generated code genuinely has.

### B10 — "the compiler's overflow builtins" is not one spelling (MAJOR — mine, measured)

`design.md` D1 and `tasks.md` 3.3 both say "the compiler's overflow builtins" as though it were a
single portable facility. I compiled it: `__builtin_add_overflow_p` — the GCC form, which returns the
overflow flag without producing the result — is **`use of undeclared identifier`** on Clang at every
standard. Only the three-argument `__builtin_add_overflow(a, b, &out)` is common to GCC and Clang.
MSVC has neither.

`compat.hpp` must be self-contained *and* buildable on both compilers (D6), so this is a constraint on
the header, not a detail. It is also precisely the kind of thing that would be found on the first CI
run on the other compiler and not before.

### B11 — the `-std=c++2c` claim is stale (MINOR — mine, measured)

> Clang spells it **`-std=c++2c`** … never hard-code `-std=c++26`, which is not what Clang documents.
> — `design.md` D1

Apple clang 21 accepts `-std=c++26` and builds. The **conclusion** (emit `CMAKE_CXX_STANDARD` and let
CMake choose) is right and should stay; the **reason** given for it is no longer true and will read as
a bug to anyone who tries it. Restate as: modern Clang accepts both spellings, older Clang only
`c++2c`, and letting CMake choose is how the manifest stays correct across both without probing —
which also preserves the pure-emission rule.

### B12 — `frontends/cpp/` contradicts the change's own Non-Goal (MINOR — mine)

> **New directories**: `demo/demo-python-cpp/`, `demo/demo-ts-cpp/`, and `frontends/cpp/` for the C++
> side of the corpus. — `proposal.md` Impact

`frontends/<lang>/` holds a *source* language's fixtures (`frontends/python/{compylr,fixtures,tests}`,
`frontends/typescript/fixtures`). C++ is a **reserved frontend** and this change's stated Non-Goal is
that it stays one. There is no C++ source language, so there is no "C++ side of the corpus" to put
there. No task creates it and no spec references it. **Delete the clause.**

### B13 — `tasks.md` has no group 7 (MINOR — mine)

Groups run 1, 2, 3, 4, 4a, 5, 6, **8**, 9, 10, 11, 12, 13. Group 7 was the shared-ABI crate group,
deleted with D3. Same leftover class as B5/B6 and the same cause: the D3 rewrite was applied to prose
and not to the document's skeleton. Renumber, or (better, since `4a` already sets the precedent)
leave the gap and say why in one line so the next reader does not go looking for the missing group.

### B14 — the three new research legs were dismissed slightly too fast (MINOR)

> `python-native-compilers`, `multi-target-transpilers`, `semantics-mismatch` | Never run. Lowest
> value of the ten — none would change a decision already made — `DECISION.md` §3

Accurate for D1–D8, and all three research writeups agree with it. But the `multi-target-transpilers`
leg found something that bears on **`tasks.md` 4a.3**, which this same change is writing: py2many pays
for its compile-and-run tier with a checked-in golden file per `(case, backend)` pair plus a
hardcoded `EXPECTED_COMPILE_FAILURES` allowlist — a second N×M surface. 4a.3 says only "Where a
corpus entry carries an expected value, run the compiled output and compare," and the corpus is
authored as IR, so there is **no CPython oracle** to derive the answer from. As written, 4a.3 can only
mean a hand-authored literal per entry, i.e. the trap, multiplied across backends.

**Correct to:** keep the dismissal for D1–D8 and add: "one exception — the golden-file warning applies
directly to 4a.3, which specifies no oracle."

### B15 — the audit-coverage complaint is a caveat, not a retraction (I DOWNGRADE the completeness reviewer)

They grade "23 findings confirmed by a second agent" as **major** because the audit never opened 7 of
13 crates including `compylr-core` and `compylr-bridge-python-rust`. The coverage fact is true — I
checked the crate list and it is 13 — but the claim under attack says only that 23 findings were
adversarially confirmed, which is not a claim of exhaustiveness. Nothing in §1 asserts the audit
found everything. **Downgrade to a one-line caveat** in §1: "The audit covered six crates, all on the
TypeScript/Go side; `compylr-core`, `compylr-backend-rust`, `compylr-bridge-python-rust`,
`compylr-frontend-python`, `compylr-cli`, `compylr-registry` and `compylr-diagnostics` were not read."
That is worth writing down precisely because the Python→Rust path is the reference this change copies.

I also **downgrade the evidence reviewer's second finding** (the `inference.py:503` misattribution) to
minor. The quote is real, the substantive claim is unaffected by their own account, and it lives in a
research file `DECISION.md` does not cite. Fix the line number; it changes nothing.

### B16 — the worked example emits code its own spec forbids (MAJOR — mine, missed by all three)

`proposal.md`'s Worked Example is the most-read artefact in the change, and two of its three checked
operations are emitted unchecked.

```cpp
running = running + values[static_cast<size_t>(i)];
```

Under the default Python stance — which the example explicitly claims to be under, and which I read
out of `compylr-frontend-python/src/component.rs:54-73`:

* `integer_overflow: Checked::Reported` → `running + values[...]` must be a checked add. The example
  emits a bare `+`.
* `sequence_index: { origin: IndexOrigin::FromEitherEnd, checked: Checked::Reported }` → the
  subscript must go through the helper, resolving a negative offset and reporting out of range. The
  example emits `values[static_cast<size_t>(i)]`, which is origin-from-start and unchecked — the
  exact shape `runtime.rs:477-491` exists to avoid, and the exact live bug the `semantics-mismatch`
  research found in py2many's Rust backend.

Only the `//` is shown correctly. And `specs/cpp-backend/spec.md` states the rule the example breaks:
"Emitted code SHALL select a helper by matching on the **modes** an IR node carries — … the origin and
checking of a subscript."

The example is marked `expected:` / "nothing here has been run", which covers it being *unverified* —
it does not cover it being *contrary to the requirement in the same change*. Someone implementing
from it will emit unchecked indexing, the C++ differential tier will disagree with CPython on the
first negative index or out-of-range access, and the diagnosis will start in the wrong place.

**Correct to:** show all three operations checked, or pick an example whose operations are genuinely
plain and say so. If the fully-checked form is judged too noisy to read, that is itself worth knowing
before implementation, because it is what every emitted function will look like.

---

## 2. Claims that survived a genuine attempt to break them

**S1 — "compylr's boundary is M1 by construction."** I looked for any pre-marshalled handle path:
`HostBridge`/`BuildKey`/`HostArtifact` in `compylr-core/src/bridge.rs` carry no such concept, and
`CLAUDE.md` states every argument crosses by value on every call, collections included. Both other
reviewers reached the same conclusion from `bindings.rs:212-213`. **Holds.** This is the premise that
makes the (corrected, 31×) M1 figure the relevant regime rather than M2, and it is the load-bearing
half of B3 — so B3 narrows the framing without touching the conclusion.

**S2 — the Node argument rejects a `node:ffi`-shaped hub.** Attacked by checking the ground rather
than the docs: `node --version` on this machine is **v24.11.0**, and `node:ffi` landed in v26.1.0. The
module does not exist here at all. Experimental + flag-gated + no ABI guarantee + not present on the
development machine is four independent reasons, any one sufficient. **Holds, and holds on its own** —
which matters, because it means B1 and B3 do not endanger D3.

**S3 — D5, stance and preserved guarantees decided separately.** Attacked by trying to derive one
from the other: `RUST_BEHAVIOR` is `Unchecked` on all six axes while `PRESERVES` names all three
guarantees, and Python's `integer_overflow` is `Checked::Reported`, so a derived `preserves()` would
refuse every default Python program on both existing targets, not just C++. **Holds, and is the
best-evidenced decision in the change** — it is the only one with a working in-repo precedent that
would visibly break if reversed.

**S4 — D4, flat member names.** `Unit::add_function` refuses a duplicate across a unit
(`ir.rs:1216`), and the boundary tier builds every accepted fixture into one unit, so uniqueness is
already load-bearing elsewhere. **Holds.**

**S5 — the `std::unordered_map<K, V>` / `std::unordered_set<T>` spellings.** I attacked this expecting
to find a missing `std::hash`: C++ specialises `std::hash` for neither `std::tuple` nor
`std::vector`, so a tuple-keyed mapping would emit code that does not compile. `Ty::can_key`
(`ir.rs:171-173`) restricts keys and set elements to `Int | Str | Bool`, all three of which `std::hash`
specialises. **Attack failed; the type table is safe as written.** Worth recording because it is the
obvious next worry and it is already closed.

**S6 — "This change does not touch the IR."** Attacked by looking for something C++ needs that the IR
cannot say: all six axes are already carried, ownership is a backend concern, and `Ty` already has an
instance variant. Nothing found. **Holds** — and it is the change's actual thesis, so it surviving
matters more than most.

**S7 — nanobind over pybind11.** Not re-fetched by me, but three independent reviewers fetched
nanobind's own benchmark and `why.html` pages and all four multipliers plus the 56B→24B figure and
the 3.12 stable-ABI claim came back matching. **Holds on evidence.**

**S8 — D6, `compat.hpp` as one self-contained header with two lives.** Mirrors `runtime.rs` +
`RUNTIME_SOURCE`, which exist and work. **Holds** — subject to B10, which is a constraint on its
contents, not on the decision.

**S9 — the overall D3 conclusion (no hub, two pairwise bridges).** Survives B1, B3 *and* B7 together:
strip out the 145× number entirely and strip out the retracted Node-FFI claim entirely, and what
remains — `node:ffi` is experimental and absent on the ground, both hosts ship first-class C++ binding
libraries, and a hub buys "two mechanisms, one experimental" instead of "two mechanisms, both
first-class" — still decides it. **This is why the verdict in §5 is not "stop".**

---

## 3. Revision list for `openspec/changes/add-cpp-backend/`

Ordered by artifact. Each item is actionable without re-deriving anything above.

### `proposal.md`

* **R-P-1 (blocking).** Why, ¶2 line 15: delete "**Node has no core FFI**, and `process.dlopen` loads
  only Node-API addons, requiring `napi_register_module_v1` rather than arbitrary C symbols." Replace
  with design.md D3's corrected text (node:ffi exists as of v26.1.0; experimental, `--experimental-ffi`,
  self-described unsafe, no ABI guarantee, newer than this repo's v24.11.0). **B7.**
* **R-P-2 (blocking).** Why, ¶3: re-derive "C++ is the target that needs a hub least" from the premise
  that survives — both hosts' first-class binding libraries are already C++ header libraries — since
  its current premise in this file is the sentence R-P-1 deletes. **B7.**
* **R-P-3.** Impact → Toolchain: "GCC 15+ or Clang 20+" → the measured floor. Recommended text:
  "a compiler providing C++23's `std::expected` and the two-argument `__builtin_*_overflow` family
  (GCC 13+, Clang 16+/AppleClang 15+), plus CMake 3.28+. The manifest *requests* C++26; no emitted
  feature requires it." **B8, B9, B10.**
* **R-P-4.** Impact → New directories: delete `frontends/cpp/`. **B12.**
* **R-P-5 (blocking).** Worked Example: emit the checked add and the checked, from-either-end subscript,
  or replace the program with one whose operations are genuinely plain. Add one line naming which
  operations are fallible under Python's default stance and why. **B16.**
* **R-P-6.** Impact → crate count: "13 to 16" is correct as of today (verified). Leave it; note it is
  verified so the next reader does not re-check.

### `design.md`

* **R-D-1 (blocking).** D1, line 103: "GCC 15 and Clang 20" → GCC 14 and Clang, consistent with D1's
  own table eight lines above. **B8.**
* **R-D-2 (blocking).** D1: reconcile `CMAKE_CXX_STANDARD_REQUIRED ON` with the narrower emitted set.
  Recommended: `CMAKE_CXX_STANDARD 26` + `CMAKE_CXX_STANDARD_REQUIRED OFF` +
  `target_compile_features(compylr_generated PRIVATE cxx_std_23)`. Record the measurement: the whole
  emitted set builds under `-std=c++23` on AppleClang 21/libc++, so a hard 26 gate refuses compilers
  that would have built the tree. **B9.**
* **R-D-3.** D1: restate the `c++2c` paragraph. Clang 21 accepts `-std=c++26`; older Clang accepts only
  `c++2c`. Keep the conclusion (let CMake choose; never probe, because emission is pure). **B11.**
* **R-D-4 (blocking).** D2, first "Why" paragraph: replace the false claim about what nanobind and
  node-addon-api forbid with the corrected and stronger one — both *can* translate a thrown exception,
  nanobind's default table collapses `std::exception` to `RuntimeError`, and preserving the exception
  *kind* `python-api` requires would mean a C++ exception type plus a registered translator per failure
  kind in both bridges. A returned `compylr::Error` carries the kind as data and each bridge maps it
  once. **B4.**
* **R-D-5 (blocking).** Risks, lines 352-353: the paragraph cites a D4 that no longer says this and an
  ABI spec that does not exist. Either (a) add a new decision — **D9, ownership at the boundary** —
  covering who owns a value crossing into `nb::class_` / `Napi::ObjectWrap`, when a returned collection
  is copied versus moved, and what the generated code may not hold a reference to after returning; or
  (b) rewrite the risk to point at the bridges' own scenarios. **(a) is the right answer**, because the
  risk is real and currently has no mitigation anywhere in the change. **B5.**
* **R-D-6.** Open Questions, line 393: "every scenario in the ABI spec" → name the spec that exists
  (`specs/typescript-api/spec.md`, or the new D9 if added). **B6.**
* **R-D-7.** D3: fold in the corrected M1 evidence, scoped honestly — "the rejected draft emitted a
  ctypes loader (see the change's own history); ctypes in the convert-on-every-call regime measures
  ~31× against PyO3, and compylr's boundary is that regime by construction. This condemns a ctypes
  loader, not hubs as a class." Without this, DECISION.md §5 claims a revision that was never applied.
  **B3.**
* **R-D-8.** D7, line 299: change the worked example so it does not use "GCC 15 is required" as its
  illustration of a version floor D1 puts at 14. **B8.**
* **R-D-9 (blocking, new).** Add **D10 — a mapping read reports, and never inserts.** `std::unordered_map::operator[]`
  default-constructs a missing key and is non-const; the IR states a missing mapping key **always**
  reports, with no mode. So `d[k]` must emit through a `compat.hpp` helper over `find()` returning
  `std::expected` (the analogue of `runtime.rs:620` `py_key`), never `operator[]`. Consequence worth
  stating: **a mapping read makes its function fallible**, exactly as a checked division does. This is
  the single most likely silent-wrong-answer in the whole backend and nothing in the change currently
  mentions it.
* **R-D-10 (blocking, new).** Add **D11 — class-valued signatures**. `class_valued_signatures.py` is an
  accepted fixture, `CLAUDE.md` records that it runs through **both** differential tiers, and
  `specs/fixture-corpus/spec.md` in this change requires both tiers over **every** registered pair. So
  the C++ backend must handle borrowed instance parameters on day one. Decide, and write down: the C++
  analogue of `PyRef`/`PyRefMut` (a `T&` / `const T&` parameter over the object the host holds), and
  the escape rule — a borrowed instance, **and a field read from one**, may not leave in an owned
  return, because the caller still holds it and would get a detached copy. That last rule is a located
  diagnostic in the Rust path precisely because the generated code compiles either way; in C++ the same
  mistake is a dangling reference rather than a stale copy, so it is worse, not better.
* **R-D-11.** Add to Risks: **what "nothing throws" can actually promise.** `std::vector::push_back`,
  `std::string`, and `std::unordered_map` insertion can all throw `std::bad_alloc` or `std::length_error`
  from inside emitted code. The honest promise is that no *compylr-defined failure* escapes as an
  exception and that each exported entry point terminates the few allocator exceptions that can
  originate below it — not that no exception can propagate. See R-SPEC-3.

### `tasks.md`

* **R-T-1.** Renumber, or leave the 6→8 gap with a one-line note that group 7 was the shared-ABI crate
  deleted by D3. **B13.**
* **R-T-2 (blocking).** 3.3: "the compiler's overflow builtins" → name the portable spelling
  `__builtin_add_overflow(a, b, &out)` and add a sub-task asserting `compat.hpp` builds under **both**
  GCC and Clang, since D6 requires it to be self-contained and paste-able. Measured: the GCC `_p`
  variants do not exist on Clang. **B10.**
* **R-T-3 (blocking).** 4a.3: specify the oracle. The corpus is authored as IR, so there is no CPython
  answer to derive from — an entry's expected value can only be a literal the entry itself carries.
  Say so explicitly, and bound it: expected values live **on the corpus entry**, one place, never as a
  golden file per `(entry, backend)` pair. Cite the py2many precedent in one line. **B14.**
* **R-T-4 (blocking).** 4a.5 → 4a.6: resolve the contradiction. 4a.5 says "Expect failures … File what
  is new, do not fix it here"; 4a.6 says `cargo test --workspace`; commit. If Go's corpus output does
  not build, the new check fails the commit gate by the spec's own wording. Pick one and write it down:
  (a) fix Go's corpus output here, widening the change; (b) land the tier gated to `cpp` only and file
  turning it on for Go with `fix-typescript-go-pair`; or (c) add an explicit, enumerated
  known-failure list with a filed issue per entry and a test that the list only shrinks. **(b) is
  cleanest** — it keeps this change's scope and does not invent a permanent allowlist mechanism. See
  R-SPEC-1, because the same conflict appears in two more specs.
* **R-T-5 (blocking).** Add to group 5/6: run the boundary tier under AddressSanitizer and
  LeakSanitizer at least once per bridge. `design.md` promises this ("belongs in tasks") and it is
  absent. **B5.**
* **R-T-6 (blocking, new).** Add to group 4: emit mapping reads through the reporting helper, never
  `operator[]`; assert a read of a missing key reports and **does not insert**. R-D-9.
* **R-T-7 (blocking, new).** Add to groups 4 and 5/6: class-valued parameters and returns — borrowed
  instance parameters, the escape refusal, and `class_valued_signatures` passing both differential
  tiers over `(python, cpp)`. R-D-10.
* **R-T-8 (blocking).** 9.3 badly understates the work. `differential.rs` is Rust-specific end to end —
  `rust_type()`, `crate_attributes()`, `fn main() -> Result<(), RuntimeError>`, cargo — and contains no
  occurrence of `go` or `golang`. "Change the tiers to enumerate pairs from the registry" is not an
  enumeration change; it is generalising a Rust-shaped harness to a per-target one, and it is plausibly
  the largest single task in the change. Split it and size it honestly.
* **R-T-9.** 8.2: the version floor must handle vendor-versioned compilers. This machine reports
  "Apple clang version 21.0.0", which maps to no upstream Clang release; a floor expressed as "Clang
  16+" cannot be checked against it directly. Prefer feature-probing at *build* time (a CMake
  `check_cxx_source_compiles` for `std::expected` and `__builtin_add_overflow`) over version parsing —
  note this does **not** violate the pure-emission rule, because the probe lives in the emitted
  manifest, which is the same bytes on every machine.
* **R-T-10.** 12.4: `CLAUDE.md` is already stale independently of this change — it says the workspace is
  "nine crates" (it is 13) and references `python/fixtures/` (now `frontends/python/fixtures/`). Fix
  those in the same pass so the C++ additions are not layered onto wrong text.

### `specs/pipeline-architecture/spec.md`

* **R-SPEC-1 (blocking).** "A backend that renders text which does not build SHALL fail the check" has
  no carve-out, and no xfail convention exists in the repository. **The same conflict appears in three
  new requirements, not one** — this is where I extend the reasoning reviewer:
  1. here, against the Go backend (#41);
  2. `specs/demo/spec.md`, "Every bridged pair has a demo at the same standard", derived from the
     registry — `demo-ts-go` exists but its coverage report is a stub and its benchmark table is
     fabricated (#38), so it fails the standard on the day the requirement lands;
  3. `specs/fixture-corpus/spec.md`, "Both tiers SHALL run over **every** `(source, target)` pair the
     bridge registry reports" — the `(typescript, go)` boundary tier cannot pass, because #39 reports
     the loader is not importable by Node at all.

  Each of the three turns a confirmed pre-existing defect into a failing requirement in *this* change,
  and the plan's own §6 says those defects belong to `fix-typescript-go-pair`. Fix all three the same
  way, and say which way in the spec text rather than leaving it to the implementer: state the
  requirement unconditionally (it is the right requirement) and add a scoping sentence — "this
  requirement takes effect for a pair when that pair's defects filed under #38/#39/#41 are closed;
  `(typescript, go)` is enumerated as a known-failing pair with a filed issue until then, and the list
  of such pairs may only shrink." That keeps the requirement honest, keeps the commit gate green, and
  makes the exception visible and self-expiring instead of a silent allowlist.

### `specs/cpp-backend/spec.md`

* **R-SPEC-2 (blocking, new).** Add a requirement: **A mapping read reports a missing key and does not
  insert.** Scenarios: reading a present key returns the value; reading an absent key returns a failure
  naming the key; **after a failed read the mapping is unchanged in size** (the `operator[]` trap);
  and a function containing a mapping read returns `std::expected`. R-D-9.
* **R-SPEC-3 (blocking).** Narrow the scenario "Nothing escapes as an exception" — currently "WHEN any
  exported entry point is called with **any argument** THEN no exception propagates out of it". Not
  achievable: `push_back`, `std::string`, and map insertion can throw `std::bad_alloc`. Restate as two
  scenarios: (a) no compylr-defined failure is signalled by throwing — every one is returned; (b) each
  exported entry point terminates or translates any exception originating below it, so none reaches the
  host runtime. That is testable and true. R-D-11.
* **R-SPEC-4 (blocking, new).** Add a requirement covering **class-valued signatures**: an instance
  parameter is borrowed rather than copied; a mutation through a mutable borrow is observable to the
  caller; and returning a borrowed instance, **or a field read from one**, is refused with a located
  diagnostic. The type-table row `instance of `Class` | `ClassName`` is not sufficient — by itself it
  says instances cross by value, which is the opposite of what `CLAUDE.md` records for the working
  path. R-D-10.
* **R-SPEC-5.** "The backend targets C++26 and says so in its manifest" — the requirement says the
  manifest "SHALL name the minimum compiler versions the generated source requires". With B9's
  measurement, that minimum is the **C++23** feature floor, not C++26. State the distinction in the
  requirement text: the manifest requests C++26 and requires C++23; a compiler giving only C++23 builds
  the tree.
* **R-SPEC-6.** The Scenario Outline covers three division rows and no remainder rows, though the
  requirement above it names "the sign convention and checking of a remainder" as an axis the emitted
  code must dispatch on. Add the `-7 % 2` rows under each `RemSign`, and a `TowardZero`/`Unchecked`
  division row so all four mode combinations appear.

### `specs/python-api/spec.md`

* **R-SPEC-7.** "A reported failure becomes the matching Python exception" has one scenario
  (`ZeroDivisionError`) and the stance requirement adds `OverflowError`. With B4's correction — nanobind's
  default table flattens everything to `RuntimeError` — the *kind* mapping is the thing at risk. Add
  scenarios for `KeyError` (missing mapping key, which R-SPEC-2 makes reachable) and `IndexError`
  (out-of-range subscript under Python's `Checked::Reported` origin), so the requirement pins more than
  one kind and a flattening bridge fails.

### `specs/build-pipeline/spec.md`

* **R-SPEC-8.** "A compiler too old for the target's standard is diagnosed" — reword from *standard* to
  *the features the generated source uses* (B9), and add a scenario for a vendor-versioned compiler
  whose version string does not map to an upstream release (R-T-9), since that is the machine this will
  first be run on.

---

## 4. What still has no evidence, and the cheapest experiment for each

| # | gap | cheapest experiment |
| --- | --- | --- |
| **E1** | **Does the Go backend's conformance output compile today?** This decides whether 4a.5→4a.6 can pass and therefore whether R-T-4/R-SPEC-1 are blocking or theoretical. Nothing in the audit answers it — #41 is defects against the *spec*, not against `go build`. | `cargo run -p compylr-cli -- --backend go --emit crate --out /tmp/g <fixture>` then `go build ./...`. Under 5 minutes, and it is the highest-value unknown left. |
| **E2** | nanobind vs PyO3 on compylr's own boundary. The corrected 31× is ctypes-vs-PyO3 and says nothing about this. | Unchanged from DECISION.md §3: `demo-python-cpp`'s benchmark answers it directly once it exists. No cheaper experiment is worth running first. |
| **E3** | `std::expected` / overflow-builtin floor. | **Closed this session** for Clang: builds on AppleClang 21/libc++ at `-std=c++23`, `c++26`, `c++2c`; `__builtin_add_overflow_p` is GCC-only. **Still open for GCC**: `docker run --rm -v $PWD:/w gcc:13 g++ -std=c++23 /w/t.cpp` and again on `gcc:14`. One command each. |
| **E4** | Does `nb::class_` actually give the "mutated attribute is what the caller sees next call" property the change stakes D3 and `python-api` on? | ~30-line nanobind module with a counter class, built once by hand, called twice from Python. Do this **before** writing `compylr-bridge-python-cpp`, not after. |
| **E5** | Does cmake-js + node-addon-api build against the emitted `CMakeLists.txt` on Node **24**? Task 6.2 asserts the fit; nothing has tried it, and Node 24 is what is on the ground. | Smallest possible addon exporting one function, built with cmake-js, required from Node 24. |
| **E6** | Whether `CMAKE_CXX_STANDARD 26` + `REQUIRED ON` configures on this machine. | **Closed this session**: it does, on CMake 4.4 + AppleClang 21. Which is what makes B9 a live problem rather than a hypothetical — the gate will pass here and fail on someone else's GCC 13 for no real reason. |
| **E7** | Can the `(typescript, go)` boundary tier be made to run at all? R-SPEC-1's third instance depends on it. | Try to `require()` the artifact `compylr-bridge-typescript-golang` emits for one fixture. If it cannot load (as #39 says), the fixture-corpus requirement needs the scoping sentence before it lands. |
| **E8** | Can nanobind map distinct failure kinds to distinct Python exceptions without a translator per kind? R-SPEC-7 depends on the answer. | Register two C++ exception types with `nb::exception<>` in the E4 program and check both kinds arrive distinct. Ten extra lines on an experiment already being run. |

---

## 5. Verdict

**Not safe to implement as written — but the design is sound and the repairs are documentation-level
except for four: the missing mapping-key rule (R-D-9/R-SPEC-2), the missing class-valued-signature
rule (R-D-10/R-SPEC-4), the unmitigated ownership risk (R-D-5/R-T-5), and the three new requirements
that fail on day one against the known-broken `(typescript, go)` pair (R-SPEC-1); apply those,
correct the retracted Node claim still live in `proposal.md` (B7) and the worked example that
contradicts its own spec (B16), and the plan is implementable.**
