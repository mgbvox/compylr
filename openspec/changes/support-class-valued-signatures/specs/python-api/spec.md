## MODIFIED Requirements

### Requirement: Unsupported functions are rejected when marked

A function outside the supported subset SHALL be rejected at the point it is marked, carrying
the diagnostic and its `line:column`. It SHALL NOT be accepted and then fail at first call, and
it SHALL NOT silently fall back to interpreted execution: the user asked for compilation, so
being told immediately that it cannot happen is the useful outcome.

**Two categories are deferred.** A binding whose initializer calls a function the marked source
does not define cannot be typed at this point, because each marked function is captured as its own
source and its callees live in others. For the same reason, a bare annotation that can validly name
a compiled class in another marked source cannot be resolved from the function alone. Those two
categories SHALL be deferred to the build, where every source and class is present, and SHALL be
reported there with their original source locations if they remain unresolved.

Malformed annotations, known unsupported built-ins, nested class-valued boundary annotations once
their class is known, and every other violation SHALL still be reported as soon as the available
project context can decide them. Deferral SHALL NOT become silent acceptance or interpreted
fallback.

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

#### Scenario: A class annotation can precede its class

- **WHEN** a top-level function directly taking or returning `Tally` is marked before the `Tally`
  class is marked
- **THEN** marking succeeds and the annotation resolves during the whole-project build

#### Scenario: Marking order does not matter

- **WHEN** a calling function or class-annotated function is marked before the member it needs
- **THEN** both are accepted, since the check that needs both is deferred to the build

#### Scenario: Only that category is deferred

- **WHEN** a marked function uses `complex`, a malformed generic, or contains another subset
  violation outside the two explicitly deferrable categories
- **THEN** it is still reported at the point of marking

#### Scenario: A callee that is never marked is still reported

- **WHEN** a deferred binding's callee is never marked and the project is built
- **THEN** the build fails, since deferring a check is not the same as skipping it

#### Scenario: A class annotation typo is still reported

- **WHEN** a deferred `Taly` annotation matches no class when the complete project is built
- **THEN** the build fails with a diagnostic at the `Taly` annotation

#### Scenario: A nested class annotation is reported once resolvable

- **WHEN** a marked function has a `list[Tally]` boundary annotation and the complete project
  defines `Tally`
- **THEN** the build fails with a located unsupported-boundary-type diagnostic before Rust emission

#### Scenario: No silent fallback

- **WHEN** a function is rejected
- **THEN** it is not left silently interpreted
