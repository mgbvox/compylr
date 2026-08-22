# compylr demo — common algorithms, compiled

A complete uv project you could copy, not a snippet. Sixty-eight functions and classes that each
compile to Rust through compylr, every one of them checked against an interpreted oracle, and all
of them built into **one** extension.

```bash
uv sync
uv run compylr compyle src      # compile ahead of time; otherwise the first call pays for it
uv run python -m algorithms     # run everything, then print what the build exercised
```

> **Compiling needs a Rust toolchain and maturin.** Installing compylr gets you the compiler, not
> the ability to build what it generates.

## The two halves

The project pulls in opposite directions on purpose.

**Breadth** — the modules of [`src/algorithms/`](src/algorithms). Algorithms anybody would
recognise, chosen so that between them they reach **every** form the IR can hold: each statement,
each expression, each type, each operator, and both of the division modes a Python program can
produce.

| module | what it covers |
| --- | --- |
| [`sorting.py`](src/algorithms/sorting.py) | insertion, selection, and merge sort; binary search |
| [`arithmetic.py`](src/algorithms/arithmetic.py) | gcd, lcm, integer square root, exponentiation, the sieve, base conversion |
| [`stats.py`](src/algorithms/stats.py) | mean, variance, standard deviation, normalisation — the float half |
| [`text.py`](src/algorithms/text.py) | word frequencies and membership, and the limits of `str` in the subset |
| [`graphs.py`](src/algorithms/graphs.py) | breadth-first distances, depth-first order, topological sort |
| [`dynamic.py`](src/algorithms/dynamic.py) | edit distance, longest common subsequence, coin change, knapsack |
| [`matrices.py`](src/algorithms/matrices.py) | multiply, transpose, trace — the best case for compiling |
| [`structures.py`](src/algorithms/structures.py) | a stack, a union-find, streaming statistics: state that outlives a call |

**Depth** — [`src/algorithms/nth_prime/`](src/algorithms/nth_prime). One problem, three
implementations, asserted to agree with each other and with a plain interpreted reference, then
measured compiled against interpreted in two separate processes.

| variant | exercises |
| --- | --- |
| [`recursive.py`](src/algorithms/nth_prime/recursive.py) | recursion with a base case, branching, calls between marked functions |
| [`iterative.py`](src/algorithms/nth_prime/iterative.py) | `while` and `for`, a reassigned counter, `break`, a locally built `list` returned by value |
| [`memoized.py`](src/algorithms/nth_prime/memoized.py) | a class with a mutable `dict` attribute and a hit counter — state that outlives a call |
| [`reference.py`](src/algorithms/nth_prime/reference.py) | nothing: it is never compiled. It is the oracle, so it is written to be obvious rather than fast |

Every marked member in both halves marks against the one manager in
[`_compylr.py`](src/algorithms/_compylr.py), so the whole project compiles into a single
extension. That is what lets `graphs.node_list` call `sorting.merge_sort`, and it is why a marked
name has to be unique across the project rather than within a module.

## "Showcases everything" is an assertion, not a blurb

That sentence is the kind that is true when it is written and quietly false a release later.
Someone simplifies an algorithm, the last `set` literal in the project goes away, and the sentence
is still sitting there. So it is a test.

compylr writes the IR of every build to `.compylr/ir/unit.json`.
[`ir_coverage.py`](src/algorithms/ir_coverage.py) walks it and reports which forms appear and
which member reaches each one first; `python -m algorithms` prints the table:

```
statements — 13/13          expressions — 19/19         types — 10/10
    Return       IntStack       Literal     IntStack        Int       IntStack
    ReturnUnit   UnionFind      Neg         digit_sum       Float     RunningStats
    Effect       balanced       ToFloat     RunningStats    Str       count_present
    SetAttr      IntStack       SetLit      vowel_letters   Unit      IntStack
    SetItem      IntStack       TupleLit    divide          Dict      PrimeCache
    Append       IntStack       TupleIndex  component_count Set       count_present
    Break        binary_search  Construct   balanced        Tuple     component_count
    Continue     bfs_distances  Range       UnionFind       Instance  balanced
    ...                         ...                         ...

operators — 11/11: Add Sub Mul Div Rem Eq NotEq Lt LtE Gt GtE
division modes — 2/2: Exact, Integer

Every IR form a Python program can produce is exercised by this package.
```

