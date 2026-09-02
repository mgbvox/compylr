# Adversarial review of research/DECISION.md — lens: completeness

Reviewer: a second agent, not the one that wrote DECISION.md. Method: read DECISION.md in full,
cross-referenced every claim it rests on against `research/audit-findings.json`, `crates/`,
`openspec/changes/add-cpp-backend/{design,tasks}.md` and its `specs/`, and the three research
files produced earlier in this run. Verified two external facts by fetching primary sources
(nanobind and node-addon-api docs) since WebSearch was exhausted for the session.

---

## 1. Which crates the audit never opened

`research/audit-findings.json`'s 24 findings cite evidence from exactly six locations under
`crates/`: `compylr-backend-golang`, `compylr-bridge-typescript-golang`,
`compylr-frontend-typescript`, `compylr-host-python`, `compylr-host-typescript`, and one
doc-comment quote from `compylr-ir`. Enumerating `crates/` against that list, **seven of thirteen
crates (~16,000 lines of Rust) were never opened by the audit at all**:

| crate | lines | why it matters here |
| --- | --- | --- |
| `compylr-core` | 4,182 | Holds `bridge.rs` — the `HostBridge` trait and the deferred-C-ABI-hub doc comment that D3 and DECISION.md §2 build their entire argument on. Never independently read for this audit; DECISION.md's citations to it were taken on faith, not verified in-session (I verified them separately below — they hold, but nobody checked before writing DECISION.md). |
| `compylr-backend-rust` | 4,737 | The exact precedent D2, D5, and D6 claim to mirror ("matches what the Rust backend already emits", "mirroring `runtime.rs`", "exactly as the Rust backend decides them"). Never read to confirm those mirrors are accurate. |
| `compylr-bridge-python-rust` | 579 | The **working reference bridge** for the exact (Python, *) direction the new nanobind bridge extends. This is where the M1-marshalling claim ("compylr's boundary is M1 by construction... no pre-marshalled handle to reuse") would be verified against real code, not asserted. |
| `compylr-frontend-python` | 4,507 | The only frontend feeding both working pairs. |
| `compylr-cli` | 965 | — |
| `compylr-diagnostics` | 430 | — |
| `compylr-registry` | 671 | Holds `bridges.rs`, cited directly in design.md §3 as where two new pair entries "join it; nothing about resolution changes" — a claim about existing code, never checked against the actual registry. |

The audit's stated hunting ground (`ts-frontend`, `ts-go-bridge`, `demo-integrity`,
`enforcement-tests`, `python-rust-path`, `spec-vs-reality`, `generated-docs`) is entirely about the
TypeScript/Go pair and the specs. That is a defensible thing to spend an audit on — but DECISION.md
then uses that audit as its evidentiary base for a document about a *third, unrelated* pair
(Python/TypeScript ↔ C++), leaning on claims about `compylr-core`, `compylr-backend-rust`, and
`compylr-bridge-python-rust` that the audit never touched.

**I independently checked the two highest-stakes ones this review had time for:**

- `crates/compylr-core/src/bridge.rs:1-20` — the doc comment DECISION.md and design.md quote
  ("That trade is deferred, not foreclosed") is accurate; not a false claim.
- `crates/compylr-bridge-python-rust/src/bindings.rs:212-213` — instances bind as
  `PyRef<'_, Wrapper>` / `PyRefMut<'_, Wrapper>` per call, confirming DECISION.md's "M1 by
  construction" claim about the existing Python↔Rust boundary. This one holds under direct
  inspection — see `what_holds`.

So the two spot-checks I had budget for both came back true. That is not proof the other five
crates are clean; it is evidence the *specific* claims DECISION.md happens to make about
`compylr-core` are sound, while the broader precedent-mirroring claims about `compylr-backend-rust`
(D2, D5, D6's "matches the Rust backend") were never checked by anyone, including this review.

## 2. D2 rests on a verified-false premise; D5/D6/D7/D8 are safe on reasoning alone

DECISION.md §3 lists only three research legs as "never run" and calls them "lowest value... none
would change a decision already made." It says nothing about whether design.md's own decisions 2,
5, 6, 7, 8 — none of which cite any external research — needed any. I checked each:

**D2 (`std::expected`, nothing throws across a boundary) — the stated reasoning is factually
wrong, and it was checkable.** The decision's "Why" says:

> "nanobind and node-addon-api each translate a returned failure into their host's idiom, and
> **letting a C++ exception unwind into either binding layer is the one thing both forbid**."

I fetched both projects' own documentation:

- nanobind: *"When Python calls a C++ function, that function might raise an exception instead of
  returning a result. In such a case, nanobind will capture the C++ exception and then raise an
  equivalent exception within Python."* It ships a default translation table (`std::exception` →
  `RuntimeError`, `std::out_of_range` → `IndexError`, `std::overflow_error` → `OverflowError`,
  etc.) plus purpose-built types (`nb::index_error`, `nb::key_error`, `nb::value_error`...). This
  is nanobind's **primary, documented, first-class error-reporting mechanism** — not something it
  forbids.
