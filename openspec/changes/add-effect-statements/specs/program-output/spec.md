## Purpose

Defines what a compiled program may write to the host's output stream, how each value is rendered
as text, and how that output stays correctly ordered with output the host itself produces.

## ADDED Requirements

### Requirement: A printed value is rendered by a declared convention

Output SHALL carry a rendering convention naming whose spelling of a value is produced, and the
renderer SHALL be selected from that convention rather than from the target language's own default
formatting. Under the source language's convention, a compiled program's output SHALL be
byte-identical to what the same program produces interpreted.

#### Scenario: A boolean renders the source language's spelling

- **WHEN** a program prints a boolean under the source convention
- **THEN** the text written matches what the interpreted program writes, rather than the target
  language's own boolean spelling

#### Scenario: A float renders the source language's spelling

- **WHEN** a program prints a floating-point value whose fractional part is zero
- **THEN** the text written matches the interpreted program's, rather than dropping the fractional
  part as the target's default formatting would

#### Scenario: An integer and a string render identically under both conventions

- **WHEN** a program prints an integer or a string
- **THEN** the text written is the same under either convention, and no conversion is applied to a
  string beyond writing its characters

#### Scenario: A sequence renders its elements in order

- **WHEN** a program prints a sequence or a tuple
- **THEN** the text written matches the interpreted program's, including delimiters, separators,
  and the rendering of each element

#### Scenario: The convention is a mode, not an operation name

- **WHEN** a backend emits an output operation
- **THEN** it selects the renderer from the declared convention, and a backend reading the
  operation's name instead would be wrong for the other convention

### Requirement: Printing an unordered container is refused

Output of a mapping or a set SHALL be rejected with a located diagnostic stating that iteration
order is not guaranteed, and therefore that its printed form is not a value a compiled program and
the interpreter can be required to agree on.

#### Scenario: Printing a mapping is refused

- **WHEN** lowering a statement that prints a mapping
- **THEN** lowering fails with a located diagnostic naming the unspecified iteration order as the
  reason

#### Scenario: Printing a set is refused

- **WHEN** lowering a statement that prints a set
- **THEN** lowering fails with a located diagnostic naming the unspecified iteration order as the
  reason

#### Scenario: The refusal names a workaround

- **WHEN** the diagnostic for an unordered container is produced
- **THEN** it states that printing an ordered projection of the container is accepted, so the user
  is not left without a way to inspect it

#### Scenario: A sequence of an ordered element type still prints

- **WHEN** lowering a statement that prints a sequence
- **THEN** lowering succeeds, because a sequence's order is defined

### Requirement: Output reaches the host stream in the order the program produced it

A compiled program's output SHALL be written through a sink supplied by the host rather than
directly to the target runtime's own standard output, so that output from compiled and
interpreted code appears in the order it was produced, and so that host-level redirection captures
it.

#### Scenario: Interleaved output keeps its order

- **WHEN** interpreted code prints, then calls a compiled function that prints, then prints again
- **THEN** the three lines appear in that order, including when the stream is a pipe or a file
  rather than a terminal

#### Scenario: Host redirection captures compiled output

- **WHEN** the host redirects its output stream and then calls a compiled function that prints
- **THEN** the redirected stream receives the compiled output

#### Scenario: A default sink exists without a host

- **WHEN** generated code runs with no host sink installed
- **THEN** output goes to the target runtime's own standard output, so a program built outside a
  host still prints

#### Scenario: The sink does not reach the backend

- **WHEN** the generated target source is inspected
- **THEN** it names a runtime sink and no host language, keeping the backend free of any dependency
  on the language that called it