[`tests/test_coverage.py`](tests/test_coverage.py) turns that into assertions, so a form that
stops being covered fails this project's own suite and names itself. The compiler's suite closes
the other half: `crates/compylr-host-python/tests/demo_coverage.rs` reads the IR's enum
definitions and fails when a form is *added* that these tables do not know about — otherwise the
demo would keep reporting full coverage of a subset that grew underneath it.

Three declared semantics are deliberately **not** claimed: the remainder's sign, the index origin,
and the units a string's length is counted in each have exactly one Python reading, so no Python
program can exercise the alternatives. The compiler's conformance corpus covers those, which is
why it is authored as IR rather than as Python.

## Oracles

Every answer is checked against something that is not a copy of the implementation.

**The standard library wherever one exists** — `sorted`, `bisect`, `math.gcd`, `math.isqrt`,
`statistics.pstdev`, `collections.Counter`, `graphlib`. It was written by somebody else years ago,
which is exactly what makes it a better oracle than a reference written next to the code it
checks: a reference written that way tends to make the same mistake.

**A differently-shaped implementation where none does.** Edit distance and longest common
subsequence are compiled bottom-up into a table and checked against memoised recursion. Coin
change is a table checked against a breadth-first search. Knapsack is checked against enumerating
every subset — not an algorithm at all, just the answer.

**Seeded random inputs as well as hand-picked ones.** The hand-picked ones cover the edges anybody
would think of. The random ones cover the ones nobody did.

## Two defects this demo found

Both produce **wrong or slow answers with no error**, which is the class compylr's whole design is
organised against, and neither was reachable by anything already in the compiler's test suite.

**`table[i][j] = v` wrote into a copy of the row.** A mutation target is emitted as a *place*
rather than a value, and that had been fixed for an attribute — `self.entries[k] = v` — and not for
a subscript. So every write to a two-dimensional table was silently lost and every
dynamic-programming function returned the value the table was initialised with. Nothing raised.
Zero is a plausible edit distance. None of the compiler's accepted fixtures contained a nested
mutation, which is how it survived; one does now.

**`m[i][j]` cloned the whole row to read one element of it.** The read side of the same asymmetry.
In the inner loop of a matrix multiply that is an allocation and an O(n) copy per element access —
an O(n³) algorithm doing O(n⁴) work, with every answer correct. Only a benchmark could find it,
and this one did: matrix multiply came out at **1.0×** against interpreted Python. It is 8.2× now.

A third came out of the same session and is not a compiler defect but a cost nobody had measured:
`ensure_built` re-ran the whole compiler on every call to compute a fingerprint it already held.
With 68 marked members that was ~180 ms each, once per member. The first call to a compiled
function reported 171 ms; it is 0.02 ms now.

## Benchmarks

Both run the two modes in **separate processes**, because a marked function reaches other marked
functions through module globals — an "interpreted" outer call in a compiled process would still
land in compiled code. `COMPYLR_DISABLE=1` is what makes an interpreted run interpreted all the
way down. Timings are the best of several batches, per call: noise only adds, so the minimum is
the closest estimate of the work, and a warm cache hit takes hundreds of nanoseconds, which timing
once would report as zero.

**Read the speedup column against the noise floor, not against 1.0.** Every batch is kept, not
only the best, and each row reports the `spread` between them. The floor comes from the
never-compiled `reference` row: its true ratio is exactly 1.0 by construction, so whatever it
reports instead is this machine's noise. On the runs recorded below that floor was **2–5%**.

A row closer to 1.0 than the floor prints `not resolvable` instead of a ratio, because a ratio
would be one. `matrices.transpose` is the example worth looking at: earlier versions of this table
reported it as `1.0x`, which reads like a finding and is really the harness sitting still.

