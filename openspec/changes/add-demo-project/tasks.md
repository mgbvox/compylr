## 1. Project skeleton

- [x] 1.1 Confirm `add-control-flow`, `add-collection-mutation`, `add-classes`, and `add-cli-precompile` are all archived — the demo is written against what exists, not what is planned
- [x] 1.2 Create `./demo/` with its own `pyproject.toml` declaring compylr as a dependency, per design.md D3
- [x] 1.3 Create the demo package with a module for each variant and a shared entry point
- [x] 1.4 Confirm the demo gets its own `.compylr/` by project-root discovery, with no configuration

## 2. The interpreted reference

- [x] 2.1 Write a plain, uncompiled nth-prime as the reference every variant is checked against
- [x] 2.2 Write a test asserting the reference returns 2, 3, 5, 7, 11 for the first five
- [x] 2.3 Keep it deliberately simple and obviously correct; it is the oracle, so it must be readable rather than clever

## 3. Recursive variant

- [x] 3.1 Write the recursive variant, recursing over **primes found** rather than candidates tested, per design.md D2
- [x] 3.2 Write a test asserting it agrees with the reference over the documented range
- [x] 3.3 Determine the depth at which it aborts, and document the supported range from that measurement rather than guessing
- [x] 3.4 Confirm it compiles rather than falling back

## 4. Iterative variant

- [x] 4.1 Write the iterative variant using a loop, a reassigned counter, and a locally built collection of primes
- [x] 4.2 Write a test asserting it agrees with the reference over the documented range
- [x] 4.3 Confirm it computes independently rather than delegating to another variant, per design.md's fourth risk
- [x] 4.4 Confirm it compiles rather than falling back

## 5. Memoized variant

- [x] 5.1 Write the memoized variant as a class holding a mutable cache and a hit counter
- [x] 5.2 Write a test asserting it agrees with the reference over the documented range
- [x] 5.3 Write a test asserting the second request for the same n increments the hit counter — otherwise "memoized" is a claim about shape, not behaviour, per design.md D5
- [x] 5.4 Write a test asserting two instances hold independent caches
- [x] 5.5 Confirm it compiles rather than falling back

## 6. Agreement

- [x] 6.1 Write a test asserting all three agree with each other for every n over the documented range
- [x] 6.2 Write a test asserting all three agree with the interpreted reference
- [x] 6.3 Write a test asserting each variant's documented behaviour for an n below one, so the edge is defined rather than discovered
- [x] 6.4 Write a test asserting one build covers all three

## 7. Precompiling

- [x] 7.1 Verify `compylr compyle ./demo` compiles the whole project
- [x] 7.2 Write a test asserting a run after precompiling performs no build
- [x] 7.3 Measure cold-precompile and warm-run timings on a stated machine
- [x] 7.4 Write the demo README: what it shows, the toolchain requirement, the precompile flow with those measurements, and the recursion-depth bound with its reason

## 8. The repository verifies it

- [x] 8.1 Write a test in the repository's suite that builds the demo and exercises all three variants
- [x] 8.2 Assert each variant is genuinely compiled, not interpreted — a silently interpreted demo demonstrates nothing
- [x] 8.3 Group the check behind the slow marker, sharing one build across its assertions, per design.md D4
- [x] 8.4 Confirm the fast suite excludes it and the slow suite includes it
- [x] 8.5 Write a test asserting every feature the demo's README claims is exercised, so the claim cannot drift from the code

## 9. Findings

- [x] 9.1 Record every gap the demo exposes as it is written — this is the first program written to be useful rather than to exercise a rule, so its findings are the ones a user meets first
- [x] 9.2 Do **not** fix them here. Report them at the end so each can become its own change

## 10. Verification

- [x] 10.1 Run the repository's full suite, Rust and Python, including slow tests
- [x] 10.2 Run `ruff check` and `mypy` over the demo as well as the package
- [x] 10.3 Verify the demo end to end by hand: fresh environment, install, precompile, run, and read its output
- [x] 10.4 Update the repository README to point at the demo as the worked example
- [x] 10.5 Update `CLAUDE.md` with the demo's location and how to run its checks
- [x] 10.6 Run `openspec validate add-demo-project --strict` and confirm every scenario has a passing test
