## ADDED Requirements

### Requirement: An array argument binds to the caller's buffer

The bridge SHALL bind an array argument as a view over the memory the caller supplied, without
copying, and SHALL release the view when the call returns. A shared parameter SHALL bind a
read-only view and a mutable parameter a writable one.

#### Scenario: Binding does not copy

- **GIVEN** a compiled function taking an array parameter
- **WHEN** it is called with a large array
- **THEN** no copy of the buffer is made
- **AND** the setup cost does not grow with the element count

#### Scenario: A write reaches the caller's buffer

- **GIVEN** compiled code that writes through a mutably bound array parameter
- **WHEN** the call returns
- **THEN** the caller's array holds the new value

#### Scenario: A strided array binds without copying

- **GIVEN** a non-contiguous array
- **WHEN** it is passed to a compiled function
- **THEN** it binds as a strided view
- **BUT** it is not made contiguous by copying

#### Scenario: A wrong storage or rank is refused at the boundary

- **GIVEN** an array whose storage or rank does not match the declared parameter
- **WHEN** the compiled function is called with it
- **THEN** the call raises an error naming the expected storage and rank
- **AND** no compiled code runs

#### Scenario: The view does not outlive the call

- **GIVEN** a compiled function bound to a caller's buffer
- **WHEN** the call returns
- **THEN** nothing retains a view over that buffer

#### Scenario: A view is held without releasing the host runtime lock

- **GIVEN** compiled code holding a view over the caller's buffer
- **WHEN** the body runs
- **THEN** the host runtime's lock is not released while the view is live

### Requirement: Overlapping mutable arguments are refused at the boundary

Where a function has more than one array parameter and at least one is mutably bound, the bridge
SHALL determine whether any two of those arguments overlap in memory and SHALL raise before running
compiled code when they do. The check SHALL be skipped where it cannot apply, so a single-array
call pays nothing.

#### Scenario: The same array for two parameters is refused

- **GIVEN** a function with two array parameters, one mutably bound
- **WHEN** the same array is supplied for both
- **THEN** the call raises an error naming the overlap
- **AND** no compiled code runs

#### Scenario: Overlapping slices are refused

- **GIVEN** two arguments that are overlapping views of one buffer, one mutably bound
- **WHEN** the function is called
- **THEN** the call raises an error naming the overlap

#### Scenario: Non-overlapping arguments proceed

- **GIVEN** two array arguments occupying disjoint memory
- **WHEN** the function is called
- **THEN** the call proceeds normally

#### Scenario Outline: The check is skipped when it cannot apply

- **GIVEN** a function with <shape>
- **WHEN** it is called
- **THEN** no overlap check is performed

**Examples:**

| shape                             |
| --------------------------------- |
| at most one array parameter       |
| every array parameter shared      |
