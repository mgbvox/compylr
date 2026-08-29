## ADDED Requirements

### Requirement: A parameter is emitted by its declared mode

The Rust backend SHALL emit each parameter according to its passing mode, and SHALL NOT infer
ownership from the parameter's type or from whether it is mutated. A borrowed parameter SHALL NOT
be cloned where it is read.

#### Scenario: An owned parameter emits an owned type

- **WHEN** emitting a parameter whose mode is owned
- **THEN** the emitted signature takes the owned spelling of its type

#### Scenario: A shared borrow emits a shared reference

- **WHEN** emitting a parameter whose mode is a shared borrow
- **THEN** the emitted signature takes a shared reference, and reads of it emit no clone

#### Scenario: A mutable borrow emits a mutable reference

- **WHEN** emitting a parameter whose mode is a mutable borrow
- **THEN** the emitted signature takes a mutable reference

#### Scenario: The mode is not re-derived

- **WHEN** a parameter is never mutated but its mode is owned
- **THEN** the emitted signature is owned, and the backend does not substitute a borrow

#### Scenario: Emitted code compiles for every shape

- **WHEN** the corpus is emitted and built
- **THEN** every generated crate compiles, including the shapes that force ownership
