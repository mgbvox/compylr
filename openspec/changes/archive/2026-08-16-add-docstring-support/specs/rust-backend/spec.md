## ADDED Requirements

### Requirement: A function's docstring is emitted as documentation

When a function carries a docstring, the backend SHALL emit it as a doc comment on the generated
function. The generated source is written to disk for people to read, and a translated function
stripped of the explanation its author wrote is harder to check against the original than it
needs to be.

The emitted text SHALL denote the same characters as the docstring, including when it contains
characters that would otherwise end or escape a comment.

#### Scenario: A docstring reaches the generated source

- **WHEN** a function with a docstring is emitted
- **THEN** the generated Rust carries that text as a doc comment on the function

#### Scenario: A function without a docstring emits none

- **WHEN** a function with no docstring is emitted
- **THEN** no doc comment is emitted for it

#### Scenario: A multi-line docstring stays readable

- **WHEN** a function whose docstring spans several lines is emitted
- **THEN** each line appears in the doc comment, and the result compiles

#### Scenario: A docstring cannot break out of its comment

- **WHEN** a docstring containing a newline, a `*/`, and a backslash is emitted
- **THEN** the generated Rust still compiles and the comment denotes the original characters

#### Scenario: Emission stays deterministic

- **WHEN** the same documented function is emitted twice
- **THEN** the two outputs are byte-identical
