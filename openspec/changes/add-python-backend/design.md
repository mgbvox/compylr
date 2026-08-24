## Context

See `proposal.md` — Why. The facts that shape the approach:

* `tests/conformance.rs` enumerates backends from the registry and already asserts every implemented
  one renders the corpus and that the result compiles. It needs no change to cover a second backend
  — only a second backend.
* The conformance corpus is **authored as IR, not as Python**, precisely so it contains trees Python
  cannot express. That is what makes a Python backend a real test rather than an identity function.
* `PYTHON_BEHAVIOR` lives in `crates/compylr-frontend-python/src/component.rs`, and
  `Ty::python_name` / `BinOp::python_symbol` live in that crate's `spelling.rs` as extension traits.
  `compylr-frontend-python` is the only crate permitted to depend on a Python parser, and
  `crate_boundaries.rs::only_the_python_frontend_depends_on_a_python_parser` enforces it — so a
  Python backend cannot reach either of them through that crate.
* `crate_boundaries.rs::a_stance_declaration_names_only_its_own_language` is driven by a hardcoded
  table of `(crate, own language, foreign languages)`. It grows an entry per crate that declares a
  stance.
* `crates/compylr-cli/src/main.rs` resolves the target-code form through
  `compylr_backend_rust::rust::GENERATED_PATH` — the CLI knows one backend by name.
* `Emit::Crate` already looks up a bridge and reports `BridgeError::Unbridged` when there is none.
  `(python, python)` needs no handling; it gets the existing answer.
* `add-differential-fixture-testing` has landed, so `python/fixtures/drivers/` exists and every
  accepted fixture is driven. This change reuses those drivers rather than inventing an oracle.

## Goals / Non-Goals

**Goals**

* A second real consumer of the IR, so target-neutrality is tested and not merely enforced.
* Emitted Python readable enough that it answers "what did compylr understand?".
* An oracle for translation that needs an interpreter and nothing else.

**Non-Goals**

* A `(python, python)` bridge. Nothing calls this from anywhere; it is a translation, not a runtime.
* Optimizing the emitted Python, or making it fast. It is for reading and for checking.
* Emitting Python that a person would have written. It emits Python that *means* what the unit
  declares, which is sometimes not what a person would write — that difference is the finding.
* Moving Rust's stance declaration out of the Rust backend. D2 records the trigger for that.
* Growing the accepted subset in any direction.

## Decisions

### D1. A language crate: `compylr-lang-python`

**Decision.** Python's declared semantics (`PYTHON_BEHAVIOR`) and Python's spellings (`Ty` and
`BinOp` extension traits) move into a new `compylr-lang-python`, depending on `compylr-ir` and
nothing else. `compylr-frontend-python` and `compylr-backend-python` both depend on it.

**Why.** Four placements, three of them blocked by rules this repository already holds:

| Placement | Verdict |
| --- | --- |
| Leave in `compylr-frontend-python` | The backend would depend on it and therefore on ruff. `only_the_python_frontend_depends_on_a_python_parser` fails. |
| Duplicate in the backend | Two declarations of what Python means, free to drift, with nothing able to detect the disagreement. This is the failure mode the whole behavior model exists to prevent. |
| `compylr-core` | Core names no concrete language; `behavior_resolution_names_no_concrete_language` fails. |
| `compylr-ir` | The IR names no language at all; `the_ir_source_names_no_python_syntax` fails. |
| A language crate | Nothing objects, and it says something true. |

**What it says.** *A language's declared semantics belong to the language, not to the direction you
are travelling through it.* compylr already states half of this — "a stance declaration names only
its own language" — and this makes the other half structural. The comment in `CLAUDE.md` that
spellings "belong to the frontend that read it" was true when only a frontend existed; the
underlying reason was always that they belong to *Python*, and reading was the only thing anyone
did with Python.

**Trigger for the symmetric move.** `RUST_BEHAVIOR` stays in `compylr-backend-rust` because Rust has
exactly one component and therefore no second reader to disagree with. It moves to
`compylr-lang-rust` the moment anything else needs to read what Rust means — a Rust frontend, or a
bridge that must know. Moving it now would be symmetry with no second reader, which is architecture
for its own sake.

**`crate_boundaries.rs` changes.** `compylr-lang-python` joins the stance table with
`("compylr-lang-python", "python", ["rust", "typescript"])`, and a new assertion states that a
language crate depends on the IR alone — so a language crate cannot quietly become a place to put
things.

### D2. The backend reads modes and emits helpers, mirroring the Rust backend

**Decision.** Emission produces `generated.py` (the translated functions), `compat.py` (a runtime of
mode helpers), and `__init__.py`. Where a node's declared mode is Python's own, the operator is
emitted directly; where it is not, a `compat.py` helper is called.

**Why.** This is the same split as `generated.rs` / `compat.rs`, for the same stated reason: the
translated file should open on the user's code. It also keeps emission a pure function — `compat.py`
is a constant, embedded the way `runtime.rs` is embedded.

**The consequence to state plainly.** A unit declaring only Python's modes emits no helper call and
no import, so it round-trips through the frontend. A unit declaring a mode that is not Python's
emits `from .compat import ...`, and imports are outside the accepted subset — so it does **not**
round-trip. That is correct and is written into the spec as a named case rather than discovered
later. The interesting artifact falls out of it: a program compiled under `behavior="rust"`, emitted
as Python, is a readable Python program that does what Rust's arithmetic does.

