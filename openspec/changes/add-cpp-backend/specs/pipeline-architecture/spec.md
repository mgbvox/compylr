## ADDED Requirements

### Requirement: A backend's conformance output SHALL be compiled and run, not merely rendered

Rendering the shared corpus SHALL NOT by itself constitute conformance coverage. For every
implemented backend, the corpus output SHALL be **compiled** with that target's toolchain and, where
the entry has an expected value, **run** and compared against it. A backend that renders text which
does not build SHALL fail the check.

Where the target toolchain is unavailable on the machine, the check SHALL report itself **skipped**,
naming the missing toolchain, and SHALL NOT report success.

A requirement of this kind SHALL take effect for a `(source, target)` pair once that pair's
confirmed defects are closed. `(typescript, go)` SHALL be enumerated as a **known-failing pair**,
each entry naming its filed issue (#38, #39, #41), until those close. The list of known-failing
pairs SHALL only shrink: adding to it SHALL require a filed issue, and a pair SHALL NOT be added to
silence a defect introduced after this change.

Without that scoping the requirement would fail on the day it lands — not because it is wrong, but
because it correctly describes a pair that is already broken, and this change is not where that pair
gets fixed.


This is not a new idea but a correction: the requirement that every implemented backend renders the
corpus has been satisfied by a backend whose emitted output was never compiled, so the check
established that text was produced and nothing more. "Renders" was doing work the word cannot carry.

#### Scenario: Rendered output is compiled

- **GIVEN** an implemented backend and the shared conformance corpus
- **WHEN** the conformance check runs
- **THEN** the emitted source for every corpus entry is compiled with that target's toolchain

#### Scenario: Output that does not build fails the check

- **GIVEN** a backend whose emitted source for a corpus entry does not compile
- **WHEN** the conformance check runs
- **THEN** it fails, naming the entry and the backend
- **BUT** it does not report that backend as covered

#### Scenario: A missing toolchain is a skip, not a pass

- **GIVEN** a machine without the toolchain an implemented backend requires
- **WHEN** the conformance check runs for that backend
- **THEN** it reports itself skipped and names the missing toolchain
- **BUT** it does not report success

#### Scenario: Answers are compared where the corpus states them

- **GIVEN** a corpus entry carrying an expected result
- **WHEN** the compiled output for an implemented backend is run
- **THEN** its answer is compared against that expected result

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
