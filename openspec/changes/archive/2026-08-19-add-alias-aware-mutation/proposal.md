## Why

`add-collection-mutation` rejects mutating a parameter, because collections cross the boundary by
value and a mutated parameter could not be observed by the caller — a wrong answer with no error.
One extra line defeats the rule:

```python
@c.compyle
def f(xs: list[int]) -> list[int]:
    copied = xs          # in Python this is an alias, not a copy
    copied[0] = 99
    return copied
```

Interpreted, the caller's list becomes `[99, ...]`. Compiled, it does not. That is the same silent
divergence the parameter rule exists to prevent, reached by aliasing first.

That change's spec sanctions this explicitly, reasoning that "the local is the function's own
value". The reasoning holds under compylr's value semantics and **not** under Python's, where
binding a name to a collection does not copy it. The rule is therefore correct about one spelling
of the hazard and blind to the other.

## What Changes

- **Track aliases of parameters.** A local bound directly to a parameter, or to another such local,
  carries the parameter's origin. Mutating one SHALL be rejected on the same terms and with the
  same explanation as mutating the parameter itself.
- **BREAKING (in a change not yet released):** the scenario *"A local copied from a parameter may be
  mutated"* in `ir-lowering` is replaced by its inverse. Nothing shipped depends on it; the pytest
  that documents the divergence flips from asserting it to asserting its absence.
- **The diagnostic names the alias and the parameter it came from.** "`copied` aliases the parameter
  `xs`" is the missing half — without it the user sees a refusal pointing at a local they just
  created and has no reason to look at the signature.
- Copying a parameter **explicitly** stays available and becomes the documented workaround: building
  a fresh local and appending into it is already accepted and already unambiguous.

Explicitly **not** in this change: reference semantics across the boundary, aliasing between two
locals (neither is observable by the caller, so no divergence exists), or aliasing through a
container.

## Capabilities

### New Capabilities

None — this narrows one existing capability.

### Modified Capabilities

- `ir-lowering`: the mutation rule extends from parameters to their aliases, and one scenario
  blessing alias mutation is replaced.

## Impact

- **This is a rule getting stricter, so it can only reject programs that compile today.** Every such
  program is one whose compiled behaviour already differs from its interpreted behaviour, which is
  why the rejection is the fix rather than the cost.
- **The alias relation is transitive and must be, or one more line defeats it again.** `a = xs; b = a;
  b[0] = 1` is the same hazard at one further remove.
- **Reassignment interacts.** `copied = xs` followed by `copied = []` leaves `copied` holding the
  function's own value, and mutating it afterwards is safe. Whether to track that precisely or
  reject conservatively is the design question; see design.md.
- **Code**: `src/lower.rs` only — the origin tracking and the widened rejection. No IR change, no
  backend change, no fingerprint movement.
- **Ordering**: follows `add-collection-mutation`, and should land before `add-classes`, which
  introduces a second thing a local can alias.
