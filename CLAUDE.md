# Target End State

I `uv add compylr` in a project.
Then, in my code:
```python
import compylr

@compylr.compyle
def my_cool_function[T, U, V](a:T, b:U) -> V:
    ... # logic
```

Under the hood:
* first run: transpile my_cool_function to rust with python bindings via maturin, install in the project venv
* subsequent runs: usage of my_cool_function is imported from the rust bindings and replaced by the decorator at runtime.

All decorated functions in a project share **one** maturin crate. Adding or editing one
rebuilds that single artifact.

# Setup

`vendored/ruff` is a git submodule pinned to an upstream commit. `cargo build` will fail
without it, because `Cargo.toml` depends on those crates by path:

```bash
git clone --recurse-submodules <url>
# or, in an existing clone:
git submodule update --init
```

# Current state

The pipeline is complete end to end for the supported subset:

```
source text ──frontend──> tree ──lower──> IR ──verify──> passes ──backend──> Rust ──bridge──> extension
```

The workspace is nine crates and the root is a virtual manifest — no language's crate sits above
the others. Three crates name Python, and each is named for the *job*: a frontend that reads it, a
bridge that makes generated Rust callable from it, and a host binding that exposes the compiler to
it. A TypeScript host would be `compylr-host-typescript` beside them. The dependency edges are the
enforcement mechanism rather than a convention. `compylr-backend-rust` cannot name Python because no Python parser is reachable
from it; `compylr-ir` cannot name Rust for the same reason. `tests/crate_boundaries.rs` reads the
manifests and fails when an edge appears that would make either claim false. If you find yourself
wanting to add a dependency to `compylr-diagnostics` or `compylr-ir`, that is the signal to stop:
whatever you pull in there reaches every crate in the workspace.

Both ends of the pipeline are named components resolved through `compylr-registry`, and there is
a third: a **host bridge**, keyed by the `(source, target)` **pair**. That asymmetry is real and
deliberate — a calling convention is a negotiation between two runtimes, so it cannot belong to
either language alone, and bridges cost N × M where frontends and backends cost N + M. See
`crates/compylr-core/src/bridge.rs` for why, and for the C-ABI escape hatch that is deferred
rather than foreclosed.

`import compylr` works. `compylr.initialize()` returns a manager; `@c.compyle` marks a function,
validating it immediately and compiling the whole project on the first call. Both intermediates
(the IR as JSON, the generated Rust) are written under `.compylr/` on every build.

Supported Python subset: top-level `def`s only, fully annotated (`int`/`float`/`bool`/`str`, the
collections `list[T]`/`dict[K,V]`/`set[T]`/`tuple[...]`, plus `None` as a return type); bodies of
`return`, `pass`, assignment, reassignment, `if`/`elif`/`else`, `while`, `for`, `break`, and
`continue`, optionally preceded by a docstring; expressions of literals, names, unary minus,
`+ - * / // %`, comparisons, and calls. **Recursion works**, including mutual recursion.
Local bindings infer their type whenever the initializer determines it, **including calls to
functions in the same source**: signatures are collected in a first pass, so a function may call
one defined below it. A call to a function in another module stays undetermined and needs an
annotation — the decorator validates one function at a time, so rejecting an unseen callee would
make acceptance depend on decoration order. `Unit::validate` still catches a callee that exists
nowhere.

A function declaring a return type must return one **on every path**; `def f() -> int: pass` is a
located lowering error rather than a backend failure. Reachability lives in
`compylr_ir::returns_on_all_paths` and is shared by lowering, the verifier, and the backend
deliberately — two implementations disagreeing would mean either rejecting a valid program or
emitting code that does not compile, and the second surfaces as a complaint about Rust rather than
about the user's function. The verifier adds the neighbouring rule for frontends that have not
re-derived it: a function declaring a value may not return *without* one anywhere in its body,
which would be a function with two return types.

Three control-flow rules are **stricter than Python**, and each will look like a bug until you know
why:

