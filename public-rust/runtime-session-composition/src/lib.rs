//! Public Rust facade for distinct statically composed RuntimeSession and
//! pre-runtime project-authoring cells.
//!
//! Downstream addons register concrete gameplay modules through the gameplay
//! SDK, then choose `DeferredRuntimeSessionBuilder` for manifest-driven atomic
//! runtime activation, or `StaticProjectAuthoringBuilder` for immutable
//! provider schema/codec authority without ProjectBundle activation. Runtime
//! topology is compiled from admitted project content rather than assembled by
//! downstream boot code.

#![forbid(unsafe_code)]

pub use runtime_bridge_api::*;
