## Why

A numpy array is a pointer to a contiguous, C-allocated buffer. That is the whole reason numpy is
fast, and it is the one argument shape where compylr's boundary cost can go to **zero** instead of
merely down.

Everything else crosses by value and must. A Python list is an array of object pointers, so the
boundary walks it and converts element by element whatever signature the target uses —
`add-borrowed-parameters` says so explicitly rather than claiming a saving it cannot make. A numpy
array is the exception: the bytes are already laid out the way Rust wants them, so the generated
code can take a *view* over the caller's buffer and never copy at all.

The cost of not doing this is not a constant factor. [`CLAUDE.md`](../../../CLAUDE.md) records the
boundary at roughly **4 ns per element** for integers — so a function handed a million-element array
pays milliseconds before it computes anything, and "a body doing O(log n) work over an O(n) argument
can therefore lose compiled." For array workloads, copying at the boundary means the compiled
version is slower than numpy, which is the one outcome that makes the whole tool pointless for the
users most likely to want it.

Rust already has the counterpart: `ndarray` for the array type and the `numpy` crate for the
zero-copy binding, which hands back a view over the same allocation.

**Why last.** This change needs an ownership axis in the IR — a view *is* a borrow — and that axis
was built once and reverted. `add-borrowed-parameters` builds it as its own reviewable change with
the reverted work's own test as the gate. Building it here instead would bury the riskiest change in
the repository's history inside a change about a new type, where the type would get the review.

## What Changes

- **DEPENDS ON `add-borrowed-parameters`**, and transitively on `add-typed-ir-expressions`. A
  zero-copy array parameter is a borrowed parameter; without that mode there is nothing to emit.

- **A new IR type: an array of a declared element storage and a declared rank.** Rank is part of the
  type, not discovered at runtime, because indexing, shape, and the emitted view type all depend on
  it.

- **Rank must be written down.** `compylr.Array1[np.float64]` and `compylr.Array2[np.float64]` are
  the accepted spellings; a bare `np.ndarray` or an unranked `NDArray[...]` is rejected naming the
  ranked form. This is the rule
  [`bare_list_annotation.py`](../../../frontends/python/fixtures/rejected/bare_list_annotation.py)
  already states for `list`: an element type — here also a rank — that is not written down is not a
  type compylr can use.

- **Storage is `float64` and `int64` in this change.** `float32` and `int32` are reserved and
  diagnosed as planned. Supporting them means defining what assigning an out-of-range value into a
  narrower cell means, which is a real semantic question and not one this change needs to answer to
  deliver the copy elimination.

- **Reading an element widens to the existing scalar types.** Storage describes the buffer; a read
  yields the IR's `Int` or `Float` from [`Ty`](../../../crates/compylr-ir/src/ir.rs#L103), exactly
  as CPython yields a Python scalar. This keeps the new type from introducing integer widths into a
  model that deliberately has one.

- **Arrays are parameters, not returns, in this change.** A function may read and mutate array
  parameters and return a scalar. Returning or constructing an array needs array *creation*
  (`zeros`, `empty`, `arange`) and an owned array type crossing back — a coherent follow-on, and
  not needed for the stated goal, since in-place output through a parameter is numpy's own idiom.

- **Mutation is decided by the existing fixpoint, and is visible to the caller.** A function that
  writes to an array parameter borrows it mutably; one that only reads borrows it shared. Both are
  zero-copy. This is a **deliberate, documented divergence** from the collections rule: a `list[int]`
  parameter crosses by value and may not be mutated, while an array parameter is a view onto the
  caller's buffer, so a write is visible next line. The two rules differ because the underlying
  representations differ, which is the same reason an *instance* is already treated differently from
  a collection.

- **Strided views are supported, not just contiguous ones.** A slice like `a[::2]` is a strided view
  of the same allocation, and taking it as a strided view keeps it zero-copy. Requiring contiguity
  would silently copy exactly the arrays users slice.

- **Aliasing is checked at the boundary.** Calling `f(a, a)` where one parameter is mutably borrowed
  would create two Rust references to one buffer, one of them mutable — undefined behavior, not
  merely a wrong answer. The bridge compares the underlying buffers and raises when a mutably
  borrowed parameter overlaps another array parameter.

- **BREAKING (artifact format).** A type is added, so
  [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) advances.

## Worked Example

Two functions over the same array type: one reads and reduces to a scalar, one writes in place. That
pair is the whole change — a shared view, a mutable view, and the mutation rule that contradicts the
one users already learned for collections.

**Input** — `arrays.py`:

