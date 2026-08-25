//! Shared scaffolding for the tests that read the fixture corpus.
//!
//! Lives in a subdirectory so cargo does not build it as a test binary of its own: only files
//! directly under `tests/` become one.

pub mod drivers;
