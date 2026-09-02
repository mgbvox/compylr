# research/

Durable output of the `add-cpp-backend` review: what was audited, what survived adversarial
refutation, what the research established, and the decisions that followed.

Tracked deliberately. `context/` is gitignored and is for scratch — compiler probes, throwaway
build directories, transcripts. Anything here is meant to outlive the session that produced it,
because several of these files are the *only* record of work whose agents died before returning.

## Start here

- **[DECISION.md](DECISION.md)** — the synthesis. What is broken and in what order, what the
  research settles, what it does not and the cheapest experiment for each, and what belongs in a
  separate change.

## Audit

An eight-dimension sweep for one defect class: *a claim the repository makes that is not true*.
24 findings, then a per-dimension adversarial pass over three lenses — correctness, intent,
materiality. **23 confirmed, 1 refuted.** Filed as issues #37–#44.

| file | |
| --- | --- |
| [`audit-findings.json`](audit-findings.json) | the 24 findings as structured data |
| [`refutation-verdicts.json`](refutation-verdicts.json) | per-finding verdicts, all three lenses |
| `audit-ts-frontend.md` | TypeScript frontend semantics → #43 |
| `audit-ts-go-bridge.md` | the `(typescript, go)` bridge → #39 |
| `audit-go-backend.md` | Go backend vs its own spec → #41 |
| `audit-spec-vs-reality.md` | specs describing what does not exist → #41, #44 |
| `audit-enforcement-tests.md` | tests that cannot fail → #42 |
| `audit-generated-docs.md` | generated-doc and CI integrity → #38, #40 |
| `audit-demo-integrity.md` | both demos → #38 |
| `audit-python-rust-path.md` | the mature path — the one refuted finding |

## Research

| file | settles |
| --- | --- |
| [`python-call-overhead.md`](python-call-overhead.md) | **The decisive one.** ctypes is ~145× slower than PyO3 when arguments convert on every call (~2.2× when marshalled once). compylr's boundary is the former by construction. |
| [`nanobind.md`](nanobind.md) | nanobind over pybind11: 2.7–4.4× compile, 3–5× binary, ~10× on class passing |
| [`cpp26-support.md`](cpp26-support.md) | GCC 14 accepts the mode; Clang has no contracts or reflection at any version |
| [`node-api.md`](node-api.md) | Node-API's ABI guarantee — and that `node:ffi` **does** exist as of v26.1.0, refuting an earlier claim |
| [`universal-bindings.md`](universal-bindings.md) | UniFFI, Diplomat, cbindgen, flapigen, SWIG, WIT, GraalVM |
| [`python-cpp-alternatives.md`](python-cpp-alternatives.md) | pybind11 / nanobind / cppyy / SWIG / Cython |
| [`ts-native.md`](ts-native.md) | AssemblyScript's answer to `number` — actionable for #43 |

Three topics remain unrun: `python-native-compilers`, `multi-target-transpilers`,
`semantics-mismatch`. They are scheduled ahead of the adversarial review of `DECISION.md`.

## Provenance, and a caveat

Findings were produced by subagents and **adversarially verified** by independent agents that
re-read every citation and re-ran the commands. Where a claim could not be verified it is marked
so. Several corrections in these files are to earlier claims of my own — they are recorded rather
than quietly overwritten, so nobody re-derives a decision from a premise that has since changed.

`DECISION.md` itself has **not** been adversarially reviewed. It was written by the same agent that
made the decisions it defends. That review is pending.
