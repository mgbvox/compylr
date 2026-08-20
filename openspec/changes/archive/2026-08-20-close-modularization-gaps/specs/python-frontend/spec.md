## ADDED Requirements

### Requirement: The Python frontend declares Python's container semantics

When lowering, the Python frontend SHALL set each container operation's declared semantics to what
Python means: a subscript counts a negative index from the end of a sequence, and a length counts
code points. It SHALL NOT rely on any other component defaulting to Python's interpretation.

#### Scenario: Subscripting declares counting from either end

- **WHEN** `xs[i]` is lowered
- **THEN** the resulting node declares that a negative index counts from the end

#### Scenario: Length declares code points

- **WHEN** `len(s)` is lowered
- **THEN** the resulting node declares that it counts code points

#### Scenario: The declaration is asserted, not the node's name

- **WHEN** the lowered form of a subscript or a length is examined
- **THEN** its meaning is determined by the declared mode rather than by which variant it is
