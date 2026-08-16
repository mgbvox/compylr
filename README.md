# compylr

Transpiles a strict, fully annotated Python subset to Rust, and calls the result from Python.

```python
import compylr

c = compylr.initialize(backend="rust", llm_assist=False)

@c.compyle
def add(a: int, b: int) -> int:
    return a + b

@c.compyle
def floordiv(a: int, b: int) -> int:
    return a // b

add(2, 3)          # 5, computed by compiled Rust
floordiv(-7, 2)    # -4, floored the way Python floors -- not Rust's -3
```

The first call to a marked function compiles **every** marked function in the project into one
shared Rust extension, builds it with maturin, installs it, and swaps the compiled
implementations in. Later runs reuse it. Rebuild decisions key off a fingerprint of the IR, not
of the source text, so comments and reformatting cost nothing.

> **Compiling needs a Rust toolchain and maturin on the machine running your project.** The
> decorator shells out to `cargo` on the first call. Installing compylr gets you the compiler,
> not the ability to build what it generates. Removing that requirement means shipping prebuilt
> wheels, which is a distribution problem rather than a compiler one.

## Status

The pipeline is complete end to end for the supported subset.

```
source text ──frontend──> ruff AST ──lower──> IR ──backend──> Rust ──maturin──> extension
     ✓                       ✓            ✓          ✓             ✓
```

Both intermediates are written to disk on every build, so nothing between your Python and the
compiled artifact is a black box:

```
.compylr/
  ir/unit.json        the IR, as JSON
  crate/src/lib.rs    the generated Rust
  state.json          fingerprint of the last successful build
```

| Capability | What it covers |
| --- | --- |
| `python-frontend` | Parsing Python source text into a syntax tree, with structured I/O and syntax errors |
| `ir-lowering` | Translating the syntax tree into IR, enforcing the subset and type rules |
| `ir` | The program model and type system every backend consumes, and its on-disk artifact |
| `rust-backend` | IR to Rust source: concrete type spellings, and Python's operator semantics |
| `python-bindings` | The PyO3 layer generated onto compiled functions, and how failures become exceptions |
| `native-bridge` | `compylr._core`, exposing the compiler to Python and its diagnostics as exceptions |
| `build-pipeline` | The shared crate, the artifacts on disk, and the fingerprint-keyed rebuild decision |
| `python-api` | `initialize`, the decorator's two forms, settings resolution, and swapping in |

Specs live in `openspec/specs/`; they are the authoritative description of behavior.

Not built yet: `llm_assist` (accepted as a setting, refused when enabled), and the TypeScript,
Go, and C++ backends (reserved names that fail with a message saying so).

## Try it now

```bash
git clone --recurse-submodules https://github.com/mgbvox/compylr.git
cd compylr
uv venv && source .venv/bin/activate
uv pip install maturin && maturin develop --release
```

Then the snippet at the top of this file works. There is also a CLI that stops at the IR, which
is useful for seeing what a program lowers to:

```bash
cargo run -- python/fixtures/accepted/inference.py
```

```
unit fingerprint: bcddf18219a7c991
  comparisons (1 params) -> bool
  expressions (1 params) -> int
  literals (0 params) -> str
```

## Supported subset

Functions at top level only, with mandatory parameter and return annotations.

| Python | IR type | Notes |
| --- | --- | --- |
| `int` | integer | 64-bit signed |
| `float` | float | 64-bit binary floating point |
| `bool` | bool | deliberately **not** a number; `True + 1` is rejected |
| `str` | string | UTF-8 |
| `None` | unit | return annotation only |

Operators: `+` `-` `*` `/` `//` `%` and the comparisons `==` `!=` `<` `<=` `>` `>=`, plus unary
negation and calls to functions in the same unit.

Statements: `return`, `pass`, and local bindings.

A **docstring** is permitted in first position and carries no runtime meaning, so ordinary
documented code compiles:

```python
@c.compyle
def add(a: int, b: int) -> int:
    """Return the sum."""
    return a + b
```

It is kept on the function, emitted as a `///` comment on the generated Rust — which PyO3 then
lifts back onto the compiled function's `__doc__` — and excluded from the fingerprint, so editing
documentation never triggers a rebuild. The exception is deliberately narrow: any *other* bare
expression statement, including a string in second position, is still rejected, because a value
that is computed and discarded is either dead code or a side effect this subset cannot express.

