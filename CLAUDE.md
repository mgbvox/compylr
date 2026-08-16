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

Supported Python subset: top-level `def`s only, fully annotated (`int`/`float`/`bool`/`str`, plus
`None` as a return type); bodies of `return`, `pass`, and assignment, optionally preceded by a
docstring; expressions of literals, names, unary minus, `+ - * / // %`, comparisons, and calls.
Local bindings infer their type whenever the initializer determines it; an initializer containing
a call still needs an annotation, because lowering does not resolve callees.

A **docstring** is accepted in first position and carries no runtime meaning; it is kept on the IR
function, emitted as a `///` comment, and deliberately **excluded from the fingerprint**, so
editing prose never triggers a rebuild. The exception is narrow: any other bare expression
statement — including a string in second position — is still rejected, because a discarded value
is either dead code or a side effect the subset cannot express.

Known gaps worth knowing before you trip on them:

* **Compiling needs `cargo` and `maturin` at runtime.** Installing compylr gets the compiler,
  not the ability to build what it generates.
* **`llm_assist` is accepted but refused when enabled**, and `typescript`/`go`/`cpp` are reserved
  backend names that fail with a message saying they are planned.

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
# Rust
cargo test
cargo clippy -p compylr --all-targets -- -D warnings
cargo llvm-cov -p compylr --ignore-filename-regex '(vendored/|/main\.rs)' --summary-only
cargo run -- python/fixtures/accepted/aliases.py   # CLI: stops at the IR

# Python (needs the venv; `maturin develop` rebuilds compylr._core after Rust changes)
uv venv && source .venv/bin/activate
uv pip install maturin pytest pytest-cov ruff mypy && maturin develop --release
pytest                    # includes slow tests that compile Rust; -m "not slow" to skip
ruff check python/ && mypy python/compylr

./scripts/render_change_epub.py             # spec -> EPUB in reports/
./scripts/send_to_kindle.py <file> --dry-run
```

**Never lint `python/fixtures/`.** They are compiler inputs, and `rejected/` is deliberately
invalid — `ruff check --fix` once deleted the `import os` from `import_statement.py`, silently
removing the construct the fixture exists to test. `pyproject.toml` excludes them.
