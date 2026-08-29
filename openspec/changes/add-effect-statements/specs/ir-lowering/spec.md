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

- **WHEN** lowering a function whose first body statement is a string literal
- **THEN** lowering succeeds

#### Scenario: The docstring does not become a statement

- **WHEN** a function with a docstring and a single `return` is lowered
- **THEN** the IR body contains only the return statement

#### Scenario: The docstring is retained on the function

- **WHEN** a function with a docstring is lowered
- **THEN** the IR function carries the docstring's text

#### Scenario: A function with only a docstring and no return

- **WHEN** lowering a function annotated `-> None` whose body is just a docstring
- **THEN** lowering succeeds and the body produces no value

#### Scenario: Editing a docstring does not change the fingerprint

- **WHEN** the same function is lowered twice with different docstring text
- **THEN** both produce the same fingerprint

#### Scenario: Adding a docstring does not change the fingerprint

- **WHEN** a function is lowered with and without a docstring, its code otherwise identical
- **THEN** both produce the same fingerprint

#### Scenario: A string statement after the first is rejected

- **WHEN** lowering a body whose second statement is a bare string literal
- **THEN** lowering fails with a diagnostic naming the unsupported statement

#### Scenario: A non-string expression statement is still rejected

- **WHEN** lowering a body whose first statement is a bare expression such as `a + 1`
- **THEN** lowering fails with a diagnostic naming the unsupported statement

#### Scenario: A bare call statement is still rejected

- **WHEN** lowering a body whose first statement is a bare call to a function in the unit,
  discarding its result
- **THEN** lowering fails, because the subset cannot express a call made for its side effect

#### Scenario: An effectful intrinsic statement is accepted

- **WHEN** lowering a body statement consisting of an effectful intrinsic call
- **THEN** lowering succeeds and produces an effect statement, because the operation declares no
  result and so discards nothing

## ADDED Requirements

### Requirement: Output arguments are checked against what can be rendered

Lowering SHALL accept an output operation applied to any number of positional arguments whose types
have a defined rendering, and SHALL reject an argument whose type does not, naming the type and the
reason. A mapping or a set SHALL be rejected because its iteration order is not guaranteed.

#### Scenario: Scalars and ordered containers are accepted

- **WHEN** lowering an output of an integer, float, boolean, string, sequence, or tuple
- **THEN** lowering succeeds

#### Scenario: A mapping argument is refused with its reason

- **WHEN** lowering an output of a mapping
- **THEN** lowering fails with a located diagnostic naming the unspecified iteration order

#### Scenario: A set argument is refused with its reason

- **WHEN** lowering an output of a set
- **THEN** lowering fails with a located diagnostic naming the unspecified iteration order

#### Scenario: A nested unordered container is refused

- **WHEN** lowering an output of a sequence whose element type is a mapping or a set
- **THEN** lowering fails, because rendering the sequence would render its unordered elements

#### Scenario: An instance argument is refused

- **WHEN** lowering an output of a class instance
- **THEN** lowering fails with a located diagnostic, because the subset defines no rendering for a
  user-defined type

#### Scenario: Keyword arguments need no new refusal

- **WHEN** lowering an output call written with a keyword argument such as `sep` or `end`
- **THEN** lowering fails through the existing rejection of keyword arguments, and no separate
  diagnostic is added

#### Scenario: Multiple arguments are separated as the source language separates them

- **WHEN** lowering an output of several positional arguments
- **THEN** the IR carries them in order, and the rendering convention determines the separator and
  the terminator
