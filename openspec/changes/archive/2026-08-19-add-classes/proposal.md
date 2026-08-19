## Why

Everything compylr compiles is a free function over values. There is no way to hold state between
calls, so the shape most worth compiling — a computation with a cache — cannot be written:

```python
@c.compyle
class PrimeCache:                    # class definitions are not supported
    def __init__(self) -> None:
        self._cache: dict[int, int] = {}
```

Mutation exists now, but only over locals, which vanish when the function returns. A memoized
function needs somewhere for the cache to live that outlives a call, and the subset has no
module-level state — top-level statements other than `def` are rejected, deliberately.

An object is the natural home. It also gives mutable state a scope, rather than introducing globals
into generated code, where they would need a synchronisation story before anything could use them.

## What Changes

- Add **class definitions** carrying an `__init__` and methods, marked with the same decorator.
- Add **instance types**: a class name becomes a type usable in annotations, including inside
  collections, so `list[PrimeCache]` works.
- Add **attributes**, declared in `__init__` with mandatory annotations, on the same terms as every
  other binding. Attributes are **mutable** — that is what the cache needs.
- Add **methods**, taking `self` first. A method that mutates an attribute is distinguished from
  one that does not, because the two compile to different things.
- Add **attribute access and assignment**: `self.x` and `self.x = v`, and `obj.x` from outside.
- Add **construction**: `PrimeCache()` is a call whose type is the instance type.
- **BREAKING (internal)**: a unit stops being a collection of functions and becomes a collection of
  functions **and classes**. Anything walking a unit changes shape.

Explicitly **not** in this change: inheritance, `@property`, class attributes and class methods,
`__slots__`, dunder methods other than `__init__`, `@dataclass`, instances crossing into
collections owned by other instances, and any form of aliasing between instances.

## Capabilities

### New Capabilities

None — this widens five existing capabilities. A class is a new kind of *thing in a unit*, not a
new area of behaviour: it lowers, types, emits, binds, and is decorated exactly where functions
already do.

### Modified Capabilities

- `ir`: a unit gains classes; the type model gains an instance type; expression forms gain
  attribute access and construction; statement forms gain attribute assignment.
- `ir-lowering`: class definitions, `__init__`, methods, `self`, attributes, and construction gain
  rules; the fingerprint covers a class's structure.
- `rust-backend`: a class emits a struct and an implementation block; methods that mutate take a
  mutable receiver.
- `python-bindings`: a class is exposed to Python as a type whose methods are callable and whose
  constructor works.
- `python-api`: the decorator accepts a class.

## Impact

- **The unit's shape changes, and so does its fingerprint.** `Unit` currently holds a
  `BTreeMap<String, Function>`. Classes share the same namespace — a class and a function cannot
  have the same name — so the unit holds both, and the fingerprint must cover a class's methods and
  attributes. Every existing fingerprint stays the same for a unit containing no classes, which
  keeps caches valid.
- **Mutable receivers are a real decision, not a detail.** A method that assigns to an attribute
  needs `&mut self`; one that does not needs `&self`. Emitting `&mut self` everywhere would make
  two methods unable to be called on the same object, so the distinction has to be derived, and
  derived correctly — the failure mode is a borrow-checker error rather than a diagnostic.
- **Attributes must be declared in `__init__`.** Python allows an attribute to appear anywhere.
  Requiring every one to be annotated in `__init__` is the same rule the subset already applies to
  parameters, and without it a struct's fields would depend on which methods happened to be called.
- **Instance identity does not survive the boundary.** A compiled object handed to Python and back
  is subject to the same by-value story as collections. What that means for `self` — which must be
  the *same* object across calls for a cache to work — is the central design question, and PyO3's
  `#[pyclass]` is what makes it answerable.
- **Code**: `src/ir.rs` (a `Class` type, `Ty::Instance`, attribute and construction expressions),
  `src/lower.rs` (the largest share), `src/backend/rust.rs` and `bindings.rs`, and
  `python/compylr/_manager.py` for the decorator.
- **Ordering**: third of five. Depends on `add-collection-mutation` for a mutable cache attribute
  to be worth anything.
