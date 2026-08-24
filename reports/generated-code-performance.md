# Speeding up the generated code

A benchmark run and a ranked brainstorm, 2026-08-21. Every number below is measured on this
machine (Apple silicon, CPython 3.14, `--release`), not estimated. Each candidate was tested by
hand-patching `demo/.compylr/crate/` and rebuilding, so the deltas are real rather than argued.
The crate was restored to its committed state afterwards; **no compiler source changed in this
branch.**

Reproduce the baseline with `make demo SCALE=4`.

## The baseline, and where it loses

`make demo SCALE=4`, compiled against interpreted:

| workload | compiled | interpreted | speedup |
| --- | ---: | ---: | ---: |
| matrices.multiply | 151.70us | 4080.35us | 26.9x |
| arithmetic.collatz_length | 0.21us | 4.62us | 21.6x |
| dynamic.knapsack | 39.77us | 754.29us | 19.0x |
| arithmetic.sieve | 2.75us | 32.50us | 11.8x |
| structures.component_count | 17.89us | 203.97us | 11.4x |
| stats.standard_deviation | 15.68us | 53.32us | 3.4x |
| sorting.merge_sort | 202.06us | 647.80us | 3.2x |
| sorting.insertion_sort | 13.70us | 43.68us | 3.2x |
| dynamic.edit_distance | 537.34us | 1323.00us | 2.5x |
| stats.normalize | 34.15us | 69.15us | 2.0x |
| graphs.topological_order | 445.98us | 782.20us | 1.8x |
| matrices.transpose | 48.59us | 59.56us | 1.2x |
| **text.joined** | **352.74us** | **325.02us** | **0.9x** |
| **graphs.bfs_distances** | **167.65us** | **114.32us** | **0.7x** |
| **text.word_count** | **210.13us** | **56.53us** | **0.3x** |

The three bold rows are the interesting ones: compiling them makes them *slower*. They are not
outliers, they are three different systemic costs, and each is fixable.

## Ranked, cheapest first

### 1. The generated crate has no `[profile.release]` at all — 10-25%, one config line

`crates/compylr-bridge-python-rust` writes a `Cargo.toml` with `[package]`, `[lib]`, and
`[dependencies]`, and nothing else. So every user's crate builds at `codegen-units = 16`,
`lto = false`. `compat.rs` and `generated.rs` are separate modules, and at 16 codegen units the
runtime helpers frequently land in a different unit from their only caller — which means
`PyAdd::py_add`, `py_index`, `resolve_index` and friends are *not inlined*. Given that every
arithmetic operation in the subset is a trait call, that is a lot of un-inlined calls.

Adding this to the generated manifest:

```toml
[profile.release]
lto = "fat"
codegen-units = 1
```

Measured, SCALE=4:

| workload | before | after | |
| --- | ---: | ---: | ---: |
| structures.component_count | 17.89us | 13.43us | 1.33x |
| stats.standard_deviation | 15.68us | 12.06us | 1.30x |
| matrices.transpose | 48.59us | 39.92us | 1.22x |
| stats.normalize | 34.15us | 29.33us | 1.16x |
| sorting.insertion_sort | 13.70us | 11.90us | 1.15x |
| arithmetic.sieve | 2.75us | 2.48us | 1.11x |
| matrices.multiply | 151.70us | 138.66us | 1.09x |

Cost: the generated crate's build went from ~7s to ~10s. That is paid once per fingerprint
change, and the artifact is imported thousands of times.

**Rejected on measurement:** `-C target-cpu=native`. No row moved outside noise, and it would
make a copied `.compylr/` directory illegal-instruction on a different machine. Not worth it.

`panic = "abort"` is *not* available — PyO3 needs unwinding to turn a panic into a Python
exception.

### 2. `x = x + y` on a string is quadratic where CPython's is not — 4.1x on one workload

`text.joined` emits this per word:

