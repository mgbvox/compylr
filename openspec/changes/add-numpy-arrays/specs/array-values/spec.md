## Purpose

Defines the array type a compiled program can accept: how its element storage and rank are
declared, what indexing and shape mean, and the guarantee that an array parameter is a view over
the caller's buffer rather than a copy of it.

## ADDED Requirements

### Requirement: An array type carries a declared storage and rank

An array type SHALL carry the storage of its elements and its rank, both declared in the source.
Rank SHALL be part of the type rather than discovered at runtime. An annotation that does not
declare both SHALL be rejected with a located diagnostic naming the accepted spelling, for the
reason [`bare_list_annotation.py`](../../../../../frontends/python/fixtures/rejected/bare_list_annotation.py)
is refused: a fact that is not written down is not a type compylr can use.

#### Scenario: A ranked annotation is accepted

- **GIVEN** a function whose signature is

  ```python
  def dot(a: compylr.Array1[np.float64], b: compylr.Array1[np.float64]) -> float: ...
  ```

- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering succeeds
- **AND** each parameter's type carries both storage and rank

#### Scenario Outline: An annotation missing a declared fact is refused

- **GIVEN** a parameter annotated `<annotation>`
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming <names>

**Examples:**

| annotation             | names                                          |
| ---------------------- | ---------------------------------------------- |
| `np.ndarray`           | the ranked spelling, and the missing storage    |
| `NDArray[np.float64]`  | the ranked spelling                             |

#### Scenario: An unsupported storage is refused as planned

- **GIVEN** a parameter annotated as an array of a storage outside the supported set, such as
  `np.float32`
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic reporting that storage as planned
- **BUT** it does not report the annotation as unknown

#### Scenario: Two ranks are two types

- **GIVEN** an array of rank one and an array of rank two over the same storage
- **WHEN** one is passed where the other is declared
- **THEN** lowering fails, because they are different types

### Requirement: Reading an element yields a scalar of the existing model

Reading an array element SHALL yield a value of the existing integer or floating-point type
according to the array's storage, so that the array type introduces no new scalar type and no new
integer width into [`Ty`](../../../../../crates/compylr-ir/src/ir.rs#L103).

#### Scenario Outline: An element reads as the model's own scalar

- **GIVEN** an array whose storage is <storage>
- **WHEN** one of its elements is read
- **THEN** the read's type is the model's <scalar> type

**Examples:**

| storage   | scalar         |
| --------- | -------------- |
| `float64` | floating-point |
| `int64`   | integer        |

#### Scenario: An element participates in ordinary arithmetic

- **GIVEN** an element read from an array
- **WHEN** it is combined with another numeric expression
- **THEN** the existing operator type rules and numeric promotion apply unchanged

### Requirement: Indexing names one element per rank

Indexing an array SHALL supply exactly one index per rank and SHALL yield an element. Supplying
fewer indices than the rank SHALL be refused, because the result would be a view that outlives the
expression. The declared index origin and checking mode SHALL apply as they do for a sequence.

#### Scenario Outline: A full index yields an element

- **GIVEN** an array of rank <rank>
- **WHEN** it is subscripted with <indices> in one subscript
- **THEN** the read succeeds and yields an element

**Examples:**

| rank | indices     |
| ---- | ----------- |
| 1    | one index   |
| 2    | two indices |

#### Scenario: Partial indexing is refused

- **GIVEN** a rank-two array subscripted with a single index
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic explaining that every index must be supplied,
  because a partial index would produce a view

#### Scenario: A negative index resolves by the declared origin

- **GIVEN** an array indexed with a negative offset
- **WHEN** the unit is compiled and run
- **THEN** it resolves according to the declared index origin, as a sequence does

#### Scenario: An out-of-range index honours the declared checking mode

- **GIVEN** an array indexed outside its extent under the reported checking mode
- **WHEN** the compiled function runs
- **THEN** the failure is recoverable and carries a located message
- **BUT** it does not abort

### Requirement: An array parameter is a view over the caller's buffer

An array parameter SHALL be bound as a view over the memory the caller supplied, and SHALL NOT be
copied at the boundary. A write through a mutably bound array parameter SHALL be visible to the
caller after the call returns.

#### Scenario: No copy is made at the boundary

- **GIVEN** a compiled function taking an array parameter
- **WHEN** it is called with arrays of increasing size
- **THEN** the time taken before the body runs does not grow with the number of elements

#### Scenario: A write is visible to the caller

- **GIVEN** a compiled function that writes to an element of a mutably bound array parameter
- **WHEN** the caller calls it and then reads its own array
- **THEN** the caller observes the new value

#### Scenario: A read-only parameter does not permit writing

- **GIVEN** a function that only reads an array parameter
- **WHEN** the unit is lowered and emitted
- **THEN** the parameter is bound as a shared view
- **AND** the emitted code cannot write through it

#### Scenario: A strided array stays a view

- **GIVEN** a non-contiguous array, such as a strided slice
- **WHEN** a compiled function is called with it
- **THEN** it is bound as a strided view
- **BUT** it is still not copied

#### Scenario: The contrast with collections holds

- **GIVEN** a function declaring both a sequence parameter and an array parameter, mutating each
- **WHEN** the function is lowered
- **THEN** mutating the sequence parameter is refused as it is today
- **BUT** mutating the array parameter is accepted and observed by the caller

### Requirement: Overlapping mutable array parameters are refused

Where a function takes more than one array parameter and at least one is mutably bound, the call
SHALL be refused when two of those parameters refer to overlapping memory. Two Rust references to
one buffer with one mutable is undefined behavior rather than a wrong answer, and nothing in the
type system catches it.

#### Scenario: The same array passed twice is refused

- **GIVEN** a compiled function taking two array parameters, one mutably bound
- **WHEN** it is called with the same array for both
- **THEN** the call raises an error naming the overlap
- **AND** no compiled code runs

#### Scenario: Overlapping views are refused

- **GIVEN** two parameters that are different views over overlapping regions of one buffer, one
  mutably bound
- **WHEN** the function is called
- **THEN** the call raises an error naming the overlap

#### Scenario: Distinct arrays are accepted

- **GIVEN** two array parameters referring to separate buffers
- **WHEN** the function is called
- **THEN** the call proceeds

#### Scenario: Shared-only parameters do not need the check

- **GIVEN** a function every one of whose array parameters is bound as a shared view
- **WHEN** it is called with the same array for several parameters
- **THEN** the call is accepted

### Requirement: An array may not be returned or constructed

Returning an array, constructing one, or storing one SHALL be refused with a located diagnostic
naming the capability as not yet supported. A function taking array parameters MAY return a scalar,
and in-place output through a mutably bound parameter is the supported idiom.

#### Scenario: Returning an array is refused

- **GIVEN** a function declaring an array return type
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering fails with a located diagnostic naming array creation as not yet supported

#### Scenario: Storing an array is refused

- **GIVEN** an assignment of an array parameter to an attribute or a collection
- **WHEN** the unit is lowered
- **THEN** lowering fails, because the view would outlive the call

#### Scenario: A scalar return is accepted

- **GIVEN** a function that reduces an array parameter to a scalar
- **WHEN** the function is lowered by the `python` frontend
- **THEN** lowering succeeds

#### Scenario: In-place output is the supported idiom

- **GIVEN** a function that writes its results into a mutably bound array parameter
- **WHEN** the caller calls it
- **THEN** lowering succeeds and the caller observes the results