- node-addon-api: exceptions are supported when `NAPI_CPP_EXCEPTIONS` is defined at build time —
  *"the return value will be ignored"* and the C++ exception converts to a JS exception
  automatically. Opt-in, not forbidden.

So the claim attacked — *"letting a C++ exception unwind into either binding layer is the one
thing both forbid"* — is false for both libraries named. D2's other two reasons ("visible in
generated source", "matches the Rust backend's Result-returning style") are independent and still
stand, so I am not asserting the *conclusion* (use `std::expected`) is wrong — only that one-third
of its published justification is a fabricated constraint, and it was never researched despite
being two `WebFetch` calls away. This belongs in the same bucket DECISION.md itself created for the
Node claim in D3 ("my stated fact was false; the conclusion survives") — except this one was never
caught, because nobody looked.

**D5 (unchecked stance, all three guarantees preserved anyway) — safe on reasoning alone.** This is
not a new invention; it is `LanguageBehavior`/`Guarantee` applied to a third backend exactly the way
`compylr-backend-rust` already does it (confirmed by reading `rust.rs:189` and `rust.rs:226`, cited
correctly in design.md). No external fact could overturn an internally-consistent application of an
existing, working mechanism.

**D6 (`compat.hpp` single header, `cpp26-contracts` declared-and-refused) — safe on reasoning
alone**, for the same reason: it mirrors `unchecked-arithmetic` in the Rust backend, an existing
precedent, not a new claim about the world.

**D7 (toolchain preflight moves to the backend) and D8 (shared demo/benchmark machinery, not
copied) — safe on reasoning alone.** Both are internal refactors justified by properties of this
codebase (`_build.py`'s unconditional check; the fixture-list drift `CLAUDE.md` already documents),
not by any claim about an external system that research could confirm or refute.

**Net: of the five, only D2 needed research and didn't get it, and it turns out research would
have caught a real error.** D5–D8 are correctly left unresearched.

## 3. A stale cross-reference that produces a real gap: the "ABI spec" and "D4" that no longer exist

Design.md's Risks section says:

> "**Manual memory management at the boundary is the one place this target can leak or
> double-free, and neither shows up as a wrong answer.** → **D4** makes the allocating side the
> freeing side, and **the ABI spec** requires a handle to be released exactly once... running the
> boundary tier under a **sanitizer** is the cheap check and belongs in tasks."

And the Open Questions section:

> "Both satisfy every scenario **in the ABI spec**..."

Neither referent exists in this document as it stands today:

- **D4**, as written in this same file, is "Flat member names make either binding library
  straightforward" (§4) — nothing about allocation or freeing.
- **No "ABI spec" file exists.** `openspec/changes/add-cpp-backend/specs/` holds
  `build-pipeline`, `cli`, `cpp-backend`, `demo`, `fixture-corpus`, `pipeline-architecture`,
  `python-api`, `semantic-behavior`, `typescript-api` — no ABI spec, and no scenario anywhere in
  those nine files mentions a handle, a release, or an ownership rule (grepped for
  `release|handle|free|own` across every spec — zero hits on the topic).

This is leftover text from the pre-D3 draft, when a shared `compylr-bridge-cpp-abi` crate with its
own C-ABI ownership protocol existed and D3/D4 numbered differently. DECISION.md §5 claims the
revision that deleted that hub ("`cpp-abi-bridge` capability deleted") is "Done, committed,
`openspec validate --strict` passing" — and it is: `openspec validate` checks spec syntax and
scenario structure, not whether prose elsewhere in `design.md` still points at a document the
revision deleted. Nobody re-read the Risks and Open Questions sections after the rewrite.

**The consequence is concrete, not cosmetic.** `tasks.md` has thirteen numbered groups; I grepped
all of them for `sanitiz`, `free`, `leak`, `valgrind`, `asan`, `double-free`, `memory` —
**zero matches.** The plan's own highest-severity, most novel risk (this is the project's first
backend with manual memory management at all) has no task anywhere that runs a sanitizer, tests a
double-free, or asserts a handle is released exactly once. The mitigation design.md promises
("belongs in tasks") was never written into tasks.md, and the reason it's easy to miss is that the
promise itself points at a decision letter and a spec file that no longer exist — there is nothing
to click through to and notice is missing.

## 4. The three research legs this run produced: one finding was directly relevant and got dropped

DECISION.md §3 dismisses `python-native-compilers`, `multi-target-transpilers`, and
`semantics-mismatch` as lowest-value, "none would change a decision already made" — and each
research file's own `changes_a_decision: false` self-assessment agrees. That self-assessment is
correct for the *decisions already made* (D1–D8). It is not the whole story, because
`multi-target-transpilers.md` surfaced something that bears directly on a task this same run wrote,
not on a past decision:

