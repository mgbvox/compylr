//! `@compylr/core`: the compiler, exposed to Node.
//!
//! One of two host bindings; `compylr-host-python` is the other, and it links PyO3 where this
//! links napi-rs. `crate_boundaries.rs` states the "only a host may link a host runtime" rule
//! over the `compylr-host-*` prefix rather than over a crate name, which is why this crate
//! satisfies it without being special-cased.
//!
//! This is the seam between the two languages. Above it, TypeScript decides *what* to compile
//! and *when*; below it, everything is the language-neutral pipeline. Compiling in-process
//! rather than shelling out keeps diagnostics structured: a subprocess would have to format an
//! error into text and have the host parse it back, and the location would be the first thing
//! lost.
//!
//! Compiler errors cross as structured fields rather than as a message, so a caller can read the
//! line and column off a lowering failure instead of scraping them out of a string.

use napi_derive::napi;

#[napi]
pub fn version() -> String {
    "0.1.0".to_string()
}
