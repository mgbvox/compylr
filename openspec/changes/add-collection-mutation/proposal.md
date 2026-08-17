## Why

Collections are read-only, so nothing can be built up:

```python
@c.compyle
def primes_below(limit: int) -> list[int]:
    found: list[int] = []
    for n in range(2, limit):
        found.append(n)     # only simple name targets are supported in assignments
    return found
```

Control flow made loops possible; without mutation a loop can only compute a scalar. The
accumulate-into-a-collection shape is most of what loops are for.

Membership is the same gap from the other side: `if key in cache` cannot be written, so a cache
cannot be consulted even if one could be filled.

## What Changes

- Add **element assignment**: `xs[i] = v` and `d[k] = v`.
- Add **`append`** on sequences. It is the one method worth having before a general method-call
  mechanism exists, because it is what turns a loop into a builder.
- Add **membership**: `x in xs`, `k in d`, `x in s`, and `not in`. Included here rather than with
  collections because it is what makes a mutable cache usable, and shipping mutation without it
  would leave the memoized case still unwritable.
- **Mutation applies to locals only.** Assigning to an element of a *parameter* SHALL be rejected.
  See Impact — this is the significant decision in the change.
- A collection that is mutated is bound mutably, and one that is not is not, on the same terms
  reassignment established for scalars.

Explicitly **not** in this change: any other method (`extend`, `pop`, `insert`, `remove`, `update`,
`add`, `discard`), deletion, slicing, comprehensions, or mutation through an alias.

## Capabilities

### New Capabilities

None — this widens four existing capabilities.

### Modified Capabilities

- `ir`: statement forms gain element assignment; expression forms gain membership; a call form for
  the one supported method.
- `ir-lowering`: element assignment, `append`, and membership gain type rules; mutating a parameter
  is rejected.
- `rust-backend`: emission of element assignment, `append`, and membership, with a missing key
  inserted rather than reported — assignment creates, unlike a read.
- `python-bindings`: the by-value divergence becomes reachable and must be restated as something a
  caller can observe.

## Impact

- **The by-value divergence stops being hypothetical.** The collections change specified that
  collections cross the boundary by value and noted the difference was unobservable "because
  nothing in the supported subset can mutate", adding that "adding mutation has to confront it
  deliberately". This is that moment.

  A compiled function receiving a `list[int]` gets a copy. If it could mutate that copy, the
  caller's list would be silently unchanged — an interpreted function would have modified it. That
  is a wrong answer with no error, which is the worst failure this project can ship.

  So mutation is confined to **locals**. A parameter cannot have its elements assigned, which makes
  the divergence unreachable rather than merely documented. The cost is that
  `def f(xs: list[int]) -> None: xs.append(1)` is rejected; the alternative is that it compiles and
  does nothing, which is worse.
- **Cloning has to be revisited.** The backend clones a collection wherever it is consumed, so a
  name read twice is not moved. Mutating a clone is a no-op — a collection that is mutated must be
  bound once and mutated in place, so the clone rule needs a mutation-aware exception.
- **Dictionary assignment inserts.** Reading a missing key is a `KeyError`; assigning to one is
  not, in Python. The emitted code must therefore not go through the read helper.
- **Code**: `src/ir.rs`, `src/lower.rs` (mutation targets, membership typing, the parameter rule),
  `src/backend/rust.rs` and `runtime.rs` (membership over three container types), and the
  `python-bindings` spec.
- **Ordering**: second of five. Depends on `add-control-flow` for the loops that make mutation
  worth having; `add-classes` depends on this for a mutable attribute.
