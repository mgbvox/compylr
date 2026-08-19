## Context

See proposal.md — Why. What the current code assumes, and what a class breaks:

* `Unit` holds `BTreeMap<String, Function>`, and everything that consumes a unit iterates
  functions. A class is a second kind of member.
* `Ty` is a closed, structural set. An instance type is **nominal** — two classes with identical
  attributes are different types — which is the first non-structural type in the model.
* Emission clones a collection wherever it is consumed. An object holding a collection makes that
  rule reachable through a field, where cloning the object to read one field would be much worse
  than wasteful.
* PyO3's `#[pyclass]` keeps state on the Python side, which is what makes a cache work across
  calls; nothing else in the binding layer has needed it.

## Goals / Non-Goals

**Goals:**

* State that outlives a call, scoped to an object rather than to a module.
* A mutable-receiver rule derived correctly, so generated code borrows the way a reader expects.
* Keep every existing fingerprint stable for units containing no classes.

**Non-Goals:**

* Inheritance, properties, class attributes, dunders beyond `__init__`, `@dataclass`.
* Aliasing between instances, or an instance held inside another instance's collection. Both raise
  ownership questions the read-only-parameter rule currently sidesteps.

## Decisions

### D1. `Ty::Instance(String)` — a nominal type in a structural model

Every other type is structural: `list[int]` equals `list[int]` because of what it contains. An
instance type equals another only when the class *name* matches. That is the first place the model
carries a name rather than a shape, and it has two consequences worth stating:

* Two classes with identical attributes are different types, which is what a user means by writing
  two classes.
* A type is only meaningful relative to the unit that defines the class. Serializing an artifact
  therefore carries the class definitions alongside, which it already would, because classes are
  unit members.

Instances are excluded from `can_key` — no defined hash or ordering — and from
`is_trivially_copyable`, so the existing clone-where-consumed rule applies to them.

### D2. Attributes are declared in `__init__`, with annotations, or not at all

A struct's fields cannot depend on which methods ran. Requiring every attribute to be annotated in
`__init__` is the same rule already applied to parameters and returns, and it makes the class's
shape readable in one place.

The rule is stricter than Python in a way users notice, so the diagnostic for an undeclared
attribute must say *where* to declare it rather than merely that the attribute is unknown.

### D3. Mutable receivers are derived, transitively

A method needs `&mut self` when it assigns an attribute, mutates a collection attribute, **or calls
a method that does**. The transitive case is the one that will be got wrong: a method whose body is
only `self.record(x)` mutates through the call, and emitting `&self` there produces a borrow error
about generated code.

So the analysis is a fixpoint over the class's methods: mark the directly-mutating ones, then
repeatedly mark any method calling a marked one, until nothing changes. A class has few methods, so
this is a handful of passes over a small set.

*Alternative considered:* `&mut self` everywhere. Rejected — two methods could not then be used on
one object in the same expression, and the resulting error would be about the compiler's output
rather than the user's program.

### D4. `#[pyclass]` carries the state, and it lives on the Python side

The object Python holds *is* the compiled struct, wrapped by PyO3. Method calls borrow it from the
Python object, so mutation persists between calls without compylr owning any lifetime.

This is what makes a memoized class work, and it is worth contrasting with collections: a
collection **argument** is converted by value at the boundary, but an instance is not converted at
all — Python holds the Rust value itself. The by-value divergence therefore does not apply to
`self`, which is exactly why an attribute can be a cache while a parameter cannot be mutated.

### D5. Classes and functions share a namespace, and one build

A unit refuses a class whose name is taken by a function. They compile into the same generated file
and the same extension module, so a collision would be a Rust collision; catching it in the unit
gives a diagnostic instead.

`Unit` gains a second map, and its ordering and fingerprint guarantees extend over both. A unit
containing no classes must fingerprint exactly as it does today, or every existing cache
invalidates on upgrade for no reason — so the class map contributes nothing when empty.

### D6. Construction is its own expression form

`Counter()` is not a function call. Leaving it as one would mean unit validation resolving it
against functions, and a class and function of the same name are already refused — but the type
rules differ enough (arguments check against `__init__`, the result is an instance type) that a
distinct form keeps both paths simple. It also mirrors what already happened with `len` and
`range`: a thing that looks like a call in Python and is not one in the IR.

## Risks / Trade-offs

* **The mutable-receiver fixpoint is the likeliest source of bugs** → Its failure mode is a
  borrow-checker error about generated code, which is exactly the kind of message this project
  tries never to produce. Tests must include the transitive case and a method that reads while
  another mutates.
* **Nominal typing in a structural model** → Every place that compares types now has one case where
  names matter. Worth stating in `ir.rs` where `Ty` is defined, because a reader will assume
  structural equality throughout.
* **An instance inside another instance's collection is unexplored** → `list[Counter]` is
  representable by D1 and its ownership story is not worked out. Excluded from scope; the honest
  position is that the type is expressible and the operations on it are not, and lowering should
  say so rather than emit something that fails to compile.
* **Fingerprint stability is load-bearing** → If the empty class map contributes to the hash, every
  cached build invalidates on upgrade. A test must pin that a class-free unit's fingerprint is
  unchanged.

## Migration Plan

No existing program changes meaning, and a unit containing no classes fingerprints exactly as
before, so caches stay valid across the upgrade.

## Open Questions

* Whether a method should be callable on an instance held in a collection. Deferrable: the type is
  representable, but no operation reaches it, so nothing depends on the answer yet.
