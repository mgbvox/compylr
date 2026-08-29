## Context

<!-- Current state and constraints that shape the approach. See proposal.md for motivation -
     don't restate it. Link what you name: this file sits at openspec/changes/<change-id>/,
     so the repository root is ../../../ , e.g.
     [`Unit::fingerprint`](../../../crates/compylr-ir/src/ir.rs#L1299). -->

## Goals / Non-Goals

**Goals:**
<!-- What this design aims to achieve -->

**Non-Goals:**
<!-- What is explicitly out of scope -->

## Decisions

<!-- One numbered block per decision. Every decision shows its face in code: the snippet goes
     right after **Decision** and is the smallest form that makes the choice concrete - the
     type or signature, the arm that dispatches on it, the call site as it would then read, or
     the target source that comes out. Under ~15 lines, elide with `// ...`, tag every fence.
     A decision with no code face says so in one line instead of growing a snippet. -->

### 1. <!-- the decision, as a sentence -->

**Decision:** <!-- what is being chosen -->

```rust
// before - <what it is today>
// after  - <what it becomes>
```

**Why:** <!-- the reasoning. What breaks if this goes the other way? -->

**Alternatives considered:** <!-- what was rejected, and on what grounds -->

### 2. <!-- the decision, as a sentence -->

**Decision:**

```rust
```

**Why:**

**Alternatives considered:**

<!-- A decision that touches the IR owes BOTH forms:

     1. The definition delta, as Rust, against compylr-ir - the before/after snippet above.
     2. The value, for the worked example's program, as the JSON `--emit ir` writes. Get the
        real thing rather than writing it from memory:

          cargo run -p compylr-cli -- --emit ir   path/to/example.py
          cargo run -p compylr-cli -- --emit rust path/to/example.py

     Trim it to the nodes that move and mark what you trimmed:

     ```json
     {
       "version": 4,
       "functions": [
         { "name": "add",
           "params": [{ "name": "a", "ty": "Int" }, { "name": "b", "ty": "Int" }],
           "ret": "Int",
           "body": [
             { "Return": { "Binary": { "op": { "Add": { "checked": "Reported" } },
                                       "left": { "Name": "a" },
                                       "right": { "Name": "b" } } } }
           ] }
       ],
       "origin": { "frontend": "python", "requires": ["IntegerOverflowReported"] }
     }
     ```

     Then answer, here or under Risks: is it language-neutral; is it a mode on an existing
     form or a distinct form, and why; does the artifact `version` move; does
     `Unit::fingerprint()` cover it; and does the demo coverage claim need an algorithm or a
     narrower README. -->

## Risks / Trade-offs

<!-- Known risks and trade-offs. Format: [Risk] → Mitigation -->
