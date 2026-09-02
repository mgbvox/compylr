# Node-API and node:ffi (primary sources, fetched 2026-09-01)

## Node-API — https://nodejs.org/api/n-api.html
> "This API will be Application Binary Interface (ABI) stable across versions of Node.js... allow
> modules compiled for one major version to run on later major versions of Node.js without
> recompilation."

- `NAPI_VERSION` pins the surface: `#define NAPI_VERSION 3` before `#include <node_api.h>`.
  Defaults to **8**; current is **10** (Node v22.14.0+, v23.6.0+).
- The guarantee covers Node-API **only**. An addon must avoid `node.h`, `node_buffer.h`, `uv.h`,
  and `v8.h`, and any external library it links carries no guarantee.

## node:ffi — https://nodejs.org/api/ffi.html — **THE CORRECTION**
Node **does** ship an FFI, and the design doc previously claimed it does not.

- Module `node:ffi`, **added v26.1.0**, **Stability 1 (Experimental)**.
- Requires `--experimental-ffi` *and* FFI support compiled into the build.
- `dlopen(path, definitions)` resolves symbols from a plain C library with no addon:
  ```js
  const { lib, functions } = dlopen(`./mylib.${suffix}`, {
    add_i32: { arguments: ['int32','int32'], return: 'int32' },
  });
  ```
- Its own docs: "unsafe, experimental API. Incorrect pointer usage, wrong signatures, or accessing
  freed memory can crash the process or corrupt memory."

## Consequences
The C-ABI hub was rejected on the claim that Node *cannot* consume a C ABI. That is false as of
v26.1.0. **The conclusion survives, on different grounds** — experimental, flag-gated, self-described
unsafe, no ABI guarantee, and newer than this project's own Node (v24.11.0). And once Python goes
through nanobind, a hub yields two mechanisms anyway, one of them experimental.

Recorded because a decision resting on a fact that has since changed is worth knowing about before
someone re-derives it from the stale premise.
