# compylr demo — three nth-prime implementations

A complete uv project you could copy, not a snippet. Three implementations of the same function,
each compiled to Rust by compylr, each exercising a different part of the supported subset — and all
asserted to agree with a plain interpreted reference and with each other.

```bash
uv sync
uv run compylr compyle src      # compile ahead of time; otherwise the first call pays for it
uv run python -m nth_prime 25
```

```
the 25th prime, four ways:
  reference (interpreted)        97   0.015 ms
  recursive (compiled)           97   1.916 ms
  iterative (compiled)           97   1.844 ms
  memoized (compiled)            97   0.002 ms

the cache answered 1 request(s) without recomputing
all four agree
```

Those per-call numbers are not a benchmark, and the demo does not pretend otherwise: the first
compiled call in a process also resolves and imports the extension, which is most of what you see
above. At n=25 the work itself is far too small to measure against that.

> **Compiling needs a Rust toolchain and maturin.** Installing compylr gets you the compiler, not
> the ability to build what it generates.

## What each variant shows

| variant | exercises |
| --- | --- |
| `recursive.py` | recursion with a base case, branching, calls between marked functions |
| `iterative.py` | `while` and `for`, a reassigned counter, `break`, a locally built `list` returned by value |
| `memoized.py` | a class with a mutable `dict` attribute and a hit counter — state that outlives a call |

`reference.py` is plain interpreted Python. It is the oracle, so it is written to be obvious rather
than fast; every other variant is checked against it.

The memoized variant is the one that could not exist before classes: a cache has to outlive the call
that fills it, and the subset has no module-level state. It works because an **instance** is not
converted at the boundary — the Python object holds the Rust value, so a mutated attribute is what
you see next call. A collection **parameter** is a copy and could not do this.

## Precompiling

Measured on this project (Apple Silicon, macOS 15, 8 marked members):

| | time |
| --- | --- |
| cold `compylr compyle src` | **8.14 s** |
| a later `compylr compyle src` that reuses it | **0.017 s** |
| first call after precompiling | **7.7 ms** |

Without precompiling that ~8 s lands on whichever call happens first. `compylr compyle` imports
every module beneath the root — a decorator only registers when it runs — so module-level code
executes; environments, caches, and build output are skipped.

## The recursion bound, and why it is stated

`recursive.py` recurses once per **prime found**, not once per candidate integer. A version doing the
latter would reach a depth in the thousands for a modest `n`.

There is no tail-call elimination, and a stack overflow in compiled code is a **process abort**, not
a recoverable error — no traceback, no exception, nothing to catch. Measured here:

| n | result |
| --- | --- |
| 100,000 | 1,299,709 |
| 150,000 | process killed, SIGSEGV, no output |

The tests stay well below that. This is the first place the project meets a limit that is not a
subset restriction: the subset permits the program, and the machine does not.

## Gaps this demo found

This is the first program here written to be *useful* rather than to exercise a rule, so what it
tripped over is what a user meets first. Recorded rather than papered over, and not fixed here —
each deserves its own change.

**Marked names are shared across a whole project.** All three variants naturally want to be called
`nth_prime`, and only one can be: they compile into one module. So each variant's compiled functions
carry a prefix and each module re-exports the readable name:

```python
nth_prime = recursive_nth_prime
```

**There is no `not`.** `iterative.py` wants `if not divisible:` and has to spell it as a function
returning the negation. Every linter will suggest the version that does not compile.

**A loop cannot be a function's only exit.** compylr does not assume a loop body runs, so
`while True: ... return x` is rejected as having a path that produces no value. `next_prime` carries
its answer out in a variable instead — correct, and one line longer than the obvious version.

## Checks

```bash
uv run pytest          # every variant against the reference and against each other
uv run ruff check .
uv run mypy src
```

The repository's own suite also builds this project and exercises all three variants, so the demo
cannot rot unnoticed.
