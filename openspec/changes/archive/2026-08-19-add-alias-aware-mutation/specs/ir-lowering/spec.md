## MODIFIED Requirements

### Requirement: Mutation is confined to locals

Lowering SHALL reject mutating a collection that arrived as a **parameter**, whether by element
assignment or by appending, and SHALL reject mutating any local that **aliases** one. A local
aliases a parameter when it is bound directly to that parameter, or to another local that aliases
it; the relation is transitive. The diagnostic SHALL explain that a collection parameter is a copy,
so the mutation could not be observed by the caller, and where an alias is involved SHALL name both
the local and the parameter it came from.

Collections cross the boundary by value. A compiled function mutating a parameter would leave its
caller's collection unchanged, where an interpreted function would have modified it — a wrong
answer with no error.

Aliasing is the same hazard at one remove. In Python, binding a name to a collection does not copy
it, so `copied = xs` leaves both names denoting one object and mutating either is observable to the
caller. Under compylr's value semantics the bind is a copy, so the caller sees nothing. Permitting
it because "the local is the function's own value" is true of the emitted code and false of the
Python it claims to translate.

A collection built locally and returned is unaffected, which is the shape mutation exists to
enable. Copying a parameter's contents explicitly — building a fresh collection and filling it — is
also unaffected, and is the workaround the diagnostic points at.

#### Scenario: A local collection may be mutated

- **WHEN** lowering a body that binds an empty sequence, appends to it, and returns it
- **THEN** lowering succeeds

#### Scenario: A parameter may not be mutated

- **WHEN** lowering a body that appends to one of its sequence parameters
- **THEN** lowering fails, explaining that the parameter is a copy and the caller would not see it

#### Scenario: Assigning into a parameter is rejected

- **WHEN** lowering a body that assigns to an element of a mapping parameter
- **THEN** lowering fails

#### Scenario: Reading a parameter is unaffected

- **WHEN** lowering a body that reads elements of a parameter without mutating it
- **THEN** lowering succeeds

#### Scenario: A local aliasing a parameter may not be mutated

- **WHEN** lowering a body that binds a local to a collection parameter and then mutates the local
- **THEN** lowering fails, because in Python the local and the parameter denote one object and the
  caller would have observed the mutation

#### Scenario: The diagnostic names the alias and its origin

- **WHEN** mutating a local that aliases a parameter is rejected
- **THEN** the diagnostic names both the local and the parameter it came from, because a refusal
  pointing only at a local the user just created gives them no reason to look at the signature

#### Scenario: Aliasing is transitive

- **WHEN** lowering a body that binds one local to a parameter, a second local to the first, and
  mutates the second
- **THEN** lowering fails, because otherwise one more binding defeats the rule

#### Scenario: Copying a parameter's contents explicitly may be mutated

- **WHEN** a body builds a fresh collection, fills it from a parameter, and mutates it
- **THEN** lowering succeeds, because the fresh collection is not the parameter under any semantics

#### Scenario: Aliasing a non-collection is unaffected

- **WHEN** a body binds a local to a scalar parameter
- **THEN** lowering succeeds and nothing about it is restricted, because a scalar has no mutation
  to observe

#### Scenario: A local that stops aliasing may be mutated

- **WHEN** a body binds a local to a parameter, rebinds it to a fresh collection, and then mutates
  it
- **THEN** lowering succeeds, because after the rebinding the local no longer denotes the caller's
  collection
