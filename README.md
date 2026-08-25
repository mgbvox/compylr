# compylr

[![Rust](https://github.com/mgbvox/compylr/actions/workflows/rust.yml/badge.svg)](https://github.com/mgbvox/compylr/actions/workflows/rust.yml)
[![Python](https://github.com/mgbvox/compylr/actions/workflows/python.yml/badge.svg)](https://github.com/mgbvox/compylr/actions/workflows/python.yml)
[![Benchmark](https://github.com/mgbvox/compylr/actions/workflows/benchmark.yml/badge.svg)](https://github.com/mgbvox/compylr/actions/workflows/benchmark.yml)

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
source text ──frontend──> tree ──lower──> IR ──verify──> passes ──backend──> Rust ──bridge──> extension
     ✓                      ✓          ✓        ✓          ✓          ✓                ✓
```

Each stage is a separate crate, and each end of the pipeline is a *named component* rather than
the only implementation present:

* a **frontend** turns source text into IR (`python`; `typescript`, `go`, and `cpp` reserved),
* a **backend** turns IR into target source (`rust`; the same three reserved),
* a **host bridge** makes the result callable, and belongs to the `(source, target)` **pair** —
  `(python, rust)` today.

The third one is where compylr stops resembling LLVM. LLVM's frontends and backends compose
N + M because it emits object code and never calls back into the source language. compylr's whole
purpose is that the source language calls the result, and a calling convention is a negotiation
between two runtimes — who owns the memory, how errors signal, how strings encode. Python→Rust is
PyO3; Python→Go would be cgo and a C array someone has to free; nothing carries over. So bridges
cost N × M, and the design's job is to keep that visible rather than pretend otherwise: a pair
with a backend and no bridge is a specific answer — *compylr can generate Go, and cannot yet call
it from Python* — not a missing method.

Both intermediates are written to disk on every build, so nothing between your Python and the
compiled artifact is a black box:

```
.compylr/
  ir/unit.json            the IR, as JSON
  crate/src/generated.rs  your functions, translated — the file worth reading
  crate/src/compat.rs     the semantics the IR declared, in Rust; identical in every project
  crate/src/bindings.rs   the PyO3 boundary
  crate/src/lib.rs        module declarations and the module registration
  state.json              fingerprint of the last successful build
```

The crate is split by concern so `generated.rs` opens on your code. It used to be one file, where
a single one-line function produced 238 lines and the translation was lines 200–212.

| Capability | What it covers |
| --- | --- |
| `python-frontend` | Parsing Python source text into a syntax tree, with structured I/O and syntax errors |
| `ir-lowering` | Translating the syntax tree into IR, enforcing the subset and type rules |
| `ir` | The program model and type system every backend consumes, and its on-disk artifact |
| `rust-backend` | IR to Rust source: concrete type spellings, and the semantics each node declares |
| `python-bindings` | The PyO3 layer generated onto compiled functions, and how failures become exceptions |
| `native-bridge` | `compylr._core`, exposing the compiler to Python and its diagnostics as exceptions |
| `build-pipeline` | The shared crate, the artifacts on disk, and the fingerprint-keyed rebuild decision |
| `python-api` | `initialize`, the decorator's two forms, settings resolution, and swapping in |
| `cli` | The command line: precompiling a project, what it emits, and how it reports rejections |
| `demo` | The worked example: what it must contain, that it compiles, and that this repo verifies it |
| `pipeline-architecture` | What a frontend, a backend, and a host bridge are, and how each is resolved |
| `ir-optimization` | Verification and the pass pipeline between lowering and emission |
| `semantic-behavior` | Behavior axes, how each language declares its stance, and how a request resolves |
| `generated-code-performance` | What an optimization may not change, and how a speedup is measured and guarded |
| `fixture-corpus` | The accepted and rejected corpora, their drivers, and CPython as the oracle |

Specs live in `openspec/specs/`; they are the authoritative description of behavior.

### What the corpus proves

Generated by `scripts/update_subset.py` from the fixture corpus and the translation tier's
results. A form is listed only because a fixture exercising it translated, built, ran, and agreed
with CPython, so this table cannot claim more than the compiler does. Editing it by hand is
editing output.

<!-- subset:matrix -->
51 of 53 IR forms are exercised by a fixture that translated, built, ran, and agreed with CPython. A form with no such fixture is not listed.

| Form | Kind | Exercised by |
| --- | --- | --- |
| `Return` | statement | `aliases.py` |
| `Bind` | statement | `aliases.py` |
| `Assign` | statement | `branching.py` |
| `Effect` | statement | `class_valued_signatures.py` |
| `SetAttr` | statement | `class_valued_signatures.py` |
| `SetItem` | statement | `classes.py` |
| `Append` | statement | `classes.py` |
| `If` | statement | `branching.py` |
| `While` | statement | `loops.py` |
| `For` | statement | `loops.py` |
| `Break` | statement | `loops.py` |
| `Continue` | statement | `loops.py` |
| `Literal` | expression | `aliases.py` |
| `Name` | expression | `aliases.py` |
| `Neg` | expression | `arithmetic.py` |
| `ToFloat` | expression | `call_inference.py` |
| `Binary` | expression | `arithmetic.py` |
| `ListLit` | expression | `classes.py` |
| `DictLit` | expression | `classes.py` |
| `SetLit` | expression | `collections.py` |
| `TupleLit` | expression | `collections.py` |
| `TupleIndex` | expression | `collections.py` |
| `Attribute` | expression | `class_valued_signatures.py` |
| `Construct` | expression | `class_valued_signatures.py` |
| `MethodCall` | expression | `class_valued_signatures.py` |
| `Contains` | expression | `classes.py` |
| `Not` | expression | `mutation.py` |
| `Subscript` | expression | `collections.py` |
| `Len` | expression | `classes.py` |
| `Range` | expression | `loops.py` |
| `Call` | expression | `call_inference.py` |
| `Int` | type | `aliases.py` |
| `Float` | type | `call_inference.py` |
| `Bool` | type | `aliases.py` |
| `Str` | type | `aliases.py` |
| `Unit` | type | `class_valued_signatures.py` |
| `List` | type | `classes.py` |
| `Dict` | type | `classes.py` |
| `Set` | type | `collections.py` |
| `Tuple` | type | `collections.py` |
| `Instance` | type | `class_valued_signatures.py` |
| `Add` | operator | `arithmetic.py` |
| `Sub` | operator | `arithmetic.py` |
| `Mul` | operator | `arithmetic.py` |
| `Div` | operator | `arithmetic.py` |
| `Rem` | operator | `arithmetic.py` |
| `Eq` | operator | `comparisons.py` |
| `NotEq` | operator | `comparisons.py` |
| `Lt` | operator | `branching.py` |
| `LtE` | operator | `comparisons.py` |
| `Gt` | operator | `branching.py` |
<!-- /subset:matrix -->

Not built yet: `llm_assist` (accepted as a setting, refused when enabled), and the TypeScript,
Go, and C++ backends (reserved names that fail with a message saying so).

## Try it now

```bash
git clone --recurse-submodules https://github.com/mgbvox/compylr.git
cd compylr
uv venv && source .venv/bin/activate
uv pip install maturin && maturin develop --release
```

Then the snippet at the top of this file works. There is also a CLI for seeing what a program
compiles to, without a build:

```bash
cargo run -p compylr-cli -- python/fixtures/accepted/inference.py
```

```
unit fingerprint: bcddf18219a7c991
  comparisons (1 params) -> bool
  expressions (1 params) -> int
  literals (0 params) -> str
```

`--emit` selects what it prints. Output goes to stdout and diagnostics to stderr, so redirecting
gives you a usable file:

```bash
cargo run -p compylr-cli -- --emit ir    python/fixtures/accepted/inference.py   # the IR, as JSON
cargo run -p compylr-cli -- --emit rust  python/fixtures/accepted/inference.py   # just the translated code
cargo run -p compylr-cli -- --emit crate --out ./out python/fixtures/accepted/inference.py
```

Artifacts live in `.compylr/`, found by searching upward from the working directory for a
`pyproject.toml` or an existing `.compylr/`. Running a project from a subdirectory therefore
reuses what it already built. *If you built with an earlier version, the first run after upgrading
may rebuild once as the directory moves to the project root — that is the move, not a cache bug.*

## The worked example

[`demo/`](demo/) is a complete uv project, not a snippet: sixty-eight functions and classes that
each compile, every one checked against an interpreted oracle, all of them built into **one**
extension.

```bash
cd demo && uv sync
uv run compylr compyle src
uv run python -m algorithms          # run everything, then print what the build exercised
```

It has two halves. **Breadth** is sorting, number theory, statistics, text, graphs, dynamic
programming, matrices, and data structures — algorithms chosen so that between them they reach
*every* form the IR can hold. That is not a claim in prose: the demo walks the IR of its own build
and asserts it, and a test in this repository fails when a form is added to the compiler that the
demo does not know about. **Depth** is `nth_prime`, one problem three ways, measured compiled
against interpreted in two separate processes.

The benchmark reports the spread rather than a headline, and this table is **generated** — written
back by [`scripts/update_benchmarks.py`](scripts/update_benchmarks.py) from a real run, never
edited by hand. Its ends are the finding: the top is arithmetic in a tight loop, where there is
nothing for the interpreter to do but dispatch, and the bottom is work dominated by **crossing the
boundary** — collections are converted element by element on every call, so compiling pays
only when the generated body saves more than that conversion costs.

<!-- benchmark:summary -->
| workload | compiled | interpreted | speedup |
| --- | ---: | ---: | ---: |
| `arithmetic.collatz_length` | 0.28us | 6.11us | **22.0x** |
| `dynamic.knapsack` | 21.54us | 246.76us | **11.5x** |
| `structures.component_count` | 6.51us | 60.01us | **9.2x** |
| … | | | |
| `text.joined` | 77.27us | 61.71us | **0.8x** |
| `graphs.bfs_distances` | 49.62us | 27.17us | **0.5x** |
| `text.word_count` | 74.16us | 18.69us | **0.3x** |
| `reference (never compiled)` | 38.80us | 40.17us | 1.0x |

_scale 1 — measured on Darwin arm64, Python 3.14.0, 2026-08-24._
<!-- /benchmark:summary -->

The `reference` row is never compiled, so its ratio is what "no difference" looks like on the
machine that produced the table — read every other row against that rather than against 1.0. The
demo's own README carries the whole table, the before/after this change measured, what the subset
costs as it shows up in real code, and the defects the benchmark found despite every answer being
correct.

## Turning it off

```bash
COMPYLR_DISABLE=1 python your_program.py
```

Every `@c.compyle` then hands back exactly what it was given: nothing is validated, nothing is
registered, no build is attempted, and the project runs as ordinary Python. The original comes back
rather than a pass-through wrapper, so compylr stays out of your tracebacks and profiles entirely.

Two jobs. The first is answering "is this compylr, or is it my code?" without editing anything — and
it works even when the reason for asking is that compylr *rejects* the code, since a disabled
decorator does not validate. The second is measurement: a marked function calls other marked
functions through module globals, so timing an "interpreted" call in a compiled process would still
reach compiled code. Only a whole process running interpreted gives an honest number.

`compylr.initialize(enabled=False)` does the same from inside the project. Switching mid-process is
refused — the members marked before the change would be in a different mode from the ones after.

## Precompiling

The first call to a marked function builds the project, which makes that call slow. For anything
starting under a request — a container image, a serverless handler — that cost lands in the wrong
place. Move it to build time:

```bash
compylr compyle ./my-project
```

```
compylr: /path/to/my-project
  imported 3 module(s); found 4 function(s) and 1 class(es)
  built
```

Measured on a one-function project: **7.36s** to precompile cold, **0.009s** for a later run that
reuses it. The first call after precompiling does no work at all.

> **`compylr` is the Python command**, installed with the wheel. The Rust binary keeps its `--emit`
> surface, lives in the `compylr-cli` crate, and is reached through `cargo run -p compylr-cli --`
> during compiler development — if you were invoking the binary as `compylr`, that is what changed.

Discovery **imports** every module beneath the root, so module-level code runs. That is inherent: a
decorator only registers when it runs, and anything reading source text instead would need its own
notion of what `@c.compyle` looks like — one that drifts from the runtime's the moment someone
aliases the import or decorates conditionally. A precompiler that silently misses a function is
worse than none, because the symptom is a slow first call rather than an error.

The cost is bounded rather than hidden: never installed packages, and never `.venv`, `__pycache__`,
`.git`, `.compylr`, or build output. A module that raises on import is reported and skipped, so one
broken file does not stop the rest.

Three outcomes are distinguishable from the exit status alone: built or reused (`0`), nothing marked
(`3`), and failure (`1`, or `2` for a bad root). Nothing-marked is deliberately not success — an
image that precompiles nothing has failed at what it was there for.

## Supported subset

Functions at top level only, with mandatory parameter and return annotations.

| Python | IR type | Notes |
| --- | --- | --- |
| `int` | integer | 64-bit signed |
| `float` | float | 64-bit binary floating point |
| `bool` | bool | deliberately **not** a number; `True + 1` is rejected |
| `str` | string | UTF-8 |
| `None` | unit | return annotation only |
| `list[T]` | sequence | any element type, nested to any depth |
| `dict[K, V]` | mapping | keys restricted to `int`/`str`/`bool` |
| `set[T]` | set | elements restricted the same way |
| `tuple[A, B]` | tuple | a type per position |

Operators: `+` `-` `*` `/` `//` `%` and the comparisons `==` `!=` `<` `<=` `>` `>=`, plus unary
negation and calls to functions in the same unit.

Statements: `return`, `pass`, local bindings, reassignment, `if`/`elif`/`else`, `while`, `for`,
`break`, and `continue`.

Collections support literals, subscripting, `len`, membership, and mutation of **locals**:

```python
@c.compyle
def total(xs: list[int]) -> int:
    first = xs[0]        # int
    last = xs[-1]        # counts from the end, as Python does
    return first + last + len(xs)   # code points for a string, not bytes
```

Both of those are *declared* on the IR rather than assumed, because they are the two container
operations the supported languages disagree about: Go, C++, and TypeScript all treat a negative
index as out of range, and `len` counts UTF-8 bytes in Go and UTF-16 units in TypeScript. The
three readings agree on ASCII, which is exactly what would make a wrong assumption survive a test
suite.

Two divergences worth knowing. **Collections cross the boundary by value**, so a compiled function
cannot affect a list its caller still holds — currently unobservable, since nothing in the subset
mutates, but stated so that adding mutation has to confront it. And **a returned `dict` does not
preserve insertion order**, and its order varies between runs; sort explicitly if you need one.
Sequences and tuples keep their order.

Keys and set elements are restricted to `int`, `str`, and `bool`: a float key can never be
retrieved once it is `nan`, and most targets cannot hash a float at all.

```python
@c.compyle
def evens_below(limit: int) -> list[int]:
    found: list[int] = []
    for n in range(limit):
        if n % 2 == 0:
            found.append(n)     # the shape loops exist for
    return found

@c.compyle
def counts(words: list[str]) -> dict[str, int]:
    seen: dict[str, int] = {}
    for word in words:
        if word in seen:        # `in` tests a mapping's keys, as Python does
            seen[word] = seen[word] + 1
        else:
            seen[word] = 1      # assignment creates a key; reading a missing one still raises
    return seen
```

**Mutating a parameter is rejected**, and this is the one rule most likely to surprise:

```python
@c.compyle
def f(xs: list[int]) -> None:
    xs.append(1)                # rejected: a collection parameter is a copy
```

Collections cross the boundary by value. If that compiled, the caller's list would be silently
unchanged where the interpreted original would have modified it — a wrong answer with no error.
Rejecting it makes the program not exist rather than making the divergence documented.

**Aliasing does not get around it.** `copied = xs` binds a second name to the same object in Python
and copies in compylr, so mutating `copied` is the same hazard one line further out and is rejected
the same way, transitively. Build a *fresh* collection and fill it:

```python
@c.compyle
def doubled(xs: list[int]) -> list[int]:
    out: list[int] = []
    for x in xs:
        out.append(x * 2)     # the workaround, and the shape you wanted anyway
    return out
```

`append` is the only supported method; any other is rejected with a diagnostic naming it. `in` and
`not in` work over a list, dict, set, and str — a **dict tests its keys** and a **str tests
substrings** (`"ab" in "cab"` is true), both matching Python.

Not supported: comprehensions, slicing, deletion, and every method except `append`.

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

A **call** is inferred too, when the function being called is defined in the same source:

```python
def double(n: int) -> int:
    return n * 2

def demo(n: int) -> int:
    doubled = double(n)      # int — taken from double's signature
    return doubled
```

Signatures are collected before any body is lowered, so a function may call one defined *below*
it and get the same answer either way. Arity and argument types are checked against the signature,
and an integer passed where a float is declared carries an explicit conversion.

A call to a function in **another** module is still undetermined, and needs an annotation:

```python
total: int = from_elsewhere(n)   # annotation required
```

That is not an oversight. Each decorated function is validated on its own, so a callee in another
module is invisible at that moment; rejecting it would make whether your code compiles depend on
which function you happened to decorate first. Such a call is still checked — once every source is
assembled into one unit.

### Behavior axes

By default, compiled operations keep the source language's behavior. Set a project-wide behavior
with `compylr.initialize(behavior="rust")`, override one member with
`@c.compyle(behavior="rust")`, or select individual axes while inheriting the rest:

```python
from compylr import Behavior

c = compylr.initialize()

@c.compyle(behavior=Behavior(overflow="rust", index="python"))
def selected(xs: list[int], n: int) -> int:
    return xs[n] + 1
```

Behavior is validated when `initialize` or the decorator runs. Each selected language must be the
source or target of this compilation — `python` or `rust` today. Members with different behaviors
may share one artifact because the choice belongs to each operation; members with different
backends are still refused because a project produces one target artifact.

| Behavior field | IR axis | Python behavior | Rust behavior |
| --- | --- | --- | --- |
| `overflow` | `integer_overflow` | report 64-bit integer overflow | use Rust's native operator; generated release builds wrap |
| `floor_div` | `integer_division` | round toward negative infinity; report zero divisor | truncate toward zero; use native failure |
| `true_div` | `exact_division` | report zero divisor | IEEE-754 result, including infinity |
| `modulo` | `remainder` | sign follows divisor; report zero divisor | sign follows dividend; use native failure |
| `index` | `sequence_index` | negative indexes count from the end; report out of range | indexes count from the start; use native failure |
| `text_len` | `text_length` | count Unicode code points | count UTF-8 bytes |

### Control flow

Branches, both loop forms, and `break`/`continue`:

```python
@c.compyle
def nth_prime(n: int) -> int:
    found = 0
    candidate = 1
    while found < n:
        candidate = candidate + 1
        if is_prime(candidate):
            found = found + 1
    return candidate

@c.compyle
def fib(n: int) -> int:
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)     # recursion works: signatures are collected first
```

Three rules are **stricter than Python**, each to keep a runtime surprise from becoming a compiled
one:

* **A test must be a `bool`.** `if n:` is rejected. A subset that demands annotations everywhere
  should not then infer that an integer means a condition.
* **A block is a scope.** A name bound inside a branch or loop is gone when it ends, so reading it
  afterwards is rejected — Python leaks such a name into the function and fails at runtime if the
  branch did not run.
* **A name keeps the type it was first bound at.** `i = 0` then `i = "x"` is rejected; `i = i + 1`
  is fine, and updates the binding rather than shadowing it. An annotation on a rebinding is a
  re-declaration and is also rejected.

`for` iterates a range or a collection. A mapping yields its **keys**, as Python does:

```python
for i in range(10, 0, -2):    # 10, 8, 6, 4, 2 — a negative step, which Rust's `..` cannot express
    ...
for key in mapping:           # keys, not values or pairs
    ...
```

`range` takes one, two, or three integers and is only valid as what a `for` iterates — there is no
range value in the subset. A zero step raises `ValueError` before the loop starts rather than
spinning forever. `range` and `len` are both reserved names.

A function that declares a return type must return one **on every path**: a conditional counts only
when it has an `else` and both branches return, and a loop never counts, because its body may run
zero times.

```python
def f(a: int) -> int:
    if a > 0:
        return 1     # rejected: the path where the test is false produces no value
```

### Classes

A class gives state somewhere to live that outlives a call — which is what a memoized function
needs, and what free functions over values cannot provide:

```python
@c.compyle
class PrimeCache:
    def __init__(self) -> None:
        self.known: dict[int, bool] = {}     # every attribute declared here, annotated

    def is_prime(self, n: int) -> bool:
        if n in self.known:
            return self.known[n]
        ...
        self.known[n] = answer                # persists to the next call
        return answer
```

**Every attribute is declared in `__init__`, with an annotation, or not at all.** Python lets one
appear anywhere; without this rule the compiled struct's fields would depend on which methods
happened to run.

The contrast worth holding onto, because it is the thing people get wrong:

| | crosses as | so a compiled function… |
| --- | --- | --- |
| a collection **parameter** | a copy | cannot mutate it — mutation is rejected |
| an **instance** | itself | can mutate it, and the caller sees it next call |

An instance is not converted at the boundary at all: the Python object holds the Rust value, and a
method borrows it from there. That asymmetry is exactly why an attribute can be a cache.

A method taking a mutable receiver is derived, including through calls — a method whose body is only
`self.bump()` mutates too.

Not supported: inheritance, `@property`, class attributes, `@dataclass`, and any dunder but
`__init__`.

Not supported yet: imports, generics, `try`, `with`, and `for`/`else`.

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
cargo test --workspace                        # Rust: unit, fixture, emission, execution
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

uv venv && source .venv/bin/activate
uv pip install -e ".[dev]"
pytest                                        # Python: the package and the native boundary
ruff check python/ scripts/                   # lint
ruff format --check python/ scripts/          # formatting
ty check python/compylr                       # types
```

The binary prints the unit fingerprint and each function's signature, and reports rejections
with a `line:column` location:

```
$ cargo run -p compylr-cli -- python/fixtures/rejected/boolean_arithmetic.py
error: 2:12: operator '+' is not defined for 'bool' and 'bool'; booleans are not numbers in compylr
```

## Layout

A Cargo workspace, and the root is a workspace and nothing else — no language's crate sits above
the others. Three of the nine name Python, and each is named for the *job* rather than for the
project: a frontend that reads it, a bridge that makes generated Rust callable from it, and a host
binding that exposes the compiler to it. A TypeScript host would be `compylr-host-typescript`
beside them, with the same standing; what makes Python's three look privileged is only that they
are the ones that exist.

The dependency edges are the enforcement mechanism, not a convention: a crate that does not depend
on a Python parser cannot name a Python construct, and only a `compylr-host-*` crate may link a
host language's runtime.

```
crates/
  compylr-diagnostics/              spans and located diagnostics; depends on nothing
  compylr-ir/                       the IR: types, expressions, statements, Unit, fingerprints, artifact
  compylr-core/                     the traits and the component model; knows no implementation
  compylr-frontend-python/          ruff parsing and lowering; the only crate that depends on ruff
  compylr-frontend-typescript/      oxc parsing and lowering for TypeScript
  compylr-backend-rust/             IR -> Rust source, plus the runtime shim embedded in generated crates
  compylr-backend-golang/           IR -> Go source emission
  compylr-bridge-python-rust/       the PyO3 layer generated onto compiled functions, for (python, rust)
  compylr-bridge-typescript-golang/ the CGo/JS loader generated onto compiled functions, for (typescript, go)
  compylr-host-python/              compylr._core: the compiler itself, exposed to Python
  compylr-host-typescript/          Node-API native addon exposing the compiler to TypeScript/Node.js
  compylr-registry/                 where implementations are registered; the one crate that knows them all
  compylr-cli/                      the `compylr` binary and its --emit surface
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

Two different things use PyO3 and conflating them causes lasting confusion.
`crates/compylr-host-python/` exposes **the compiler** to Python as `compylr._core`, built from
this repo. `crates/compylr-bridge-python-rust/` *generates* PyO3 code onto **your** functions,
built at runtime into a separate crate. Different crates, different lifecycles — and note that the
generating crate does not itself depend on PyO3, because it emits PyO3 source as text.

## Design invariants

Three rules that are easy to break and expensive to discover later:

**The IR names no target language.** Concrete spellings — `int` becoming `i64`, `str` becoming
`String` — belong to a backend, never to the IR. Rust is the first backend, but Go, C++, and
TypeScript backends should consume the same tree unchanged.

**Operations carry the semantics a resolved behavior declared, not a language's by default.**
`BinOp::Div` carries exact or integer division, its rounding, and its checking mode; `BinOp::Rem`
carries sign and checking modes; `Expr::Subscript` carries index origin and checking mode; and
`Expr::Len` carries text units. The default resolves every axis to Python, while a behavior
selection may resolve any axis to Rust. The backend reads those modes off each node and never the
operation's name, which lets one artifact contain functions with different behaviors. Lowering
inserts an explicit widening node for exact division, so a backend never re-derives a conversion.

The IR's own rendering says the mode rather than a symbol — `//` is Python's way of writing one
particular rounding, not the rounding itself — so quoting a programmer's syntax back at them
belongs to the frontend that read it.

Three container behaviours deliberately have **no** mode, and the absence is a conclusion rather
than an omission: reading a mapping with an absent key always reports it, iterating a mapping
yields keys, and membership in a string tests substrings. The last two are what every language in
the supported list does. The first is a difference in the *shape* of the operation rather than a
setting on it — Go's `v, ok := m[k]` is a different expression with a different result type — so a
frontend that means it lowers to a different form.

**Rebuild decisions key off the IR, not the source text.** `Unit::fingerprint` hashes structure,
so comments and reformatting do not trigger a recompile, and it is order-independent so
decoration order does not either. It also hashes the unit's origin — which frontend produced it
and what that language requires preserved — because two units with identical bodies and different
requirements can legitimately emit different code, and a cache that could not tell them apart
would hand back the wrong build.

> **Upgrading past this release rebuilds every project once.** The IR changed shape — first the
> arithmetic operators, subscripting and length, and operation checking modes — so the artifact
> format is at version 4
> and every fingerprint moved. The build state records the compiler version, so this happens
> automatically rather than as a stale-artifact bug.

## Development

Planning goes through [OpenSpec](https://github.com/Fission-AI/OpenSpec) before code:

```bash
/opsx:propose    # write proposal, spec deltas, design, tasks
/opsx:apply      # implement the tasks
/opsx:archive    # sync deltas into openspec/specs/ and archive the change
```

Conventions: tests before implementation; `cargo fmt`, `cargo clippy --workspace --all-targets --
-D warnings`, and `cargo test --workspace` green before committing; commit at each checkpoint.

Several tests exist to stop documentation, structure, and behaviour drifting apart, and they are
worth knowing about before a change surprises you. All but one live in
`crates/compylr-host-python/tests/`, which is where the workspace's integration suite sits.

**That the code does what this file says:**

* `readme.rs` checks this file against the code, so the type table, operator list, crate layout,
  capability list, subset matrix, and referenced paths cannot drift silently.
* `crate_boundaries.rs` reads the manifests, so an edge that would let a backend name Python, the
  IR reach a parser, or a non-host crate link a host runtime fails the suite rather than passing
  review.
* `conformance.rs` renders a corpus of hand-built IR through every backend the registry reports as
  implemented, and fails if any IR node form is unexercised in a position it is legal in. It is
  authored as IR rather than as Python on purpose: a tree Python cannot express is a tree the
  fixtures can never contain, and that is exactly where a backend can be silently wrong.

**That a compiled function answers what the same Python answers.** CPython is the oracle: no
expected value is written anywhere, so there is nothing for anyone to type incorrectly. Each
accepted fixture has a *driver* in `python/fixtures/drivers/` naming the calls that exercise it,
and the same driver runs both ways.

* `differential.rs` — the **translation tier**. Emits the crate, writes a `main` around it,
  compiles, runs, and compares transcripts as text. Seconds, no maturin, on every `cargo test`.
* `python/tests/test_differential.py` — the **boundary tier**. The same corpus through PyO3, as a
  user reaches it, comparing *values* rather than text. `slow`, one build for the whole corpus.
  The two fail differently on purpose: a program can be translated correctly and converted wrongly
  at the boundary, and only the second sees that.
* `corpus.rs` — the frontend over Python nobody wrote for this compiler: this repository's own,
  the demo, the scripts, and the standard library of whichever interpreter is installed. Every
  outcome must be a lowered unit or a diagnostic carrying a source position; a panic fails.
* `fixtures.rs` — every accepted fixture has exactly one driver and that driver reaches every
  member it defines, and every rejected fixture is *still* rejected. That last one is inverted on
  purpose: a refused construct that starts compiling fails the suite, so growing the subset is a
  decision rather than something that happens quietly.
