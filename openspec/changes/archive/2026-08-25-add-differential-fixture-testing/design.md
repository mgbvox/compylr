## Context

See `proposal.md` — Why. The constraints that shape the approach are all existing facts about this
repository:

* `crates/compylr-host-python/tests/execution.rs` already emits a whole crate into
  `$CARGO_TARGET_TMPDIR`, writes an extra `src/main.rs` around it, shells `rustc`, runs the binary,
  and asserts on stdout. The translation tier is that harness pointed at the corpus.
* `python/tests/test_end_to_end.py` already builds through maturin behind
  `pytestmark = [pytest.mark.slow, needs_toolchain]`. The boundary tier is that harness pointed at
  the corpus.
* `COMPYLR_DISABLE=1` returns every marked member untouched **without validating it**, so an
  interpreted run is interpreted all the way down — including calls between marked members, which
  reach each other through module globals.
* `python/fixtures/accepted/` and `rejected/` are enumerated from the directory by `fixtures.rs` and
  `emit_quality.rs`, deliberately: hardcoded lists drifted once and hid a real defect. They are also
  excluded from `ruff`, deliberately: `ruff check --fix` once deleted the `import os` a rejection
  fixture existed to test.
* The subset promises **no mapping or set iteration order**, and generated maps use `FastHasher`.
  Any comparison that sees a mapping as an ordered thing would be flaky rather than correct.
* `scripts/update_benchmarks.py` owns a `Region` abstraction over `<!-- name -->` /
  `<!-- /name -->` markers, with a `--check` mode that verifies addressability without measuring.

## Goals / Non-Goals

**Goals**

* One statement of what calls exercise a fixture, consumed by both tiers.
* A comparison that cannot be flaky: no reliance on mapping or set order, and no float equality.
* A robustness property established against Python nobody wrote for this compiler.
* Documentation of the subset that is counted rather than remembered.

**Non-Goals**

* Any change to compiler behavior. If this change alters an answer, it is wrong.
* Performance measurement. The demo owns that; this change owns correctness.
* Growing the accepted subset. Fixtures are added only where an existing accepted construct has no
  driver coverage.
* A threshold on the acceptance rate over the robustness corpus. That corpus varies with the
  installed Python; a threshold would make the suite fail for reasons unrelated to the compiler.

## Decisions

### D1. A driver is a call manifest plus a shared runner, not free Python

**Decision.** A driver declares its calls as literal data — member name, arguments, and for a class
its constructor arguments and the sequence of method calls — and a single shared runner turns that
declaration into calls and a transcript. Drivers live in `python/fixtures/drivers/<name>.py`.

**Why.** The two tiers do not share a language. The boundary tier calls compiled members from
Python; the translation tier calls generated Rust from a `main` the harness writes. Free Python
would be consumable only by the first, so the second would need its calls written a second time in
Rust, and the two statements would drift the first time someone edited one. Literal data is
readable by both — Python imports it, and the Rust harness reads it with `ast.literal_eval` via one
`python3` invocation, or from a JSON file the Python side emits.

**Alternatives considered.**

* *Add `main()` to each fixture.* Rejected: fixtures are minimal subset specimens and would stop
  being that, `if __name__ == "__main__"` is itself a construct the subset rejects, and the
  fixtures are excluded from `ruff` for a reason that would no longer hold.
* *One TOML table for the whole corpus.* Rejected on legibility: when a case fails you want to open
  one file named after it, not scroll a shared table.
* *Free Python drivers, and derive the Rust main by parsing them.* Rejected: parsing arbitrary
  Python to recover calls is the compiler's job, and using the compiler to test the compiler is how
  a bug hides itself.

### D2. The two tiers compare different things, on purpose

**Decision.** The **boundary tier** compares *values* in Python. The **translation tier** compares
*canonical transcripts* as text.

**Why.** The boundary tier already has both answers as Python objects, so it can compare with `==`
— which makes mappings and sets compare by content rather than by iteration order, and lets floats
compare within a tolerance. Converting them to text first would invent an ordering problem the
comparison does not have.

The translation tier has no Python object on the Rust side, so a text transcript is the only shared
form. That transcript is defined below and is not `Debug` output.

### D3. The canonical transcript is JSON with sorted keys

**Decision.** The translation tier renders each result as JSON: mappings with keys sorted, sets as
sorted arrays, tuples as arrays, booleans as `true`/`false`, and floats with a single fixed
representation. The Python side produces the expected transcript with `json.dumps(sort_keys=True)`
and the same float rule; the Rust harness renders the same shape from the function's declared return
`Ty`, which it reads off the IR.

**Why.** Sorting is what makes an unordered container comparable without asserting an order the
language does not promise — the alternative, asserting on iteration order, is exactly the flakiness
`CLAUDE.md` warns about. JSON also settles the three spelling differences that would otherwise be
noise: `True` against `true`, `'a'` against `"a"`, and Python's `repr` of a float against Rust's
`Display`.

**Cost, stated plainly.** The Rust harness needs a renderer over `Ty` — roughly two hundred lines of
test code. That is the price of the fast tier, and it is paid once.

