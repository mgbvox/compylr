## 1. Normalized comparison form

- [x] 1.1 Add a comparison-time normalizer over an IR unit in the language-neutral middle end,
      operating on a copy — not registered in the pass pipeline.
- [x] 1.2 Normalize the order of independent local bindings.
- [x] 1.3 Normalize commutative operand order deterministically, only where both operands are free
      of side effects.
- [x] 1.4 Test that normalizing does not change the unit a backend emits from, nor its fingerprint.
- [x] 1.5 Test that two units differing only in a reorderable ordering normalize to the same form,
      and that `f() + g()` and `g() + f()` do not.

## 2. Divergence score

- [x] 2.1 Implement the structural comparison over normalized units, producing a score `D`.
- [x] 2.2 Disregard resolved semantic modes (overflow and division checking, division rounding,
      remainder sign, index origin, text length units), source spans, and documentation.
- [x] 2.3 Report which members and which nodes account for a nonzero score, not only the number.
- [x] 2.4 Test `D == 0` for units differing only in modes, only in spans, and only in docstrings;
      test `D > 0` for a genuine structural difference.
- [x] 2.5 Expose the comparison as a library API, re-exported from the crate root.

## 3. The backend-independence invariant

- [x] 3.1 Add a test that lowers one source file with one frontend for two different backends and
      asserts the units are identical.
- [x] 3.2 Fail naming the differing member and node, so a target leak reads as a located defect
      rather than as a score.

## 4. Recorded cross-language divergence

- [x] 4.1 Add a cross-language tier in `compylr-registry` — the only crate permitted both frontends
      — that pairs accepted corpus **members** by name and measures each pair.
- [x] 4.2 Record the measured table in a generated file beside the test, listing every pair's score
      and, separately, the members only one corpus defines.
- [x] 4.3 Have the test recompute and require an exact match, so a score that rises fails, one that
      falls fails until recorded, and a hand-edited value fails. Regenerate under an env var.
- [x] 4.4 Confirm the check runs in the Makefile, CI, and the pre-commit hooks, so it is not a check
      people discover in a pull request.
- [x] 4.5 Grow the TypeScript corpus toward the Python one so the metric has something to measure,
      naming each member for its Python counterpart, and record what cannot be ported and why.
- [x] 4.6 Confirm the corpus oracles still pass, so the recorded baseline reflects a project that
      still agrees with its source languages.
