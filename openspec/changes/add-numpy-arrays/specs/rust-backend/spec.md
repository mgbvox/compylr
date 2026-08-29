## ADDED Requirements

### Requirement: Array parameters emit as views

The Rust backend SHALL emit an array parameter as an array view of the declared rank and storage,
shared or mutable according to the parameter's passing mode, and SHALL NOT emit an owned array or
any copy of one.

#### Scenario: A shared array parameter emits a shared view

- **WHEN** emitting an array parameter bound as a shared view
- **THEN** the emitted signature takes a shared array view of the declared rank

#### Scenario: A mutable array parameter emits a mutable view

- **WHEN** emitting an array parameter bound as a mutable view
- **THEN** the emitted signature takes a mutable array view

#### Scenario: No clone is emitted for an array

- **WHEN** an array parameter is read repeatedly, iterated, or passed onward
- **THEN** no clone of the array appears in the emitted source

#### Scenario: Element access emits an indexed read

- **WHEN** emitting an element read supplying one index per rank
- **THEN** the emitted code indexes the view directly under the declared checking mode

#### Scenario: An element write emits a place

- **WHEN** emitting an element assignment into a mutable array view
- **THEN** the assignment emits as a place so the write lands in the caller's buffer, not in a copy

#### Scenario: Arrays are emitted in every position they are legal in

- **WHEN** an array parameter is used in a free function body, a method body, and a loop body
- **THEN** each emits correctly and the conformance check covers the pairs
