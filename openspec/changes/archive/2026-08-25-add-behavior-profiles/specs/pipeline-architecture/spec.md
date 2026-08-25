## ADDED Requirements

### Requirement: A component declares its language's behavior, not the pair's

Both a frontend and a backend SHALL declare, for every behavior axis, what their own language means
by that operation. Neither SHALL declare anything about the other's language, and no component
SHALL hold a mapping from one language's stance to another's.

Resolving a behavior for a compilation SHALL read the two declarations and the user's request and
produce one stance per axis. Adding a language SHALL therefore cost one declaration and SHALL NOT
require editing any existing component — the same N + M property frontends and backends already
have, rather than the N × M a pairwise table would create.

#### Scenario: Both endpoints declare

- **WHEN** a frontend and a backend are resolved for a compilation
- **THEN** each answers, for every axis, what its own language means

#### Scenario: A declaration mentions no other language

- **WHEN** a component's behavior declaration is inspected
- **THEN** it names only its own language's meanings

#### Scenario: Adding a language costs one declaration

- **WHEN** a new frontend or backend is registered with a complete behavior declaration
- **THEN** it composes with every existing component on the other side without any of them being
  edited

#### Scenario: A behavior is resolved before lowering

- **WHEN** a compilation begins
- **THEN** its behavior is resolved and validated before any source is lowered, so that an invalid
  request is reported without a parse

## MODIFIED Requirements

### Requirement: Components declare capabilities rather than being probed

A frontend SHALL declare the semantic guarantees its source language requires be preserved, and a
backend SHALL declare which of those guarantees it can preserve. A **unit** SHALL record the
guarantees the program it holds requires, derived from what its own operations declare rather than
from a fixed list belonging to its frontend. compylr SHALL refuse a combination whose declarations
conflict, and SHALL report which guarantee could not be met. Discovering the conflict by inspecting
emitted code, or by a runtime difference in results, SHALL NOT be the mechanism.

#### Scenario: Compatible declarations compile

- **WHEN** a backend declares it preserves every guarantee the unit requires
- **THEN** compilation proceeds

#### Scenario: Conflicting declarations are refused before emission

- **WHEN** a unit requires a guarantee the selected backend does not declare
- **THEN** compilation fails before any target source is generated, naming the guarantee

#### Scenario: A program may require less than its language

- **WHEN** a unit's resolved behavior waives an axis's guarantee
- **THEN** the unit requires fewer guarantees than its frontend declares for the language, and the
  negotiation reads the unit's

### Requirement: Target-specific post-processing is opt-in and bounded

A backend MAY apply target-specific transformations to generated code after emission. Such a
transformation SHALL run only when it preserves the guarantees **the unit** requires, or when it has
been explicitly permitted by configuration. A transformation that only affects the readability of
generated source, and not its meaning, SHALL be permitted unconditionally.

#### Scenario: Meaning-preserving formatting always runs

- **WHEN** generated source is written out for a human to read
- **THEN** cosmetic formatting is applied without requiring permission

#### Scenario: A semantics-altering transformation is withheld

- **WHEN** a target offers a transformation that would violate a guarantee the unit requires
- **THEN** it is not applied, and the reason is reportable

#### Scenario: Explicit permission overrides the default

- **WHEN** configuration explicitly permits a transformation the unit did not require preserved
- **THEN** the transformation is applied

#### Scenario: A waived guarantee makes an option available

- **WHEN** a unit's resolved behavior waives the guarantee a target option would break
- **THEN** that option is no longer withheld for that unit, and the report of withheld options no
  longer lists it
