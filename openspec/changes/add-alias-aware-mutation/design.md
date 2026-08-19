## Context

See proposal.md — Why. The relevant current behaviour:

* `add-collection-mutation` rejects mutating a parameter, using a set of parameter names carried on
  the lowering context (`Ctx::params`). The check fires when a mutation target is a bare `Expr::Name`
  in that set.
* Binding is `Stmt::Bind`, and lowering already knows the initializer's type. Nothing records where
  a value *came from*.
* The rejected case and the accepted case differ by one line, and both produce a compiled function
  whose behaviour is identical — it is the *caller's* observation that differs.

## Goals / Non-Goals

**Goals:**

* Close the alias hole completely, including transitively, so the rule cannot be defeated by adding
  bindings.
* Keep the diagnostic actionable: name the parameter, not only the local.

**Non-Goals:**

* Reference semantics across the boundary. That is what would have to be solved before a parameter
  could be mutated at all, and it is a distribution and lifetime problem rather than a compiler one.
* Aliasing between two locals. Neither is observable by the caller, so no divergence exists and
  rejecting it would refuse correct programs.
* Aliasing through a container — a list of lists, an attribute. Nothing in the subset can produce
  one yet; `add-classes` is where that becomes reachable, and the rule belongs there.

## Decisions

### D1. Origin is tracked on the binding, not inferred at the mutation

The alternative is to walk backwards from a mutation target to whatever bound it, which means
keeping the statements around and re-deriving a chain that lowering already had in hand. Instead
each binding records an origin at the moment it is created, and the mutation check is still a set
lookup.

The origin is the parameter a local ultimately came from, or nothing. `copied = xs` gives `copied`
the origin `xs`; `alias = copied` copies `copied`'s origin, which is what makes the relation
transitive without a second pass. Any other initializer — a literal, a call, an expression — gives
no origin, because it produced a fresh value.

*Alternative considered:* a general dataflow pass. Rejected — the subset has no way to produce an
alias except by binding one name directly to another, so the chain is exactly as long as the
bindings and a map lookup is the whole analysis.

### D2. The origin lives beside the type, in the scope frame

The scope already maps a name to its type per frame, and origin has the same lifetime: it is
introduced with the binding and gone when the block ends. Storing it separately would mean two
structures that must be pushed, popped, and looked up in lockstep, which is a bug waiting to be
written once.

Reassignment updates the origin, which is what makes the last scenario work: `copied = xs` then
`copied = []` leaves `copied` with no origin, and mutating it afterwards is safe. That falls out of
treating the origin as part of what a binding records rather than as a fact about a name.

*Alternative considered:* reject conservatively — once a name has ever aliased a parameter, it may
never be mutated. Simpler, and wrong in a way users would hit: rebinding a name to a fresh
collection is exactly the workaround the diagnostic recommends, and it would be refused.

### D3. Only collection parameters are tracked

A scalar has no mutation to observe, and marking every alias of every parameter would make the
diagnostic fire on programs with no hazard. Origin is recorded only when the parameter's type is a
collection — the types that cross by value and can be mutated.

This is worth stating because the check is cheap and the temptation is to skip it. A user who binds
`n = count` and later reassigns it should never see a word about aliasing.

## Risks / Trade-offs

* **A user hits the rule and reads it as arbitrary** → The refusal points at a local they just
  wrote, so the diagnostic must name the parameter and say what to do: build a fresh collection and
  fill it. Without that, the fix is not discoverable.
* **The tracking is quiet when it is wrong** → An origin that fails to propagate reopens the hole
  silently: the program compiles and diverges, which is the same failure this change exists to
  close. The transitive scenario is the test that catches it, and it must assert on a chain of at
  least two bindings.
* **It refuses a program that is genuinely safe** → `copied = xs; copied.append(1)` where the caller
  never looks at its list again is harmless, and is now rejected. Accepted deliberately: the
  compiler cannot see the caller, and the cost is one explicit copy.

## Migration Plan

Nothing shipped depends on the behaviour being reversed — `add-collection-mutation` is not released.
The pytest that documents the divergence flips from asserting it to asserting its absence, and the
scenario blessing alias mutation is replaced rather than added to. Fingerprints do not move: this
rejects programs, and never changes the IR of one it accepts.
