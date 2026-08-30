## MODIFIED Requirements

### Requirement: Frontend and backend names resolve with the same three answers
Resolving a frontend or backend name SHALL distinguish the same three cases: **implemented** (compile with it),
**reserved** (a language compylr intends to support, reported as planned rather than unknown), and
**unknown** (not a recognized name, reported with the names that would have worked). Collapsing "reserved" into
"unknown" SHALL NOT occur. Resolving `"typescript"` (frontend) and `"go"` (backend) SHALL return the implemented
components in addition to existing implementations.

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

### Requirement: Host bindings belong to a source/target pair
Making generated target code callable from the source language SHALL be modeled as a **host bridge**
belonging to the pair `(source language, target language)`, not to the frontend or the backend
alone. A bridge SHALL be resolved by that pair. Generation and bridging SHALL be independently
available: a pair with a backend but no bridge SHALL report that compylr can generate the target but
cannot yet call it back from that source language. The `("typescript", "go")` pair SHALL be registered as
an implemented host bridge.

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

#### Scenario: Unbridged pairs fail with descriptive error
- **WHEN** the `("typescript", "rust")` or `("python", "go")` pair is resolved
- **THEN** resolution fails with a `BridgeError::Unbridged` naming both languages
