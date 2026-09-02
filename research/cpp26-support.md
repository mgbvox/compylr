# C++26 compiler support (primary sources, fetched 2026-09-01)

## GCC — https://gcc.gnu.org/projects/cxx-status.html
- `-std=c++26` accepted from **GCC 14**. GCC calls it "experimental support for the next revision
  of the C++ standard, which is expected to be published in 2026."
- **Contracts** (P2900R14): GCC **16**.
- **Reflection**: GCC 16, requires `-freflection`, incomplete (`apply_result`,
  `is_applicable_type`, `is_nothrow_applicable_type` missing).

## Clang — https://clang.llvm.org/cxx_status.html
- Spells the mode **`-std=c++2c`**, not `-std=c++26`. Overall status: **Partial**.
- **Contracts: No.**
- **Reflection: No** — P2996R13, P3394R4, P3293R3, P3491R3, P3096R12, P3598R0 all unimplemented.
- Implemented C++26 bits are small: pack indexing (19), variadic friends (20), constexpr placement
  new (20), structured binding as condition (21).

## Consequences for `add-cpp-backend` D1
1. The floor is **GCC 14**, not GCC 15 as the design originally said. Corrected.
2. **Clang has neither contracts nor reflection at any version.** A generated feature set reaching
   for C++26's headline additions would not build on Clang at all — which is precisely the failure
   mode D1's "select the standard, confine the feature set" split avoids. The decision is validated
   by data rather than by assertion.
3. Emit `CMAKE_CXX_STANDARD 26` and let CMake choose the per-compiler spelling. Hard-coding
   `-std=c++26` would be wrong for Clang.
4. `cpp26-contracts` as a declared-but-unimplemented target option is well founded: contracts exist
   on exactly one compiler, at its newest major.

## Not established
`std::expected` (C++23) minimum versions. cppreference returns 403 to WebFetch and the libstdc++
status page truncated before its C++23 table. Widely believed GCC 12+ / libc++ 16+ but **not
verified here** — check before relying on it.