Local bindings infer their type whenever the initializer determines it — literals, names,
negation, arithmetic, and comparisons, composed to any depth:

```python
def demo(n: int) -> float:
    label = "x"          # str
    count = 3            # int
    ratio = 1.3          # float
    scaled = n * 2       # int
    big = n > 100        # bool
    return count / 2     # float — `/` always yields a float
```

An initializer containing a **call** is undetermined, because lowering does not resolve callees,
so it still needs an annotation:

```python
total: int = helper(n)   # annotation required
```

Not supported yet: control flow, loops, classes, imports, collections, generics, reassignment,
and inferring a binding from a call's return type.

## Getting started

`vendored/ruff` is a submodule and `Cargo.toml` depends on it by path, so the build fails
without it:

```bash
git clone --recurse-submodules https://github.com/mgbvox/compylr.git
# or, in an existing clone:
git submodule update --init
```

Then:

```bash
cargo test                                    # Rust: unit, fixture, emission, execution
cargo clippy -p compylr --all-targets -- -D warnings

uv venv && source .venv/bin/activate
uv pip install -e ".[dev]" || (uv pip install maturin pytest pytest-cov ruff mypy && maturin develop)
pytest                                        # Python: the package and the native boundary
ruff check python/ && mypy python/compylr
```

The binary prints the unit fingerprint and each function's signature, and reports rejections
with a `line:column` location:

```
$ cargo run -- python/fixtures/rejected/boolean_arithmetic.py
error: 2:12: operator '+' is not defined for 'bool' and 'bool'; booleans are not numbers in compylr
```

## Layout

```
src/
  frontend.rs   parse source text -> ruff AST
  lower.rs      ruff AST -> IR, plus the type checker
  ir.rs         the IR: types, expressions, statements, Unit, fingerprints, artifact
  error.rs      frontend, lowering, and artifact diagnostics
  span.rs       byte-offset source locations
  bridge.rs     compylr._core: the compiler, exposed to Python
  backend/
    mod.rs      the Backend trait and the name registry
    rust.rs     IR -> Rust source
    bindings.rs the PyO3 layer generated onto compiled functions
    runtime.rs  Python arithmetic semantics, embedded into generated crates
python/
  compylr/      the Python package: initialize, the decorator, the build pipeline
  tests/        pytest suite for the package and the native boundary
  fixtures/
    accepted/   programs that must lower
    rejected/   one program per rejection rule
openspec/
  specs/        current behavior, by capability
  changes/      in-flight and archived change proposals
scripts/
  render_change_epub.py   render a change's artifacts to EPUB
  send_to_kindle.py       email a document to a Kindle
reports/        rendered spec EPUBs
```

Two different things use PyO3 and conflating them causes lasting confusion. `src/bridge.rs`
exposes **the compiler** to Python as `compylr._core`, built from this repo.
`src/backend/bindings.rs` *generates* PyO3 code onto **your** functions, built at runtime into a
separate crate. Different crates, different lifecycles.

## Design invariants

Three rules that are easy to break and expensive to discover later:

**The IR names no target language.** Concrete spellings — `int` becoming `i64`, `str` becoming
`String` — belong to a backend, never to the IR. Rust is the first backend, but Go, C++, and
TypeScript backends should consume the same tree unchanged.

**Operators carry Python semantics, not the target's.** Floor division rounds toward negative
infinity and remainder takes the sign of the divisor, where most targets truncate toward zero.
True division always yields a float, where `/` between two integers is integer division in Rust,
Go, and C++. Lowering inserts an explicit widening node so a backend never has to re-derive a
conversion. A backend that maps these operators to same-named native ones is wrong on negative
and integer operands.

**Rebuild decisions key off the IR, not the source text.** `Unit::fingerprint` hashes structure,
so comments and reformatting do not trigger a recompile, and it is order-independent so
decoration order does not either.

## Development

Planning goes through [OpenSpec](https://github.com/Fission-AI/OpenSpec) before code:

```bash
/opsx:propose    # write proposal, spec deltas, design, tasks
/opsx:apply      # implement the tasks
/opsx:archive    # sync deltas into openspec/specs/ and archive the change
```

Conventions: tests before implementation; `cargo fmt`, `cargo clippy -- -D warnings`, and
`cargo test` green before committing; commit at each checkpoint.

`tests/readme.rs` checks this file against the code, so the type table, operator list, and
referenced paths cannot drift silently.
