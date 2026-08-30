# typescript-api Specification

## Purpose
The user-facing TypeScript package: the marker that nominates a function for compilation, manager
initialization, build orchestration through the Go toolchain, the fingerprint-keyed rebuild
decision, swapping the compiled implementation in at runtime, and the environment switch that
turns all of it off.

## Requirements

### Requirement: Configuration manager initialization
The package SHALL provide an initialization entrypoint returning a configured manager. It SHALL
accept the target backend, defaulting to `go`; the behavior whose semantics the generated code
preserves; and `llmAssist`, which is accepted as a setting and refused when enabled.

#### Scenario: Initialization defaults to the pair that works
- **GIVEN** a project that names no backend
- **WHEN** the manager is initialized
- **THEN** it targets the Go backend
- **AND** it resolves semantics to TypeScript's stance on every axis

#### Scenario: An unimplemented setting is refused rather than ignored
- **GIVEN** a project that enables `llmAssist`
- **WHEN** the manager is initialized
- **THEN** initialization fails saying the setting is not implemented
- **BUT** the same setting left disabled is accepted

### Requirement: Marking a function for compilation
The manager SHALL provide a marker usable as a decorator and as a wrapping function. On marking,
it SHALL recover the function's source text and validate it against the TypeScript frontend
immediately, rather than deferring the diagnostic to the first call.

#### Scenario: An unsupported function is refused where it is written
- **GIVEN** a function whose body is outside the supported subset
- **WHEN** it is marked
- **THEN** marking fails
- **AND** the diagnostic points at the offending construct
- **BUT** no build has been attempted

### Requirement: Single shared artifact and first-call compilation
Every marked member in a project SHALL compile into one shared Go package. Compilation SHALL
happen on the first call to any marked member, SHALL build the artifact under `.compylr/`, and
SHALL swap the compiled implementation in so later calls reach it directly.

#### Scenario: The first call builds the whole project
- **GIVEN** a project with several marked members and no build on disk
- **WHEN** any one of them is called
- **THEN** every marked member is compiled into one artifact
- **AND** the call is answered by the compiled implementation

#### Scenario: A later call pays nothing to build
- **GIVEN** a project whose artifact has already been built and loaded
- **WHEN** a marked member is called again
- **THEN** the compiled implementation answers it
- **AND** no build is invoked

### Requirement: Cache validation and fingerprinting
The manager SHALL record the IR fingerprint, the backend, and the compylr version in build state.
A run whose fingerprint and version match the recorded ones SHALL skip building entirely.

#### Scenario: An unchanged project reuses its build
- **GIVEN** build state recording a fingerprint and version that match the current project
- **WHEN** a marked member is called
- **THEN** the existing artifact is loaded
- **AND** the Go toolchain is not invoked

#### Scenario: An upgraded compiler rebuilds once
- **GIVEN** build state recording an older compylr version
- **WHEN** a marked member is called
- **THEN** the project is rebuilt
- **BUT** the source text has not changed

### Requirement: Environment disable switch
When the disable variable is set in the environment, the manager SHALL return every marked member
untouched, without validating or compiling it.

#### Scenario: The environment turns compilation off for a process
- **GIVEN** a process with the disable variable set
- **WHEN** a marked member is called
- **THEN** the original TypeScript implementation answers it
- **AND** nothing has been validated or built