```python
import numpy as np
import compylr


def dot(a: compylr.Array1[np.float64], b: compylr.Array1[np.float64]) -> float:
    total = 0.0
    i = 0
    while i < len(a):
        total = total + a[i] * b[i]
        i = i + 1
    return total


def scale(values: compylr.Array1[np.float64], factor: float) -> None:
    i = 0
    while i < len(values):
        values[i] = values[i] * factor
        i = i + 1
```

**Today** — the import stops it, and the second function would be refused even without one. Both
are real runs against the CLI at the tip of this branch:

```text
$ cargo run -p compylr-cli -- arrays.py
error: 1:1: imports are not supported; only function definitions may appear at top level

$ cargo run -p compylr-cli -- arrays.py    # imports deleted, annotation changed to list[float]
error: 4:9: 'values' is a parameter, and a collection parameter is a copy — this mutation could not be observed by the caller. Build a local collection and return it instead
```

That second diagnostic is the one this change has to sit beside without contradicting. It stays
exactly true for `list[float]`, and it is extended to name the contrast, because a rule that
silently reverses for a neighbouring type is where users stop trusting the compiler.

**After** — the two functions take views, and the difference between them is decided from the body:

```rust
// expected — the mechanism does not exist yet
pub fn dot(a: ArrayView1<f64>, b: ArrayView1<f64>) -> Result<f64, RuntimeError> { /* ... */ }
pub fn scale(values: ArrayViewMut1<f64>, factor: f64) -> Result<(), RuntimeError> { /* ... */ }
```

Neither signature copies anything. `ArrayViewMut1` is what makes `scale`'s write land in the
caller's buffer.

**At the boundary** — the reduction answers a scalar and the in-place write is visible on return:

```pycon
>>> import numpy as np, arrays
>>> a = np.array([1.0, 2.0, 3.0]); b = np.array([4.0, 5.0, 6.0])
>>> arrays.dot(a, b)
32.0
>>> v = np.array([1.0, 2.0, 3.0])
>>> arrays.scale(v, 2.0)
>>> v
array([2., 4., 6.])
```

Those two answers are numpy's own, run while writing this proposal (`a @ b` and `v *= 2.0`), so they
are what the fixture's driver compares against rather than expected values. The second one is the
divergence stated as a transcript: the caller's `v` changed, which the same code over a
`list[float]` parameter could never do.

## Capabilities

### New Capabilities
- `array-values`: the array type, how rank and storage are declared, what indexing and shape mean,
  and the guarantee that an array parameter is a view over the caller's buffer.

### Modified Capabilities
- `ir`: an array type carrying storage and rank; the artifact version advances.
- `ir-lowering`: the ranked annotation is accepted and the unranked one refused; indexing and shape
  are typed from the rank; array returns are refused naming the deferred capability.
- `rust-backend`: array parameters emit as array views, and element access emits without bounds
  work beyond the declared checking mode.
- `native-bridge`: an array parameter binds to the caller's buffer without copying, and overlapping
  mutable parameters are refused.
- `generated-code-performance`: an array argument costs no per-element conversion, and this is
  measured.
- `build-pipeline`: the generated crate declares the array and binding dependencies, and a missing
  numpy at build time is reported as a located setup failure.
- `fixture-corpus`: arrays are exercised against numpy as the oracle, including a mutation observed
  by the caller.

## Impact

**Modified**
- [`ir.rs`](../../../crates/compylr-ir/src/ir.rs#L103) — the array type, the artifact version, the
  fingerprint.
- [`lower.rs`](../../../crates/compylr-frontend-python/src/lower.rs) — the annotation, indexing,
  shape, and refusals.
- [`spelling.rs`](../../../crates/compylr-frontend-python/src/spelling.rs) — how an array type is
  quoted back.
- [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs) — view types and element access.
- [`compylr-bridge-python-rust`](../../../crates/compylr-bridge-python-rust/src/lib.rs) — the
  zero-copy binding and the aliasing check.
- The generated manifest — the array and numpy-binding dependencies.
- [`frontends/python/`](../../../frontends/python/), [`README.md`](../../../README.md),
  [`CLAUDE.md`](../../../CLAUDE.md).

**New**
- `compylr.Array1` / `compylr.Array2` annotation aliases in the Python package, written so `ty`
  accepts them.

**Unaffected**
- Every existing type, and the rule that collections cross by value and may not be mutated. This
  change adds a type with different rules; it does not change the rules of the existing ones.

**Costs**
- One rebuild per project.
- The generated crate gains two dependencies and a build-time requirement on numpy's headers, which
  joins `cargo` and `maturin` on the list of things compiling needs at runtime.
- A second mutation rule for users to learn. Mitigated by the diagnostic on a mutated collection
  parameter, which already explains the by-value rule and can now name the contrast.
