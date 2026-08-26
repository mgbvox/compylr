//! Shared scaffolding for the tests that read the fixture corpus.
//!
//! Lives in a subdirectory so cargo does not build it as a test binary of its own: only files
//! directly under `tests/` become one.
//!
//! Each test binary that includes this uses a different part of it, so what one does not reach is
//! not dead code -- it is another binary's.
#![allow(dead_code)]

pub mod drivers;
