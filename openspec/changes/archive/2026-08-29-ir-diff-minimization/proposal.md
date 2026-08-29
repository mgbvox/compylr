## Why

The IR is meant to be universal: one middle end that several source languages lower into and several
targets emit from. Nothing currently measures whether that is true. Python and TypeScript already
lower into the same IR, and whether they produce the *same shape* for the same program is at present
a matter of opinion — which means the shared middle end can drift apart one frontend change at a
time, and nobody would notice until a pass written against one frontend's habits turned out to be
wrong for the other's. This change makes the divergence a number the repository records and is not
allowed to raise.

## What Changes

- Introduce a **normalized form** of an IR unit, used only for comparison, that standardizes
  orderings carrying no meaning — independent local bindings, and commutative operands where both
  are side-effect-free. It never reaches a backend and never changes a fingerprint.
- Introduce a **semantic divergence score** `D` over normalized units that disregards what the IR
  carries deliberately: resolved semantic modes, source spans, and documentation. What is left is
  structural disagreement between frontends.
- **Record the divergence of every cross-language fixture pair in the repository**, generated from a
  real run, and fail the build when any pair's score rises. There is no hand-chosen threshold: the
  baseline is what the project currently achieves, and the only permitted direction is down.
- Assert the neighbouring **invariant** that one frontend's IR is identical regardless of the
  backend it was directed at — a difference there is a target leak in the frontend, not a score.

## Capabilities

### New Capabilities
- `ir-diff-checker`: the normalized comparison form, the divergence score `D`, and the rule that a
  recorded score may not increase.

### Modified Capabilities

None. Normalization is deliberately *not* an optimization pass — it is a comparison-time view, so
`ir-optimization` is untouched and the compiled program is unaffected. `fixture-corpus` is likewise
untouched: it is the capability of the Python corpus and its CPython oracle, and cross-language
divergence is a property of the diff checker rather than of that corpus.

## Impact

- **Measurement:** a recorded, generated divergence table checked in CI, alongside the existing
  generated benchmark and subset tables.
- **Testing:** a new cross-language tier, which needs both frontends at once and therefore belongs
  in the one crate permitted to know them both.
- **Forcing function:** once divergence is visible per pair, the levers for reducing it — tightening
  a supported subset, or defaulting the resolved behavior to the intersection of the languages —
  become measurable proposals rather than arguments. Pulling those levers is out of scope here.
