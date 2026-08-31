## ADDED Requirements

### Requirement: A bridge may be composed from a shared target surface and a source-side loader

A host bridge MAY be implemented as a shared, source-language-independent surface belonging to the
target, plus a small loader belonging to the source language. Where it is, the composition SHALL be
invisible to resolution: each pair SHALL still be registered and resolved as its own bridge, and an
unregistered pair SHALL still report that the pair is not bridged.

Where a target admits such a surface, adding a source language that calls it SHALL cost one loader
rather than one bridge, so the N × M bridge cost is paid once per target instead of once per pair.
A target that admits no such surface SHALL remain free to implement each pair directly; this is a
permission, not an obligation.

The trait is already shaped for this — [`bridge.rs`](../../../../../crates/compylr-core/src/bridge.rs#L18)
records it as deferred rather than foreclosed — so nothing about resolution changes.

#### Scenario: A composed bridge resolves like any other

- **GIVEN** two source languages whose bridges to one target share a generated surface
- **WHEN** either pair is resolved from the bridge registry
- **THEN** resolution returns a bridge whose source and target are that pair

#### Scenario: The shared half is not itself a bridge

- **GIVEN** a crate holding a target's shared export surface
- **WHEN** the bridge registry is enumerated
- **THEN** that crate is not registered as a bridge for any pair

#### Scenario: A third source language costs a loader

- **GIVEN** a target with a shared export surface and two registered pairs
- **WHEN** a third source language is added by supplying only a loader
- **THEN** the pair resolves
- **AND** the two existing pairs' emitted artifacts are unchanged

#### Scenario: An unregistered pair is still unbridged

- **GIVEN** a target with a shared export surface
- **WHEN** a pair whose loader has not been supplied is resolved
- **THEN** resolution fails naming both languages and stating the pair is not bridged

## MODIFIED Requirements

### Requirement: Frontend and backend names resolve with the same three answers
Resolving a frontend or backend name SHALL distinguish the same three cases: **implemented** (compile with it),
**reserved** (a language compylr intends to support, reported as planned rather than unknown), and
**unknown** (not a recognized name, reported with the names that would have worked). Collapsing "reserved" into
"unknown" SHALL NOT occur. Resolving `"typescript"` (frontend) and `"go"` (backend) SHALL return the implemented
components in addition to existing implementations. Resolving `"cpp"` as a **backend** SHALL return an
implemented component; resolving `"cpp"` as a **frontend** SHALL continue to report a reserved name.

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

#### Scenario: TypeScript frontend is implemented
- **WHEN** the frontend name `"typescript"` is resolved
- **THEN** resolution succeeds and returns the `TypeScriptFrontend` instance

#### Scenario: Go backend is implemented
- **WHEN** the backend name `"go"` is resolved
- **THEN** resolution succeeds and returns the `GoBackend` instance

#### Scenario: The C++ backend is implemented
- **GIVEN** the backend registry
- **WHEN** the backend name `"cpp"` is resolved
- **THEN** resolution succeeds and returns the C++ backend

#### Scenario: C++ remains a reserved frontend
- **GIVEN** the frontend registry
- **WHEN** the frontend name `"cpp"` is resolved
- **THEN** resolution fails identifying the name as planned but not yet available

### Requirement: Host bindings belong to a source/target pair
Making generated target code callable from the source language SHALL be modeled as a **host bridge**
belonging to the pair `(source language, target language)`, not to the frontend or the backend
alone. A bridge SHALL be resolved by that pair. Generation and bridging SHALL be independently
available: a pair with a backend but no bridge SHALL report that compylr can generate the target but
cannot yet call it back from that source language. The `("typescript", "go")` pair SHALL be registered as
an implemented host bridge, and so SHALL the `("python", "cpp")` and `("typescript", "cpp")` pairs.

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

#### Scenario: TypeScript to Go bridge is selected
- **WHEN** the `("typescript", "go")` pair is resolved from the bridge registry
- **THEN** resolution succeeds and returns the `TypeScriptGoBridge` instance

#### Scenario: Python to C++ bridge is selected
- **GIVEN** the bridge registry
- **WHEN** the `("python", "cpp")` pair is resolved
- **THEN** resolution succeeds and returns a bridge whose source is `python` and whose target is
  `cpp`

#### Scenario: TypeScript to C++ bridge is selected
- **GIVEN** the bridge registry
- **WHEN** the `("typescript", "cpp")` pair is resolved
- **THEN** resolution succeeds and returns a bridge whose source is `typescript` and whose target is
  `cpp`

#### Scenario: Unbridged pairs fail with descriptive error
- **WHEN** the `("typescript", "rust")` or `("python", "go")` pair is resolved
- **THEN** resolution fails with a `BridgeError::Unbridged` naming both languages
