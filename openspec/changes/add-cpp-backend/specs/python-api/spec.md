## ADDED Requirements

### Requirement: C++ is selectable as a target from Python

The package SHALL accept `"cpp"` as a backend, globally and per member, and calls to a member marked
under it SHALL land on compiled C++ without the calling code changing. Everything the surface
already guarantees SHALL hold for it unchanged: one shared artifact per project, a rebuild keyed on
the IR fingerprint, per-member behavior overrides, and the environment switch that hands back
untouched members.

A failure the generated code returns SHALL be raised as the Python exception the corresponding
Python operation would have raised, so that a caller does not have to know which target it is on.

#### Scenario: A member marked for C++ runs compiled

- **GIVEN** a manager initialized with the `cpp` backend
- **WHEN** a marked function is called
- **THEN** the call reaches the compiled implementation
- **AND** the answer is the one the Python source would have produced

#### Scenario: One project, one artifact

- **GIVEN** a manager initialized with the `cpp` backend and three marked members
- **WHEN** the first call is made
- **THEN** exactly one artifact is built and it holds all three members

#### Scenario: A reported failure becomes the matching Python exception

- **GIVEN** a manager initialized with the `cpp` backend and a marked function that divides
- **WHEN** it is called with a zero divisor under a behavior that reports division by zero
- **THEN** `ZeroDivisionError` is raised

#### Scenario: A missing mapping key raises KeyError

- **GIVEN** a manager initialized with the `cpp` backend and a marked function reading a mapping key
- **WHEN** it is called with a mapping that does not contain the key
- **THEN** `KeyError` is raised

#### Scenario: An out-of-range subscript raises IndexError

- **GIVEN** a manager initialized with the `cpp` backend and a marked function indexing a sequence
- **WHEN** it is called with an index outside the sequence under Python's reporting stance
- **THEN** `IndexError` is raised

#### Scenario: The failure kind survives the bridge

- **GIVEN** three marked functions failing in three different ways
- **WHEN** each is called so that it fails
- **THEN** three distinct exception types are raised
- **BUT** they are not all flattened to one type

#### Scenario: An instance attribute survives the call

- **GIVEN** a manager initialized with the `cpp` backend and a marked class whose method mutates an
  attribute
- **WHEN** an instance is constructed and that method is called twice
- **THEN** reading the attribute reflects both calls

#### Scenario: The environment switch still returns members untouched

- **GIVEN** a manager initialized with the `cpp` backend and compilation disabled by the environment
- **WHEN** a member is marked
- **THEN** the member is returned untouched and is not validated

#### Scenario: The default behavior needs no adjustment for C++

- **GIVEN** a manager initialized with the `cpp` backend and no behavior requested
- **WHEN** a member whose arithmetic can overflow is marked and called with values that overflow
- **THEN** `OverflowError` is raised, as the Python source would have raised
- **BUT** no behavior override was needed to get it