```rust
out = PyAdd::py_add(&(PyAdd::py_add(&(out), &(separator))?), &(word))?;
```

`PyAdd for String` allocates a fresh `String` and copies both sides, so that is **two full copies
of the accumulator per iteration**. CPython special-cases `s = s + t` when the target holds the
only reference and resizes in place, which makes the interpreted version amortized linear. So
this is a case where compiling a loop makes it asymptotically worse.

Patching the two lines to `out.push_str(&separator); out.push_str(&word);`:

| workload | before | after | |
| --- | ---: | ---: | ---: |
| text.joined | 343.76us | 83.08us | **4.1x** |

That moves the row from 0.9x (losing to the interpreter) to 3.8x.

The shape of the fix: recognise `Assign { name, value: Binary { op: Add, left: Name(name), .. } }`
— an accumulator that reads itself — and emit an in-place update. Keep it type-directed the way
everything else is, via a `PyAddAssign` trait: `String` gets `push_str`, `i64` gets the checked
add it has now. The backend still never learns the operand's type.

Worth noting `text.joined`'s docstring already says "It is also quadratic — each `+` builds a new
string — which `str.join` is not. Compiling something is not the same as making it fast." That is
true of the *algorithm*, but the generated code is currently quadratic with a constant factor of
two on top, and that part is ours.

### 3. Every mapping and set uses SipHash — up to 1.9x on map-heavy code

`rust_ty` emits bare `HashMap`/`HashSet`, so generated code inherits `RandomState`. SipHash is
the right default for a map whose keys arrive over a network; it is the wrong default for a map
whose keys come from the user's own program. CPython, meanwhile, hashes small ints to themselves
and caches a string's hash in the string object — so for `dict[int, ...]` compylr is doing
strictly more work per lookup than the interpreter it is supposed to beat.

