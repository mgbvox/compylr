<!-- Every task is a checkbox - the apply phase parses `- [ ] X.Y ` and tracks nothing else.
     The test comes first in each group. Name the check that proves a task done. Where a task
     names a file, link it relatively with a line anchor; this file sits at
     openspec/changes/<change-id>/, so the repository root is ../../../ . -->

## 1. <!-- Task Group Name -->

- [ ] 1.1 <!-- the failing test this group's behavior has to satisfy -->
- [ ] 1.2 <!-- Task description -->

## 2. <!-- Task Group Name -->

- [ ] 2.1 <!-- Task description -->
- [ ] 2.2 <!-- Move the proposal's worked example into frontends/<language>/fixtures/ with its driver -->

## 3. Checks

- [ ] 3.1 <!-- `cargo test --workspace` / `make check` / `make demo` - name what proves this change done -->
