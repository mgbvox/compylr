## ADDED Requirements

### Requirement: The boundary converts a parameter according to its mode

The bridge SHALL convert each argument at the host boundary according to the parameter's passing
mode, and SHALL avoid copying where the host's representation permits a borrow to be taken directly.
A borrow taken at the boundary SHALL NOT outlive the call.

#### Scenario: A borrowed text argument is not copied

- **GIVEN** a compiled function taking a borrowed text parameter
- **WHEN** the host calls it with a text value
- **THEN** the boundary borrows the host's buffer
- **BUT** it does not copy it

#### Scenario: An owned argument is converted as before

- **GIVEN** a compiled function taking an owned parameter
- **WHEN** the host calls it
- **THEN** the boundary converts it as it did before this change, with the same result

#### Scenario: A borrow does not outlive the call

- **GIVEN** a borrowed argument passed across the boundary
- **WHEN** the call returns
- **THEN** the borrow ends
- **AND** nothing retains it

#### Scenario: A collection argument is still converted element by element

- **GIVEN** a compiled function taking a sequence or mapping parameter under any mode
- **WHEN** the host calls it
- **THEN** the boundary still converts each element, because the host's representation is not a
  contiguous block of the element type

#### Scenario: Answers are unchanged

- **GIVEN** every driver in [`drivers/`](../../../../../frontends/python/fixtures/drivers/)
- **WHEN** each runs before and after this change
- **THEN** every answer is identical