> "py2many's compile-and-run tier... requires a golden expected-output file checked into the repo
> per (case, backend) pair... plus a hardcoded `EXPECTED_COMPILE_FAILURES` allowlist... prefer
> deriving pass/fail from actually running against CPython's answer... to avoid inheriting
> py2many's second N×M surface."

`tasks.md` 4a.3 — the task that builds the new compile-and-run conformance tier this same document
calls "the root" fix (DECISION.md §1: "#42 is the root") — reads: *"Where a corpus entry carries an
expected value, run the compiled output and compare."* I checked what a corpus entry actually is:
`conformance.rs`'s own module doc states the corpus is "authored as IR, not as Python," specifically
*because* a Python-derived corpus can't reach every IR shape. That means there is no CPython
process to run as a live oracle for these entries — an "expected value" for an IR-authored fixture
can only be a hand-written literal baked into the fixture, i.e. exactly the golden-value-per-entry
pattern the research just named as a maintenance-cost trap, now multiplied across N backends
instead of one.

This is not a case where the research should have changed a design decision — it should have
changed one line of task 4a.3 (e.g., "and flag entries whose only oracle is a hand-authored literal
as exactly that, so the corpus doesn't silently reacquire py2many's second N×M surface"). Filing
these three legs as "wouldn't change a decision" is true and also let a directly-applicable,
same-run finding go unconnected to the one task it was relevant to.

## 5. What defect class did nobody hunt for

The audit's own framing (per its findings and DECISION.md §1's summary) is "claims that are not
true" — a binary check of documentation or code against reality. Two adjacent defect classes it
structurally cannot catch, both found here:

1. **Cross-reference rot inside the planning documents themselves** — a decision letter or spec
   file a later paragraph still cites, after an earlier revision renumbered or deleted it (§3
   above). This isn't "a claim that is not true" about the *codebase*; it's a claim about *this
   document's own contents* that stopped being true when the document was edited, and
   `openspec validate --strict` has no opinion on it.
2. **A promised safeguard that never became a task** — design.md's Risks section is written as if
   it is binding ("belongs in tasks"), but nothing checks that every risk mitigation actually
   produced a task-list entry. This is a completeness property of the *plan*, not a correctness
   property of any single file, and it is exactly the kind of gap a lens hunting for false claims
   walks past, because the Risks section's sentence is not, by itself, a false claim — it's an
   unkept promise spread across two files.

Neither is covered by the "correctness / intent / materiality" three-lens method DECISION.md
describes using on its own 24 findings; that method was applied to the audit's findings, not to
DECISION.md's or design.md's own text.

## 6. What a Python/C++ interop specialist would flag as absent

Scoped to what I could verify rather than what merely sounds plausible:

- **No sanitizer/memory-safety task**, covered in full in §3 — the single most obvious gap to
  anyone who has shipped a native extension: the plan introduces manual memory management for the
  first time in this project and ships no ASan/UBSan/valgrind run anywhere in `tasks.md`.
- **Exception-vs-`std::expected` choice for the *bridge glue itself*, not just generated code** is
  underspecified given §2's finding: if nanobind's idiomatic path is exception translation, the
  bridge crate emitting `bindings.cpp` (task 5.2-5.4) needs to translate `std::expected`'s
  `.error()` into a *thrown* `nb::value_error`/`nb::index_error`/etc. at the binding-layer edge
  regardless — task 5.4 says exactly that ("Translate a returned `std::expected` failure into the
  Python exception the source operation would have raised"), so the plan already does the right
  thing here operationally. The gap is narrower than "missing capability": it's that design.md's
  own D2 prose gives a *wrong reason* for a *right task*, which is worse for a future reader than
  either being wrong outright, since the correct task will look justified by a reason that doesn't
  hold.

## `what_holds`

Genuinely tried to break these and could not:

- **"compylr's boundary is M1 by construction"** — confirmed by reading
  `compylr-bridge-python-rust/src/bindings.rs:212-213` directly: every instance parameter binds as
  `PyRef`/`PyRefMut` per call, no marshal-once path exists. The audit never checked this, but the
  claim is true.
- **D5/D6/D7/D8 not needing research** — each is an application of an already-working, in-repo
  precedent (`compylr-backend-rust`'s stance/guarantee split, its `unchecked-arithmetic` declared
  option, its own `runtime.rs` embedding pattern), not a claim about an external system a websearch
  could confirm or refute. No amount of research changes an internally-consistent analogy.
- **`bridge.rs`'s deferred-hub doc comment** — quoted accurately by both design.md and DECISION.md;
  read the source directly at `crates/compylr-core/src/bridge.rs:1-20`, it says what they say it
  says.
- **The overall D3 conclusion (no C-ABI hub, two pairwise bridges)** — survives every attack I
  could construct, including the corrected D2 reasoning: even granting that nanobind and
  node-addon-api both *can* carry exceptions, that has no bearing on the hub-vs-pairwise question,
  which turns on `node:ffi` being experimental/unshipped and on Node-API/nanobind each already
  being the host's first-class C++ binding surface. Nothing in this review threatens that.
