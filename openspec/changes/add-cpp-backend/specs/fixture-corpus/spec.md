## MODIFIED Requirements

### Requirement: Agreement is checked at both the translation and the call boundary

Agreement SHALL be established at two tiers, because a translated program can be right and the way
its values cross into the host language can be wrong, and neither tier observes the other's failure.

The **translation tier** SHALL exercise the generated target source directly, without the host
language's calling convention. It SHALL cover the whole accepted corpus and SHALL run as part of the
ordinary test suite.

The **boundary tier** SHALL exercise the same corpus through the host bridge, as a user reaches it.
It SHALL cover the whole accepted corpus and MAY be marked as slow, because it builds an extension.

Neither tier SHALL be treated as standing in for the other.

Both tiers SHALL run over **every** `(source, target)` pair the bridge registry reports, enumerated
from that registry rather than from a list a test maintains. A pair checked at neither tier is a
pair whose answers nobody has compared against the source language's, which is what the corpus
exists to prevent — and a hand-maintained list is how the fixture lists in this repository once
drifted and hid a real defect.

Where a pair's target toolchain is unavailable on the machine, that pair's run SHALL report itself
skipped and name the missing toolchain, rather than reporting success.

A requirement of this kind SHALL take effect for a `(source, target)` pair once that pair's
confirmed defects are closed. `(typescript, go)` SHALL be enumerated as a **known-failing pair**,
each entry naming its filed issue (#38, #39, #41), until those close. The list of known-failing
pairs SHALL only shrink: adding to it SHALL require a filed issue, and a pair SHALL NOT be added to
silence a defect introduced after this change.

Without that scoping the requirement would fail on the day it lands — not because it is wrong, but
because it correctly describes a pair that is already broken, and this change is not where that pair
gets fixed.


#### Scenario: The translation tier covers the corpus

- **WHEN** the ordinary test suite runs
- **THEN** every accepted fixture has been checked for agreement through generated target source

#### Scenario: The boundary tier covers the corpus

- **WHEN** the full check runs
- **THEN** every accepted fixture has been checked for agreement through the host bridge

#### Scenario: A conversion defect is caught where a translation defect is not

- **WHEN** a value is translated correctly but converted incorrectly at the host boundary
- **THEN** the boundary tier fails and the translation tier passes

#### Scenario: Every bridged pair is covered

- **GIVEN** the bridge registry
- **WHEN** the full check runs
- **THEN** every accepted fixture has been checked for agreement over every registered pair

#### Scenario: A newly registered pair is covered without a list being edited

- **GIVEN** a bridge newly registered for a pair
- **WHEN** the full check runs
- **THEN** the accepted corpus is checked over that pair, without a test having been edited to name
  it

#### Scenario: A missing target toolchain is a skip, not a pass

- **GIVEN** a machine without the toolchain a registered pair's target requires
- **WHEN** the full check runs
- **THEN** that pair's run reports itself skipped and names the missing toolchain
- **BUT** it does not report success

#### Scenario: One pair disagreeing fails the check

- **GIVEN** two registered pairs and one accepted fixture
- **WHEN** the fixture's driver produces the source language's answer through one pair and a
  different answer through the other
- **THEN** the check fails naming the fixture, the call, and the pair that disagreed
