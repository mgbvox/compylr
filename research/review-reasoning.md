# Adversarial review of research/DECISION.md — lens: reasoning

Reviewer: a second agent, attacking the reasoning only (facts granted). Read DECISION.md,
design.md, proposal.md, tasks.md, specs/pipeline-architecture/spec.md, conformance.rs,
research/python-call-overhead.md, research/python-cpp-alternatives.md, research/universal-bindings.md,
git history of design.md (69491a5, e995091, 32d6489), and the audit-*.md evidence files, before
writing anything below.

---

## Finding 1 (fatal) — "#42 is the root" is asserted, not established, and DECISION.md's own §6 contradicts it

**Claim attacked:** "**#42 is the root.** Nearly every other finding is something a corpus that
compiled its output would have caught on the first run. Fix ordering should follow from that, not
from severity labels."

**What I checked:** `conformance.rs`'s own module doc states its scope in plain language:

> "A corpus of IR units every implemented backend must render. Authored as IR, not as Python. That
> is the whole point: a backend's job is to render the IR... `frontends/python/fixtures/accepted/`
> is a good test of the Python *frontend* and a poor test of a backend."

Group 4a (the fix for #42) only extends this corpus — which starts from **hand-authored IR**, never
from source text — to *compile and run the backend's output*. It cannot, by the corpus's own stated
design, exercise anything upstream of the IR: no frontend lowering, no bridge/binding code, no host
package.

I then checked each of the 7 other findings against that boundary:

| # | what it is | layer | would group 4a's fix have caught it? |
|---|---|---|---|
| #37 | TS `/` lowers to integer division | **frontend** (`compylr-frontend-typescript/src/lower.rs`) | **No** — corpus is IR-authored, bypasses the frontend entirely |
| #38 | `demo-ts-go`'s `ir_coverage.ts` is a hardcoded stub; its test never reads a real artifact | demo's own reporting scripts | **No** — unrelated mechanism, not `conformance.rs` |
| #39 | `(typescript, go)` bridge exports 18/75 members, wrong ABI, loader unimportable | **bridge** (`compylr-bridge-typescript-golang`) | **No** — conformance.rs never invokes a bridge crate; only the differential tier (a separate, pre-existing mechanism) touches bridges |
| #40 | checks that cannot fail; `make check` ≠ CI; stale docs | test/tooling hygiene | **Mostly no** — one sub-item (a stale `readme.rs` claim about Go) is adjacent; "`make check` ≠ CI" and stale commands are not |
| #41 | Go backend: five defects vs. its own spec | **backend** | **Plausibly yes** — this is exactly the layer group 4a touches |
| #42 | corpus renders without compiling | backend-check itself | (root, by definition) |
| #43 | four more TS frontend defects | **frontend** | **No**, same reason as #37 |
| #44 | `typescript-api`/`typescript-bindings` specs describe surfaces that don't exist | **host package** (`compylr-host-typescript` is a `version()` stub) | **No** — has nothing to do with corpus compilation |

Of the seven, only **#41** sits in the layer group 4a's fix actually reaches. #39 might eventually
be caught by *fixing the differential tier* too, but that is a different, pre-existing mechanism
that DECISION.md's own §6 puts into a *different* change.

**This is not just my inference — DECISION.md contradicts itself on the same page.** §6, in the
very same document, files #37, #39, #41, #43, #44 into `fix-typescript-go-pair` — described as "not
a small change" needing its own design question settled (#44) — and #40 plus "#42's Go fallout"
into a second, separate change, `harden-the-checks`. If these were genuinely downstream
consequences of #42 that "a corpus that compiled its output would have caught," fixing #42 and
re-running would surface them as ordinary compile errors inside *this* change, not require two
more scoped changes with their own design work. Task 4a.5 says exactly this in its own words:
"Expect failures... File what is new, **do not fix it here**" — i.e., the author already knows
these are not trivially resolved by #42's mechanism, which is the opposite of "the root."

**Correction:** "#42 is the root" holds, at most, for #41. For #37, #38, #39, #43, and #44 — five of
the seven — it is false by the corpus's own documented scope, and DECISION.md's own §6 already
treats them as needing separate remediation, not as automatic fallout.

---

## Finding 2 (major) — the 145×-ctypes number is presented as the decisive argument against a hub, but design.md's actual decision never uses it

**Claim attacked:** "This was the decisive unknown flagged when the hub was first questioned. It is
now answered, and it answers *against* the hub far more strongly than the Node argument did."

**What I checked:** `grep -n "145\|ctypes\|nanobind vs PyO3\|M1"` over `design.md` and `proposal.md`
— zero matches for "145" or "ctypes" in either file. D3's actual "Why" and "Alternatives considered"
sections reject the shared-hub design on **exactly one** ground: the Node fact (`node:ffi` is
experimental, flag-gated, self-described unsafe, newer than this project's own Node) plus the
observation that once Python→C++ already goes through nanobind, "one mechanism everywhere" is
already gone. The ctypes/PyO3 M1 number that DECISION.md calls the stronger of the two arguments
does not appear anywhere in the document it is supposedly the basis for.

This matters for reasoning-soundness in two ways:

1. **Internal inconsistency.** DECISION.md's §5 claims "D3 rewritten" and "D3 corrected again" as
   revisions already applied — implying the design document reflects the research. It does not
   reflect this particular piece of research at all. Either the number should be in D3 and isn't
   (an incomplete revision), or DECISION.md is retroactively crediting the decision with an
   argument the decision was not actually made on — which is close to the "motivated reasoning" the
   prompt asked me to test for.

2. **The number may not even transfer to the live alternative.** DECISION.md's own §3 admits: "The
   145x number is ctypes-vs-PyO3, not nanobind-vs-PyO3." I went one step further and checked
   `research/python-cpp-alternatives.md`, already in this repo: nanobind vs. pybind11 — a fellow
   *compiled, static-binding* tool, the same category PyO3 is in — differs by **2.7–4.4× on compile
   time, 3–5× on binary size, ~3–10× on call/class overhead**. Nowhere near two orders of magnitude.
   The 145× figure is specific to `ctypes`/`libffi`'s *dynamic* dispatch mechanism (per the paper's
   own words, quoted in `research/python-call-overhead.md`: "ctypes has shown the most lacking
   alternative... due to `libffi`"). A hub built from a nanobind-style compiled dispatcher — as
   opposed to the ctypes loader the original draft actually specified (confirmed in git history,
   `69491a5:design.md:136`, `emit_ctypes_loader`) — was never evaluated, and the one comparable data
   point available (nanobind vs. pybind11) suggests it would not pay anything like the ctypes tax.

**What holds:** the original draft's specific proposal — a ctypes-shaped Python loader over a shared
`extern "C"` surface — is fairly rejected by the M1/145× numbers; that draft is confirmed by git
history to have used `emit_ctypes_loader`, so the number does apply to *what was actually on the
table at the time*. What does not hold is generalizing that into "the C-ABI hub" as a category, and
calling it more decisive than the Node argument, when (a) the design document that supposedly
incorporates this conclusion never cites it, and (b) the one measured data point on a
non-ctypes-shaped compiled hub points the other way.

**Correction:** narrow the claim to "a ctypes-shaped hub is ruled out by the M1 numbers" and drop
"far more strongly than the Node argument" — on the evidence actually in `design.md`, the Node
argument is the *only* one doing the work, and it is sound on its own; the ctypes number is
decorative in the artifact that matters.

---

## Finding 3 (minor-to-major) — "C++ is the target that needs a hub least" is a defensible narrow claim dressed as a superlative

**Claim attacked:** "The inversion is the interesting part: **C++ is the target that needs a hub
least**, because both hosts' first-class binding libraries are already C++ header libraries."

**What holds:** read narrowly — against the *specific* case that originally motivated wanting a hub
for C++ (proposal.md's Context: "an earlier draft... argued... C++ is where the hub could finally be
cashed in, because its idiomatic export surface already is a C ABI and both hosts could consume one
with what they ship") — the argument is coherent and I could not break it: that premise's second
half (Node can consume a C ABI with what it ships) is verifiably false, and once nanobind is already
required for the Python side, the marginal cost of also writing a node-addon-api bridge instead of a
hub loader is genuinely low, because both are idiomatic, first-class, already-existing tooling
rather than hand-rolled FFI glue.

**What doesn't fully hold:** the word "least" is a superlative over *all* targets, and no comparison
to Rust or Go is ever made — the document doesn't (and structurally can't, since Rust and Go already
have their own pairwise bridges, PyO3 and cgo, neither of which was ever a hub candidate) establish
a ranking. The claim is really "C++ needs a hub less than the *reason someone originally wanted one
for C++* implied," not "C++, compared against other targets, needs one least." That's a much
narrower and more defensible claim than the sentence as written asserts. This is a rhetorical
overclaim, not a broken inference — flagging it as minor rather than major.

---

## Finding 4 (major) — the plan does not reconcile "the check SHALL fail" with "expect failures, don't fix them here"

**Claim attacked (tasks.md, group 4a):** "4a.5 Run it against the existing Go backend and record
what it finds. Expect failures... File what is new, do not fix it here" followed immediately by
"4a.6 `cargo test --workspace`; commit."

**What I checked:** the requirement this group implements
(`specs/pipeline-architecture/spec.md`, "A backend's conformance output SHALL be compiled and run")
states unconditionally: "A backend whose emitted source for a corpus entry does not compile SHALL
fail the check" — with no carve-out for a backend with known, filed defects. I grepped the whole
`crates/` tree for any existing xfail/known-failure convention (`#[ignore]` used as "known broken",
`xfail`, `KNOWN_BROKEN`) and found none — this project has no established idiom for "this test is
allowed to fail, we know."

Given #39's own audit evidence (`research/audit-ts-go-bridge.md`) — the TS→Go loader "cannot be
loaded by Node at all," and #41's five confirmed spec violations — running 4a's new compile-and-run
check against the real, already-registered Go backend should, by the spec's own words, **fail
`cargo test --workspace`**, which every checkpoint in `tasks.md` (and `CLAUDE.md`'s house rule)
treats as the commit gate. Nothing in `tasks.md`, `design.md`, or the spec says how 4a.6 is supposed
to pass once 4a.5 has, by design, surfaced real, unfixed Go failures. This isn't a subtle inference:
it's an unaddressed gap between two adjacent numbered tasks, and it bears directly on "neither
[follow-up change] should block `add-cpp-backend`" — if the new check turns the whole workspace red
the moment it's built, *this* change is blocked on deciding how to quarantine a known-broken Go
backend, which is exactly the design question §6 defers to `harden-the-checks`.

**Correction:** either tasks.md needs an explicit step (e.g., "mark the Go entries `#[ignore]` with
a comment naming #39/#41, tracked by `harden-the-checks`") or the pipeline-architecture requirement
needs a stated exception for a backend with filed, tracked non-conformance. As written, the two
documents describe an impossible checkpoint.

---

## Finding 5 (minor) — proposal.md still carries the GCC floor DECISION.md and design.md say was corrected

**Claim attacked (DECISION.md):** "The floor was wrong, the strategy was right... GCC accepts
`-std=c++26` from **14**, not 15" — and §5: "**D1 corrected** — GCC 14 floor, Clang's `c++2c`
spelling, the contracts/reflection matrix."

**What I checked:** `design.md` D1 does say GCC 14. `proposal.md`'s own Impact section, in the same
change, still reads: "building generated C++ needs a C++26 compiler (**GCC 15+** or Clang 20+)."
`grep -n -i "gcc" proposal.md` returns exactly this one, uncorrected occurrence. So within the
change bundle itself, `design.md` and `proposal.md` disagree with each other on the same fact
DECISION.md claims was fixed. This is a small, mechanical thing — but it is precisely the kind of
drift `tests/readme.rs` and the generated-docs scripts exist to catch elsewhere in this repo, and it
means DECISION.md's "revisions applied" list is not fully applied.

---

## Places design.md and DECISION.md disagree (direct list, per the assignment)

1. **The hub rejection's decisive argument.** DECISION.md ranks the ctypes/145× finding above the
   Node argument ("far more strongly than the Node argument did"); design.md's D3 rejects the hub
   using only the Node argument and never mentions the 145× figure, ctypes, or M1/M2 at all.
   (Finding 2.)
2. **Whether the nanobind-vs-PyO3 gap is "the decisive unknown."** DECISION.md's §3 treats it as
   the single open question that would validate or overturn the hub call. But design.md's actual D3
   doesn't depend on any PyO3/nanobind performance comparison — its argument is architectural
   (Node's FFI story, and the "one mechanism" argument), not a benchmark claim — so the "decisive
   unknown" DECISION.md names is decisive for a justification design.md doesn't make.
3. **GCC floor consistency inside the same change.** Not a design.md/DECISION.md disagreement
   directly, but DECISION.md asserts the correction is "applied," and design.md and proposal.md (both
   part of the same change DECISION.md is reporting on) still disagree with each other. (Finding 5.)
4. **No disagreement found** on: nanobind-over-pybind11 numbers (DECISION.md's figures match
   `research/python-cpp-alternatives.md` and are consistent with design.md's qualitative claims);
   the `node:ffi` correction itself (both documents state the same corrected fact); D4/D5/D6/D7/D8
   (DECISION.md doesn't discuss these decisions at all, so there is nothing to disagree with —
   worth noting only because it means DECISION.md's "review" of the design is partial, covering D1
   and D3 and the checks, silent on D2 and D4–D8).

## What holds

- **"compylr's boundary is M1 by construction"** — verified directly against `CLAUDE.md` ("Every
  argument crosses by value, text and collections alike... a body doing O(log n) work over an O(n)
  argument can therefore lose compiled") and against the absence of any handle-reuse mechanism in
  `bridge.rs`/`HostArtifact`. This is solid and I could not break it.
- **The Node `node:ffi` argument itself** (experimental, flag-gated, unsafe-by-its-own-docs, newer
  than the project's own Node version) — checked against design.md's D3 and found internally
  consistent and sufficient on its own to reject a hub, independent of the ctypes number. This is
  the argument actually carrying D3, and it holds.
- **The narrow reading of "C++ needs a hub least"** (Finding 3) — against the specific motivating
  premise it rebuts, this holds; only the superlative framing overreaches.
- **The GCC-14/Clang-`c++2c`/no-contracts-on-Clang table in D1** — I did not have live web access to
  re-verify compiler version claims (WebSearch exhausted, cppreference blocked per the task's own
  constraints), so this is graded on internal consistency only, where it is consistent within
  design.md itself.
