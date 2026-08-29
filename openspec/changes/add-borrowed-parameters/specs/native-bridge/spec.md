## ADDED Requirements

### Requirement: The boundary converts a parameter according to its mode

The bridge SHALL convert each argument at the host boundary according to the parameter's passing
mode, and SHALL avoid copying where the host's representation permits a borrow to be taken directly.

#### Scenario: A borrowed text argument is not copied

- **WHEN** a compiled function takes a borrowed text parameter and the host supplies a text value
- **THEN** the boundary borrows the host's buffer rather than copying it

#### Scenario: An owned argument is converted as before

- **WHEN** a compiled function takes an owned parameter
- **THEN** the boundary converts it as it did before this change, with the same result

#### Scenario: A borrow does not outlive the call

- **WHEN** a borrowed argument is passed across the boundary
- **THEN** the borrow ends when the call returns, and nothing retains it

#### Scenario: A collection argument is still converted element by element

- **WHEN** a compiled function takes a sequence or mapping parameter under any mode
- **THEN** the boundary still converts each element, because the host's representation is not a
  contiguous block of the element type

#### Scenario: Answers are unchanged

- **WHEN** every corpus driver runs before and after this change
- **THEN** every answer is identical