A row is marked `!` when its own batches varied by more than 25% — unstable enough that its figure
is not worth reading. `sorting.merge_sort` earns that mark on most runs, ranging from 160us to
277us across builds that were in some cases *byte-identical*. That spread is wider than most of
the improvements anyone would want to measure, which is precisely why the column exists.

### Every algorithm

```bash
make demo                                   # from the repository root
uv run python -m algorithms.benchmark       # or directly, from here
```

```
workload                           compiled    interpreted   speedup
--------------------------------------------------------------------
arithmetic.collatz_length            0.22us         4.71us     21.5x
dynamic.knapsack                    16.42us       184.12us     11.2x
structures.component_count           4.83us        45.87us      9.5x
matrices.multiply                    7.37us        60.66us      8.2x
arithmetic.sieve                     0.93us         6.64us      7.1x
stats.standard_deviation             4.11us        14.52us      3.5x
sorting.insertion_sort               3.82us         9.43us      2.5x
dynamic.edit_distance               35.96us        83.47us      2.3x
stats.normalize                      8.66us        17.89us      2.1x
sorting.merge_sort                  66.34us       132.75us      2.0x
graphs.topological_order           109.25us       160.03us      1.5x
matrices.transpose                   4.34us         4.31us      1.0x
reference (never compiled)          30.47us        29.99us      1.0x
text.joined                         57.11us        46.08us      0.8x
graphs.bfs_distances                37.91us        20.39us      0.5x
text.word_count                     55.25us        14.82us      0.3x
```

**The spread is the point.** A demo reporting one speedup would be hiding what is worth knowing.

The rows at the top are arithmetic in a tight loop, where there is nothing for the interpreter to
do but dispatch. The rows at the bottom are dominated by **crossing the boundary**: collections go
by value, so `bfs_distances` converts a 200-node mapping of lists on the way in and a mapping of
integers on the way out, and the traversal between them is not enough work to pay for that.
`word_count` is worse — building a dictionary is most of what it does, and CPython's dictionary is
C. `joined` loses because it is quadratic string concatenation either way and Python's `str` is
very good at it.

The shape of the rule: **conversion is proportional to the size of the argument, and compiling
speeds up the computation.** The ratio between those two is what a row measures. Nothing about
being compiled makes a small function over a large collection faster.

`reference` is the control. It is never compiled, so its ratio is what "no difference" looks like
on the machine you ran this on. Read every other row against that, not against 1.0.

### The nth prime

```bash
uv run python -m algorithms.nth_prime.benchmark --n 500
```

```
variant                          compiled    interpreted   speedup
------------------------------------------------------------------
reference (never compiled)      1058.15us      1049.28us      1.0x
recursive                         38.13us      1243.62us     32.6x
iterative                         19.25us       613.14us     31.8x
memoized (cold cache)             36.21us      1058.52us     29.2x
memoized (warm cache)              0.10us         0.08us      0.8x
```

**A warm cache hit is *slower* compiled** — 0.10 µs against 0.08 µs. Crossing the boundary costs
more than a dictionary lookup saves. That is not a defect; it is the shape of the tradeoff, and
the same one the bottom of the table above is measuring.

## Precompiling

Measured on this project (Apple Silicon, macOS 15, 68 marked members):

| | time |
| --- | --- |
| cold `compylr compyle src` | **13.8 s** |
| a later `compylr compyle src` that reuses it | **0.33 s** |
| importing the package in a fresh process | **50 ms** |
| resolving and importing the built extension | **185 ms** |
| first call to a compiled function after that | **0.03 ms** |

Without precompiling, the ~14 s lands on whichever call happens first. `compylr compyle` imports
every module beneath the root — a decorator only registers when it runs — so module-level code
executes; environments, caches, and build output are skipped. It imports 21 modules here,
including a nested subpackage, which is the only place in this repository that path is exercised
end to end.

## The recursion bound, and why it is stated

`nth_prime/recursive.py` recurses once per **prime found**, not once per candidate integer. A
version doing the latter would reach a depth in the thousands for a modest `n`.

