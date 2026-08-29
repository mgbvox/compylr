## ADDED Requirements

### Requirement: Every shape that forces ownership is exercised

The corpus SHALL contain a case for each shape that forces a parameter to be owned, and each case
SHALL assert both that the program produces the right answer and that the parameter's mode is
owned. A shape covered only by the answer SHALL fail the coverage check.

#### Scenario: The four reverted shapes each have a case

- **WHEN** the corpus is checked
- **THEN** appending a parameter, storing one as a mapping value, ordering-comparing a text
  parameter, and testing membership of one each have a case asserting an owned mode

#### Scenario: A text parameter remains usable in every position

- **WHEN** the existing text-parameter test runs
- **THEN** it passes unchanged, and it compiles a text parameter in every position it is legal in

#### Scenario: Mode is asserted on the unit, not on emitted text

- **WHEN** a case asserts a parameter's mode
- **THEN** it inspects the lowered unit rather than matching against generated source, except where
  the emitted form is itself the property under test

#### Scenario: A newly borrowable shape cannot pass silently

- **WHEN** a change causes a shape that previously forced ownership to become borrowed
- **THEN** its case fails, so the change is deliberate rather than discovered later
