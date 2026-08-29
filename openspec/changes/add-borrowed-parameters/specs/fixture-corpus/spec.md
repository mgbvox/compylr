## ADDED Requirements

### Requirement: Every shape that forces ownership is exercised

The corpus SHALL contain a case for each shape that forces a parameter to be owned, and each case
SHALL assert both that the program produces the right answer and that the parameter's mode is
owned. A shape covered only by the answer SHALL fail the coverage check, because an answers-only
suite cannot distinguish this change working from this change doing nothing — which is how the
reverted attempt passed the whole suite while it was broken.

#### Scenario Outline: Each reverted shape has a case asserting an owned mode

- **GIVEN** a fixture whose body applies `<shape>` to its parameter
- **WHEN** the corpus suite runs
- **THEN** the case asserts the parameter's mode is owned
- **AND** it asserts the program's answer

**Examples:**

| shape            |
| ---------------- |
| `xs.append(who)` |
| `d[k] = who`     |
| `who < "m"`      |
| `who in xs`      |

#### Scenario: A text parameter remains usable in every position

- **GIVEN** the existing text-parameter gate test
- **WHEN** the suite runs
- **THEN** it passes unchanged
- **AND** it compiles a text parameter in every position it is legal in

#### Scenario: Mode is asserted on the unit, not on emitted text

- **GIVEN** a case asserting a parameter's mode
- **WHEN** the assertion is made
- **THEN** it inspects the lowered unit
- **BUT** it does not match against generated source, except where the emitted form is itself the
  property under test

#### Scenario: A newly borrowable shape cannot pass silently

- **GIVEN** a change causing a shape that previously forced ownership to become borrowed
- **WHEN** the corpus suite runs
- **THEN** its case fails, so the change is deliberate rather than discovered later
