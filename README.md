# compylr

Transpiles a strict, fully annotated Python subset to Rust.

> **There is no Python package yet.** `import compylr` does not work, and `uv add compylr`
> installs nothing — no `pyproject.toml`, no PyO3 bindings, and no maturin build exist in this
> repo today. compylr is currently a **Rust crate with a CLI**. See [Status](#status) for what
> actually runs, and [Try it now](#try-it-now) to use it.

## The goal

A Python package installed with `uv add compylr`, where a decorator compiles the function it
wraps:

```python
# TARGET DESIGN — not implemented yet
import compylr

@compylr.compyle
def add(a: int, b: int) -> int:
    return a + b
```

On first run the decorated function would be transpiled to Rust with PyO3 bindings, built via
maturin, and installed into the project venv. On later runs the decorator would swap in the
compiled implementation at import time. Every decorated function in a project is exposed by
**one** shared maturin crate, and adding or editing any of them rebuilds that single artifact.

Reaching that needs, roughly in order: a Rust backend that emits code from the IR, PyO3
binding generation, a maturin build and install step, and the Python-side decorator and
rebuild cache. None of those exist yet.

## Status

The pipeline reaches Rust source. The IR is emitted as an inspectable artifact along the way,
and the emitted Rust preserves Python's arithmetic semantics — but nothing builds it into an
importable module yet, which is why `import compylr` still does not work.

```
source text ──frontend──> ruff AST ──lower──> compylr IR ──backend──> Rust source ──build──> extension
     ✓                       ✓                    ✓            ✓                     not built
```

What the backend does today: maps the IR's semantic types onto `i64`, `f64`, `bool`, `String`,
and `()`; emits every statement and expression form; and reproduces Python's `//`, `%`, and `/`
rather than mapping them to Rust's same-named operators, which disagree on negative and integer
operands. Division by zero and `i64` overflow are recoverable errors instead of a panic or a
silent wrap.

Not built yet: PyO3 binding generation, the maturin build, and the Python package with its
decorator.

| Capability | What it covers |
| --- | --- |
| `python-frontend` | Parsing Python source text into a syntax tree, with structured I/O and syntax errors |
| `ir` | The program model and type system every backend consumes |
| `ir-lowering` | Translating the syntax tree into IR, enforcing the subset and type rules |

Specs live in `openspec/specs/`; they are the authoritative description of behavior.

## Try it now

The only interface today is the CLI. It parses a Python file, lowers it to IR, and reports the
unit fingerprint and each function's signature — or a located diagnostic if the program is
outside the subset:

```bash
git submodule update --init          # required; see Getting started
cargo run -- python/fixtures/accepted/inference.py
```

```
unit fingerprint: bcddf18219a7c991
  comparisons (1 params) -> bool
  expressions (1 params) -> int
  literals (0 params) -> str
```

No Rust source is emitted — that is the backend, which does not exist yet.

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
cargo test                                    # unit + fixture tests
cargo clippy -p compylr --all-targets -- -D warnings
cargo run -- python/fixtures/accepted/inference.py
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
  backend/
    mod.rs      the Backend trait and the name registry
    rust.rs     IR -> Rust source
    runtime.rs  Python arithmetic semantics, embedded into generated crates
python/fixtures/
  accepted/     programs that must lower
  rejected/     one program per rejection rule
openspec/
  specs/        current behavior, by capability
  changes/      in-flight and archived change proposals
scripts/
  render_change_epub.py   render a change's artifacts to EPUB
  send_to_kindle.py       email a document to a Kindle
reports/        rendered spec EPUBs
```

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
