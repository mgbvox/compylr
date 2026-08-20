//! The component model: what a frontend, a backend, and a host bridge are, and how one is found.
//!
//! This crate defines interfaces and knows no implementation of them. The concrete tables live
//! in `compylr-registry`, which may depend on both this crate and every implementation — putting
//! them here would make core depend on the crates that depend on core.

pub mod backend;
pub mod bridge;
pub mod folding;
pub mod frontend;
pub mod negotiation;
pub mod pass;
pub mod verify;

pub use backend::{Backend, BackendError, GeneratedFiles, format_source};
pub use bridge::{BridgeError, BuildKey, HostArtifact, HostBridge};
pub use compylr_ir::Guarantee;
pub use folding::ConstantFolding;
pub use frontend::{Frontend, FrontendError, LoweringError};
pub use negotiation::{TargetOption, UnmetGuarantee, WithheldOption, negotiate};
pub use pass::{DirectedPass, Optimization, Pass, PassConfig, PipelineReport};
pub use verify::{VerificationError, verify};
