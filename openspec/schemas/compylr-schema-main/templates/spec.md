## Purpose
<!-- New capabilities only: one or two sentences (50+ characters) on what this capability is for.
     Delete this section for an existing capability. -->

<!-- SCENARIOS ARE GHERKIN. This file is a Gherkin feature wearing markdown: the capability is
     the Feature, each `### Requirement:` is a Rule, and each `#### Scenario:` is a Scenario
     whose bullets are steps. Strip the markdown decoration and what is left must be a valid
     .feature body.

     Steps, in order, one keyword per bullet:
       - GIVEN  the state of the world beforehand. More continue with AND. Omit only when
                there genuinely is no precondition.
       - WHEN   the one thing that happens. EXACTLY ONE per scenario - a second WHEN is a
                second scenario, and two actions joined by "and" is a step to split.
       - THEN   what is then observably true. More continue with AND, a contrast with BUT.
       - AND / BUT inherit the type of the step above, so a scenario never opens with one.

     Describe behavior, not implementation - what compylr accepts, rejects, emits, or answers,
     never which function does it. One behavior per scenario, named for that behavior. Assert
     only what the language promises: mapping and set iteration order is not promised.

     A step may carry a code block - Gherkin's doc string - fenced, tagged, and indented under
     the step. Sketch:

       - **GIVEN** a module whose only function is
     <fenced python block, indented two spaces, holding the program>
       - **WHEN** the module is lowered by the `python` frontend
       - **THEN** lowering fails
       - **AND** the diagnostic names the parameter `a` and points at line 1

     One behavior over a set of inputs is a Scenario Outline: write it once with
     <placeholders> and follow it, inside the same scenario block, with an **Examples:**
     markdown table whose columns are those placeholders.

     Cite code in requirement prose, never inside a step. A delta spec sits at
     openspec/changes/<change-id>/specs/<capability-path>/spec.md, so the repository root is
     five levels up: [`Expr`](../../../../../crates/compylr-ir/src/ir.rs#L441). No absolute
     paths, no file:// URLs, no https://github.com/... permalinks. -->

## ADDED Requirements

### Requirement: <!-- requirement name -->
<!-- What the system SHALL do. SHALL/MUST, never should/may. -->

#### Scenario: <!-- the behavior this scenario pins, as a sentence -->
- **GIVEN** <!-- the precondition -->
- **WHEN** <!-- the single action -->
- **THEN** <!-- the observable outcome -->
- **AND** <!-- a further outcome, if any -->