**Alternative considered.** *Inline every mode as an expression, avoiding helpers and preserving
round-trip for everything.* Rejected: truncating division inlined is
`-(-a // b) if (a < 0) != (b < 0) else a // b` at every site, which makes `generated.py` unreadable
— defeating reason 3 in the proposal — and duplicates a definition the Rust backend keeps in one
place.

### D3. Validity is checked by parsing, and round-trip by fingerprint

**Decision.** `conformance.rs` gains a Python sibling to
`every_corpus_entry_compiles_for_the_rust_backend`: the emitted source is handed to an interpreter
to parse. Separately, for every unit lowered from Python whose modes are all Python's own, the
emitted source is lowered again and the two units' **fingerprints** are compared.

**Why fingerprints rather than text.** Comparing emitted text to the input would assert that the
backend reproduces formatting, which is not the property. Comparing fingerprints asserts the
property that matters — the same program — and `Unit::fingerprint` already excludes spans and
docstrings, so it is exactly the right equality.

**Why this is not circular.** The round-trip runs the frontend over the backend's output, so a
matched pair of bugs could cancel. That is why it is only one of three checks: the corpus is
authored as IR (so it contains modes no Python-originated round trip can reach), the emitted source
is parsed by CPython rather than by compylr, and the driver oracle in D4 compares *answers* rather
than structure.

### D4. The oracle is the drivers from `add-differential-fixture-testing`

**Decision.** For every accepted fixture, run its driver against the fixture under CPython and
against the **emitted Python** under CPython, and require the transcripts to match.

**Why.** It is the same comparison the translation and boundary tiers make, with a third
translation, and it needs no toolchain at all — so it runs where the other two cannot. It also
means the Python backend is checked against answers on its first day, which is the standard the
Rust backend took three changes to reach.

### D5. Identifier escaping is the backend's, not inherited

**Decision.** `compylr-backend-python` gets its own identifier escaping over Python's keywords,
independent of the Rust backend's `rust_ident`.

**Why.** A unit is not necessarily Python-shaped. A TypeScript frontend could produce a member named
`lambda` or `pass`, which is a valid name there and a syntax error here. The Rust backend already
had to learn this; the answer does not transfer, because the keyword sets differ.

### D6. `Backend` declares its translated-source file

**Decision.** The `Backend` trait gains a method naming which emitted file holds the translation,
with **no default implementation**. `compylr-backend-rust` returns `GENERATED_PATH`; the CLI asks
the backend instead of naming a crate.

**Why no default.** A default would be a guess about a backend's file layout, and the one thing a
default could return — "the only file" — is wrong for both backends that exist. Every backend
answering is one line and cannot be forgotten.

**Note.** This is a latent defect being fixed, not a new requirement: the `cli` spec already says
the target-code form prints "the translated functions **for the selected backend**", which the
current implementation cannot do for a backend it does not name.

## Risks / Trade-offs

**The backend becomes an identity function and proves nothing** → It cannot, for the corpus: that
corpus is authored as IR and contains mode combinations Python has no syntax for, so those entries
have no input text to echo. Mitigation is to keep the conformance corpus, not the fixtures, as the
primary evidence — and `the_corpus_covers_both_stances_of_every_axis_in_every_position` already
guarantees the coverage that makes this true.

**Round-trip cancels a matched pair of bugs** → Addressed by D3's three independent checks; the
driver oracle in D4 is the one that cannot cancel, because CPython evaluates both sides.

**Moving `spelling.rs` churns every diagnostic** → It is a pure move; no message text changes.
Mitigation: make the move its own commit with no other edit in it, so `cargo test` before and after
is a clean comparison, and the existing diagnostic tests are the check.

**Every future IR form now costs two backends** → Intended, and cheaper now than later. Mitigation:
none needed; `conformance.rs` is what will say so, immediately, in the change that adds the form.

**Scope creep into a Python-to-Python tool** → A non-goal above. The backend emits and stops; it
does not format beyond `ruff format` as `post_process`, does not optimize, and gains no flags.

**`(python, python)` looks like a hole to fill** → It is not, and the spec says so: asking for a
callable artifact reports the pair unbridged, which is the fourth answer the architecture already
models. Anyone reaching to add a bridge should be sent to `crates/compylr-core/src/bridge.rs`.

## Migration Plan

Additive. No artifact format moves, no fingerprint changes, no cache is invalidated, and the default
backend stays `rust`, so no existing project behaves differently.

The one structural move — `PYTHON_BEHAVIOR` and `spelling.rs` into `compylr-lang-python` — is a
re-export away from being invisible, and is deliberately *not* re-exported: leaving the old paths
working would leave two ways to reach one declaration, which is the ambiguity the move exists to
remove.

Rollback is deleting two crates and one registry entry.

## Open Questions

* **Whether `compat.py` should be embedded as a constant or generated per unit.** The Rust backend
  embeds `runtime.rs` verbatim and that is the starting answer; if the file grows large enough that
  emitting only the helpers a unit uses becomes worthwhile, that is a later optimization which
  changes neither the specs nor the task breakdown.
* **How emitted Python spells a tuple type of length one.** `tuple[int]` is unambiguous in an
  annotation; the question is only whether the emitted *value* needs the trailing comma in every
  position. Answerable while writing the first emitter, and visible immediately if wrong.
