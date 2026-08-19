## Context

See proposal.md — Why. What the demo has to work within:

* compylr installs as a wheel built by maturin, and compiling requires `cargo` and `maturin` on the
  machine. A demo that a reader cannot build is not a demo, so its README has to lead with that.
* `.compylr/` is found by walking upward for a project marker, so a demo with its own
  `pyproject.toml` gets its own artifact directory without configuration.
* The repository's slow tests already compile crates and are grouped behind a marker. The demo
  check belongs there.
* Every capability is currently tested alone. Nothing exercises control flow *and* mutation *and*
  classes together in one program.

## Goals / Non-Goals

**Goals:**

* A project someone can copy, rather than a snippet they must assemble.
* An executable check that the features compose, which is the thing isolated tests cannot give.
* Three implementations whose agreement is itself the assertion.

**Non-Goals:**

* Benchmarking against CPython. A demo that also claims speed invites an argument about
  methodology, and this one is about *what compiles*, not how fast.
* Publishing the demo, or making it a template repository.
* Any compiler change. If the demo needs something absent, that is a finding for its own change.

## Decisions

### D1. Three algorithms chosen for coverage, not variety

| Variant | Reaches |
| --- | --- |
| recursive | branching, a base case, self-recursion, calls between marked functions |
| iterative | `while`/`for`, reassignment, a locally built collection, `append` |
| memoized | a class, a mutable attribute, membership, insertion, state across calls |

Together they are close to the smallest set covering everything the four preceding changes add.
Their agreement is the assertion: three implementations that each compile and disagree is precisely
the failure isolated tests cannot catch, because each passes its own.

### D2. Recursion depth is a real constraint, and the demo is bounded because of it

A recursive nth-prime that recurses once per candidate integer reaches a depth in the thousands for
a modest n. There is no tail-call elimination, and a stack overflow in compiled code is a **process
abort**, not a recoverable error — the one failure mode with no diagnostic at all.

So two decisions follow. The recursive variant's recursion is over *primes found* rather than over
*candidates tested*, which keeps depth proportional to n rather than to the numbers examined. And
the demo's documented range is bounded, with the README saying why rather than leaving a reader to
discover it by crashing.

This is worth stating loudly because it is the first place the project meets a limit that is not a
subset restriction: the subset permits the program, and the machine does not.

### D3. The demo depends on compylr as a consumer, not as a sibling

Its `pyproject.toml` declares compylr as a dependency and it is installed into its own environment.
It does not reach into `python/compylr/` by path, because then it would be testing the working tree
rather than the package, and would not answer "what does using this look like?"

For the repository's own check, the installed package is the one already built for the test run, so
the demo exercises the same artifact everything else does.

### D4. The repository's suite builds and runs it, behind the slow marker

A demo that stops compiling while the README says it works is worse than no demo, so the check has
to exist. It compiles a crate, so it goes with the other slow tests rather than in the fast path.

The check asserts three things, in increasing order of what they would catch: that each variant
compiled rather than falling back; that all three agree with each other and with an interpreted
reference; and that the memoized variant's second call is served from its cache. The last matters
because a class that recomputes would pass the first two while demonstrating nothing.

### D5. The memoized variant caches, observably

```python
@c.compyle
class PrimeCache:
    def __init__(self) -> None:
        self._cache: dict[int, int] = {}
        self._hits: int = 0

    def nth(self, n: int) -> int:
        if n in self._cache:
            self._hits = self._hits + 1
            return self._cache[n]
        ...
```

The hit counter exists so the test can assert the cache is *used*, not merely present. Without it,
"memoized" is a claim about the code's shape rather than its behaviour, and a refactor that broke
caching would still pass.

### D6. The README shows measurements, not adjectives

The precompile section gives real timings for the cold build and the warm run, taken on a stated
machine. This project has consistently preferred a number to a claim — the 238-line `lib.rs`, the
8.89s/0.003s build figures — and a demo is the worst place to start asserting instead of measuring.

## Risks / Trade-offs

* **A stack overflow aborts the process** → D2. The recursive variant is structured to keep depth
  proportional to n, and the documented range is bounded. This is the demo's sharpest limitation and
  it belongs in its README, not only here.
* **The demo will expose gaps** → Expected, and the point. The risk is the temptation to fix them
  inside this change, which would turn a demonstration into a grab-bag. Each finding becomes its own
  change; the demo is written against what exists.
* **A slow check that people skip** → If building the demo makes the suite unbearable, someone will
  exclude it and it will rot. Hence the slow marker and one build shared across the demo's
  assertions rather than one per test.
* **Three algorithms that are subtly the same** → If all three end up sharing a helper that does the
  real work, they stop being three implementations and the agreement assertion proves nothing. Each
  must compute independently, and that is worth checking in review rather than by test.

## Migration Plan

Nothing to migrate: the demo is new and consumes the compiler without changing it.
