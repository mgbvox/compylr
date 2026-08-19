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
source text ──frontend──> ruff AST ──lower──> IR ──backend──> Rust ──maturin──> extension
```

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
located lowering error rather than a backend failure. Reachability lives in `ir::returns_on_all_paths`
and is shared by lowering and the backend deliberately — two implementations disagreeing would mean
either rejecting a valid program or emitting code that does not compile, and the second surfaces as
a complaint about Rust rather than about the user's function.

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

Mutation targets emit as **places**, not values. The ordinary rule clones a collection wherever it
is consumed, and that reaches through attributes: `self.entries[k] = v` once mutated a copy of the
field and silently lost every write. Assert on values after mutation, never on emitted text.

`range` is a reserved name like `len`, valid only as what a `for` iterates — there is no range value
in the IR. It is not emitted as Rust's `..`: that counts up by one, `step_by` takes an unsigned step,
and neither composes with a computed or negative step, so the loop is written out against a cursor
the body cannot disturb. A zero step is rejected *before* the loop, because the condition would never
change and a hang leaves nothing to diagnose from.

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
  cost real time once already.
* **`COMPYLR_DISABLE=1` turns compilation off for a process**, returning every marked member
  untouched without validating it. That is what makes an interpreted measurement honest: a marked
  function reaches other marked functions through module globals, so `python_function` alone gives
  an interpreted outer call into compiled inner ones.
* **`compylr` is now the Python console script**, not the Rust binary. The binary keeps `--emit`
  and is reached through `cargo run`. Precompiling **imports** the project, because a decorator only
  registers when it runs; discovery is bounded to the root and skips environments, caches, and build
  output.
* **`llm_assist` is accepted but refused when enabled**, and `typescript`/`go`/`cpp` are reserved
  backend names that fail with a message saying they are planned.
* **Both fixture lists are read from the directory, not hardcoded.** `tests/emit_quality.rs` and
  `tests/fixtures.rs` enumerate `python/fixtures/accepted/`. They were once lists, drifted, and
  hid a real defect: tuple indexing emitted a `py_subscript` call with no tuple impl, so
  `collections.py` had been producing code that did not compile. Keep them derived.

# Two PyO3 roles

Do not conflate them:

* `src/bridge.rs` exposes **the compiler** to Python as `compylr._core`, built from this repo.
* `src/backend/bindings.rs` **generates** PyO3 code onto the user's functions, built at runtime
  into a separate crate (`compylr_generated_<fingerprint>`).

The fingerprint is in the generated module's name because CPython cannot reliably re-import an
extension module under a name already in `sys.modules`.

# Conventions

* The IR is independent of Python **and** of any target language. Concrete type spellings
  (`int` → `i64`) belong to a backend capability, never to the IR — Go/C++/TypeScript
  backends should consume the same tree.
* IR operators carry **Python** semantics: `FloorDiv` rounds toward negative infinity, `Mod`
  takes the sign of the divisor. Most targets' native `/` and `%` disagree on negative
  operands, so a backend must emit semantics-preserving code rather than map to the
  same-named native operator.
* Rebuild decisions key off `Unit::fingerprint()` (over the IR), not source text, so comments
  and reformatting do not trigger recompiles.
* TDD: write tests before implementation. Run `cargo fmt`, `cargo clippy -- -D warnings`, and
  `cargo test` before committing. Commit at each checkpoint rather than batching.
* **Keep `README.md` in sync.** It is the entry point for anyone who has not read the specs, so
  it must never describe a state the code is not in. `tests/readme.rs` enforces the mechanical
  half — the type table, operator list, capability list, module layout, and every referenced
  path — and fails `cargo test` on drift. The prose half is on you: when a change alters the
  supported subset, adds a capability or pipeline stage, changes the setup steps, or makes the
  backend real, update the README in the *same* change, not afterwards.
* Planning happens in OpenSpec (`openspec/changes/`). `/opsx:propose` to plan, `/opsx:apply`
  to implement.

# Commands

```bash
# Precompile a project ahead of its first run (the Python console script)
compylr compyle path/to/project

# Rust
cargo test
cargo clippy -p compylr --all-targets -- -D warnings
cargo llvm-cov -p compylr --ignore-filename-regex '(vendored/|/main\.rs)' --summary-only
cargo run -- python/fixtures/accepted/aliases.py            # summary
cargo run -- --emit ir   python/fixtures/accepted/aliases.py   # the IR as JSON
cargo run -- --emit rust python/fixtures/accepted/aliases.py   # translated code only
cargo run -- --emit crate --out ./out python/fixtures/accepted/aliases.py

# Python (needs the venv; `maturin develop` rebuilds compylr._core after Rust changes)
uv venv && source .venv/bin/activate
uv pip install maturin pytest pytest-cov ruff mypy && maturin develop --release
pytest                    # includes slow tests that compile Rust; -m "not slow" to skip
ruff check python/ && mypy python/compylr

# Run any project interpreted, with compylr out of the way entirely
COMPYLR_DISABLE=1 python your_program.py

# Compare compiled against interpreted (runs both modes in separate processes)
cd demo && PYTHONPATH=src python -m nth_prime.benchmark --n 500

# The demo project (its own uv project; verified by python/tests/test_demo.py)
cd demo && uv sync && uv run compylr compyle src && uv run python -m nth_prime 25
cd demo && uv run pytest && uv run ruff check . && uv run mypy src

./scripts/render_change_epub.py             # spec -> EPUB in reports/
./scripts/send_to_kindle.py <file> --dry-run
```

**Run `cargo llvm-cov` with the venv deactivated.** The bridge tests auto-initialize a Python
interpreter, and an active venv makes that mismatch what PyO3 linked against — the suite aborts
with "no Python frame", which looks like a real failure and is not. `cargo test` is unaffected.

**Never lint `python/fixtures/`.** They are compiler inputs, and `rejected/` is deliberately
invalid — `ruff check --fix` once deleted the `import os` from `import_statement.py`, silently
removing the construct the fixture exists to test. `pyproject.toml` excludes them.
