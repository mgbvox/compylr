## MODIFIED Requirements

### Requirement: Emission is deterministic

The backend SHALL produce byte-identical output for the same unit across runs and across
addition orders, so that a rebuild decision made on the fingerprint is never contradicted by
the generated source. This SHALL hold for **every** file emitted, and the set of file names
itself SHALL be determined by the unit alone.

#### Scenario: Same unit, repeated emission

- **WHEN** the same unit is emitted twice in one process
- **THEN** the two outputs are byte-identical

#### Scenario: Addition order does not change output

- **WHEN** the same functions are added to two units in different orders and both are emitted
- **THEN** the two outputs are byte-identical

#### Scenario: The file set is stable

- **WHEN** two different units are emitted
- **THEN** both produce the same file names, differing only in contents

### Requirement: Emitted source is valid Rust

Output SHALL compile without errors or warnings under the same lint settings the project
applies to its own code, so that a malformed emission is caught at build time rather than
surfacing as an unexplained failure to the user. The files SHALL compile **together**, as the
crate they describe, rather than each being separately valid.

#### Scenario: Every accepted fixture compiles

- **WHEN** each accepted Python fixture is lowered and emitted
- **THEN** the resulting Rust compiles cleanly

#### Scenario: The crate root reaches every other file

- **WHEN** an emitted crate is compiled from its root file
- **THEN** every other emitted file is reached through a module declaration, so none is dead
  weight on disk

## ADDED Requirements

### Requirement: Emission produces a named set of files

The backend SHALL emit a crate as a mapping from relative path to contents, rather than as one
source string. Each file SHALL hold one concern:

| File | Holds |
| --- | --- |
| `src/lib.rs` | module declarations and the module registration, and nothing that grows with the program |
| `src/generated.rs` | the translated functions, and nothing else |
| `src/bindings.rs` | the Python-boundary wrappers and the mapping from runtime failures to exceptions |
| `src/compat.rs` | the helpers reproducing Python's semantics |

The division exists to be **read**. Generated source is written to disk so a user can check what
their Python became; a single file that opens with two hundred identical lines in every project
buries the twelve lines they came for.

#### Scenario: The crate is emitted as separate files

- **WHEN** a unit is emitted
- **THEN** the result names each file separately rather than concatenating them

#### Scenario: Translated code stands alone

- **WHEN** the file holding translated functions is read
- **THEN** it contains the functions and nothing else — no helpers, no boundary code, no
  lint allowances

#### Scenario: The crate root does not grow with the program

- **WHEN** units of one function and of fifty functions are emitted
- **THEN** their crate roots are the same size

#### Scenario: Boundary code is separate from translated code

- **WHEN** a unit is emitted
- **THEN** the Python-boundary wrappers are in a different file from the translated functions

#### Scenario: The helpers are identical across projects

- **WHEN** two unrelated units are emitted
- **THEN** the file holding the Python-semantics helpers is byte-identical in both, since it
  depends on nothing about the program

#### Scenario: Emitting the same unit yields the same file set

- **WHEN** a unit is emitted twice
- **THEN** both results name exactly the same files

### Requirement: What is generated does not change

Rearranging output into files SHALL NOT change the code that is generated. The same functions,
helpers, and wrappers SHALL be produced, so a compiled artifact behaves exactly as before and no
fingerprint moves.

This is a readability change. Anything that alters behavior belongs in a change that says so.

#### Scenario: Fingerprints are unaffected

- **WHEN** a unit is fingerprinted before and after this change
- **THEN** the fingerprint is the same, because it is computed over the IR and not the output

#### Scenario: The compiled result is unchanged

- **WHEN** a unit is compiled and called before and after this change
- **THEN** every function returns the same values, including on the operands where Python and
  Rust semantics diverge

#### Scenario: The same helpers are present

- **WHEN** the emitted files are taken together
- **THEN** they contain the same helper definitions the single file previously did
