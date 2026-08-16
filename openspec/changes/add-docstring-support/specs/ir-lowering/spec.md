## ADDED Requirements

### Requirement: Docstrings are accepted and carry no runtime meaning

Lowering SHALL accept a **docstring**: a bare string-literal expression statement in the first
position of a function body. It SHALL contribute nothing to the function's behavior, matching
Python, where the interpreter records the docstring from the code object rather than by executing
the statement.

The exception SHALL be exactly this narrow. A bare expression statement anywhere else in a body,
or in first position but not a string literal, SHALL remain rejected: its value is discarded, so
it is either dead code or a side effect the subset cannot express.

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

- **WHEN** lowering a body whose first statement is a bare call, discarding its result
- **THEN** lowering fails, because the subset cannot express a call made for its side effect

## MODIFIED Requirements

### Requirement: Reject constructs outside the subset

Lowering SHALL reject any statement or expression outside the supported subset, including
control flow, class and import statements, and top-level statements other than function
definitions. The diagnostic SHALL name the unsupported construct. The single exception is a
leading docstring, defined in "Docstrings are accepted and carry no runtime meaning".

#### Scenario: Control flow is rejected

- **WHEN** lowering a function body containing an `if` statement
- **THEN** lowering fails with a diagnostic naming the conditional as unsupported

#### Scenario: Top-level statement is rejected

- **WHEN** lowering a source containing an `if __name__ == '__main__':` guard
- **THEN** lowering fails with a diagnostic reporting that only function definitions are
  permitted at top level

#### Scenario: A module-level docstring is still rejected

- **WHEN** lowering a source whose first statement is a module-level string literal
- **THEN** lowering fails, because the docstring exception applies only inside a function body

#### Scenario: Import is rejected

- **WHEN** lowering a source containing an import statement
- **THEN** lowering fails with a diagnostic naming the import as unsupported

#### Scenario: Non-simple parameter forms are rejected

- **WHEN** lowering a function declaring variadic parameters (`*args` or `**kwargs`),
  keyword-only or positional-only parameters, or a parameter with a default value
- **THEN** lowering fails with a diagnostic naming the unsupported parameter form

#### Scenario: Decorated or async function is rejected

- **WHEN** lowering a function that carries a decorator or is declared `async def`
- **THEN** lowering fails with a diagnostic naming the unsupported function form

#### Scenario: True division is accepted

- **WHEN** lowering an expression using `/`
- **THEN** lowering succeeds, because true division is now part of the supported subset

#### Scenario: Unsupported operator is rejected

- **WHEN** lowering an expression using an operator outside the supported set, such as
  exponentiation or a bitwise operator
- **THEN** lowering fails with a diagnostic naming the operator as unsupported

#### Scenario: Out-of-range integer literal is rejected

- **WHEN** lowering an integer literal too large to be represented as an `i64`
- **THEN** lowering fails with a diagnostic reporting that the literal exceeds the supported
  integer range, rather than silently truncating it

#### Scenario: Non-finite float literal is not producible

- **WHEN** lowering any floating-point literal written in source
- **THEN** lowering succeeds, since Python source cannot spell infinity or NaN as a literal
