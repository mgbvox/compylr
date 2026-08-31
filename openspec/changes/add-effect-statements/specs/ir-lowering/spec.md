## MODIFIED Requirements

### Requirement: Docstrings are accepted and carry no runtime meaning

Lowering SHALL accept a **docstring**: a bare string-literal expression statement in the first
position of a function body. It SHALL contribute nothing to the function's behavior, matching
Python, where the interpreter records the docstring from the code object rather than by executing
the statement.

The exception SHALL be exactly this narrow. A bare expression statement anywhere else in a body,
or in first position but not a string literal, SHALL remain rejected: its value is discarded, so
it is either dead code or a side effect the subset cannot express. The one addition is an
**effectful intrinsic**, which the registry declares to produce no result and to be performed for
its effect — nothing is discarded, so the reason for the rejection does not apply to it.

A docstring SHALL NOT affect a function's fingerprint. It is prose about the function rather than
part of what the function computes, and a rebuild triggered by editing documentation would break
the existing guarantee that reformatting costs nothing.

#### Scenario: A documented function lowers

- **GIVEN** a function whose first body statement is a string literal
- **WHEN** it is lowered
- **THEN** lowering succeeds

#### Scenario: The docstring does not become a statement

- **GIVEN** a function with a docstring and a single `return`
- **WHEN** it is lowered
- **THEN** the IR body contains only the return statement

#### Scenario: The docstring is retained on the function

- **GIVEN** a function with a docstring
- **WHEN** it is lowered
- **THEN** the IR function carries the docstring's text

#### Scenario: A function with only a docstring and no return

- **GIVEN** a function annotated `-> None` whose body is just a docstring
- **WHEN** it is lowered
- **THEN** lowering succeeds and the body produces no value

#### Scenario: Editing a docstring does not change the fingerprint

- **GIVEN** one function, written twice with different docstring text
- **WHEN** both are lowered and fingerprinted
- **THEN** both produce the same fingerprint

#### Scenario: Adding a docstring does not change the fingerprint

- **GIVEN** one function, written with and without a docstring and otherwise identical
- **WHEN** both are lowered and fingerprinted
- **THEN** both produce the same fingerprint

#### Scenario: A string statement after the first is rejected

- **GIVEN** a body whose second statement is a bare string literal
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming the unsupported statement

#### Scenario: A non-string expression statement is still rejected

- **GIVEN** a body whose first statement is a bare expression such as `a + 1`
- **WHEN** it is lowered
- **THEN** lowering fails with a diagnostic naming the unsupported statement

#### Scenario: A bare call statement is still rejected

- **GIVEN** a body whose first statement is a bare call to a function in the unit, discarding its
  result
- **WHEN** it is lowered
- **THEN** lowering fails, because the subset cannot express a call made for its side effect

#### Scenario: An effectful intrinsic statement is accepted

- **GIVEN** a function whose body contains

  ```python
  print(label, total)
  ```

- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering succeeds and produces an effect statement, because the operation declares no
  result and so discards nothing

## ADDED Requirements

### Requirement: Output arguments are checked against what can be rendered

Lowering SHALL accept an output operation applied to any number of positional arguments whose types
have a defined rendering, and SHALL reject an argument whose type does not, naming the type and the
reason. A mapping or a set SHALL be rejected because its iteration order is not guaranteed.

#### Scenario Outline: A type with a defined rendering is accepted

- **GIVEN** a function whose body prints a value of type <type>
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering succeeds

**Examples:**

| type     |
| -------- |
| `int`    |
| `float`  |
| `bool`   |
| `str`    |
| `list`   |
| `tuple`  |

#### Scenario Outline: A type with no agreed rendering is refused with its reason

- **GIVEN** a function whose body prints a value of type <type>
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming <reason>

**Examples:**

| type                        | reason                                            |
| --------------------------- | ------------------------------------------------- |
| `dict`                      | the unspecified iteration order                   |
| `set`                       | the unspecified iteration order                   |
| `list[dict[str, int]]`      | the unordered element the sequence would render   |
| a class instance            | that the subset defines no rendering for the type |

#### Scenario: Keyword arguments need no new refusal

- **GIVEN** a function whose body contains an output call written with a keyword argument such as
  `sep` or `end`
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering fails through the existing rejection of keyword arguments
- **BUT** no separate diagnostic is added

#### Scenario: Multiple arguments are separated as the source language separates them

- **GIVEN** a function whose body prints several positional arguments
- **WHEN** the function is lowered by the `python` frontend
- **THEN** the IR carries them in order
- **AND** the rendering convention determines the separator and the terminator