There is no tail-call elimination, and a stack overflow in compiled code is a **process abort**,
not a recoverable error — no traceback, no exception, nothing to catch. Measured here:

| n | result |
| --- | --- |
| 100,000 | 1,299,709 |
| 150,000 | process killed, SIGSEGV, no output |

The tests stay well below that. This is the first place the project meets a limit that is not a
subset restriction: the subset permits the program, and the machine does not.

## What the subset costs, as it shows up here

Each of these is a place where the obvious Python does not compile, and where a linter will
suggest the version that does not.

**There is no `not`, and no `and` or `or`.** `if not divisible:` becomes an `else`, and
`while j >= 0 and out[j] > key:` becomes a `while` with an `if` and a `break`. `text.most_common`
carries `# noqa: SIM102 - no and in the subset` for exactly this.

**There is no `min`, `max`, `sorted`, `sum`, or `abs`.** Only `len` and `range` are builtins.
`stats.extremes` is the loop `min` and `max` would have hidden, and five call sites here carry a
`noqa` because ruff suggests rewriting them with functions the compiler does not have.

**`append` is the only collection method.** No `pop`, no `insert`, no `add`, no `.items()`, no
`list.copy`, and no string methods at all. A queue is a list plus a read cursor — which is O(1)
where `pop(0)` is O(n), so it is what the interpreted version should have been anyway. A stack is
a list plus a `top` index, which is [`IntStack`](src/algorithms/structures.py).

**A `str` cannot be indexed or iterated.** No `s[0]`, no `for ch in s`, no `.split()`. What a
string *can* do is concatenate, compare, report its `len`, and answer `in`. So the unit of work in
`text.py` is the **word**, handed in as a `list[str]` that ordinary Python tokenised. That is the
first thing to fix if you want compylr for text.

**There is no swap.** `a, b = b, a` is not in the subset, so an exchange takes a temporary.
`sorting.selection_sort` is where that shows.

**There are no comprehensions and no slicing.** `merge_sort` builds its halves with a loop rather
than `xs[:mid]`, and `dynamic.table_of_zeros` is what `[[0] * n for _ in range(m)]` would have
been — which is also the honest version, since the comprehension allocates the same rows.

**A collection parameter is a copy and may not be mutated.** An in-place sort is therefore not
expressible — not because sorting in place is hard, but because the caller could never see the
result. Every sort here builds a fresh list and returns it.

**A loop cannot be a function's only exit.** compylr does not assume a loop body runs, so
`while True: ... return x` is rejected as having a path that produces no value.

**Marked names are shared across a whole project.** All three nth-prime variants naturally want to
be called `nth_prime`, and only one can be: they compile into one module. So each variant's
compiled functions carry a prefix and each module re-exports the readable name. At sixty-eight
members this is the constraint you feel most.

## The emitted Rust is committed

[`.compylr/crate/src/`](.compylr/crate/src) and [`.compylr/ir/unit.json`](.compylr/ir/unit.json)
are checked in, so you can read what compylr actually produces without installing a toolchain:

| file | what it is |
| --- | --- |
| `ir/unit.json` | the IR: every function and class, target-language neutral |
| `crate/src/generated.rs` | your code, translated — the file worth reading |
| `crate/src/compat.rs` | the semantics the IR declared, in Rust; identical in every project |
| `crate/src/bindings.rs` | the PyO3 boundary, including `#[pyclass]` for each class |

`target/` and `dist/` are not committed — they are rebuilt on demand. Neither is `state.json`,
which records one machine's last build.

Two caveats, since committed generated output invites both mistakes. It is a **snapshot**: rebuild
and it changes, so treat a diff there as output, not as something to edit. And
`crate/.cargo/config.toml` carries the linker flags for the platform it was generated on — macOS
here.

## Checks

```bash
make demo-check        # from the repository root: sync, precompile, test, lint, type-check

uv run pytest          # or piecemeal, from here
uv run ruff check .
uv run mypy src
```

The repository's own suite also builds this project, runs every algorithm, and asserts the
coverage claim, so the demo cannot rot unnoticed.
