# add-cpp-backend — decision record

Synthesis of the audit (24 findings, 23 confirmed adversarially), the research legs, and the prior
art. Written 2026-09-01, in-session, after three workflow runs stopped on the session limit.

---

## 1. What is actually broken

23 findings confirmed by a second agent applying correctness / intent / materiality lenses. Filed:

| issue | what |
| --- | --- |
| #37 | TypeScript `/` compiles to integer division |
| #38 | `demo-ts-go` never runs compiled Go; benchmark table and IR-coverage report both fabricated |
| #39 | The `(typescript, go)` bridge has never executed — 24% of members exported, wrong ABI, loader unimportable |
| #40 | Five checks that cannot fail; `make check` ≠ CI; stale documented commands |
| #41 | Go backend: five defects against its own spec |
| #42 | The conformance corpus renders without compiling |
| #43 | TypeScript frontend: four more semantic defects |
| #44 | `typescript-api` / `typescript-bindings` specs describe surfaces that do not exist |

**#42 is the root.** Nearly every other finding is something a corpus that compiled its output would
have caught on the first run. Fix ordering should follow from that, not from severity labels.

One finding was **refuted**: `compylr compyle`'s exit-code contract. Correctness held — the code path
is as described — but intent and materiality both refuted it. Worth noting as evidence the lenses do
independent work; without them it would have been a sixth issue.

## 2. What the research settles

### The C-ABI hub would have been catastrophic for Python — settled, with numbers

Basso do Amaral, Ferreira & Goldman (2025), arXiv:2507.00264, measuring array-argument bindings:

| regime | ctypes vs PyO3 |
| --- | --- |
| **M1** — convert on every call | **~145× slower** (1.978e5 ms vs 6.423e3 ms) |
| M2 — marshal once, reuse | ~2.2× slower (1,369 ms vs 634.7 ms) |

**compylr's boundary is M1 by construction.** `CLAUDE.md` records that collections cross by value on
every call; there is no pre-marshalled handle to reuse. So the ctypes-shaped loader the original
C-ABI design proposed would have landed in the regime where ctypes is ~145× off the pace, not the
one where it is 2.2× off. The paper's own words: ctypes is "the most lacking alternative, requiring
manual API redefinitions and expensive type constructions due to `libffi`."

This was the decisive unknown flagged when the hub was first questioned. It is now answered, and it
answers *against* the hub far more strongly than the Node argument did.

Secondary: PyO3 dispatch is ~33.5 ns/call against ~12.6 ns for a hand-written
`_PyCFunctionFastWithKeywords` — a ~20 ns macro tax. Real, and irrelevant next to a 145× marshalling
difference.

### nanobind over pybind11 — settled

2.7–4.4× faster compiles, 3–5× smaller binaries, ~3× faster on simple functions and **~10× when
classes are passed around**; per-instance overhead 56 B → 24 B. Compile time is what a user feels,
since compylr compiles on first call. Class passing is what compylr does. Stable ABI from Python
3.12.

### C++26 — the floor was wrong, the strategy was right

GCC accepts `-std=c++26` from **14**, not 15. Clang spells it `-std=c++2c`, has **no** contracts and
**no** reflection at any version. So confining the emitted feature set to C++23-era constructs is not
caution, it is the only way the output builds on Clang at all. `cpp26-contracts` as a
declared-but-refused option is well founded: contracts exist on exactly one compiler, at its newest
major.

### Node — my stated fact was false; the conclusion survives

`node:ffi` **does** exist, added **v26.1.0**. The design doc claimed it did not. Corrected in D3.
It remains the wrong foundation: experimental, `--experimental-ffi`-gated, self-described unsafe,
no ABI guarantee, and newer than this project's own Node (v24.11.0). Node-API is ABI-stable across
majors by contract.

### A C-ABI hub is a real pattern — and not free

UniFFI, Diplomat, flapigen, SWIG and WIT all do multi-language-from-one-definition, so the shape is
proven. But WIT — the most rigorous of them — states a goal of "zero overhead on synchronous calls"
while crossing component boundaries via a host call costs **~3.5×**. A hub is not free anywhere, and
compylr's per-call, by-value boundary is the worst case for one.

## 3. What the research does NOT settle

| gap | cheapest experiment |
| --- | --- |
| `std::expected` minimum GCC/libstdc++ and Clang/libc++ versions | Compile a three-line file with each; cppreference 403s to WebFetch |
| **nanobind vs PyO3 head-to-head on compylr's own boundary** | The 145× number is ctypes-vs-PyO3, not nanobind-vs-PyO3. Once `demo-python-cpp` exists, its benchmark answers it directly |
| `python-native-compilers`, `multi-target-transpilers`, `semantics-mismatch` | Never run. Lowest value of the ten — none would change a decision already made |
| Whether compylr's behavior axes are novel | Folded into the above; interesting, not load-bearing |

## 4. Prior art worth acting on

- **AssemblyScript** — directly actionable for #43. Introduces explicit `i32`/`i64`/`f64` and makes
  annotation mandatory rather than mapping `number` to a guess. compylr already accepts `int`/`float`
  as named types (`lower.rs:215`); the fix is to finish that pattern, not invent one. See
  `inspiration/assemblyscript.md`.
- **UniFFI** — the reference implementation of one-definition-many-hosts, and of handle-based object
  identity (`Arc::into_raw` to a `u64`, refcount adjusted by explicit FFI calls). Relevant if a hub
  is ever revisited. See `inspiration/uniffi.md`.
- **Diplomat** — same problem, different shape; worth reading beside UniFFI.

## 5. Revisions applied to `add-cpp-backend`

Done, committed, `openspec validate --strict` passing:

- **D3 rewritten** — nanobind + node-addon-api, pairwise. `cpp-abi-bridge` capability deleted.
- **D3 corrected again** — the `node:ffi` claim, with the real reason the conclusion holds.
- **D1 corrected** — GCC 14 floor, Clang's `c++2c` spelling, the contracts/reflection matrix.
- **Context corrected** — the false claim that conformance compiles what it renders.
- **`pipeline-architecture`** — new requirement that corpus output be compiled and run.
- **tasks 4a** — build that tier before the C++ backend leans on it.
- **Demo parity** — the new demos establish the standard rather than joining `demo-ts-go`'s.

## 6. What belongs in a separate change

Everything in §1 except #42's mechanism. Specifically:

- **`fix-typescript-go-pair`** — #37, #39, #41, #43, #44. This is not a small change: the pair is
  non-functional end to end, and #44 raises whether its specs should be narrowed to reality or the
  original change reopened. That question should be settled before code is written.
- **`harden-the-checks`** — #40 plus #42's Go fallout. The unifying rule: a check is introduced with
  a deliberate failure proving it fails. `demo-python-rust`'s `TestTheMeasurementItself` already does
  this; the discipline never crossed to the TypeScript/Go side.

Neither should block `add-cpp-backend`. But #42's tier will surface #41's failures the moment it
runs, so tasks 4a.5 says to record and file, not fix.