**Alternative considered.** *Have the translation tier compare against values too, by round-tripping
through the bridge.* That is the boundary tier. It would collapse the two tiers into one and lose
the fast one.

### D4. Floats are compared with a tolerance, and the tolerance is named

**Decision.** Both tiers compare floats within a relative tolerance rather than exactly, and the
tolerance is a single named constant that both sides read.

**Why.** `demo/src/algorithms/__main__.py` already made this call and recorded the reason: the
compiled and interpreted paths can differ in the last bit, "usually identical" is not something to
assert on, and a demo that fails on the last bit of a standard deviation is a demo nobody trusts.
The same reasoning applies here, and the two places should agree.

### D5. The robustness corpus is located, not vendored

**Decision.** The corpus is this repository's own `python/compylr/`, `demo/src/`, and `scripts/`,
plus the standard library of whichever interpreter is on the machine, located by asking it
(`sysconfig.get_paths()["stdlib"]`). Files that do not parse on the running version are skipped as
parse failures, which is a fact about the interpreter and not about lowering.

**Why.** Vendoring third-party Python into the repository to test the compiler would be a
maintenance burden with no upside; the property being established — no panic, every rejection
located — does not depend on *which* Python, only on it being Python nobody wrote for us.

**Consequence.** The corpus differs between machines, so the check asserts a property and reports a
rate. It never asserts the rate, and D-N1 below records why.

### D6. The subset matrix is a sibling script, sharing the region machinery

**Decision.** Extract `Region`, `find_region`, and `replace_region` from
`scripts/update_benchmarks.py` into a shared module, and add `scripts/update_subset.py` alongside
it. Both grow a `--check` mode; the Makefile, the pre-commit hooks, and the CI workflows run both.

**Why.** They are different jobs with the same output mechanism. Benchmarks measure and take
minutes; the subset matrix counts and takes milliseconds. Folding the second into the first would
make a documentation check depend on a benchmark run.

**What the matrix reports.** Per construct in the accepted subset: the fixture that exercises it,
and that the fixture agreed with CPython. A construct with no passing fixture does not appear —
which is the property worth having, and is exactly how `py2many`'s `LANGUAGES.md` stays honest.

### D7. Coverage of the corpus is checked, not assumed

**Decision.** `fixtures.rs` gains two checks: every accepted fixture has exactly one driver, and
every member a fixture defines is called by that driver. It already owns the completeness guard over
`rejected/`; the inverted guard joins it there.

**Why.** The corpus's value is entirely in its coverage, and coverage that is not checked is
coverage that decays. This repository already learned that twice — the fixture lists that drifted,
and `conformance.rs`, whose `(form, position)` check found four defects on its first run.

## Risks / Trade-offs

**`make check` gets slower** → The boundary tier builds an extension. Mitigation: it builds **one**
extension for the whole corpus, not one per fixture — every accepted fixture becomes a member of a
single unit, which is also how a real project is built. Measure it as part of the work and report
the number; if it is intolerable, it becomes its own Makefile target rather than being dropped, and
the translation tier still covers the corpus on every `cargo test`.

**Cross-source name collisions** → Building the whole accepted corpus as one unit means every
fixture's members share a namespace, and `Unit::add_function` refuses a duplicate. Mitigation: the
existing `unit_from_fixtures` grouping in `emit_quality.rs` already solves this and is the model;
collisions are resolved by renaming in the fixture, which is free.

**The Rust-side JSON renderer disagrees with Python's** → A renderer written twice is a renderer
written wrong. Mitigation: a small table-driven test over each `Ty`, asserting the two renderings of
the same value match — the same shape as the existing test that keeps `runtime.rs`'s mirrored
`IndexOrigin` in step with the IR's.

**Drivers become busywork** → Every new fixture owes one. Mitigation: that is the intended tax, and
the runner is shared, so a driver is a literal list. The check that names a fixture without a driver
makes the omission loud rather than letting the corpus quietly stop proving things.

**A fixture's driver is trivially satisfiable** → A driver that calls each member once with `0`
technically passes D7 while proving little. Mitigation: not a mechanical check — it is a review
concern, and the tasks call for driver inputs that include the boundary values each fixture's
constructs actually distinguish (negative operands for division and indexing, an empty collection, a
non-ASCII string for length).

**The robustness corpus finds a panic and blocks the change** → Likely, and welcome. Mitigation: a
panic found this way is a defect in lowering, fixed in this change; if one turns out to be large,
the corpus is narrowed with the specific module named and a follow-up recorded, rather than the
check being weakened.

## Migration Plan

Nothing to migrate. No artifact format moves, no cache is invalidated, no user-visible surface
changes. The work is additive and can land incrementally: the translation tier is useful before the
boundary tier exists, and the robustness walk is independent of both.

Rollback is deleting the new test files.

## Open Questions

* **Which standard-library modules to include in the robustness corpus.** The whole of it is a few
  hundred files and seconds of parsing, so "all of them" is the starting answer; if a subset turns
  out to be needed for runtime, choosing it does not change the specs, the approach, or the tasks.
* **Whether the subset matrix belongs in `README.md`, `demo/README.md`, or both.** A presentation
  question, answerable when the first generated table exists and can be looked at.
