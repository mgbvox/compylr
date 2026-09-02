# UniFFI (Mozilla)

Generates foreign-language bindings for a Rust library from one definition.
https://github.com/mozilla/uniffi-rs — MPL-2.0

## Why it matters to compylr

It is the working reference for the thing `crates/compylr-core/src/bridge.rs` defers: turning an
N × M binding problem into N + M. Worth reading before anyone revisits a canonical-ABI hub.

## Architecture

Interface described either by **proc-macros** on the Rust side or a **UDL file** (an IDL modelled on
WebIDL). From that, UniFFI emits Rust scaffolding *and* per-language bindings. Targets Kotlin,
Swift, and Python fully; Ruby mostly; more via third-party backends.

Its internals name the two directions explicitly — **"lifting, lowering and serialization"** — which
is the same split compylr's bridges perform ad hoc per pair.

## Object identity across the boundary

The interesting part, and the one compylr's instance handling has to solve too: the "arc-to-pointer
dance". `Arc::into_raw` turns an `Arc` into a `u64` handle. Foreign code clones a handle by calling
back into an FFI function running `Arc::increment_strong_count`, and frees by calling one running
`Arc::decrement_strong_count`. Lifetime stays owned by Rust; the host holds an opaque token.

That is precisely the shape a C-ABI hub for compylr would have needed, and precisely what
`Napi::ObjectWrap` and nanobind's `nb::class_` provide for free in the pairwise design.

## What compylr should take

- The **IDL-or-macros** duality is a good model if a hub is ever built: compylr already has the IR,
  which is a better starting point than a hand-written UDL.
- The **handle discipline** for object identity — refcount adjusted only through explicit FFI calls,
  never inferred — is the correct answer regardless of which binding technology is used.

## What it does not solve

Its own docs are direct: UniFFI *"will not help you ship a Rust library to these platforms, but it
will help you avoid writing bindings code by hand."* Distribution, build orchestration, and toolchain
management stay the caller's problem — which is most of what compylr's build pipeline does.

Note also what it does **not** target: JavaScript/TypeScript is not in its supported set.
