## Why

compylr's central architectural claim is that the IR names no target language, and that Rust is the
first backend rather than the only shape a backend can take. The claim is enforced *structurally* —
`crate_boundaries.rs` fails when an edge appears that would let the IR reach a Python parser or the
Rust backend name Python — and it has never been **tested**, because testing it requires a second
consumer and there is exactly one.

`tests/conformance.rs` is written for two or more. It enumerates backends from the registry rather
than from a list, and asserts every implemented one renders the whole corpus and that the result
compiles. Today that iterates a set of size one, so what it actually establishes is that the Rust
backend renders IR the Rust backend was built alongside. `pipeline-architecture` carries the
requirement *Every implemented backend renders the shared conformance corpus*, and it costs nothing
to satisfy while the set is a singleton.

A second real backend is the only thing that answers the question, and the cheapest honest one is
**Python**. In `inspiration/py2many/`, whose comparison prompted this, the Python target is not a
curiosity: at 64 of 73 cases it has the widest coverage of any of its thirteen backends, and it
exists precisely so the compiler's type inference can be round-tripped and read.

The payoff is four things for one crate:

1. **The neutrality claim gets tested.** Anything the Rust backend was quietly relying on shows up
   as a Python backend that cannot be written without it.
2. **A differential oracle that needs no toolchain.** Emitted Python must produce the same answers
   as the Python it came from, checkable with an interpreter and nothing else — which makes it
   usable in environments where `add-differential-fixture-testing`'s tiers cannot run.
3. **"What did compylr understand?" becomes answerable.** `.compylr/ir/unit.json` technically
   contains the answer and nobody reads JSON to get it. Emitted Python shows inference, numeric
   promotion, and resolved behavior in the language the user already reads — and a program compiled
   under `behavior="rust"` emitted back as Python shows, *in Python*, what Rust's arithmetic
   actually does to it.
4. **It costs no bridge.** `(python, python)` needs no calling convention, so this exercises the
   frontend/backend axis without paying any part of the N × M bridge cost.

**Why now.** It comes after the differential corpus, which gives it something to be checked against,
and before the typed-IR change, because the whole point is to find out what the IR is actually
missing *before* its shape is changed. Two known findings are already visible from reading the code
and are described below; the interesting ones are the ones this change discovers.

## What Changes

- **A new crate, `compylr-backend-python`**, implementing `Backend`: IR to Python source, reading
  each node's declared modes rather than its name, exactly as the Rust backend does. Registered in
  `compylr-registry::backends` beside `rust`, and selectable everywhere a backend name is.

- **Python's declared semantics and spellings move to the language, not the direction.**
  `PYTHON_BEHAVIOR` and the `Ty::python_name` / `BinOp::python_symbol` spellings currently live in
  `compylr-frontend-python`, on the stated ground that how a construct is spelled back to a
  programmer belongs to the frontend that read it. A Python *backend* needs the same spellings and
  the same behavior declaration, and cannot depend on the frontend without transitively depending on
  ruff — which `crate_boundaries.rs` correctly forbids. So they move to a small
  `compylr-lang-python` crate that both read. This is the first finding: **a language's declared
  semantics belong to the language, not to whether you are reading it or writing it**, which is what
  compylr's own rule that "a stance declaration names only its own language" already implies.

- **A backend names its own translated-source file.** `--emit rust` currently reaches for
  `compylr_backend_rust::rust::GENERATED_PATH` by name, so the CLI knows one backend specially even
  though its spec already says it prints "the translated functions **for the selected backend**".
  The second finding, and a latent defect rather than a new requirement: the `Backend` trait gains a
  declaration of which of its files holds the translation.

- **Emission is semantics-preserving, not spelling-preserving.** The backend does not get to emit
  `//` because the IR node came from `//`. Under `Rounding::TowardNegInf` it may; under
  `Rounding::TowardZero` — reachable through `behavior="rust"` — it must emit Python that truncates.
  Same for remainder sign, index origin, text units, and every checking mode. This is the real test
  of whether the IR carries what it claims to.

- **`ruff format` becomes a second `post_process`**, which is where a formatter-shaped assumption in
  the pipeline would surface.

- **`(python, python)` stays deliberately unbridged.** Asking for the whole crate reports the
  existing fourth answer the architecture already models — compylr can generate this and cannot call
  it back — rather than a missing method. Nothing about it needs to be special-cased.

- **No change to the Python frontend's observable behavior**, to the Rust backend, to the IR, or to
  any artifact format. No cache is invalidated.

## Capabilities

### New Capabilities
- `python-backend`: IR to Python source — concrete spellings, statement and expression emission,
  honoring every declared mode rather than the operation's name, deterministic and pure, and the
  named set of files it produces. The counterpart of `rust-backend`.

### Modified Capabilities
- `python-frontend`: *The Python frontend owns Python spellings* becomes a statement about the
  **language** rather than about the frontend. Diagnostics still spell Python and the IR still
  offers no spelling; what changes is that the frontend and the backend read one declaration, so a
  type cannot be named `dict[str, int]` in a diagnostic and something else in generated source. The
  same applies to Python's declared operator and container semantics.
- `pipeline-architecture`: a backend SHALL declare which of the files it emits holds the translated
  code, so a caller wanting only the translation does not have to know which backend it selected.

## Impact

**Added**
- `crates/compylr-backend-python/` — the backend.
- `crates/compylr-lang-python/` — Python's declared semantics and spellings, depending on
  `compylr-ir` and nothing else.

**Modified**
- `crates/compylr-frontend-python/` — reads `compylr-lang-python` instead of owning the
  declarations; `spelling.rs` moves.
- `crates/compylr-core/src/backend.rs` — the `Backend` trait names its translated-source file.
- `crates/compylr-backend-rust/src/rust.rs` — implements that declaration.
- `crates/compylr-cli/src/main.rs` — stops naming the Rust backend for the target-code form.
- `crates/compylr-registry/src/backends.rs` — one entry.
- `crates/compylr-host-python/tests/crate_boundaries.rs` — the new edges are asserted, and the rule
  that only the Python *frontend* may depend on a Python parser is restated so that
  `compylr-lang-python` (which parses nothing) is clearly on the right side of it.
- `README.md` and `CLAUDE.md` — two backends, and what the second one is for.

**Cost**
- `tests/conformance.rs` now runs the whole corpus through two backends, and its
  `every_corpus_entry_compiles_for_the_rust_backend` gains a Python-shaped sibling that checks the
  emitted source parses. Both are fast; neither needs a toolchain beyond an interpreter.
- Every future IR form owes emission in two backends rather than one. That is the point, and it is
  the cost the architecture was always going to charge — paid now, against one small backend,
  instead of later against a large one.
