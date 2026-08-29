## ADDED Requirements

### Requirement: An array argument binds to the caller's buffer

The bridge SHALL bind an array argument as a view over the memory the caller supplied, without
copying, and SHALL release the view when the call returns. A shared parameter SHALL bind a
read-only view and a mutable parameter a writable one.

#### Scenario: Binding does not copy

- **WHEN** a compiled function is called with a large array
- **THEN** no copy of the buffer is made, and the setup cost does not grow with the element count

#### Scenario: A write reaches the caller's buffer

- **WHEN** compiled code writes through a mutably bound array parameter
- **THEN** the caller's array holds the new value after the call

#### Scenario: A strided array binds without copying

- **WHEN** a non-contiguous array is passed
- **THEN** it binds as a strided view rather than being made contiguous by copying

#### Scenario: A wrong storage or rank is refused at the boundary

- **WHEN** an array whose storage or rank does not match the declared parameter is passed
- **THEN** the call raises an error naming the expected storage and rank, before any compiled code
  runs

#### Scenario: The view does not outlive the call

- **WHEN** a call returns
- **THEN** nothing retains a view over the caller's buffer

### Requirement: Overlapping mutable arguments are refused at the boundary

Where a function has more than one array parameter and at least one is mutably bound, the bridge
SHALL determine whether any two of those arguments overlap in memory and SHALL raise before running
compiled code when they do.

#### Scenario: The same array for two parameters is refused

- **WHEN** the same array is supplied for two array parameters and one is mutably bound
- **THEN** the call raises an error naming the overlap and no compiled code runs

#### Scenario: Overlapping slices are refused

- **WHEN** two arguments are overlapping views of one buffer and one is mutably bound
- **THEN** the call raises an error naming the overlap

#### Scenario: Non-overlapping arguments proceed

- **WHEN** two array arguments occupy disjoint memory
- **THEN** the call proceeds normally

#### Scenario: The check is skipped when it cannot apply

- **WHEN** a function has at most one array parameter, or every array parameter is shared
- **THEN** no overlap check is performed
