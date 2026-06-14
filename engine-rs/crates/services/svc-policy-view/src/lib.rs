//! Deterministic policy world-layer service (#2391, #2392, #2394).
//!
//! # Lane
//!
//! `rust-service` — the authority-side projector and validator for the constrained
//! policy world layer. It reads authority state (`core-entity`) and produces the
//! read-only [`protocol_policy_view::PolicyWorldView`], validates proposed
//! [`protocol_policy_view::PolicyWorldCommand`]s into events or classified
//! rejections, and records the proposed/accepted/rejected replay path. It never
//! renders and never lets a policy mutate authority directly.
//!
//! Determinism is the contract: every function here is a pure transform of its
//! inputs (authority state + the deterministic envelope), so the same world and the
//! same proposals always yield the same view, outcomes, and replay record.

#![forbid(unsafe_code)]

pub mod project;
pub mod replay;
pub mod tick;
pub mod validate;

pub use project::{project_world_view, AssetStatusMap};
pub use replay::{render_tick_record, run_proposals, PolicyProposalRecord, PolicyTickRecord};
pub use tick::{run_policy_tick, PolicyTickEnvelope, PolicyTickReport};
pub use validate::validate_and_apply;
