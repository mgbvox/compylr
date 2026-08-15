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

The pipeline is implemented up to the IR:

```
source text ──frontend──> ruff AST ──lower──> compylr IR ──backend──> target code
                                                             (not built yet)
```

Supported Python subset: top-level `def`s only, fully annotated (`int`/`bool`/`str`, plus
`None` as a return type); bodies of `return`, `pass`, and annotated assignment; expressions
of literals, names, unary minus, `+ - * // %`, comparisons, and calls.

One inference rule: `b = a` infers `b`'s type from `a` when the initializer is a bare name.
Literals, expressions, and calls still require an annotation.

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
cargo test                                  # 82 tests
cargo clippy -p compylr --all-targets -- -D warnings
cargo llvm-cov -p compylr --ignore-filename-regex '(vendored/|/main\.rs)' --summary-only
cargo run -- python/fixtures/accepted/aliases.py

./scripts/render_change_epub.py             # spec -> EPUB in reports/
./scripts/send_to_kindle.py <file> --dry-run
```
