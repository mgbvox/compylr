## Why

Every type compylr supports is a scalar. That is enough to demonstrate a pipeline and not enough
to compile anything anyone would want compiled — the functions worth moving to Rust are the ones
doing work over data, and the data arrives in a list.

```python
@c.compyle
def total(xs: list[int]) -> int:      # UnsupportedType: list[int]
    return xs[0] + len(xs)
```

The type model was deliberately closed and flat while the pipeline was being proved out. It now
has a Rust backend, PyO3 bindings, and a build pipeline behind it, so the constraint has stopped
buying simplicity and started being the reason the tool cannot be used.

## What Changes

- Add **parameterised collection types** as annotations: `list[T]`, `dict[K, V]`, `set[T]`, and
  `tuple[T, ...]`, nested to any depth. **BREAKING** in one narrow sense: `Ty` stops being a flat,
  copyable enum, which is a change every consumer sees.
- Add **collection literals**: `[1, 2, 3]`, `{"a": 1}`, `{1, 2}`, `(1, "a")`. Element types must
  agree; a literal whose elements disagree is a type error, not a union.
- Add **subscripting**: `xs[i]` on a list, `d[k]` on a dict, and `t[0]` on a tuple. A tuple index
  SHALL be a literal, because each position has its own type and a computed index has no single
  answer.
- Add **`len`** on lists, dicts, sets, tuples, and strings. It is the one builtin worth having
  before iteration exists, since without it a list parameter cannot even be measured.
- **Negative indices work**, because they work in Python. `xs[-1]` is the last element, which
  Rust's native indexing does not do — the same class of divergence as `//` and `%`, and handled
  the same way.
- Add **`IndexError` and `KeyError`** to the failures a compiled function can raise, alongside the
  existing `ZeroDivisionError` and `OverflowError`.
- **Dict keys and set elements are restricted to `int`, `str`, and `bool`.** Floats are excluded:
  Rust's `f64` implements neither `Eq` nor `Hash`, and a float dict key is a hazard in Python too.
- **Collections cross the boundary by value.** A compiled function receiving a `list[int]` gets a
  copy, so a mutation inside it could never be visible to the caller. Nothing in this subset can
  mutate, so the divergence is currently unobservable — it is specified now so that it is a stated
  decision rather than an accident discovered when mutation lands.
- **A returned `dict` does not preserve insertion order.** Python dicts iterate in insertion order;
  the Rust map this compiles to does not, and its order varies between runs. See Impact.

Explicitly **not** in this change: mutation of any kind (`append`, `xs[0] = v`, mutable bindings),
iteration and comprehensions, slicing, `in`/membership, any builtin other than `len`, and
collections of `float` as dict keys or set elements.

## Capabilities

### New Capabilities

None — this widens four existing capabilities.

### Modified Capabilities

- `ir`: the type model becomes recursive, gaining parameterised list, dict, set, and tuple types;
  expression forms gain collection literals, subscripting, and length.
- `ir-lowering`: collection annotations become supported; new requirements cover literal element
  unification, subscript typing, tuple-index constancy, `len`, and the key/element restriction.
- `rust-backend`: concrete spellings for the four collection types; emission of literals,
  subscripts, and length; Python indexing semantics including negative indices; and index and key
  failures as recoverable errors.
- `python-bindings`: conversion of collections in both directions, the two new exception types,
  and the by-value and ordering divergences stated as behavior.

## Impact

- **`Ty` stops being `Copy`.** It currently derives `Copy` and is passed by value at roughly thirty
  sites across `ir.rs`, `lower.rs`, and the backend. Every one becomes a borrow or a clone. This is
  mechanical and caught by the compiler, but it is the single largest source of churn in the change
  and it touches files this change is otherwise not about.
- **Dict ordering is a real, chosen divergence.** A compiled function returning `dict[str, int]`
  hands back a dict whose iteration order is arbitrary *and varies between runs*, because the
  underlying map is randomly seeded per process. Python guarantees insertion order. Code that
  iterates a returned dict — or compares it to a literal by `list(d)`, or snapshots it in a test —
  will be non-deterministic. This was chosen deliberately over an order-preserving map for
  implementation simplicity; design.md records what switching later would cost, because the choice
  is contained and reversible.
- **`len` on a string is character count, not byte count.** Python counts code points; Rust's
  `String::len` counts UTF-8 bytes. `len("é")` is 1 in Python and 2 in Rust, so the emitted code
  must count characters or it is silently wrong on any non-ASCII input.
- **Clone pressure.** The backend already clones `String` at consuming sites so a value used twice
  is not moved. Collections need the same treatment, and the existing rule — "clone when the
  expected type is `Str`" — generalises to "clone when the type is not `Copy`", which is now most
  types. Emitted code will copy more than strictly necessary; correctness first.
- **Code**: `src/ir.rs` (recursive `Ty`, new `Expr` variants), `src/lower.rs` (annotation parsing
  for subscripted generics, element unification, subscript and `len` typing), `src/backend/rust.rs`
  (spellings and emission), `src/backend/runtime.rs` (indexing helper, two new error variants),
  `src/backend/bindings.rs` (exception mapping).
- **`len` must be reserved.** Recognising `len(x)` as a builtin while a user could define their own
  `len` in the same unit would make meaning depend on what else was decorated. A unit function named
  `len` is rejected.
- **Ordering against the other proposals**: this is the largest of the three and benefits from
  `add-deferred-quick-wins` landing first. Call-typed inference makes `xs = build_list()` work
  without an annotation, and collections make call-typed initializers far more common than they are
  today.
