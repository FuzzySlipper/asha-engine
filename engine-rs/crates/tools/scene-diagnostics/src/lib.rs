//! Scene / asset / world-bundle / render diagnostic emitters (scene-capability-06,
//! epic #2313, subtask #2331).
//!
//! # Lane
//!
//! `rust-tools` — an **observational** read layer that turns the existing
//! classified validators (`core-scene`, `core-catalog`, `svc-serialization`,
//! `rule-world-bundle`) and projection/resource state into stable, generated
//! [`protocol_diagnostics`] reports. Like `voxel-diagnostics` it is more
//! omniscient than runtime crates but **never mutates authority** — every entry
//! point takes `&` references and returns reports.
//!
//! # What each module emits
//!
//! * [`scene`] — scene document validation: duplicate id, invalid parent, parent
//!   cycle, invalid transform, wrong-kind asset, and (when a catalog is supplied)
//!   missing-asset cross-checks.
//! * [`catalog`] — asset catalog validation and asset-lock drift.
//! * [`bundle`] — world-bundle manifest validation, durable-artifact integrity,
//!   missing optional cache, and terrain generator mismatch.
//! * [`trace`] — render handle → scene node → entity → asset source traces and the
//!   broken-trace diagnostics they warrant.
//! * [`resources`] — observational renderer resource reports and leak hints.
//! * [`text`] — deterministic, greppable text rendering for goldens / devtools
//!   readback.
//!
//! # Boundaries
//!
//! Diagnostics are **observational only** and severity ties to recovery policy
//! ([`protocol_diagnostics::DiagnosticSeverity`]). The emitters produce generic
//! ASHA artifacts — there are no Den-specific fields, ids, or imports anywhere in
//! this crate, and there must never be.

#![forbid(unsafe_code)]

pub mod bundle;
pub mod catalog;
pub mod composition;
pub mod equivalence;
pub mod resources;
pub mod roundtrip;
pub mod scene;
pub mod text;
pub mod trace;

pub use bundle::{
    artifact_integrity_diagnostics, generator_mismatch_diagnostic, manifest_diagnostics,
    missing_cache_diagnostics, regen_conflict_diagnostics,
};
pub use catalog::{catalog_diagnostics, lock_diagnostics};
pub use composition::{composition_failure_diagnostic, composition_failure_set};
pub use equivalence::{world_bundle_round_trip, BundleEquivalenceReport};
pub use resources::resource_diagnostics;
pub use roundtrip::{
    check_saved_bundle, scene_round_trip, voxel_round_trip, world_fingerprint, RoundTripReport,
};
pub use scene::scene_diagnostics;
pub use trace::{build_source_traces, source_trace_diagnostics, ProjectionRecord};

// Re-export the protocol surface so a single `scene_diagnostics::` import gives a
// consumer the report types alongside the emitters.
pub use protocol_diagnostics::{
    DiagnosticCode, DiagnosticReport, DiagnosticReportSet, DiagnosticScope, DiagnosticSeverity,
    DiagnosticSourceRef, RemedyAction, RendererResourceReport, SourceTrace, SuggestedRemedy,
};
