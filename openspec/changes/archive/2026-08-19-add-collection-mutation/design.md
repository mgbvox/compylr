## Context

See proposal.md — Why. The relevant current behaviour:

* The backend clones collections at consuming sites, so a name read twice is not moved. That rule
  is exactly wrong for a mutation target.
* `add-control-flow` introduced a mutability scan over assignment targets. This change extends the
  same scan rather than adding a second one.
* `python-bindings` already states that collections cross by value, and that the divergence is
  unobservable only because nothing can mutate. That sentence was written for this change.

## Goals / Non-Goals

**Goals:**

* Make the accumulate-into-a-collection shape work, since that is most of what loops are for.
* Keep the by-value divergence unobservable, rather than documenting a wrong answer.

**Non-Goals:**

* Any method beyond `append`. A general method-call mechanism needs a signature table, and one
  method does not justify one.
* Reference semantics across the boundary. That is a distribution and lifetime problem, and it is
  what would have to be solved before a parameter could be mutated.

## Decisions

### D1. Mutation is confined to locals

The alternative is a silent wrong answer. A compiled `def f(xs): xs.append(1)` would receive a copy,
mutate it, and return — leaving the caller's list untouched, where the interpreted function they
were replacing would have changed it. Nothing raises. The test they write passes locally against
the interpreted version and fails once compiled.

Rejecting mutation of parameters makes that program not exist. The diagnostic has to explain *why*,
not merely refuse: "a collection parameter is a copy, so this mutation could not be observed by the
caller" tells the user both the rule and the workaround, which is to build a local and return it.

*Alternative considered:* allow it and document loudly. Rejected — a documented wrong answer is
still a wrong answer, and this project has consistently chosen to make divergences unreachable
where it can (negative indices, `//`, `len` on strings) rather than describe them.

*Alternative considered:* copy the mutated parameter back out across the boundary. Rejected — it
only works for a function whose parameter is not aliased, and PyO3 would have to reconstruct and
reassign a Python object the caller still holds. That is reference semantics with extra steps.

### D2. The clone rule gains a mutation-aware exception

The backend currently clones a collection wherever it is consumed. For a mutation target that is
worse than wasteful: `xs.clone().push(v)` compiles and does nothing.

So emission needs to know which locals are mutated, which is the same information `add-control-flow`
already collects for `let mut`. The scan is extended to include element-assignment and append
targets, and the clone is suppressed for exactly those names.

### D3. Assignment inserts; reading does not

`d[k] = v` emits `insert`; `d[k]` emits the checked read that reports a missing key. They are
different operations that share a spelling in Python, and conflating them would either make reads
silently create entries or make assignments fail on a new key.

### D4. Membership is a trait, like subscripting

`PyContains<T>` implemented for `Vec<T>`, `HashMap<K, V>` keyed on `K`, `HashSet<T>`, and `String`
over `&str`. Emission is `PyContains::py_contains(&(c), &(v))` and Rust picks the implementation —
the same reason arithmetic and subscripting are traits: the IR does not annotate expressions with
their types, and re-deriving them in the backend would duplicate the type checker.

Mapping membership tests keys and string membership tests substrings, both matching Python. Neither
is what a naive `contains` would do for a map, so both are worth a test that would fail if someone
"simplified" the implementation.

### D5. Append is its own IR form, not a method call

There is exactly one supported method. A general `MethodCall { receiver, name, args }` form would
need a table of method signatures per type before anything consumed it, and every backend would
have to decide what an unknown method means. An explicit `Append` form cannot be called with the
wrong name, and the rejection for every other method stays a single diagnostic that names it.

When a second method arrives — `pop`, or `add` for sets — that is the moment to generalise, and the
generalisation will have two examples to be shaped by instead of none.

## Risks / Trade-offs

* **The parameter rule will frustrate someone** → `def f(xs: list[int]) -> None: xs.append(1)` is
  natural Python and is rejected. Mitigated by a diagnostic that explains the copy and points at
  building a local and returning it. Revisit only alongside reference semantics.
* **The clone exception is easy to get subtly wrong** → If the scan misses a mutation target, the
  generated code compiles and silently does nothing, which is the failure mode hardest to notice.
  Tests must assert on *observed values after mutation*, never on emitted text.
* **String membership is a substring test, not an element test** → Correct for Python and
  surprising to a reader who expects character membership. Worth a test naming the intent.
* **Mutation plus iteration is undefined here** → Mutating a collection while iterating it is not
  rejected by this change and Rust's borrow checker will refuse it, so the failure would be a rustc
  error rather than a diagnostic. Left as a known gap; the honest fix is a lowering rule, and it
  belongs with whatever change first makes it reachable.

## Migration Plan

Nothing to migrate: the change only accepts programs that were previously rejected. Fingerprints
for programs that do not mutate are unchanged, so caches stay valid.
