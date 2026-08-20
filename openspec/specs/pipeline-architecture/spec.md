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

- **WHEN** compylr is asked to compile source text with a named frontend
- **THEN** it resolves that name to a frontend and lowers the source with it

#### Scenario: Adding a frontend touches no shared component

- **WHEN** a new source language's frontend is added
- **THEN** the IR, the optimization passes, and every backend are unchanged

#### Scenario: Source syntax stays inside its frontend

- **WHEN** any component other than a frontend is inspected
- **THEN** it contains no spelling, keyword, or syntax belonging to a source language

### Requirement: Frontend and backend names resolve with the same three answers

Resolving a frontend name SHALL distinguish the same three cases backend resolution already
distinguishes: **implemented** (compile with it), **reserved** (a language compylr intends to
support, reported as planned rather than unknown), and **unknown** (not a recognized name, reported
with the names that would have worked). Collapsing "reserved" into "unknown" SHALL NOT occur, since
it would tell a user that a language compylr has committed to does not exist.

#### Scenario: Implemented frontend

- **WHEN** an implemented frontend name is resolved
- **THEN** resolution succeeds and returns that frontend

#### Scenario: Reserved frontend

- **WHEN** a reserved-but-unimplemented frontend name is resolved
- **THEN** resolution fails with an error identifying the name as planned but not yet available

#### Scenario: Unknown frontend

- **WHEN** a name that is not in the registry is resolved
- **THEN** resolution fails with an error stating the name is not recognized and listing the names
  that can compile today

#### Scenario: A caller branches on the case, not the message

- **WHEN** a caller needs to distinguish reserved from unknown
- **THEN** it can do so from the failure's kind without matching on rendered text

### Requirement: Host bindings belong to a source/target pair

Making generated target code callable from the source language SHALL be modeled as a **host bridge**
belonging to the pair `(source language, target language)`, not to the frontend or the backend
alone. A bridge SHALL be resolved by that pair. Generation and bridging SHALL be independently
available: a pair with a backend but no bridge SHALL report that compylr can generate the target but
cannot yet call it back from that source language.

#### Scenario: Bridge resolved by pair

- **WHEN** a source language and a target language that have a bridge are used together
- **THEN** the bridge for that pair is selected and produces the callable artifact

#### Scenario: Generation without a bridge

- **WHEN** a source and target have an implemented backend but no bridge for the pair
- **THEN** generating target source succeeds, and requesting a callable artifact fails with an error
  naming both languages and stating that the pair is not bridged

#### Scenario: A bridge is not assumed from either side

- **WHEN** a new backend is added without a bridge
- **THEN** no existing source language silently reports that it can call the new target

### Requirement: Components declare capabilities rather than being probed

A frontend SHALL declare the semantic guarantees its source language requires be preserved, and a
backend SHALL declare which of those guarantees it can preserve. compylr SHALL refuse a
frontend/backend combination whose declarations conflict, and SHALL report which guarantee could not
be met. Discovering the conflict by inspecting emitted code, or by a runtime difference in results,
SHALL NOT be the mechanism.

#### Scenario: Compatible declarations compile

- **WHEN** a backend declares it preserves every guarantee the frontend requires
- **THEN** compilation proceeds

#### Scenario: Conflicting declarations are refused before emission

- **WHEN** a frontend requires a guarantee the selected backend does not declare
- **THEN** compilation fails before any target source is generated, naming the guarantee

### Requirement: Target-specific post-processing is opt-in and bounded

A backend MAY apply target-specific transformations to generated code after emission. Such a
transformation SHALL run only when it preserves the guarantees the frontend declared, or when it has
been explicitly permitted by configuration. A transformation that only affects the readability of
generated source, and not its meaning, SHALL be permitted unconditionally.

#### Scenario: Meaning-preserving formatting always runs

- **WHEN** generated source is written out for a human to read
- **THEN** cosmetic formatting is applied without requiring permission

#### Scenario: A semantics-altering transformation is withheld

- **WHEN** a target offers a transformation that would violate a guarantee the frontend requires
- **THEN** it is not applied, and the reason is reportable

#### Scenario: Explicit permission overrides the default

- **WHEN** configuration explicitly permits a transformation the frontend did not require preserved
- **THEN** the transformation is applied

### Requirement: Emission stays a pure function of the IR

Producing target source from IR SHALL depend only on the IR and the backend's own configuration: no
filesystem access, no environment inspection, and no invocation of external tools. Post-processing,
formatting, and writing files SHALL happen outside emission. This is what makes emitted output
reproducible, and therefore what makes a build cache keyed on the IR trustworthy.

#### Scenario: The same IR emits the same text

- **WHEN** the same unit is emitted twice in different environments
- **THEN** the emitted source is byte-identical

#### Scenario: Emission does not touch the filesystem

- **WHEN** a unit is emitted
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

- **WHEN** the corpus is checked against the IR's node forms
- **THEN** every statement form, expression form, and type is exercised by at least one entry

#### Scenario: The corpus covers every form in every position it is legal in

- **WHEN** the corpus is checked against the positions a backend renders separately
- **THEN** each statement form appears in every position where it is legal, and its absence from a
  position it is legal in fails the check

#### Scenario: An illegal position is not required

- **WHEN** a form cannot appear in a position, such as returning a value from a constructor
- **THEN** the check does not require a corpus entry for that combination

#### Scenario: Every implemented backend is checked

- **WHEN** the conformance check runs
- **THEN** it runs each corpus entry through every backend the registry reports as implemented,
  enumerated from the registry rather than from a hand-maintained list

#### Scenario: An unrenderable form is a failure

- **WHEN** a backend cannot render a corpus entry
- **THEN** the conformance check fails and names the entry and the backend
