## MODIFIED Requirements

### Requirement: Reject a function that cannot return its declared type

Lowering SHALL reject a function whose declared return type is not the unit type and whose body
cannot produce a value on **every path**. The diagnostic SHALL name the function and report its
location.

With branching, this is no longer the structural question of whether the last statement is a
`return`. A body returns when its final statement returns; a conditional returns only when it has
an alternative **and both branches return**; and a loop SHALL NOT be assumed to return, because its
body may never run. Treating a loop as returning would let a program through whose generated code
does not compile, and the resulting complaint would be about Rust rather than about the user's
function.

This is a program the user wrote incorrectly, so it belongs with every other subset violation. Left
to a backend, it surfaces as an internal code-generation error with no source location, which
describes the compiler's difficulty rather than the user's mistake.

#### Scenario: A body of only pass is rejected

- **WHEN** lowering `def f() -> int:` whose body is `pass`
- **THEN** lowering fails with a diagnostic naming `f` and reporting its location

#### Scenario: A body ending in a binding is rejected

- **WHEN** lowering a function declaring an integer return whose body binds a local and stops
- **THEN** lowering fails with a diagnostic naming the function

#### Scenario: A conditional returning on both branches is accepted

- **WHEN** lowering a function whose body is an `if`/`else` where both branches return
- **THEN** lowering succeeds

#### Scenario: A conditional with no alternative does not return

- **WHEN** lowering a function whose only `return` is inside an `if` with no `else`
- **THEN** lowering fails, because the path where the test is false produces no value

#### Scenario: One branch returning is not enough

- **WHEN** lowering a function whose `if` returns but whose `else` does not
- **THEN** lowering fails

#### Scenario: A return after a conditional covers it

- **WHEN** lowering a function with an `if` that returns, followed by a `return`
- **THEN** lowering succeeds

#### Scenario: A loop is not assumed to run

- **WHEN** lowering a function whose only `return` is inside a `while`
- **THEN** lowering fails, because the loop body may never execute

#### Scenario: Nested conditionals are analysed through

- **WHEN** lowering a function whose branches each contain further conditionals that all return
- **THEN** lowering succeeds

#### Scenario: A unit-returning function needs no return

- **WHEN** lowering `def f() -> None:` whose body is `pass`
- **THEN** lowering succeeds

#### Scenario: A function that does return is unaffected

- **WHEN** lowering a function whose body ends in a `return`
- **THEN** lowering succeeds

#### Scenario: The diagnostic distinguishes this from a type mismatch

- **WHEN** a function that cannot return is rejected
- **THEN** the diagnostic reports a missing return rather than a mismatch between two types

## ADDED Requirements

### Requirement: Conditionals

Lowering SHALL accept `if`, `elif`, and `else`. The test SHALL be a boolean; any other type SHALL
be rejected reporting the type found.

Python treats many values as truthy, but compylr does not: a subset whose annotations are mandatory
should not then infer that an integer means a condition. Requiring a boolean keeps the meaning of a
test written down rather than inferred.

#### Scenario: A conditional lowers

- **WHEN** lowering a body containing `if a < b:` with a returning branch
- **THEN** lowering succeeds

#### Scenario: An alternative lowers

- **WHEN** lowering a body containing `if`/`else`
- **THEN** both branches appear in the IR

#### Scenario: elif lowers as a nested conditional

- **WHEN** lowering a body containing `if`/`elif`/`else`
- **THEN** the IR nests the second conditional inside the first one's alternative

#### Scenario: A non-boolean test is rejected

- **WHEN** lowering `if n:` where `n` is an integer
- **THEN** lowering fails with a diagnostic reporting that a test must be a boolean

#### Scenario: A branch is a scope for reachability but not for names

- **WHEN** a name is bound inside a branch and read after the conditional
- **THEN** lowering rejects the read, because the binding may not have happened

### Requirement: Loops

Lowering SHALL accept `while` with a boolean test, and `for` binding one name over a range or a
supported collection. It SHALL accept `break` and `continue` inside a loop body and reject them
outside one.

Iterating a sequence SHALL bind its element type; a mapping SHALL bind its **key** type, matching
Python; a set SHALL bind its element type; and a range SHALL bind an integer.

#### Scenario: A while loop lowers

