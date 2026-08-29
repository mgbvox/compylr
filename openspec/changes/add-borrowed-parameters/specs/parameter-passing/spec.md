## Purpose

Defines what it means for a parameter to be passed owned or borrowed, how the compiler decides
which from the function's body, and how far a borrowed value may travel before ownership is
required.

## ADDED Requirements

### Requirement: A parameter is owned unless a borrow is proven safe

Every parameter SHALL carry a passing mode of owned, shared borrow, or mutable borrow. Owned SHALL
be the default and the result whenever the analysis cannot prove a borrow safe. A parameter that
cannot be borrowed SHALL NOT produce a diagnostic, because the program is correct either way.

#### Scenario: An unanalyzable body yields ownership

- **GIVEN** a parameter whose use cannot be proven not to escape
- **WHEN** the unit is lowered
- **THEN** the parameter is owned
- **AND** compilation succeeds with no diagnostic

#### Scenario: A read-only parameter is borrowed

- **GIVEN** a function whose body is

  ```python
  def is_long(word: str) -> bool:
      return len(word) > 3
  ```

- **WHEN** the unit is lowered
- **THEN** `word` carries a shared borrow

#### Scenario: Ownership is never requested by the user

- **GIVEN** any supported source program
- **WHEN** it is compiled
- **THEN** no syntax expresses a passing mode
- **AND** no diagnostic mentions one

#### Scenario: The mode does not change what the program computes

- **GIVEN** one program compiled with every parameter forced owned and again with modes inferred
- **WHEN** every call is made against both
- **THEN** both produce identical answers

### Requirement: A parameter that escapes the call is owned

A parameter SHALL be owned whenever the body lets its value outlive the call: returning it, storing
it in a collection, an attribute, or any binding that outlives the call, appending it to a
sequence, using it as a mapping key or value that is retained, or passing it to anything that
requires ownership. Ownership is a question about escape, not about mutation.

#### Scenario Outline: A shape that keeps the value forces ownership

- **GIVEN** a function whose body contains `<shape>` applied to its parameter `who`
- **WHEN** the unit is lowered
- **THEN** `who` is owned

**Examples:**

| shape             | why it keeps the value                                  |
| ----------------- | ------------------------------------------------------- |
| `return who`      | the value outlives the call                              |
| `xs.append(who)`  | the sequence retains it                                  |
| `d[k] = who`      | the mapping retains it                                   |
| `self.name = who` | the instance retains it past the call                    |
| `who < "m"`       | the ordering is unavailable across the two spellings     |
| `who in xs`       | membership needs the owned representation                |

#### Scenario: A parameter passed to a function needing ownership is owned

- **GIVEN** a function passing its parameter to another function whose corresponding parameter is
  owned
- **WHEN** the unit is lowered
- **THEN** the caller's parameter is also owned

#### Scenario: Not mutating is not sufficient to borrow

- **GIVEN** a parameter that is never mutated and is appended to a sequence
- **WHEN** the unit is lowered
- **THEN** the parameter is owned
- **BUT** it is not borrowed on the grounds that it was never mutated

### Requirement: Ownership and mutability are decided by one analysis

The decision of whether a parameter is borrowed SHALL be made by the same fixpoint that decides
whether a receiver or an instance parameter is mutable, and SHALL reach the same conclusion for a
given program on every run.

#### Scenario: One analysis decides both

- **GIVEN** a function that both mutates an instance parameter and forwards a text parameter
- **WHEN** the unit is lowered
- **THEN** the mutability of the first and the ownership of the second are decided together
- **BUT** neither is re-derived separately

#### Scenario: Transitive requirements propagate

- **GIVEN** a function passing a parameter to a callee that stores it, discovered only once that
  callee is analyzed
- **WHEN** the fixpoint runs
- **THEN** it re-runs and the caller's parameter becomes owned

#### Scenario: Mutual recursion terminates

- **GIVEN** two functions that pass parameters to one another
- **WHEN** the fixpoint runs
- **THEN** it terminates
- **AND** it produces the same modes regardless of which was analyzed first

#### Scenario: The result does not depend on declaration order

- **GIVEN** the same functions declared in a different order
- **WHEN** the unit is lowered
- **THEN** every parameter receives the same mode

### Requirement: A borrow does not reach beyond the call

A borrowed value MAY be read, mutated where its mode permits, and forwarded to a parameter that
borrows it compatibly. It SHALL NOT be returned as an owned value, stored, or forwarded to a
parameter requiring ownership. Where a program requires any of those, the parameter is owned rather
than the program refused.

#### Scenario: A borrowed value may be forwarded compatibly

- **GIVEN** a function passing a borrowed parameter to another function that also borrows it
- **WHEN** the unit is lowered and emitted
- **THEN** both remain borrowed
- **AND** no copy is made

#### Scenario: Reading a borrowed value does not copy it

- **GIVEN** a borrowed parameter read several times in a body
- **WHEN** the unit is emitted
- **THEN** no copy is made for any read

#### Scenario: The existing instance rules are unchanged

- **GIVEN** a program that returns a borrowed instance, or a field of one
- **WHEN** the program is lowered
- **THEN** it is refused by the existing diagnostics
- **BUT** this change does not relax them
