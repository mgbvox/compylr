## ADDED Requirements

### Requirement: C++ is selectable as a target from TypeScript

The package SHALL accept `"cpp"` as a backend and SHALL compile marked members to C++ without the
calling code changing. The default backend SHALL remain unchanged, so selecting C++ is an explicit
choice rather than something a project acquires by upgrading.

Everything the surface already guarantees SHALL hold for it unchanged: one shared artifact per
project, cache validation against the IR fingerprint, and the environment switch that hands back
untouched members.

A failure the generated code returns SHALL be thrown as an `Error` carrying the failure's message.

#### Scenario: A member marked for C++ runs compiled

- **GIVEN** a manager initialized with the `cpp` backend
- **WHEN** a marked function is called
- **THEN** the call reaches the compiled implementation
- **AND** the answer is the one the TypeScript source would have produced

#### Scenario: The default backend is unchanged

- **WHEN** a manager is initialized with no backend named
- **THEN** the backend selected is the one selected before C++ existed

#### Scenario: One project, one artifact

- **GIVEN** a manager initialized with the `cpp` backend and three marked members
- **WHEN** the first call is made
- **THEN** exactly one artifact is built and it holds all three members

#### Scenario: A reported failure becomes a thrown Error

- **GIVEN** a manager initialized with the `cpp` backend and a marked function that divides
- **WHEN** it is called with a zero divisor under a behavior that reports division by zero
- **THEN** an `Error` is thrown whose message names division by zero

#### Scenario: The environment switch still returns members untouched

- **GIVEN** a manager initialized with the `cpp` backend and compilation disabled by the environment
- **WHEN** a member is marked
- **THEN** the member is returned untouched and is not validated
