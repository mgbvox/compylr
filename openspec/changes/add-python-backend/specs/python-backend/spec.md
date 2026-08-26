## Purpose

Translating the IR into Python source: how each IR type and operation is spelled, and how a
declared mode that is not Python's own is still expressed in Python. It exists so the IR has a
second consumer, so a compiled program can be read back in the language it was written in, and so
translation can be checked against an interpreter without a build toolchain.

## ADDED Requirements

### Requirement: Concrete type spellings

The backend SHALL map each IR type to a Python type annotation. The mapping SHALL live in the
backend alone: no IR type carries a Python spelling. Collection spellings SHALL be derived from
their parameters, recursively. The mapping SHALL be derived from the IR's semantic types only, so a
unit produced by any frontend spells the same way.

| IR type | Python annotation |
| --- | --- |
| integer | `int` |
| float | `float` |
| bool | `bool` |
| string | `str` |
| unit | `None` |
| sequence of `T` | `list[T]` |
| mapping from `K` to `V` | `dict[K, V]` |
| set of `T` | `set[T]` |
| tuple of `T1..Tn` | `tuple[T1, .., Tn]` |

#### Scenario: Each type is spelled

- **WHEN** a function's parameters and return type cover all five scalar IR types
- **THEN** the emitted Python annotates them `int`, `float`, `bool`, `str`, and `None`

#### Scenario: Each collection type is spelled

- **WHEN** a function's parameters cover a sequence, mapping, set, and tuple
- **THEN** the emitted Python annotates them `list`, `dict`, `set`, and `tuple` respectively

#### Scenario: Nested collections spell recursively

- **WHEN** a parameter typed as a mapping from strings to sequences of integers is emitted
- **THEN** the emitted Python spells it `dict[str, list[int]]`

#### Scenario: Spelling does not depend on the producing frontend

- **WHEN** two units with identical types record different producing frontends
- **THEN** the emitted Python type spellings are identical

### Requirement: Emitted source is valid, fully annotated Python

Emitted source SHALL parse as Python, and every function SHALL carry a complete set of parameter
annotations and a return annotation.

Emitted source SHALL be accepted by the compylr Python frontend wherever the unit came from a
program that frontend accepted. A unit the frontend produced and the backend emitted SHALL lower
again — a **round trip** — so that the emitted source is not merely valid Python but Python inside
the supported subset.

A unit that cannot round-trip because it declares a mode the subset has no syntax for SHALL still
emit valid Python, and the failure to round-trip SHALL be a known, named case rather than a
surprise.

#### Scenario: Emitted source parses

- **WHEN** any unit is emitted
- **THEN** the result parses as Python

#### Scenario: Every function is annotated

- **WHEN** a function is emitted
- **THEN** every parameter carries an annotation and the function carries a return annotation

#### Scenario: A unit from Python round-trips

- **WHEN** a unit lowered from Python source is emitted and lowered again
- **THEN** lowering succeeds and the two units have the same fingerprint

### Requirement: Declared modes are honored, not the operation's name

The backend SHALL determine what to emit from each node's **declared mode**, never from which
operation it is or from what a Python programmer would have written to produce it.

Where a declared mode is Python's own, the backend MAY emit Python's operator. Where it is not, the
backend SHALL emit Python that produces the declared result. This applies to integer division
rounding, remainder sign, exact division, index origin, text units, and every checking mode.

#### Scenario: Python's own rounding emits Python's operator

- **WHEN** an integer division declaring rounding toward negative infinity is emitted
- **THEN** the emitted Python computes the floor, which `//` already does

#### Scenario: A rounding that is not Python's is still expressed

- **WHEN** an integer division declaring rounding toward zero is emitted
- **THEN** the emitted Python truncates toward zero, and `//` alone is not what is emitted

#### Scenario: A remainder sign that is not Python's is still expressed

- **WHEN** a remainder declaring the sign of the dividend is emitted
- **THEN** the emitted Python takes the sign of the dividend, and `%` alone is not what is emitted

#### Scenario: An index origin that is not Python's is still expressed

- **WHEN** a subscript declaring that indexes count from the start is emitted
- **THEN** a negative index does not count from the end of the sequence

#### Scenario: Text units that are not Python's are still expressed

- **WHEN** a length declaring UTF-8 bytes is emitted
- **THEN** the emitted Python counts bytes rather than code points

#### Scenario: An unchecked operation declines to report

- **WHEN** an arithmetic operation declaring that the program does not define its failure is emitted
- **THEN** the emitted Python does not raise where the reported form would

### Requirement: Emission is deterministic and pure

Emitting the same unit twice SHALL produce byte-identical output. Emission SHALL perform no
input or output, read no environment, and invoke no external process.

Formatting SHALL be post-processing, applied by whoever writes the files out, and SHALL preserve
meaning.

#### Scenario: Emission is byte-reproducible

- **WHEN** a unit is emitted twice
- **THEN** the two outputs are byte-identical

#### Scenario: Emission touches nothing outside itself

- **WHEN** a unit is emitted
- **THEN** no file is read or written and no process is started

#### Scenario: Formatting does not change behavior

- **WHEN** emitted source is formatted and then run
- **THEN** it produces the same results as the unformatted source

### Requirement: Emission produces a named set of files

The backend SHALL emit a named set of files rather than one stream, and SHALL declare which of them
holds the translated functions, so that a caller wanting only the translation does not have to know
which backend produced it.

#### Scenario: The translated functions are in their own file

- **WHEN** a unit is emitted
- **THEN** the translated functions are in a file the backend names, separate from any helpers

#### Scenario: The translated file is identified without naming the backend

- **WHEN** a caller asks the backend which file holds the translation
- **THEN** the backend answers, and the caller does not consult the backend's identity

### Requirement: A function's docstring is carried through

A function's docstring SHALL be emitted as the emitted function's docstring, in first position.
A function without one SHALL emit none.

#### Scenario: A docstring survives

- **WHEN** a function carrying a docstring is emitted
- **THEN** the emitted function opens with that docstring

#### Scenario: No docstring emits none

- **WHEN** a function without a docstring is emitted
- **THEN** the emitted function has no leading string expression

### Requirement: The Python backend declares what it preserves

The Python backend SHALL declare the semantic guarantees it preserves. Because Python reports
integer overflow, reports division by zero, and does not reorder floating-point arithmetic, it
SHALL declare all three.

#### Scenario: Guarantees are declared

- **WHEN** the Python backend is asked what it preserves
- **THEN** it lists overflow reporting, division-by-zero reporting, and floating-point ordering

#### Scenario: The Python frontend and the Python backend are compatible

- **WHEN** compilation is attempted from the Python frontend to the Python backend
- **THEN** negotiation succeeds without withholding any guarantee

### Requirement: Generating Python does not make it callable

The Python backend SHALL translate a unit and SHALL NOT provide a calling convention. A request for
a buildable, callable artifact for a pair that has no host bridge SHALL report that the pair is
unbridged — the same answer any other unbridged pair receives — rather than being special-cased.

#### Scenario: The translation is available without a bridge

- **WHEN** the translated source is requested for the Python backend
- **THEN** it is produced, and no bridge is consulted

#### Scenario: A callable artifact reports the pair as unbridged

- **WHEN** a complete, callable artifact is requested for a pair that has no bridge
- **THEN** the request fails naming both languages, in the same form every unbridged pair reports