- **WHEN** lowering a body containing `while a < b:`
- **THEN** lowering succeeds

#### Scenario: A non-boolean while test is rejected

- **WHEN** lowering `while n:` where `n` is an integer
- **THEN** lowering fails reporting that a test must be a boolean

#### Scenario: Iterating a range binds an integer

- **WHEN** lowering `for i in range(n):`
- **THEN** `i` is bound with the integer type

#### Scenario: Iterating a sequence binds its element type

- **WHEN** lowering `for x in xs:` where `xs` is a sequence of strings
- **THEN** `x` is bound with the string type

#### Scenario: Iterating a mapping binds its key type

- **WHEN** lowering `for k in d:` where `d` maps strings to integers
- **THEN** `k` is bound with the string type, matching Python

#### Scenario: Iterating a scalar is rejected

- **WHEN** lowering `for x in n:` where `n` is an integer
- **THEN** lowering fails reporting the type

#### Scenario: The loop variable does not escape

- **WHEN** a name bound by a `for` is read after the loop
- **THEN** lowering rejects the read

#### Scenario: Loop control inside a loop

- **WHEN** lowering a loop body containing `break` and `continue`
- **THEN** lowering succeeds

#### Scenario: Loop control outside a loop is rejected

- **WHEN** lowering `break` in a function body with no enclosing loop
- **THEN** lowering fails reporting that it is not inside a loop

#### Scenario: Loop control reaches the nearest enclosing loop

- **WHEN** lowering a `break` inside a conditional inside a loop
- **THEN** lowering succeeds

### Requirement: Reassignment keeps a name's type

Lowering SHALL accept assigning to a name already bound in the same function. The name's type is
fixed where it was first bound: a value of a different type SHALL be rejected, with promotion
applying as it does elsewhere.

Rebinding is not re-declaration. Allowing a name to change type would mean the same identifier
denotes different things at different points, which a reader has to simulate the program to follow,
and which every backend would then have to model.

#### Scenario: Reassignment lowers

- **WHEN** lowering a body binding `i = 0` and then `i = i + 1`
- **THEN** lowering succeeds and `i` keeps the integer type

#### Scenario: A different type is rejected

- **WHEN** lowering a body binding `i = 0` and then `i = "x"`
- **THEN** lowering fails reporting both types

#### Scenario: Promotion applies

- **WHEN** lowering a body binding `x: float = 1.0` and then `x = 2`
- **THEN** lowering succeeds and the integer carries an explicit conversion

#### Scenario: An annotation on a rebinding is rejected

- **WHEN** lowering a body binding `i = 0` and then `i: int = 1`
- **THEN** lowering fails, because the second annotation re-declares a name that already exists

#### Scenario: A parameter may be reassigned

- **WHEN** lowering a body assigning to one of its own parameters
- **THEN** lowering succeeds and the parameter keeps its declared type

#### Scenario: Reassignment inside a loop is the point

- **WHEN** lowering a `while` whose body increments a counter bound before it
- **THEN** lowering succeeds

### Requirement: range is reserved

Lowering SHALL recognise `range` with one, two, or three integer arguments and reject any other
arity or argument type. A function in the unit named `range` SHALL be rejected, on the same terms
as `len`: a builtin whose meaning depended on what else had been compiled would be worse than no
builtin at all.

#### Scenario: One argument

- **WHEN** lowering `range(n)`
- **THEN** the IR carries a start of zero, a stop of `n`, and a step of one

#### Scenario: Two and three arguments

- **WHEN** lowering `range(a, b)` and `range(a, b, c)`
- **THEN** each component is carried as written

#### Scenario: A non-integer argument is rejected

- **WHEN** lowering `range(x)` where `x` is a string
- **THEN** lowering fails reporting the type

#### Scenario: Wrong arity is rejected

- **WHEN** lowering `range()` or a call with four arguments
- **THEN** lowering fails reporting the argument count

#### Scenario: A user function named range is rejected

- **WHEN** lowering a source defining `def range(n: int) -> int:`
- **THEN** lowering fails reporting that `range` is reserved

#### Scenario: A range outside a loop is rejected

- **WHEN** lowering a binding whose initializer is a bare `range(n)`
- **THEN** lowering fails, because a range is only meaningful as something to iterate
