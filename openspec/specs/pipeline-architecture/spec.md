## Purpose

Defines the component model that lets compylr support many source languages and many target
languages without either side knowing about the other: what a frontend is, what a backend is, what
bridges a specific source/target pair, how each is resolved by name, and what guarantees must be
negotiated before a target is allowed to optimize generated code.

## Requirements

### Requirement: A frontend is a named, replaceable component

compylr SHALL model a source language as a **frontend**: a component that accepts source text and
produces IR, and that is selected by name rather than by being the only implementation present.
A frontend SHALL be the only component that knows its source language's syntax, spellings, and
semantics. Nothing outside a frontend SHALL be required to change in order to add one.

#### Scenario: Frontend selected by name

- **GIVEN** source text and the name of an implemented frontend
- **WHEN** compylr is asked to compile with that frontend
- **THEN** the name resolves to a frontend
- **AND** the source is lowered with it

#### Scenario: Adding a frontend touches no shared component

- **GIVEN** a workspace before a new source language is added
- **WHEN** that language's frontend is added
- **THEN** the IR, the optimization passes, and every backend are unchanged

#### Scenario: Source syntax stays inside its frontend

- **GIVEN** a component that is not a frontend
- **WHEN** it is inspected
- **THEN** it contains no spelling, keyword, or syntax belonging to a source language

### Requirement: Frontend and backend names resolve with the same three answers
Resolving a frontend or backend name SHALL distinguish the same three cases: **implemented**
(compile with it), **reserved** (a language compylr intends to support, reported as planned rather
than unknown), and **unknown** (not a recognized name, reported with the names that would have
worked). Collapsing "reserved" into "unknown" SHALL NOT occur, because it would tell someone asking
for a planned language that no such language exists, which is both false and discouraging.

The two registries SHALL be consulted independently, and a name MAY be implemented on one side and
reserved on the other: `typescript` is an implemented frontend and a reserved backend, and `go` is
an implemented backend and a reserved frontend. Being able to write a language says nothing about
being able to read it.

#### Scenario: Implemented frontend
- **GIVEN** a registry with the implemented frontends
- **WHEN** an implemented frontend name is resolved
- **THEN** resolution succeeds
- **AND** it returns that frontend

#### Scenario: Reserved frontend
- **GIVEN** a registry in which a frontend name is reserved but unimplemented
- **WHEN** that name is resolved
- **THEN** resolution fails with an error identifying the name as planned but not yet available

#### Scenario: Unknown frontend
- **GIVEN** a name absent from the registry
- **WHEN** it is resolved
- **THEN** resolution fails with an error stating the name is not recognized and listing the names
  that can compile today

#### Scenario: A caller branches on the case, not the message
- **GIVEN** a caller holding a resolution failure
- **WHEN** it needs to distinguish reserved from unknown
- **THEN** it can do so from the failure's kind without matching on rendered text

#### Scenario: TypeScript frontend is implemented
- **GIVEN** a registry with the implemented frontends
- **WHEN** the name `typescript` is resolved as a frontend
- **THEN** resolution succeeds
- **AND** it returns the TypeScript frontend

#### Scenario: Go backend is implemented
- **GIVEN** a registry with the implemented backends
- **WHEN** the name `go` is resolved as a backend
- **THEN** resolution succeeds
- **AND** it returns the Go backend

### Requirement: Host bindings belong to a source/target pair
Making generated target code callable from the source language SHALL be modeled as a **host bridge**
belonging to the pair `(source language, target language)`, not to the frontend or the backend
alone. A bridge SHALL be resolved by that pair. Generation and bridging SHALL be independently
available: a pair with a backend but no bridge SHALL report that compylr can generate the target but
cannot yet call it back from that source language. The `("typescript", "go")` pair SHALL be registered as
an implemented host bridge.

#### Scenario: Bridge resolved by pair
- **GIVEN** a source and a target language that have a bridge between them
- **WHEN** they are used together
- **THEN** the bridge for that pair is selected
- **AND** it produces the callable artifact

#### Scenario: Generation without a bridge
- **GIVEN** a source and target with an implemented backend but no bridge for the pair
- **WHEN** a callable artifact is requested
- **THEN** generating target source succeeds
- **BUT** requesting a callable artifact fails, naming both languages and stating that the pair is not bridged

#### Scenario: A bridge is not assumed from either side
- **GIVEN** a registry to which a backend has been added with no bridge
- **WHEN** each source language is asked what it can call
- **THEN** no existing source language silently reports that it can call the new target

#### Scenario: TypeScript to Go bridge is selected
- **GIVEN** a registry with the implemented bridges
- **WHEN** the `(typescript, go)` pair is resolved
- **THEN** resolution succeeds
- **AND** it returns the bridge for that pair

#### Scenario: Unbridged pairs fail with descriptive error
- **GIVEN** a pair whose backend is implemented but which has no bridge
- **WHEN** that pair is resolved
- **THEN** resolution fails
- **AND** the failure names both languages
- **BUT** it does not report the target as unavailable, which would be false

### Requirement: Components declare capabilities rather than being probed

A frontend SHALL declare the semantic guarantees its source language requires be preserved, and a
backend SHALL declare which of those guarantees it can preserve. A **unit** SHALL record the
guarantees the program it holds requires, derived from what its own operations declare rather than
from a fixed list belonging to its frontend. compylr SHALL refuse a combination whose declarations
conflict, and SHALL report which guarantee could not be met. Discovering the conflict by inspecting
emitted code, or by a runtime difference in results, SHALL NOT be the mechanism.

#### Scenario: Compatible declarations compile

- **GIVEN** a unit and a backend declaring it preserves every guarantee the unit requires
- **WHEN** the unit is compiled
- **THEN** compilation proceeds

