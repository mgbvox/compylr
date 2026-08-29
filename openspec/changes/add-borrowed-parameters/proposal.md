## Why

Every parameter crosses by value and is owned. That is a documented, measured cost: on this machine
a text argument runs about **42 ns per element** to convert, against about 4 ns for an integer. A
function doing O(log n) work over an O(n) string therefore loses compiled — the copy dominates the
work.

Passing text as `&str` was built to fix exactly this, and **reverted**.
[`CLAUDE.md`](../../../CLAUDE.md) records why, and the reason is not that borrowing is wrong:

> Not mutating a value is not the same as tolerating a borrow of it, because a parameter can also be
> *stored*. Four ordinary shapes need an owned `String` and emitted Rust that did not compile —
> `xs.append(who)`, `d[k] = who`, `who < "m"`, and `who in xs`.

And the sentence that matters most: **"The whole suite passed while it was broken."**

The failed attempt borrowed *unconditionally*, on the theory that a parameter never mutated may be
borrowed. The four shapes are all cases where the parameter is not mutated and is nevertheless
**kept** — appended, stored under a key, compared across an impl boundary, searched for. Ownership
is not about mutation; it is about escape.

So the fix is not a better guess. It is to **decide per parameter, from the body**, and to default
to owned — which is what the repository already does for one kind of parameter. `self`'s mutability
and an instance parameter's mutability are decided by a fixpoint, deliberately shared, because "two
analyses would be free to disagree". The
[`borrowed_instance_*`](../../../frontends/python/fixtures/rejected/) family in the rejected corpus
is that analysis stated as user-visible rules, and `CLAUDE.md` already generalises it: **"A borrow
reaches further than the parameter name."**

This change extends that same fixpoint from *may this be mutated* to *may this be borrowed*.

**Why now.** `add-numpy-arrays` needs it. A numpy array is a pointer to a buffer allocated on the C
side, and inheriting that pointer instead of copying it is the entire reason to support numpy at
all. Building the ownership axis for arrays alone would put the riskiest change in the repository's
history inside a change about a new type, where the type would get the review.

## What Changes

- **DEPENDS ON `add-typed-ir-expressions`** (currently 0/43 tasks, not started). That change states
  the dependency from the other side: *"Knowing an expression's type is necessary for passing text
  as `&str` and is not sufficient... It gets its own change, with that test as the gate."* This is
  that change. It cannot start before that one lands, because deciding whether an argument may be
  borrowed requires knowing what an expression's type is.

- **A parameter carries a passing mode.** Owned, shared borrow, or mutable borrow. Owned is the
  default and the fallback; a borrow is an optimization the compiler proves, never something a user
  requests or is diagnosed about.

- **The mode is decided by escape analysis, folded into the existing fixpoint.** A parameter may be
  borrowed only where the body never lets it outlive the call: not returned, not stored in a
  collection or attribute, not appended, not bound to a name that escapes, and not passed to
  anything that needs it owned. Where any of these hold, the parameter is owned — silently, with no
  diagnostic, because the user did nothing wrong.