* **A test must be a `bool`.** `if n:` is rejected. Annotations are mandatory everywhere else, so
  inferring that an integer means a condition would be the one place the subset guesses.
* **A block is a scope for names.** A name bound in a branch or loop does not survive it. The names
  that go out of scope are remembered anyway, purely so the diagnostic can say the binding may not
  have happened rather than that the name is unknown — it is right there a few lines up, so "not
  defined" reads as a compiler bug.
* **A name keeps the type it was first bound at.** Reassignment writes to the frame that *owns* the
  name, which is what makes `i = i + 1` inside a loop update the counter outside it. An annotation
  on a rebinding is a re-declaration and is rejected.

`pass` lowers to **no statement at all**. It used to lower to a unit return, which was harmless
when a body could not contain a loop and would now make `for i in range(n): pass` exit the function.

**Mutation is confined to locals.** `xs.append(v)` and `xs[i] = v` are accepted on a local and
rejected on a **parameter**, because collections cross the boundary by value and a mutated
parameter could not be observed by the caller. The diagnostic names the copy rather than merely
refusing — a rule without its reason leaves the user no workaround. `append` is the only method;
`in`/`not in` work over list, dict (keys), set, and str (substrings).

**Classes** hold state that outlives a call. Attributes are declared in `__init__` with mandatory
annotations and nowhere else, or the struct's fields would depend on which methods ran. A method's
receiver is derived by fixpoint: it is mutable when the method assigns an attribute, mutates a
collection attribute, **or calls a method that does** — the transitive case is the likeliest bug,
and its failure mode is a borrow-checker error about generated code rather than a diagnostic.

The contrast that matters, and that people get wrong: a collection **parameter** crosses by value
and may not be mutated; an **instance** is not converted at all — the Python object holds the Rust
value via `#[pyclass]`, and a method borrows it from there — so a mutated attribute is what the
caller sees next call. That is why an attribute can be a cache.

`self` is the Rust receiver: never escaped by `rust_ident`, never cloned. Lowering reserves the name
outside a method so nothing else reaches that branch.

**A constructor has no `self`.** The instance does not exist inside `__init__`, so the backend
rewrites the whole body — at every depth — so that `self.x` *is* the local `x`, and builds the
struct from those locals at the end. Handling only the top level emitted `(self).count = i` for an
assignment inside an `if` or a loop, which is ordinary Python and generated code that does not
compile. For the same reason `__init__` **may not return early**: every attribute becomes a field,
so a return before the end would leave part of the instance unassigned. A *trailing* bare `return`
means nothing in Python either and is dropped rather than refused.

Mutation targets emit as **places**, not values, and a nested read emits as a **borrow**. The
ordinary rule clones a collection wherever it is consumed, and both directions through a nested
collection are exceptions — each was a live defect the demo found:

* `self.entries[k] = v` mutated a copy of the field, and so did `table[i][j] = v` and
  `items[0].bump()`. Every write was silently lost and every answer was plausible.
* `m[i][j]` cloned the whole row to read one element of it — an O(n) copy per element access, so a
  matrix multiply did O(n^4) work and came out no faster than interpreted Python.

`emit_place` handles both, selected by `Access::{Shared, Mutable}`, and `place_root` follows the
chain when deciding which bindings are `mut` and whether a `for` must snapshot what it walks.
Assert on values after mutation, never on emitted text — except where the emitted *form* is the
property, as with the borrow.

`range` is a reserved name like `len`, valid only as what a `for` iterates — there is no range value
in the IR. It is not emitted as Rust's `..`: that counts up by one, `step_by` takes an unsigned step,
and neither composes with a computed or negative step, so the loop is written out against a cursor
the body cannot disturb. **The cursor advances immediately after the loop variable is bound, before
the body runs** — an increment below the body is one `continue` can skip, and skipping it is not a
wrong answer but a hang. A zero step is rejected *before* the loop, for the same reason: the
condition would never change and a hang leaves nothing to diagnose from.

