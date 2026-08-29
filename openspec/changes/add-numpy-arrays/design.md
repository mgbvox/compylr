## Context

See proposal.md — Why. What makes this change possible at all is that numpy's buffer is contiguous
(or regularly strided) memory of a single element type, which is what Rust's array views already
expect. Nothing else crossing the boundary has that property.

Constraints:

* `add-borrowed-parameters` supplies the ownership axis. A view *is* a borrow; without a passing
  mode there is nothing to emit.
* The mutability fixpoint already decides whether a receiver or instance parameter is mutable. An
  array parameter is decided by the same analysis.
* Collections cross by value and may not be mutated. Arrays deliberately do not follow that rule,
  and the difference must be explained where a user meets it.
* [`Ty`](../../../crates/compylr-ir/src/ir.rs#L103) has one integer type and one float type, by
  deliberate choice.
* Emission is a pure function of the unit; anything about the caller's memory is bridge work.

## Goals / Non-Goals

**Goals:**
- An array parameter that is a view over the caller's buffer, never a copy, at any size.
- Writes through a mutable array parameter visible to the caller.
- Rank and storage in the type, so indexing and emission are decided statically.
- Memory safety at the boundary, including the aliasing case.

**Non-Goals:**
- Array creation, array returns, and storing arrays. Deferred, and refused with a diagnostic naming
  the deferred capability.
- Vectorized whole-array arithmetic and broadcasting. `a + b` over two arrays is a ufunc, not the
  scalar `+` the IR carries; conflating them would give one of the two the wrong meaning.
- Slicing as an expression. Slicing is already refused and stays refused; a slice produces a view,
  which is the escaping case.
- `float32` and `int32`. Reserved and diagnosed as planned.
- Structured dtypes, object arrays, masked arrays, and anything non-numeric.

## Decisions

### 1. An array is a new `Ty` carrying storage and rank

**Decision.** Add a variant to `Ty` in which both facts are static:

```rust
// before — every aggregate type carries an element type and nothing else
List(Box<Ty>), Dict(Box<Ty>, Box<Ty>), Set(Box<Ty>), Tuple(Vec<Ty>),
// after — an array also carries how many axes it has, because indexing depends on it
Array { storage: Storage, rank: u8 },

pub enum Storage { Float64, Int64 }
```

**Why.** Indexing, the shape tuple's length, and the emitted view type all depend on the rank.
Discovering it at runtime would mean the emitted signature could not be chosen, so it is declared.

**Alternatives considered.** *Dynamic rank everywhere.* A dynamically-ranked view compiles, but then
`a[i, j]` cannot be checked at lowering, the shape tuple has no length, and every index costs a
runtime rank check. The subset's premise is mandatory annotations; this is where that premise pays.
*Reuse `List` with a marker.* Then every rule written for a by-value collection would silently apply
to a view, including the mutation rule that this change deliberately reverses.

#### The IR, in both faces

The definition delta is above. The value, for the worked example's `dot`, as the JSON `--emit ir`
writes. The envelope is real output from the tip of this branch; the `Array` type and the `passing`
field are `expected`:

```json
{
  "version": 6,
  "functions": [
    {
      "name": "dot",
      "params": [
        { "name": "a", "ty": { "Array": { "storage": "Float64", "rank": 1 } }, "passing": "Shared" },
        { "name": "b", "ty": { "Array": { "storage": "Float64", "rank": 1 } }, "passing": "Shared" }
      ],
      "ret": "Float"
    },
    {
      "name": "scale",
      "params": [
        { "name": "values", "ty": { "Array": { "storage": "Float64", "rank": 1 } },
          "passing": "Mutable" },
        { "name": "factor", "ty": "Float", "passing": "Owned" }
      ],
      "ret": "None"
    }
  ],
  "origin": { "frontend": "python", "requires": ["IntegerOverflowReported", "FloatOrderPreserved"] }
}
```

The five questions:

- **Neutrality.** `Storage` names a buffer element width, not numpy's dtype and not Rust's `f64`.
  `ArrayView1` appears only in [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs), and
  the numpy spelling only in the frontend's
  [`spelling.rs`](../../../crates/compylr-frontend-python/src/spelling.rs). Nothing named `numpy`
  reaches `compylr-ir`, which is what keeps
  [`crate_boundaries.rs`](../../../crates/compylr-host-python/tests/crate_boundaries.rs) true.
- **Mode or form?** A new **type**, which is the third answer the mode-or-form question admits. An
  array is not a differently-checked list and not a differently-shaped operation: it has its own
  representation, its own mutation rule, and its own boundary behavior. Rank and storage within it
  are *modes* on that type, for the same reason a checking mode is a mode.
- **Format version.** [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) advances. This
  change is last in the chain, so it takes the number after `add-borrowed-parameters`.
- **Fingerprint.** [`Unit::fingerprint`](../../../crates/compylr-ir/src/ir.rs#L1299) must cover
  storage and rank. Both change the emitted signature, so both are on the covered side of the
  pre-pass line.
- **Coverage.** A new `Ty` trips
  [`demo_coverage.rs`](../../../crates/compylr-host-python/tests/demo_coverage.rs), which reads the
  IR's enum definitions and fails when a type appears that the demo's tables do not list. Paid with
  an array algorithm in the demo — which the change wants anyway, since the demo is where the
  copy-elimination claim gets measured.

### 2. Storage is a property of the buffer; reads widen

**Decision.** Reading an element yields the model's existing scalar type:

```python
a: compylr.Array1[np.float64]
x = a[0]          # x is `float` — the model's Float, not a width
```

**Why.** The IR has one integer type and one float type on purpose. Adding `float32` as a *scalar*
type would introduce widths into a model that deliberately has none, and every backend and every
operator rule would have to answer for them. Storage describes the buffer only, and a read yields
`Int` or `Float` — which is also exactly what CPython does when you read a numpy scalar into Python.
Restricting storage to `float64` and `int64` here follows from the same reasoning: with those two, a
read is a widening of nothing and a write needs no narrowing rule.

**Alternatives considered.** *Add `float32` now.* It means answering what writing an out-of-range
value into a narrower cell means — a genuine semantic question, and not one this change must answer
to eliminate the copy.

### 3. Mutation is visible to the caller, and the contrast is documented at the diagnostic

**Decision.** The existing collection diagnostic is extended to name the neighbouring rule:

```text
error: 4:9: 'values' is a parameter, and a collection parameter is a copy — this mutation could not
be observed by the caller. Build a local collection and return it instead
                                                    ^ extended to add: an array parameter is a view
                                                      over the caller's buffer and may be mutated
```

**Why.** This is the change's most surprising rule, because it contradicts the collections rule a
user has already learned. The justification is representational, not a preference: a `list[int]`
parameter is converted element by element into a Rust `Vec`, so a mutation could not be observed; an
array parameter is a *view onto the caller's memory*, so a mutation is observed by construction.
That is the same distinction the codebase already draws between a collection and an *instance*.
Arrays join instances on the second side. Because a rule that contradicts another rule is where
users lose confidence, the explanation is put where they meet it —
[`CLAUDE.md`](../../../CLAUDE.md)'s standard: "a rule without its reason leaves the user no
workaround."

**Alternatives considered.** *Make arrays copy, for consistency with collections.* Consistent and
pointless: the copy is the entire cost this change exists to remove.

### 4. Strided views, not enforced contiguity

**Decision.** The emitted view type is the strided one:

```rust
ArrayView1<f64>      // strided, so a[::2] and a matrix column stay zero-copy
```

**Why.** Requiring C-contiguity would be simpler to emit and would silently copy exactly the arrays
people slice — `a[::2]`, a column of a matrix, a transposed view. Since the whole point is not
copying, supporting strided views is the requirement rather than an optimization. Rust's array views
are strided already, so this costs a view type, not an algorithm.

**Alternatives considered.** *Require contiguity and copy otherwise.* The failure is silent and hits
precisely the users who know what a view is.

### 5. Aliasing is checked at the boundary, and it is a safety requirement

**Decision.** The bridge compares memory ranges before compiled code runs:

```python
scale_both(v, v)   # RuntimeError: array arguments overlap, and one is mutated
```

**Why.** `f(a, a)` with one parameter mutably bound produces two Rust references to one buffer, one
mutable. That is **undefined behavior**, not a wrong answer — the compiler is entitled to assume it
cannot happen, and the symptom would be a miscompilation that appears under optimization and not
under debug. Nothing in the type system catches it, because both arguments are well-typed. The check
runs only when it can matter — more than one array parameter, at least one mutable — so the common
single-array call pays nothing.

**Alternatives considered.** *Copy on overlap.* Silently copying would make a call that looks
zero-copy occasionally not be, with no way for the user to know which. *Trust the user.* The failure
mode is undefined behavior in generated code, which is the single worst outcome this repository's
design principles are organized against.

### 6. Arrays are parameters only, in this change

**Decision.** An array-typed return is refused, naming the deferred capability. A scoping decision
with no type of its own.

**Why.** Returning an array requires creating one, which requires array creation operations, an
owned array type crossing back to the host, and a decision about who owns the allocation. That is a
coherent follow-on change. Not having it is less limiting than it sounds, because in-place output
through a parameter is numpy's own idiom — `out=` exists throughout numpy's own API — and reductions
to scalars return normally.

**Alternatives considered.** *Ship creation in this change.* It doubles the surface and puts the
ownership question for a heap allocation next to the ownership question for a borrow.

### 7. Partial indexing is refused

**Decision.** `a[i, j]` is required on a rank-two array; `a[i]` is refused.

```python
a[i, j]     # accepted
a[i]        # error on a rank-2 array: a partial index yields a view, which cannot escape
```

**Why.** `a[i]` on a rank-two array yields a row *view* in numpy. A view is a borrow that would
outlive the expression, which is the escaping case `add-borrowed-parameters` establishes. This also
removes a trap: in numpy `a[i][j]` and `a[i, j]` differ in cost, and refusing the first means nobody
writes the slow one by accident.

**Alternatives considered.** *Materialize the row.* It is a copy, in the change whose purpose is
removing copies.

## Risks / Trade-offs

**Undefined behavior through aliasing** → The boundary check, specified as a requirement and
exercised by a corpus case. This is the risk that justifies the check's cost.

**Holding a view across a GIL release would be unsound** → Compiled code does not release the host
runtime's lock while a view is live. Nothing in the generated code releases it at all today, so this
is a constraint to preserve rather than one to add — and it is worth stating, because releasing it
is the obvious future optimization for a long-running array kernel.

**A user expects vectorized arithmetic and gets scalars** → `a + b` over two arrays is refused
rather than silently meaning something else. The refusal names ufuncs as unsupported, so the
diagnostic teaches the boundary instead of implying arrays are broken.

**Two mutation rules to learn** → Mitigated by extending the existing collection diagnostic to name
the contrast, so a user meets the explanation at the moment they hit the rule.

**numpy becomes a build-time requirement** → Only for programs that use arrays, and reported as a
located setup failure naming numpy, joining `cargo` and `maturin` on the list of things compiling
needs at runtime.

**Reduction order may differ from numpy's** → numpy uses pairwise summation for `sum`; a naive loop
does not, and the last bits differ. Fixtures compare floating-point answers within a stated
tolerance, and the difference is a property of the arithmetic rather than a defect.

**The artifact version collides with the other in-flight changes** → This change is last in the
chain, so it takes the number after `add-borrowed-parameters`.

## Migration Plan

The artifact version advances; caches are refused once and rebuilt automatically off the recorded
compylr version. No existing program changes meaning: the array type is additive, and every existing
type keeps its rules.

The generated crate gains two pinned dependencies, but only for programs that use arrays, so
projects that do not are unaffected including in build time.

Rollback is removing the change. Programs using arrays stop compiling with an unsupported-annotation
diagnostic; no program that compiled without arrays is affected.

## Open Questions

- Whether `len` on a rank-two array should be the first extent, matching numpy, or refused as
  ambiguous. The spec takes numpy's meaning, matching what the interpreted program returns; if the
  differential corpus shows users reaching for it expecting the total element count, refusing it is
  a diagnostic change and not a spec change.
