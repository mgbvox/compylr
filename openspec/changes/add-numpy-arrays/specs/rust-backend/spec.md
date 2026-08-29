## ADDED Requirements

### Requirement: Array parameters emit as views

The Rust backend SHALL emit an array parameter as an array view of the declared rank and storage,
shared or mutable according to the parameter's passing mode, and SHALL NOT emit an owned array or
any copy of one. The view type SHALL be the strided one, so a non-contiguous argument stays a view.

#### Scenario Outline: Each passing mode emits its view

- **GIVEN** an array parameter bound as a <mode> view
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted signature takes a <spelling> array view of the declared rank

**Examples:**

| mode    | spelling |
| ------- | -------- |
| shared  | shared   |
| mutable | mutable  |

#### Scenario: No clone is emitted for an array

- **GIVEN** an array parameter read repeatedly, iterated, or passed onward
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** no clone of the array appears in the emitted source

#### Scenario: Element access emits an indexed read

- **GIVEN** an element read supplying one index per rank
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the emitted code indexes the view directly under the declared checking mode

#### Scenario: An element write emits a place

- **GIVEN** an element assignment into a mutable array view
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** the assignment emits as a place, so the write lands in the caller's buffer
- **BUT** it does not land in a copy

#### Scenario Outline: Arrays are emitted in every position they are legal in

- **GIVEN** an array parameter used in a <position>
- **WHEN** the unit is emitted for the `rust` backend
- **THEN** it emits correctly
- **AND** the conformance check covers the pair

**Examples:**

| position           |
| ------------------ |
| free function body |
| method body        |
| loop body          |