A **docstring** is accepted in first position and carries no runtime meaning; it is kept on the IR
function, emitted as a `///` comment, and deliberately **excluded from the fingerprint**, so
editing prose never triggers a rebuild. The exception is narrow: any other bare expression
statement — including a string in second position — is still rejected, because a discarded value
is either dead code or a side effect the subset cannot express.

Known gaps worth knowing before you trip on them:

* **Collections are read-only and cross by value**, and a returned `dict` has no guaranteed key
  order — it varies between runs. Chosen deliberately; `add-collection-types` design D7 records
  what reversing it costs. Iteration is where a user meets this: `for k in d` yields keys, in
  whatever order the map gives, so **never assert on mapping or set iteration order** — the suite
  would become flaky rather than the compiler being wrong.
* **Generated mappings and sets use `FastHasher`, not Rust's default randomized hasher.** This is
  not a behavior axis: the accepted subset promises neither mapping nor set iteration order, and
  equality, membership, indexing, and mutation are unchanged. A test that distinguishes hashers
  by iteration order is asserting behavior the language deliberately does not provide.
* **The Python boundary has a measurable per-element price on every call.** On this machine an
  integer argument costs about 4 ns per element to convert, text about 42 ns, and returning an
  element about 10 ns. Every argument crosses by value, text and collections alike; a body doing
  O(log n) work over an O(n) argument can therefore lose compiled.
* **A parameter may not be borrowed just because it is never mutated.** Passing text as `&str` was
  built and reverted: not mutating a value is not the same as tolerating a borrow of it, because a
  parameter can also be *stored*. Four ordinary shapes need an owned `String` and emitted Rust that
  did not compile — `xs.append(who)`, `d[k] = who`, `who < "m"` (`==` works only because std
  happens to provide that cross-impl and `PartialOrd` does not), and `who in xs`. Deciding this
  correctly needs the backend to know an expression's type, which it deliberately does not. The
  whole suite passed while it was broken, so `a_text_parameter_is_usable_in_every_position` in
  `tests/execution.rs` now compiles a text parameter in every position; check there before trying
  again.
* **Mutating a collection while iterating it is not rejected.** Rust's borrow checker will refuse
  it, so the failure is a rustc error rather than a located diagnostic. The honest fix is a
  lowering rule, and it belongs with whatever change first makes it reachable.
* **A `for` snapshots what it iterates.** Python's `for` holds the object, so rebinding the name in
  the body must not change what is iterated; the emitted code clones, which says that directly and
  keeps a loop-long borrow out of the borrow checker's way.
* **Compiling needs `cargo` and `maturin` at runtime.** Installing compylr gets the compiler,
  not the ability to build what it generates.
* **The rebuild key is the IR fingerprint, so editing the *backend* does not invalidate a cached
  build.** The state file now records the installed compylr version, which covers a user upgrading
  the package — but during development here the version does not move, so after changing emission
  you must `rm -rf .compylr` (and `demo/.compylr`) or you will benchmark last build's code. This
  cost real time once already. Every emission performance measurement starts with that removal.
* **The IR changed shape again, so every existing cache is invalid once.** The artifact format is
  at version 4 — 2 for the arithmetic operators, 3 for subscripting and length, 4 for operation
  checking modes. `_state_is_current`
  compares the recorded compylr version, so a user upgrading rebuilds automatically; there is
  nothing to do beyond knowing why the first run after upgrading is slow.
* **A statement's emission depends on where it is, not only on what it is.** The backend renders a
  constructor body, a method body, a free function body, and a loop body through different code,
  and `tests/conformance.rs` checks coverage over `(form, position)` pairs for that reason. The
  first run of that check found four defects, three reachable from ordinary Python — including a
  `continue` inside `for i in range(n)` that skipped the cursor increment and hung. Adding a
  statement form means covering it in every position it is legal in, and the test says which.
* **`COMPYLR_DISABLE=1` turns compilation off for a process**, returning every marked member
  untouched without validating it. That is what makes an interpreted measurement honest: a marked
  function reaches other marked functions through module globals, so `python_function` alone gives
  an interpreted outer call into compiled inner ones.