- **The four reverted shapes are the acceptance gate.**
  [`a_text_parameter_is_usable_in_every_position`](../../../crates/compylr-host-python/tests/execution.rs#L2728)
  exists because the suite passed while this was broken. It runs before and after, unchanged, and
  each of the four shapes gets a case asserting the parameter came out **owned**.

- **A borrow does not reach further than the call.** The rules the `borrowed_instance_*` fixtures
  already state for instances become general: a borrowed value may be read, mutated where the mode
  says so, and forwarded to something that borrows it compatibly, but may not escape into an owned
  return or into storage.

- **Honest accounting of what this actually saves.** Text is the win: a Python string's buffer is
  borrowed rather than copied. **A sequence or mapping parameter is not made free by this change** —
  a Python list is a list of objects, not a contiguous array, so the boundary must still convert it
  element by element whatever the Rust signature says. What borrowing removes for collections is
  the *internal* clone when one compiled function passes a collection to another. Saying so here
  matters, because the previous attempt's failure mode was believing a change was free when it was
  not.

- **BREAKING (artifact format).** `Param` gains a field, so
  [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) advances and caches rebuild once.

## Worked Example

Two functions, both taking one text parameter, differing only in what the body does with it. That
is the whole change: the same annotation, two conclusions, decided from the body.

**Input** — `ownership.py`:

```python
def is_long(word: str) -> bool:
    return len(word) > 3


def roster(who: str) -> int:
    names = ["ada"]
    names.append(who)
    return len(names)
```

**Today** — both are owned. This is real emitted Rust from
`cargo run -p compylr-cli -- --emit rust` at the tip of this branch:

```rust
pub fn is_long(word: String) -> Result<bool, RuntimeError> {
    Ok(((&(PyLen::py_len(&(word), TextUnits::CodePoints))) > (&(3i64))))
}

pub fn roster(who: String) -> Result<i64, RuntimeError> {
    let mut names: Vec<String> = vec![String::from("ada")];
    {
        let __compylr_value = who.clone();
        (names).push(__compylr_value);
    }
    Ok(PyLen::py_len(&(names), TextUnits::CodePoints))
}
```

`is_long` copies its entire argument in order to measure it and throw it away. `roster` copies it
too — and there the copy is *load-bearing*, because `push` keeps the value past the call.

**After** — the two diverge, and only the first one changes:

```rust
// expected — the mechanism does not exist yet
pub fn is_long(word: &str) -> Result<bool, RuntimeError> { /* ... */ }
// unchanged, and this is the point: `who` escapes into `names`, so it stays owned
pub fn roster(who: String) -> Result<i64, RuntimeError> { /* ... */ }
```

`roster` is exactly one of the four shapes that broke the reverted attempt. It is in the example
deliberately: a change that made *both* signatures borrow would be the reverted change, and would
pass any test that only checked answers.

**At the boundary** — nothing changes, which is the requirement:

```pycon
>>> import ownership
>>> ownership.is_long("ada")
False
>>> ownership.roster("grace")
2
```

Those are CPython's answers for this source, run while writing this proposal. No accepted program
changes its answer, no diagnostic is added, and a user cannot tell from the outside which mode a
parameter got — which is why the corpus has to assert the mode directly rather than the answer.

## Capabilities

### New Capabilities
- `parameter-passing`: what it means for a parameter to be owned or borrowed, how the mode is
  decided from the body, and how far a borrow may reach.

### Modified Capabilities
- `ir`: a parameter carries a passing mode; the artifact version advances; the fingerprint covers it.
- `ir-lowering`: the fixpoint decides ownership as well as mutability, defaulting to owned.
- `rust-backend`: a parameter is emitted by its declared mode, and a borrowed value is not cloned
  where it is read.
- `native-bridge`: the boundary converts a borrowed parameter without copying where the host's
  representation permits it.
- `generated-code-performance`: the text-argument conversion cost falls, and is measured.
- `fixture-corpus`: every shape that forces ownership is exercised, including the four that broke.

## Impact

**Modified**
- [`ir.rs`](../../../crates/compylr-ir/src/ir.rs) — `Param`'s mode, the artifact version, the
  fingerprint.
- [`lower.rs`](../../../crates/compylr-frontend-python/src/lower.rs) — the fixpoint, extended from
  mutability to ownership.
- [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs) — signatures and the clone-on-read
  rule.
- [`compylr-bridge-python-rust`](../../../crates/compylr-bridge-python-rust/src/lib.rs) — boundary
  conversion per mode.
- [`execution.rs`](../../../crates/compylr-host-python/tests/execution.rs#L2728) — the gate test,
  plus a case per forcing shape.
- [`README.md`](../../../README.md), [`CLAUDE.md`](../../../CLAUDE.md) — and `CLAUDE.md`'s note
  about the revert becomes a note about the rule.

**Unaffected**
- The accepted subset. No program that compiled stops compiling, and no answer changes.
- Every diagnostic. This change adds none: a parameter that cannot be borrowed is simply owned.

**Costs**
- One rebuild per project.
- The fixpoint gets harder to reason about, since it now decides two things. Mitigated by it being
  one analysis rather than two that could disagree — which is the reason it is one today.