#### Scenario: Conflicting declarations are refused before emission

- **GIVEN** a unit requiring a guarantee the selected backend does not declare
- **WHEN** the unit is compiled
- **THEN** compilation fails before any target source is generated, naming the guarantee

#### Scenario: A program may require less than its language

- **GIVEN** a unit whose resolved behavior waives an axis's guarantee
- **WHEN** the guarantees it requires are computed
- **THEN** the unit requires fewer guarantees than its frontend declares for the language, and the
  negotiation reads the unit's

### Requirement: Target-specific post-processing is opt-in and bounded

A backend MAY apply target-specific transformations to generated code after emission. Such a
transformation SHALL run only when it preserves the guarantees **the unit** requires, or when it has
been explicitly permitted by configuration. A transformation that only affects the readability of
generated source, and not its meaning, SHALL be permitted unconditionally.

#### Scenario: Meaning-preserving formatting always runs

- **GIVEN** generated source about to be written out for a human to read
- **WHEN** it is written out
- **THEN** cosmetic formatting is applied without requiring permission

#### Scenario: A semantics-altering transformation is withheld

- **GIVEN** a target offering a transformation that would violate a guarantee the unit requires
- **WHEN** the unit is compiled
- **THEN** it is not applied, and the reason is reportable

#### Scenario: Explicit permission overrides the default

- **GIVEN** configuration explicitly permitting a transformation the unit did not require
  preserved
- **WHEN** the unit is compiled
- **THEN** the transformation is applied

#### Scenario: A waived guarantee makes an option available

- **GIVEN** a unit whose resolved behavior waives the guarantee a target option would break
- **WHEN** the withheld options are computed
- **THEN** that option is no longer withheld for that unit, and the report of withheld options no
  longer lists it

### Requirement: Emission stays a pure function of the IR

Producing target source from IR SHALL depend only on the IR and the backend's own configuration: no
filesystem access, no environment inspection, and no invocation of external tools. Post-processing,
formatting, and writing files SHALL happen outside emission. This is what makes emitted output
reproducible, and therefore what makes a build cache keyed on the IR trustworthy.

#### Scenario: The same IR emits the same text

- **GIVEN** one unit
- **WHEN** it is emitted twice in different environments
- **THEN** the emitted source is byte-identical

#### Scenario: Emission does not touch the filesystem

- **GIVEN** a unit
- **WHEN** it is emitted
- **THEN** no file is read or written by emission itself

### Requirement: Every implemented backend renders the shared conformance corpus

compylr SHALL maintain a corpus of IR units, independent of any source language, that every
implemented backend is required to render. Adding a backend SHALL NOT require writing a new corpus,
and a backend that cannot render a corpus entry SHALL fail visibly rather than emitting code that
does not build.

Coverage SHALL be measured over **positions as well as forms**. A backend renders the same statement
differently depending on where it appears — a function body, a constructor, a method with a shared
receiver, a method with a mutable receiver, and a loop body are each rendered by their own path — and
a corpus that recorded only which forms appeared would report full coverage while leaving those paths
untested. Where a form is not legal in a position, the corpus SHALL NOT be required to contain it.

#### Scenario: The corpus covers every IR form

- **GIVEN** the conformance corpus and the IR's node forms
- **WHEN** the corpus is checked against them
- **THEN** every statement form, expression form, and type is exercised by at least one entry

#### Scenario: The corpus covers every form in every position it is legal in

- **GIVEN** the conformance corpus and the positions a backend renders separately
- **WHEN** the corpus is checked against them
- **THEN** each statement form appears in every position where it is legal, and its absence from a
  position it is legal in fails the check

#### Scenario: An illegal position is not required

- **GIVEN** a form that cannot appear in a position, such as returning a value from a constructor
- **WHEN** the corpus coverage is checked
- **THEN** the check does not require a corpus entry for that combination

#### Scenario: Every implemented backend is checked

- **GIVEN** the conformance corpus and the registry's implemented backends
- **WHEN** the conformance check runs
- **THEN** it runs each corpus entry through every backend the registry reports as implemented,
  enumerated from the registry rather than from a hand-maintained list

#### Scenario: An unrenderable form is a failure

- **GIVEN** a corpus entry a backend cannot render
- **WHEN** the conformance check runs
- **THEN** the conformance check fails and names the entry and the backend

### Requirement: A component declares its language's behavior, not the pair's

Both a frontend and a backend SHALL declare, for every behavior axis, what their own language means
by that operation. Neither SHALL declare anything about the other's language, and no component
SHALL hold a mapping from one language's stance to another's.

Resolving a behavior for a compilation SHALL read the two declarations and the user's request and
produce one stance per axis. Adding a language SHALL therefore cost one declaration and SHALL NOT
require editing any existing component — the same N + M property frontends and backends already
have, rather than the N × M a pairwise table would create.

#### Scenario: Both endpoints declare

- **GIVEN** a frontend and a backend resolved for a compilation
- **WHEN** each is asked what it means
- **THEN** each answers, for every axis, what its own language means

#### Scenario: A declaration mentions no other language

- **GIVEN** a component's behavior declaration
- **WHEN** it is inspected
- **THEN** it names only its own language's meanings

#### Scenario: Adding a language costs one declaration

- **GIVEN** a registry of existing components
- **WHEN** a new frontend or backend is registered with a complete behavior declaration
- **THEN** it composes with every existing component on the other side without any of them being
  edited

#### Scenario: A behavior is resolved before lowering

- **GIVEN** a compilation with a behavior request
- **WHEN** the compilation begins
- **THEN** its behavior is resolved and validated before any source is lowered, so that an invalid
  request is reported without a parse