* **`compylr` is now the Python console script**, not the Rust binary. The binary lives in
  `compylr-cli` and keeps `--emit`; a bare `cargo run` has no target to pick, so it is
  `cargo run -p compylr-cli --`. Both ends of the pipeline are selectable — `--frontend` and
  `--backend` — and both default rather than being assumed. Precompiling **imports** the project, because a decorator only
  registers when it runs, and it imports packages the way the runtime does: a synthetic root
  package is registered and a package's own module runs before anything below it. Discovery is
  bounded to the root and skips environments, caches, and build output.
* **`llm_assist` is accepted but refused when enabled**, and `typescript`/`go`/`cpp` are reserved
  names on **both** sides — frontend and backend — that fail with a message saying they are
  planned. A pair with a backend but no bridge is a fourth answer, distinct from an unknown or
  reserved target: compylr can generate it and cannot yet call it back.
* **The Rust backend declares one target option it does not implement.** `unchecked-arithmetic`
  exists so the guarantee negotiation has something real to refuse; permitting it where nothing
  forbids it fails saying it is reserved rather than silently doing nothing.
* **Both fixture lists are read from the directory, not hardcoded.** `tests/emit_quality.rs` and
  `tests/fixtures.rs` enumerate `python/fixtures/accepted/`. They were once lists, drifted, and
  hid a real defect: tuple indexing emitted a `py_subscript` call with no tuple impl, so
  `collections.py` had been producing code that did not compile. Keep them derived.
* **Every accepted fixture owes a driver**, in `python/fixtures/drivers/<name>.py`, naming the
  calls that exercise it as literal data. Both differential tiers read the same driver, and
  `fixtures.rs` fails when a fixture has none or when a driver does not reach every member the
  fixture defines. A driver carries **no expected values**: what a call should answer is what
  CPython answers. Unlike the corpora, `drivers/` is linted and type-checked.
* **The rejection corpus has an inverted guard.** A program in `python/fixtures/rejected/` that
  *starts* lowering fails the suite. Clear it by moving the program into `accepted/` and giving it
  a driver — never by adding an allowance, which turns a change in the language into a change in a
  test. `python/fixtures/rejected/README.md` says so where someone hitting the failure will look.
* **A member name must be unique across the whole accepted corpus.** The boundary tier builds
  every fixture into one unit, as a real project is built, and `Unit::add_function` refuses a
  duplicate. Four fixtures carry a header saying why a name is what it is; renaming one back
  breaks that build rather than any rule the fixture tests.
* **`class_valued_signatures.py` is excluded from the boundary tier by name.** The Python bridge
  has no `Ty::Instance` handling, so a function whose signature names a class emits bindings that
  do not compile. The translation tier covers it in full. See `HANDOFF.md`.

# Two PyO3 roles

Do not conflate them:

* `crates/compylr-host-python/` exposes **the compiler** to Python as `compylr._core`, built from
  this repo. It is the only crate that links PyO3, and `crate_boundaries.rs` states that rule over
  the `compylr-host-*` prefix rather than over its name — a `compylr-host-typescript` linking
  napi-rs would pass for the same reason, and neither is special.
* `crates/compylr-bridge-python-rust/` **generates** PyO3 code onto the user's functions, built at
  runtime into a separate crate (`compylr_generated_<fingerprint>_<variant>`). It emits PyO3
  source as *text* and does not itself depend on PyO3 — `tests/crate_boundaries.rs` asserts that.

The name carries the fingerprint because CPython cannot reliably re-import an extension module
under a name already in `sys.modules`. It carries a second tag because the fingerprint identifies
the **program** and not the **build**: the same source compiled for a different target, or under a
different pass configuration, is a different artifact, and sharing a name meant the second
silently was the first.

# Conventions

* The IR is independent of Python **and** of any target language, and the crate graph is what
  makes that true rather than a comment. Concrete type spellings (`int` → `i64`) belong to a
  backend; how a construct is spelled *back to the programmer* belongs to the frontend that read
  it, which is why `Ty::python_name` and `BinOp::python_symbol` live in
  `compylr-frontend-python::spelling` as extension traits and not on the IR.
