## MODIFIED Requirements

### Requirement: A missing toolchain is diagnosed clearly

Compiling requires build tools that are not guaranteed to be present. When a required tool is
missing, the pipeline SHALL say which one and how to install it, rather than surfacing a
file-not-found error.

Which tools are required SHALL depend on the **selected target**, and the diagnostic SHALL name that
target's tools rather than any one target's. A tool that a different target would have needed SHALL
NOT be reported as missing.

Where a target requires a minimum version of a tool — a compiler providing a language standard, for
instance — the check SHALL cover the version and not only the tool's presence, and the diagnostic
SHALL name the version required. A compiler that is present but too old is the failure this rule
exists to catch, because its own error names a missing feature rather than a missing toolchain.

#### Scenario: Rust toolchain absent

- **WHEN** a build is attempted with no Rust compiler available
- **THEN** the error names the missing toolchain and states how to install it

#### Scenario: Build tool absent

- **WHEN** a build is attempted with the extension-module build tool unavailable
- **THEN** the error names it and states how to install it

#### Scenario: The check happens before work is wasted

- **WHEN** required tools are missing
- **THEN** the failure is reported before a build is attempted

#### Scenario: The tools checked are the selected target's

- **GIVEN** a project whose selected target is not Rust
- **WHEN** a build is attempted on a machine with no Rust compiler
- **THEN** the absence of the Rust compiler is not reported

#### Scenario: A compiler too old for the target's standard is diagnosed

- **GIVEN** a project whose selected target requires a compiler providing a given language standard
- **WHEN** a build is attempted with a compiler present but older than that
- **THEN** the error names the standard required and the compiler versions that provide it
- **AND** the build is not attempted
