## ADDED Requirements

### Requirement: A parameter is emitted by its declared mode

The Rust backend SHALL emit each parameter according to its passing mode, and SHALL NOT infer
ownership from the parameter's type or from whether it is mutated. A borrowed parameter SHALL NOT
be cloned where it is read. Emission SHALL read the mode as data, the way it already reads a
checking mode rather than an operation's name.

#### Scenario Outline: Each mode emits its own spelling

- **GIVEN** a parameter whose mode is <mode>
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted signature takes <spelling>

**Examples:**

| mode           | spelling                        |
| -------------- | ------------------------------- |
| owned          | the owned spelling of its type  |
| shared borrow  | a shared reference              |
| mutable borrow | a mutable reference             |

#### Scenario: A shared borrow is not cloned where it is read

- **GIVEN** a parameter whose mode is a shared borrow
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** reads of it emit no clone

#### Scenario: The mode is not re-derived

- **GIVEN** a parameter that is never mutated but whose mode is owned
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted signature is owned
- **BUT** the backend does not substitute a borrow

#### Scenario: Emitted code compiles for every shape

- **GIVEN** the whole accepted corpus, including the shapes that force ownership
- **WHEN** each is emitted and built
- **THEN** every generated crate compiles
