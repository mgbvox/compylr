## Purpose

Tooling to perform semantic differencing between two normalized IR units to measure cross-frontend structural divergence.

## ADDED Requirements

### Requirement: Semantic IR Diffing
The tool SHALL compare two IR units and emit a quantitative divergence score `D`, ignoring canonical variations but reporting on un-normalized structural divergence.

#### Scenario: Differencing identically shaped IRs
- **WHEN** the tool is given two structurally identical IRs that differ only in expected semantic mode parameters
- **THEN** it reports a divergence score `D` of 0.

#### Scenario: Differencing divergent IRs
- **WHEN** the tool is given two IRs with distinct structural shapes (e.g. `while` vs `for` looping)
- **THEN** it reports a divergence score `D > 0`.