* IR operations carry the semantics **the resolved behavior declared**, not one language's by
  default. The six axes are integer overflow, integer division, exact division, remainder,
  sequence indexing, and text length. `BinOp::Div` carries rounding and checking modes,
  `BinOp::Rem` a sign convention and checking mode, `Expr::Subscript` an index origin and checking
  mode, and `Expr::Len` the units a string is counted in. The backend matches on those modes and
  never on the operation's name. A backend that read the name would be silently wrong for the
  other stance, which is why `tests/conformance.rs` and the hand-built entries in
  `tests/execution.rs` exist.
* **`Checked::Unchecked` is a statement about the program, not a promised machine result.** It
  says the program declines to define the failure. Rust's native integer overflow wraps in the
  generated release profile and panics in a debug build; both satisfy that statement. Do not
  rename the mode to `Wrapping` or make a pass infer it from build settings.
* **Three container behaviours deliberately have no mode**, and the reason is recorded in the IR's
  module doc, the runtime's, and the spec: a missing mapping key always reports, mapping iteration
  yields keys, and string membership tests substrings. The last two are universal across the
  supported languages. The first is a difference in the *shape* of the operation — Go's
  `v, ok := m[k]` is a different expression — so a frontend that means it lowers to a different
  form, the way `Expr::Range` is a distinct form rather than a mode on a call.
* A **guarantee** is what a source language needs preserved for a translation to still mean what
  the source meant: overflow reported, division by zero reported, float order preserved. The
  frontend declares what it requires, the backend what it preserves, and core refuses the
  combination by name before any target source exists.
* Rebuild decisions key off `Unit::fingerprint()` (over the IR), not source text, so comments
  and reformatting do not trigger recompiles. It is taken **before** the optimization passes:
  turning a pass on must not look like the user editing their code. What distinguishes two builds
  of one program is the pass configuration, recorded in build state beside the compiler version.
  Note the two fingerprints — the one in `Compiled.fingerprint` identifies the program and is
  pre-pass; the one inside the written artifact is post-pass, so the file stays self-checking
  against its own contents.
* **Emission is a pure function of the unit.** No I/O, no environment, no shelling out. That is
  what makes its output byte-reproducible and therefore safe to key a cache on. Formatting is
  `Backend::post_process`, applied by whoever writes the files out.
* TDD: write tests before implementation. Run `cargo fmt --all`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` before
  committing. Commit at each checkpoint rather than batching.
* **The benchmark tables in both READMEs are generated.** `scripts/update_benchmarks.py` rewrites
  the blocks between `<!-- benchmark:NAME -->` markers from a real run, and
  `.github/workflows/benchmark.yml` runs it and opens a pull request with the result. Editing a table by hand is
  editing output: the next run overwrites it. Moving or renaming a marker is what breaks the job,
  so the script's `--check` mode runs in CI and on commit.
* **The README's subset matrix is generated too.** `scripts/update_subset.py` rewrites the block
  between `<!-- subset:matrix -->` markers from the corpus, and a form is listed **only because a
  fixture exercising it translated, built, ran, and agreed with CPython** — so the documentation
  cannot overstate the implementation. Editing the table by hand is editing output. Its `--check`
  mode runs in the Makefile, the hooks, and CI. Both scripts share `scripts/_regions.py`; they are
  deliberately separate scripts, because folding a documentation check into a benchmark would put
  it on the benchmark's timescale.
* **CI, the Makefile and the pre-commit hooks run the same commands.** `make check` is what the
  workflows do; `.pre-commit-config.yaml` is the subset fast enough to run on a commit. When you
  add a check, add it in all three, or it is a check people discover in a pull request instead.
  Type checking is **ty** — mypy is no longer a dependency, and neither is its configuration.
* **Keep `README.md` in sync.** It is the entry point for anyone who has not read the specs, so
  it must never describe a state the code is not in. `tests/readme.rs` enforces the mechanical
  half — the type table, operator list, capability list, module layout, and every referenced
  path — and fails `cargo test` on drift. The prose half is on you: when a change alters the
  supported subset, adds a capability or pipeline stage, changes the setup steps, or makes the
  backend real, update the README in the *same* change, not afterwards.
* **The demo is one package, `demo/src/algorithms/`, and its coverage claim is checked from both
  ends.** `ir_coverage.py` walks the IR of the build and asserts every statement form, expression
  form, type, operator, and both Python-reachable division modes appear;
  `crates/compylr-host-python/tests/demo_coverage.rs` reads the IR's enum definitions and fails
  when a form is *added* that those tables do not list. Adding an IR form therefore means either
  adding an algorithm that uses it or narrowing the claim in `demo/README.md` — the point is that
  neither can happen silently. `nth_prime` is a subpackage of it, and is the only place in this
  repository a *nested* package's `__init__.py` is imported end to end.
* **The demo is where cost shows up.** Its benchmark found a quadratic clone in `for`, an O(n)
  clone per nested read, and a full recompile per marked member on the warm path — all three
  invisible to every correctness test in the repository. When a change touches emission or the
  manager, run `make demo` and compare, not just `cargo test`.
* Planning happens in OpenSpec (`openspec/changes/`). `/opsx:propose` to plan, `/opsx:apply`
  to implement.

# Commands

```bash
# Precompile a project ahead of its first run (the Python console script)
compylr compyle path/to/project

