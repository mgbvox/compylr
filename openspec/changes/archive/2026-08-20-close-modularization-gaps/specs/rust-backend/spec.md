## ADDED Requirements

### Requirement: Subscripting honors the declared index origin

The backend SHALL emit a sequence read that resolves a negative index the way the node declares. A
node declaring *from either end* SHALL count a negative index backwards from the end; a node
declaring *from the start* SHALL treat a negative index as out of range. Reading outside the
sequence SHALL be reported rather than panicking, under either origin.

#### Scenario: Negative index, counting from either end

- **WHEN** a sequence of three elements is read at index `-1` under an origin of *from either end*
- **THEN** the result is the last element

#### Scenario: Negative index, counting from the start

- **WHEN** the same read is emitted under an origin of *from the start*
- **THEN** the failure is reported as an index out of range

#### Scenario: A non-negative index is unaffected by the origin

- **WHEN** a sequence is read at index `1` under either origin
- **THEN** both produce the second element

#### Scenario: Reading past the end is reported under either origin

- **WHEN** a sequence of three elements is read at index `3`
- **THEN** the failure is reported rather than the process aborting

### Requirement: Length honors the declared text units

The backend SHALL emit a length that counts in the units the node declares. For a value that is not
text the declaration SHALL make no difference, because the length of a collection is a count of its
elements under every reading.

#### Scenario: Each unit reading counts differently

- **WHEN** the length of a string containing a two-byte character is emitted under each of code
  points, UTF-8 bytes, and UTF-16 units
- **THEN** the three results differ where the readings differ, and the byte count exceeds the code
  point count

#### Scenario: A character outside the basic plane distinguishes all three

- **WHEN** the length of a string containing a character requiring a surrogate pair is emitted under
  each reading
- **THEN** code points, UTF-8 bytes, and UTF-16 units each report a different number

#### Scenario: A collection's length ignores the declaration

- **WHEN** the length of a sequence is emitted under any declared units
- **THEN** the result is the number of elements
