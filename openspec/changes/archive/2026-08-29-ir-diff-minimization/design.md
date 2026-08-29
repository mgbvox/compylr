## Context

Python and TypeScript already lower into the same IR, and Rust and Go already emit from it. What is
missing is evidence that the middle is actually shared: that the two frontends produce the same
shape for the same program, and that the shape does not depend on the target. This change builds the
measurement and the ratchet, not the alignment work the measurement will motivate.

The original framing was to minimize `D(I_xab, I_xac)` and `D(I_xab, I_xde)`. The first is not a
minimization problem — a frontend is defined to be unaware of the target, so any nonzero value is a
defect — and it is specified here as an invariant. The cross-language `D(I_xab, I_xde)` is the
quantity this change tracks.

## Goals / Non-Goals

**Goals:**
- A normalized comparison form that removes orderings carrying no meaning, without altering what is
  compiled.
- A divergence score that disregards resolved semantic modes, spans, and documentation, and reports
  where the remaining divergence is.
- A recorded per-pair baseline that CI refuses to let rise.

**Non-Goals:**
- Zero divergence. Where the languages genuinely disagree on a construct, the honest outcome is a
  recorded nonzero score, not a frontend contorted into emitting a shape that does not mean what its
  source meant.
- Changing the accepted subsets or the default resolved behavior. Those are the levers for lowering
  `D`; this change only makes it visible enough to argue about them with numbers.
- Any change to emitted code. If a build's output moves, this change is wrong.

## Decisions

### 1. Normalization is a comparison-time view, not a pass

**Decision:** Normalization lives with the differ, over in the language-neutral middle end, and is
applied to a copy on the way into a comparison. It is not registered in the pass pipeline and the
`ir-optimization` capability is untouched.

**Why:** Two independent reasons, either sufficient. First, a pass runs on the way to a backend, so
putting it there would change the code every user compiles in order to serve a metric. Second,
`ir-optimization` already requires that *a pass preserves observable behavior*, and reordering
commutative operands does not: `Expr::Binary` takes arbitrary operands, so `f() + g()` is
representable and swapping it changes evaluation order. As a comparison-time view neither problem
arises.

**Alternatives considered:** A real `Canonicalize` pass in the pipeline, with the reordering
restricted to side-effect-free operands. Rejected: the restriction would be needed anyway (see
below), and the pass would still be rewriting user programs for the benefit of a measurement.

### 2. The side-effect restriction survives the move

**Decision:** Even at comparison time, operand order is normalized only where both operands are free
of side effects.

**Why:** Here the reason is honesty rather than correctness. Python `f() + g()` and TypeScript
`g() + f()` are different programs; normalizing them together would report agreement that is not
there, and the whole value of `D` is that a zero means something.

### 3. What `D` disregards, and why those three

**Decision:** `D` is a structural comparison over normalized units that ignores resolved semantic
modes, source spans, and documentation.

**Why:** The modes are the point of the IR — Python and TypeScript *must* differ on overflow,
division rounding, remainder sign, index origin, and text length units, because each preserves what
its source meant, and a differ that counted those would be measuring the languages rather than the
frontends. Spans are positions in two different files and would make every pair diverge in every
node, so a differ that counted them would report a large constant and never reach zero. Documentation
carries no runtime meaning. Note that `Function::fingerprint` already excludes spans and docstrings
for the same reason; the differ states the same exclusions rather than inventing its own.

**Alternatives considered:** A JSON or text diff of the serialized IR. Rejected on all three counts
at once — it would count modes, spans, and prose as divergence, and its score would be dominated by
noise.

### 4. A recorded baseline, not a threshold

**Decision:** Every pair's score is recorded in a generated file, `divergence.recorded`, beside the
test that produces it. The test recomputes and requires the file to match exactly; regenerating is
`UPDATE_DIVERGENCE=1` on the same test.

**Why:** This is the "track progress and repeat" half of the request, and a bare constant would not
provide it — a single threshold says nothing about which pair got worse, and the number would be
invented rather than measured. Comparing the whole file rather than parsing it covers every rule
at once: a score that rises, one that falls without being recorded, a value edited by hand, and a
pair quietly dropped from a corpus.

**Alternatives considered:** A generated `README.md` block written by a `scripts/update_divergence.py`
sharing `scripts/_regions.py`, matching how the benchmark and subset tables work. Rejected because
the numbers are produced in Rust and consumed in Rust: routing them through a Python script would
need a new binary to exist only so the script had something to read, and would publish the table in
the project's front door. The check is `cargo test`, which the Makefile and CI already run.

Also rejected: asserting `D == 0` on every pair. The recorded baseline happens to *be* zero today,
but fixing that in the test would mean the first genuine divergence lands as a red build with no
recorded history rather than as a number that moved.

### 5. Where the cross-language test lives

**Decision:** In `compylr-registry`.

**Why:** Measuring a pair needs both frontends in one process. The existing corpus tests live in
`compylr-host-python`, which does not depend on `compylr-frontend-typescript` — and adding that edge
is precisely what `crate_boundaries.rs` exists to refuse. `compylr-registry` is documented as the one
crate allowed to know every frontend, backend, and bridge at once, so a test needing two frontends
belongs there and nowhere else.

### 6. Pairing is by name

**Decision:** A pair is two accepted **members** with the same name in two frontends' corpora.

**Why:** Pairing by *file* was the original decision and measurement refused it. The five shared
stems — `arithmetic`, `branching`, `classes`, `collections`, `loops` — share filenames without
sharing programs: `arithmetic.py` defines six members and `arithmetic.ts` defined two, of which one
was not in the Python file at all, and `classes`, `collections`, and `loops` overlapped in nothing.
A file-level baseline would have recorded 636, of which 636 was one corpus not defining what the
other did and 0 was the compiler. Pairing by member measures what both sides actually express.

A metadata file mapping the pairs would be a second statement of a fact the member names already
carry, free to drift from them — which is why the corpora carry a header saying that a TypeScript
member is named for its Python counterpart deliberately, and that renaming it to camelCase would
drop the pair rather than tidy it.

## Risks / Trade-offs

- **Normalization masks a real difference.** → Restricted to orderings that carry no meaning, and
  refused entirely where an operand can have an effect.
- **The ratchet becomes something people route around.** Regenerating the table is how a score goes
  down, and it is also how someone hides a score going up. → The check recomputes rather than trusts,
  so a hand-edited table fails; a regenerated one shows up as a diff in review.
- **Chasing `D` distorts a frontend.** Forcing a shape that does not mean what the source meant would
  lower the score and break the program. → The corpus oracles stay authoritative: a change that
  lowers `D` while any fixture stops agreeing with its oracle is a regression.