# Rust
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo llvm-cov --workspace --ignore-filename-regex '(vendored/|/main\.rs)' --summary-only

# The binary lives in `compylr-cli` now, so a bare `cargo run` has no target to pick.
cargo run -p compylr-cli -- python/fixtures/accepted/aliases.py            # summary
cargo run -p compylr-cli -- --emit ir   python/fixtures/accepted/aliases.py   # the IR as JSON
cargo run -p compylr-cli -- --emit rust python/fixtures/accepted/aliases.py  # translated code only
cargo run -p compylr-cli -- --emit crate --out ./out python/fixtures/accepted/aliases.py

# Python (needs the venv; `maturin develop` rebuilds compylr._core after Rust changes)
uv venv && source .venv/bin/activate
uv pip install -e ".[dev]" && maturin develop --release
pytest                    # includes slow tests that compile Rust; -m "not slow" to skip
ruff check python/ scripts/ && ruff format --check python/ scripts/
ty check python/compylr   # ty, not mypy -- `make py-types`

# The same checks, on the half of the tree you touched
make hooks                # install the pre-commit hooks, once
make precommit            # run every hook over everything

# Run any project interpreted, with compylr out of the way entirely
COMPYLR_DISABLE=1 python your_program.py

# Compare compiled against interpreted (runs both modes in separate processes)
make demo                 # every algorithm; SCALE=4 for bigger inputs
make demo-primes          # the nth prime three ways; N=500

# The demo project (its own uv project; verified by python/tests/test_demo.py)
cd demo && uv sync && uv run compylr compyle src && uv run python -m algorithms
cd demo && uv run python -m algorithms.nth_prime 25
cd demo && uv run pytest && uv run ruff check . && uv run ty check src

./scripts/update_benchmarks.py              # re-measure, rewrite the README tables
./scripts/update_benchmarks.py --check      # markers only; measures nothing
./scripts/render_change_epub.py             # spec -> EPUB in reports/
./scripts/send_to_kindle.py <file> --dry-run
```

**Run `cargo llvm-cov` with the venv deactivated.** The bridge tests auto-initialize a Python
interpreter, and an active venv makes that mismatch what PyO3 linked against — the suite aborts
with "no Python frame", which looks like a real failure and is not. `cargo test` is unaffected.

**Never lint `python/fixtures/`.** They are compiler inputs, and `rejected/` is deliberately
invalid — `ruff check --fix` once deleted the `import os` from `import_statement.py`, silently
removing the construct the fixture exists to test. `pyproject.toml` excludes them.
