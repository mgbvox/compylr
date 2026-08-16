## MODIFIED Requirements

### Requirement: Unsupported functions are rejected when marked

A function outside the supported subset SHALL be rejected at the point it is marked, carrying
the diagnostic and its `line:column`. It SHALL NOT be accepted and then fail at first call, and
it SHALL NOT silently fall back to interpreted execution: the user asked for compilation, so
being told immediately that it cannot happen is the useful outcome.

**One category is deferred.** A binding whose initializer calls a function the marked source does
not define cannot be typed at this point, because each marked function is captured as its own
source and its callees live in others. Rejecting it here would demand an annotation for
`doubled = double(n)` in exactly the arrangement this API always produces. That single category
SHALL be deferred to the build, where every source is present and it can be typed — and SHALL be
reported there if it still cannot be.

Every other violation SHALL still be reported when the function is marked.

#### Scenario: Missing annotation

- **WHEN** a function with an unannotated parameter is marked
- **THEN** an error is raised naming the parameter and its location

#### Scenario: Unsupported construct

- **WHEN** a function containing a loop is marked
- **THEN** an error is raised naming the unsupported construct and its location

#### Scenario: Failure is immediate

- **WHEN** an unsupported function is marked
- **THEN** the error is raised at that point, before the function is ever called

#### Scenario: A call to another marked function needs no annotation

- **WHEN** a function is marked whose body binds the result of calling another marked function,
  without an annotation
- **THEN** marking succeeds, and the binding is typed when the project is built

#### Scenario: Marking order does not matter

- **WHEN** the calling function is marked before the function it calls
- **THEN** both are accepted, since the check that needs both is deferred to the build

#### Scenario: Only that category is deferred

- **WHEN** a marked function contains any other violation
- **THEN** it is still reported at the point of marking

#### Scenario: A callee that is never marked is still reported

- **WHEN** a deferred binding's callee is never marked and the project is built
- **THEN** the build fails, since deferring a check is not the same as skipping it

#### Scenario: No silent fallback

- **WHEN** a function is rejected
- **THEN** it is not left silently interpreted
