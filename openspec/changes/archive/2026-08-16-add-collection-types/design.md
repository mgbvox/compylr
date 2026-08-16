## Context

See proposal.md — Why. What the current code makes easy or hard:

* `Ty` derives `Copy` and is passed by value in about thirty signatures across `ir.rs`, `lower.rs`,
  and `backend/rust.rs`. Recursive types end that.
* Operator emission is already **type-directed by Rust rather than by the backend**: arithmetic is
  emitted as `PyNum::py_sub(&(a), &(b))?` and Rust selects the implementation. That trick does not
  extend to subscripting, because the *result* type differs per container, so the backend does need
  to know what it is indexing.
* Lowering already has the machinery this needs. `Option<Ty>` means "undetermined", promotion
  inserts explicit conversion nodes, and "annotation required only where the type is not
  determined" is an established rule that empty literals slot straight into.
* The backend already clones `String` at consuming sites so a name read twice is not moved. That
  rule generalises rather than being invented here.

## Goals / Non-Goals

**Goals:**

* Keep the IR target-neutral: `Vec` and `HashMap` appear only in the backend.
* Make the read-only subset genuinely usable — a function taking a list must be able to read from
  it and measure it, or the type buys nothing.
* Keep every divergence from Python either eliminated or written down.

**Non-Goals:**

* Mutation, iteration, comprehensions, slicing, membership.
* Performance of the boundary conversion. Copying a large list is a real cost, and the honest
  answer for now is to document it rather than to build a zero-copy view.

## Decisions

### D1. `Ty` becomes recursive and stops being `Copy`

```rust
pub enum Ty {
    Int, Float, Bool, Str, Unit,
    List(Box<Ty>),
    Dict(Box<Ty>, Box<Ty>),
    Set(Box<Ty>),
    Tuple(Vec<Ty>),
}
```

There is no way around this: a parameterised type contains a type. The cost is mechanical — every
`ty: Ty` becomes `ty: &Ty` or gains a `.clone()` — and the compiler finds all of it. It is called
out in the proposal because it makes the change touch files it is otherwise not about, and a
reviewer should expect that noise.

`Ty` keeps `Eq`, `Hash`, and `Ord`; all derive fine for a recursive enum. `Ord` matters because the
fingerprint hashes structure and stable ordering keeps it deterministic.

*Alternative considered:* interning types behind a `TyId` to keep `Copy`. Rejected — it adds an
arena and a lifetime to every signature to avoid clones of a type that is at most a few nodes deep.
That is a real optimisation for a compiler with thousands of types; this one has a closed set.

### D2. Key and element types are restricted at the IR level, not just in lowering

The mapping key and set element restriction to `Int`/`Str`/`Bool` is a property of the type model,
so it is checked when a type is constructed rather than only when an annotation is parsed. A
backend must be able to render every type the IR can hold; if `Dict(Float, Int)` were
representable, `HashMap<f64, i64>` would not compile, and the failure would surface as a rustc
error about `Eq` rather than as a diagnostic pointing at the user's annotation.

### D3. `len` is a distinct IR node, and the name is reserved

`Expr::Len(Box<Expr>)`, not a call. A call is resolved against the unit during validation, so
leaving `len` as a call would mean either resolving it to a function that does not exist, or
special-casing it during validation — and then `len` would mean different things depending on
whether someone had decorated a function of that name. Reserving the name and lowering to a
distinct node makes the meaning fixed.

*Alternative considered:* a general `Expr::Builtin { name, args }`. Rejected as premature: there is
exactly one builtin, and a general mechanism would need a builtin signature table before anything
needed it.

### D4. Subscripting is emitted through a helper for sequences, directly for tuples

A tuple index is a literal and its position is known at emission, so `t[1]` emits `t.1` and cannot
fail. A sequence or mapping index is a runtime value, so it goes through a helper that resolves
Python's semantics:

```rust
// sequences: negative indices count from the end
let len = xs.len() as i64;
let resolved = if i < 0 { i + len } else { i };
if resolved < 0 || resolved >= len { return Err(RuntimeError::IndexOutOfRange); }
xs[resolved as usize].clone()
```

The clone is what makes the read-only subset work without borrow plumbing: an element is handed
back as an owned value. For scalars this is free; for a nested collection it is a copy, which is
consistent with the by-value story at the boundary.

`RuntimeError` gains `IndexOutOfRange` and `MissingKey(String)`. The key is rendered as text
because `KeyError` in Python shows the key, and the alternative — making the error generic over the
key type — would infect every signature in the runtime for one message.

### D5. Length counts characters for strings

`s.chars().count()`, not `s.len()`. Rust's `len` is UTF-8 bytes and Python's is code points, so any
non-ASCII string silently disagrees. This is the same category of mistake as mapping `//` to `/`:
correct for ASCII, wrong in a way tests written in English will not catch. A non-ASCII case is
required in the tests for exactly that reason.

### D6. Cloning generalises from "is it a string" to "is it copyable"

The backend currently clones when the expected type is `Str`. That becomes: clone when the type is
not trivially copyable — every collection, and `Str`. Emitted code copies more than strictly
necessary, and the alternative is borrow inference in a code generator, which is where correctness
goes to die. If it becomes a measured problem, elision at return sites is the contained fix.

### D7. Mappings use `HashMap`, and the ordering divergence is accepted

**Chosen deliberately, against the recommendation, and recorded so the trade is visible.**

Python dicts iterate in insertion order. `HashMap` does not, and because its hasher is randomly
seeded per process, the order also *varies between runs*. A caller who iterates a returned dict,
compares `list(d)`, or snapshots one in a test will see non-deterministic behavior — the failure
mode is a test that passes locally and fails in CI, which is expensive to diagnose because nothing
in the user's code changed.

The specification therefore states the divergence as behavior rather than leaving it implicit, and
the tests assert on *contents* rather than order so they do not themselves become flaky.

Reversing this later is contained: it is the map type in `rust_ty`, the literal construction in the
backend, and the `HashMap` import in the emitted runtime. No IR change, no lowering change, no
spec change beyond deleting one requirement. If insertion order is wanted, swapping in an
order-preserving map is a single-file change plus one dependency in the generated crate.

### D8. Empty literals reuse the existing "undetermined" rule

`xs = []` has no elements, so its type is undetermined, so it requires an annotation — the same
sentence that already governs `b = helper(a)`. Nothing new is invented; `lower_expr` returns
`(Expr, None)` for an empty literal and the existing binding rule produces the diagnostic.

## Risks / Trade-offs

* **Non-deterministic dict ordering reaches users** → D7. Specified as behavior, tests assert on
  contents, and the reversal path is documented. This is the sharpest known risk in the change.
* **`Ty` churn touches unrelated code** → Entirely compiler-caught, so the risk is review fatigue
  rather than defects. Landing the `Ty` change as its own commit keeps the diff readable.
* **Boundary conversion is O(n)** → A compiled function over a large list pays a copy in and a copy
  out, which can exceed the compute saved. Worth measuring on a realistic list before claiming a
  speed-up anywhere.
* **Clone-everything is slower than it needs to be** → D6. Accepted for correctness; contained if
  it matters.
* **Element type errors surface at the boundary, not at the call site** → PyO3 reports a
  `TypeError` when a list element fails to convert. The message names the position, which is enough
  to act on.

## Migration Plan

No existing program changes meaning: every annotation that lowers today still lowers, and no
fingerprint changes for a program that uses no collections. Caches stay valid across the upgrade.

## Open Questions

* Whether `len` should also accept a `bytes` type once one exists. Deferrable — it changes one match
  arm and no structure.
