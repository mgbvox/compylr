## Purpose

Defines what a compiled program may write to the host's output stream, how each value is rendered
as text, and how that output stays correctly ordered with output the host itself produces.

## ADDED Requirements

### Requirement: A printed value is rendered by a declared convention

Output SHALL carry a rendering convention naming whose spelling of a value is produced, and the
renderer SHALL be selected from that convention rather than from the target language's own default
formatting. Under the source language's convention, a compiled program's output SHALL be
byte-identical to what the same program produces interpreted. The convention rides on the operation
the way `units` rides on [`Expr::Len`](../../../../../crates/compylr-ir/src/ir.rs#L575).

#### Scenario Outline: A value renders the source language's spelling, not the target's

- **GIVEN** a program that prints <value> under the source convention
- **WHEN** the program is compiled for the `rust` backend and run
- **THEN** the text written is <source>
- **BUT** it is not <target>, which the target's own default formatting would produce

**Examples:**

| value                     | source  | target |
| ------------------------- | ------- | ------ |
| the boolean true          | `True`  | `true` |
| a float with zero fraction| `5.0`   | `5`    |

#### Scenario: An integer and a string render identically under both conventions

- **GIVEN** a program that prints an integer or a string
- **WHEN** the program is compiled and run
- **THEN** the text written is the same under either convention
- **AND** no conversion is applied to a string beyond writing its characters

#### Scenario: A sequence renders its elements in order

- **GIVEN** a program that prints a sequence or a tuple
- **WHEN** the program is compiled and run
- **THEN** the text written matches the interpreted program's, including delimiters, separators,
  and the rendering of each element

#### Scenario: Multiple arguments are joined by a single space

- **GIVEN** a program whose body contains

  ```python
  print(label, total)
  ```

- **WHEN** the program is compiled and run
- **THEN** the two rendered values are separated by a single space
- **AND** the line is terminated by a newline

#### Scenario: The convention is a mode, not an operation name

- **GIVEN** a backend emitting an output operation
- **WHEN** the emission is selected
- **THEN** it is selected from the declared convention
- **BUT** it is not selected from the operation's name, which would be wrong for the other
  convention

### Requirement: Printing an unordered container is refused

Output of a mapping or a set SHALL be rejected with a located diagnostic stating that iteration
order is not guaranteed, and therefore that its printed form is not a value a compiled program and
the interpreter can be required to agree on. The diagnostic SHALL name an accepted workaround.

#### Scenario Outline: Printing an unordered container is refused with its reason

- **GIVEN** a program whose body prints a <container>
- **WHEN** the program is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming the unspecified iteration order as the
  reason

**Examples:**

| container |
| --------- |
| mapping   |
| set       |

#### Scenario: The refusal names a workaround

- **GIVEN** a program whose body prints an unordered container
- **WHEN** the diagnostic is produced
- **THEN** it states that printing an ordered projection of the container is accepted, so the user
  is not left without a way to inspect it

#### Scenario: A sequence still prints

- **GIVEN** a program whose body prints a sequence
- **WHEN** the program is lowered by the `python` frontend
- **THEN** lowering succeeds, because a sequence's order is defined

### Requirement: Output reaches the host stream in the order the program produced it

A compiled program's output SHALL be written through a sink supplied by the host rather than
directly to the target runtime's own standard output, so that output from compiled and
interpreted code appears in the order it was produced, and so that host-level redirection captures
it. A default sink SHALL exist for a program running with no host.

#### Scenario: Interleaved output keeps its order

- **GIVEN** interpreted code that prints, calls a compiled function that prints, then prints again
- **WHEN** the program runs with its stream directed to a pipe or a file
- **THEN** the three lines appear in the order they were produced

#### Scenario: Host redirection captures compiled output

- **GIVEN** a host that has redirected its output stream
- **WHEN** it calls a compiled function that prints
- **THEN** the redirected stream receives the compiled output

#### Scenario: A default sink exists without a host

- **GIVEN** generated code running with no host sink installed
- **WHEN** the program prints
- **THEN** output goes to the target runtime's own standard output, so a program built outside a
  host still prints

#### Scenario: The sink does not reach the backend

- **GIVEN** a generated target source containing output
- **WHEN** the source is inspected
- **THEN** it names a runtime sink
- **BUT** it names no host language, keeping the backend free of any dependency on the language
  that called it