Adding a self-contained FxHasher to `runtime.rs` (about 40 lines, no external crate, in keeping
with the file's "must stay self-contained" rule) and defaulting generated maps to it:

| workload | before | after | |
| --- | ---: | ---: | ---: |
| graphs.bfs_distances | 159.36us | 82.49us | **1.93x** |
| graphs.topological_order | 421.48us | 271.33us | 1.55x |
| text.word_count | 189.51us | 166.44us | 1.14x |

`bfs_distances` goes from 0.7x to **1.4x** — from losing to winning.

**This surfaced a latent design bug.** The runtime's impls are written as
`impl<K, V> PyIndexable<K> for std::collections::HashMap<K, V>`, which silently pins `S =
RandomState`. The hasher is not currently a *choice* — it is baked into ten trait impls. Making
them generic over `S` is a small mechanical change (`impl<K, V, S: BuildHasher> ... for HashMap<K,
V, S>`) and is worth doing on its own merits, whatever hasher ends up the default. Once it is
generic, the hasher is a natural `TargetOption` on the Rust backend, next to
`unchecked-arithmetic`.

One knock-on: `HashMap::from([...])` only exists for `RandomState`, so set and dict literals need
to emit `FromIterator` instead. Trivial, but it is an emitter change, not just a runtime one.

### 4. `for x in xs` clones every element — 33% off a trivial loop

`PyIterate for Vec<T>` is `self.iter().cloned()`, and `collection_loop` binds the clone into an
owned local. For `Vec<i64>` that is free. For `Vec<String>` it is an allocation and a copy per
element, per loop.

`text.total_length` is the cleanest measurement available — its body is one `len()` per word:

| N=2000 | as generated | iterating by reference | |
| --- | ---: | ---: | ---: |
| text.total_length | 88.52us | 59.43us | **1.49x** |

The loop variable is only read there, and lowering already knows whether the body assigns to it —
`is_assigned` is computed a few lines above, purely to decide `mut`. The same answer says whether
a borrow would do.

**What blocks it:** the runtime traits are implemented on owned types only, so a `&String` loop
variable fails to satisfy `PyLen`. Fixing it needs blanket `impl<T: PyLen + ?Sized> PyLen for &T`
(and the same for `PyContains`, `PyAdd`) or a deref at each use site. That is the real work here;
the emitter change is three lines.

### 5. A returned local is cloned on the way out — 25 sites out of 25 in the demo

`Stmt::Return` routes through `emit_expr`, which clones any non-`Copy` `Expr::Name`. So
`return counts` emits `Ok(counts.clone())` — a full deep copy of the collection, immediately
before the function ends and the original is dropped.

Removing all 25 in the demo compiled clean and changed no answer. The gain is modest here
(`topological_order` 271us → 243us) because the demo's returned collections are small relative to
its loops, but it is a pure win and it scales with result size.

**Safety:** it is not unconditionally sound — `for v in xs: return xs` would move out of something
the loop borrows. Restricting the move to *tail position* (the `emit_body` tail branch, which is
the last statement at depth 0 and so cannot be inside any loop) is trivially safe and covers all
25 sites.

Related, same file: `d[k] = v` emits `let __compylr_index = k.clone();` and then `py_set` does
`self.insert(key.clone(), value)`. That is **two** allocations per dict write where one is needed.

### 6. The boundary is O(n) per collection argument — and it is the ceiling for everything else

This is the biggest one, and the only one that is not a local fix.

A collection parameter is converted element by element on every call. Timed with a fixed N=2000:

| probe (N=2000) | compiled | interpreted | |
| --- | ---: | ---: | ---: |
| `binary_search(list[int])` — O(log n) body | 8.12us | 0.49us | **16x slower** |
| `is_sorted(list[int])` — O(n) body | 8.61us | 64.55us | 7.5x faster |
| `mean(list[float])` — O(n) body | 9.83us | 16.59us | 1.7x faster |
| `total_length(list[str])` — O(n) body | 83.58us | 32.33us | 2.6x slower |
| `copy_of(list[int])` — in **and** out | 28.43us | 16.15us | 1.8x slower |

Per element, roughly:

- `list[int]` in: **~4 ns** — cheap, `PyLong_AsLongLong` and no allocation.
- returning a `list[int]`: **~10 ns** — a fresh `PyLong` per element.
- `list[str]` in: **~42 ns** — a Rust `String` allocated and UTF-8 copied per element. **Ten times
  the int cost**, and the whole reason every `list[str]` workload in the demo loses.

`binary_search` is the sharpest statement of the problem: compiling it converts 2000 elements to
do eleven comparisons, turning an O(log n) algorithm into an O(n) one and losing to the
interpreter by 16x. No amount of codegen quality fixes that.

Directions, roughly in cost order:

- **Borrow read-only string parameters.** A `str` parameter that is never mutated does not need an
  owned `String`. PyO3 can hand back the interned UTF-8 without allocating (`Cow<'_, str>`, or a
  borrowed `&str` with a lifetime on the generated function). This alone removes the 10x str
  penalty, and read-only is already the subset's rule for parameters — the compiler *knows* the
  parameter is never mutated, because mutating one is a rejected program.
- **Stop converting per call.** An instance behind `#[pyclass]` already crosses once and stays
  Rust-side; that is why an attribute can be a cache. A compylr-owned list the user holds across
  calls would extend the same trick to free functions, so a pipeline of compiled calls converts
  once instead of once per hop.
- **The buffer protocol** for numeric lists, where the caller can supply `array`/`memoryview`.
- **Cheapest of all: say so.** A collection parameter costs O(n) even when the body is O(1). That
  is currently written down nowhere, and `demo/README.md` reads as though compiled is always at
  least as fast. One paragraph would stop the next person losing an afternoon to it.

### 7. Smaller runtime items, worth a sweep

- **`py_index` bounds-checks twice.** `resolve_index` proves `resolved < length`, then
  `items[resolved]` checks again. The `i64`/`usize` round trip probably defeats LLVM's ability to
  elide the second. `get(..).unwrap_or_else(unreachable)` or restructuring `resolve_index` to
  return the element would fix it without `unsafe`.
- **`py_str_len` under `CodePoints` is O(n) per call.** `value.chars().count()` decodes the whole
  string every time `len(s)` is evaluated. An `is_ascii()` fast path returning `len()` is exact
  when it hits, and ASCII is the common case. (This is a real share of `total_length`'s residual
  59us.)
- **Three hash lookups per `d[k] = d[k] + 1`.** `k in d`, then `d[k]`, then `d[k] = `. The
  `entry` API does it in one. Combined with the borrow-iterate fix, a hand-written ideal body for
  `word_count` runs in **62.31us against 167.27us as generated** — 2.7x, and level with CPython's
  56us. The remainder is the boundary from item 6.
- **No `with_capacity` anywhere.** `vec![]` then `push` in a loop whose trip count is often a
  `range` bound that is already in hand.

## A measurement problem worth fixing first

`sorting.merge_sort` returned 202, 277, 235, 256, 264, and 160 us **across runs of binaries that
were in some cases byte-identical**. It is recursive and allocation-heavy, and best-of-5-batches
is not enough to stabilise it. Any conclusion drawn from that row today is noise — I discarded my
own apparent "regression" on it for that reason.

Before optimising against this benchmark, it is worth making the harness report a spread rather
than a single best, so a real 10% regression is distinguishable from this.

## Where each one lives

| # | change | site |
| --- | --- | --- |
| 1 | add `[profile.release]` | `crates/compylr-bridge-python-rust/src/bindings.rs:283` (`cargo_manifest`) |
| 2 | `x = x + y` peephole | `crates/compylr-backend-rust/src/rust.rs:920` (`Stmt::Assign`), `runtime.rs:157` (`PyAdd for String`) |
| 3 | hasher | `rust.rs:74` (`rust_ty`), plus the ten `HashMap`/`HashSet` impls in `runtime.rs` |
| 4 | borrow-iterate | `rust.rs:1087` (`collection_loop`), `runtime.rs:623` (`PyIterate for Vec`) |
| 5 | return-site move | `rust.rs:842` (`emit_body` tail), `rust.rs:1397` (`emit_expr` name clone) |
| 6 | the boundary | `crates/compylr-bridge-python-rust/src/bindings.rs` (signature emission) |
| 7 | runtime sweep | `runtime.rs:299` (`py_index`), `runtime.rs:339` (`py_str_len`) |

## Suggested order

Items 1, 2 and 5 are self-contained, testable, and touch no semantics — they are a single change
each. Item 3 is a small runtime refactor with a real payoff and an existing latent bug to fix on
the way. Item 4 needs the blanket impls first. Item 6 is a design change and deserves its own
OpenSpec proposal; the documentation half of it can land immediately.

Cumulative effect of items 1, 2, 3 and 5 as measured together, SCALE=4:

| workload | baseline | patched | |
| --- | ---: | ---: | ---: |
| text.joined | 352.74us | 83.08us | **4.2x** |
| graphs.bfs_distances | 167.65us | 80.87us | **2.07x** |
| graphs.topological_order | 445.98us | 245.69us | **1.82x** |
| structures.component_count | 17.89us | 13.45us | 1.33x |
| stats.standard_deviation | 15.68us | 12.17us | 1.29x |
| matrices.transpose | 48.59us | 39.38us | 1.23x |
| text.word_count | 210.13us | 178.74us | 1.18x |
| dynamic.knapsack | 39.77us | 34.11us | 1.17x |
| sorting.insertion_sort | 13.70us | 11.70us | 1.17x |
| stats.normalize | 34.15us | 30.26us | 1.13x |
| arithmetic.sieve | 2.75us | 2.47us | 1.11x |
| matrices.multiply | 151.70us | 136.53us | 1.11x |

Two of the three workloads that lost to the interpreter now win. `text.word_count` is the one
that still loses, and item 6 is why.
