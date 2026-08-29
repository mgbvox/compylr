## Why

<!-- Explain the motivation for this change. What problem does this solve? Why now?
     Cite, don't restate: the accepted subset and its reasoning live in
     ../../../CLAUDE.md, the requirements in ../../../openspec/specs/. -->

## What Changes

<!-- Describe what will change. Be specific about new capabilities, modifications, or removals.
     Mark breaking changes with **BREAKING**. -->

## Worked Example

<!-- ONE example, as code, that exercises everything this change adds or alters. A reader who
     reads only this section should be able to say what the change does.

     Keep the stages this change actually touches, delete the rest, and keep them in pipeline
     order. Real code with concrete values - not `def f(x: T) -> U`. Never invent output: if
     you have not run it, mark it `expected:` and say so. Write the input so it can be lifted
     into frontends/<language>/fixtures/accepted/ unchanged when the tasks phase gets there.

     One formatting trap: `openspec show` reads the first line beginning with `# ` at column
     zero as the change's title, and it does not skip code fences. Keep comments in the
     example indented inside a body, or leave them out. -->

### Input

<!-- The source program, in the frontend language. Under ~20 lines, accepted by the subset. -->

```python
def add(a: int, b: int) -> int:
    return a + b
```

### Today

<!-- What that program does now: the answer, the diagnostic (quoted exactly, span included),
     the generated code, or the command output. Delete this block only if the form has no
     behavior today at all - and say so. -->

```text
```

### After

<!-- What it does once this change lands. -->

```text
```

### At the boundary

<!-- The call and its answer, when the change is observable from the host language. -->

```pycon
>>> add(2, 3)
5
```

<!-- Include the IR or the generated target source here only where this change moves them,
     and only the lines that move. design.md is where the IR gets dissected. -->

## Capabilities

### New Capabilities
<!-- Capabilities being introduced. Use kebab-case for path segments you introduce
     (e.g., user-auth or identity/user-auth) that follow the project's existing
     spec organization. Each creates specs/<capability-path>/spec.md. -->
- `<capability-path>`: <brief description of what this capability covers>

### Modified Capabilities
<!-- Existing capabilities whose REQUIREMENTS are changing (not just implementation).
     Only list here if spec-level behavior changes. Each needs a delta spec file.
     Use the exact existing path under openspec/specs/. Leave empty if no requirement
     changes. A change with no capabilities at all (pure refactor, tooling, docs)
     must set `skip_specs: true` in its .openspec.yaml - openspec validate rejects
     a zero-delta change without that marker. Do not invent a requirement just to
     satisfy validation. -->
- `<existing-capability-path>`: <what requirement is changing>

## Impact

<!-- Affected code, APIs, dependencies, systems. Link every one of them relatively, with a
     line anchor, so the link resolves in an editor and on GitHub both. This file sits at
     openspec/changes/<change-id>/, so the repository root is ../../../ :

       - [`Unit::fingerprint`](../../../crates/compylr-ir/src/ir.rs#L1299)
       - [`crate_boundaries.rs`](../../../crates/compylr-host-python/tests/crate_boundaries.rs)

     Link text is the identifier, not the path. No absolute paths, no file:// URLs, no
     https://github.com/... permalinks - a permalink pins a commit and is dead locally. -->
