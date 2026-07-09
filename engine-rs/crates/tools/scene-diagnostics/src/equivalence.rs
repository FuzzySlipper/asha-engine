//! Load → bootstrap → tick/edit → save → reload **equivalence harness**
//! (world-runtime-composition, #2362).
//!
//! This drives the *real* composition code paths end to end — it builds an
//! abstract fixture bundle (scene + voxel section), loads it through the
//! [`rule_project_bundle::execute_load_plan`] executor, applies a deterministic
//! edit sequence, saves through the real [`compact_voxel_save`] compaction path,
//! reloads through the executor, and compares pre-save (B) against post-reload
//! (C) authority-equivalent state. It is not a stub smoke: nothing here hardcodes
//! the reloaded result.
//!
//! # What "equivalent" means here
//!
//! - **Scene / entity authority**: entity count, `scene node → entity` source
//!   trace, and the deterministic bootstrap spatial-session hash must reproduce.
//! - **Voxel authority**: the voxel state fingerprint after the edits must survive the
//!   save→reload (snapshots + retained log reconstruct identical chunk content).
//!
//! A mismatch is reported as a structured [`DiagnosticCode::RoundTripMismatch`]
//! report (not just an assertion failure), so a harness failure is agent-legible.
//!
//! This harness focuses on the scene-bootstrap identity + voxel authority facets,
//! so the scene/entity side here round-trips its bootstrapped identity. Runtime
//! *divergence* persistence — saving runtime-created entities and diverged
//! transforms/capabilities/relations into a `sessionStateSnapshot` artifact — is now
//! wired separately (#2484): see [`crate::session_state::session_state_round_trip`] for
//! the runtime-authority equivalence harness and the executor's
//! `RestoreSessionState` stage for the load path.

use core_events::VoxelEditEvent;
use core_ids::{RuntimeSessionId, SceneId};
use core_scene::SpatialSessionHash;
use core_space::VoxelGridSpec;
use protocol_diagnostics::{
    DiagnosticCode, DiagnosticReport, DiagnosticReportSet, DiagnosticSourceRef, RemedyAction,
    SuggestedRemedy,
};
use rule_project_bundle::{
    compact_voxel_save, execute_load_plan, BundleArtifacts, LoadExecutionError,
    ProjectBundleLoadResult,
};
use svc_serialization::{BundleHash, LoadPlan, LoadStep};

use crate::roundtrip::voxel_state_fingerprint;

/// A deterministic comparison of pre-save (B) vs post-reload (C) authority.
#[derive(Debug, Clone)]
pub struct BundleEquivalenceReport {
    pub entities_b: usize,
    pub entities_c: usize,
    pub source_trace_b: usize,
    pub source_trace_c: usize,
    pub spatial_session_hash_b: SpatialSessionHash,
    pub spatial_session_hash_c: SpatialSessionHash,
    /// Voxel state fingerprint after the edits (B') and after reload (C).
    pub voxel_hash_b: Option<BundleHash>,
    pub voxel_hash_c: Option<BundleHash>,
    /// Structured mismatch diagnostics (empty == equivalent).
    pub diagnostics: DiagnosticReportSet,
}

impl BundleEquivalenceReport {
    /// `true` if every compared authority facet matched and no mismatch was found.
    pub fn is_equivalent(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// A deterministic, greppable summary (golden-friendly).
    pub fn to_report_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "equivalence equivalent={} entitiesB={} entitiesC={} traceB={} traceC={}\n",
            self.is_equivalent(),
            self.entities_b,
            self.entities_c,
            self.source_trace_b,
            self.source_trace_c,
        ));
        out.push_str(&format!(
            "spatialSessionHashB={:016x} spatialSessionHashC={:016x}\n",
            self.spatial_session_hash_b.0, self.spatial_session_hash_c.0
        ));
        out.push_str(&format!(
            "voxelHashB={} voxelHashC={}\n",
            self.voxel_hash_b
                .map(|h| h.to_hex())
                .unwrap_or_else(|| "none".into()),
            self.voxel_hash_c
                .map(|h| h.to_hex())
                .unwrap_or_else(|| "none".into()),
        ));
        out.push_str(&crate::text::report_set_to_text(&self.diagnostics));
        out
    }
}

/// The mandatory scene-only plan (no voxel stage).
fn scene_plan(
    scene_artifact: &str,
    scene: SceneId,
    runtime_session: RuntimeSessionId,
) -> Vec<LoadStep> {
    vec![
        LoadStep::ValidateVersions {
            bundle_schema_version: 1,
            protocol_version: 1,
        },
        LoadStep::LoadAssetLock {
            artifact: "assets/lock.json".into(),
            asset_count: 0,
        },
        LoadStep::LoadSceneDocument {
            artifact: scene_artifact.into(),
            scene,
        },
        LoadStep::BootstrapScene {
            scene,
            runtime_session,
        },
        LoadStep::ValidateFinalState,
    ]
}

/// Insert a voxel stage (after the scene document, before bootstrap) into a plan.
fn with_voxel_stage(
    mut steps: Vec<LoadStep>,
    edit_logs: Vec<String>,
    snapshots: Vec<String>,
) -> LoadPlan {
    // Scene document is index 2; voxel edits run after it, before bootstrap.
    steps.insert(
        3,
        LoadStep::ApplyVoxelEdits {
            edit_logs,
            snapshots,
            histories: Vec::new(),
        },
    );
    LoadPlan { steps }
}

/// Run the full bundle round-trip: load → edit → save → reload → compare.
///
/// `scene_json` is the bundle's scene artifact; `spec`/`initial_log` describe the
/// loaded voxel section; `tick_edits` are the deterministic operations applied
/// after load (the "tick/edit"); `retain_recent` controls save compaction.
pub fn project_bundle_round_trip(
    scene_json: &str,
    scene: SceneId,
    runtime_session: RuntimeSessionId,
    spec: VoxelGridSpec,
    initial_log: &[VoxelEditEvent],
    tick_edits: &[VoxelEditEvent],
    retain_recent: usize,
) -> Result<BundleEquivalenceReport, LoadExecutionError> {
    use rule_voxel_edit::persist::encode_edit_log;

    // ── Load (state B) ───────────────────────────────────────────────────────
    let initial_log_text = encode_edit_log(initial_log);
    let load_artifacts = BundleArtifacts::new()
        .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
        .with_artifact("scene/scene.json", scene_json)
        .with_artifact("voxel/edits.log", initial_log_text)
        .with_voxel_spec(spec);
    let plan_b = with_voxel_stage(
        scene_plan("scene/scene.json", scene, runtime_session),
        vec!["voxel/edits.log".into()],
        vec![],
    );
    let result_b: ProjectBundleLoadResult = execute_load_plan(&plan_b, &load_artifacts)?;

    // ── Tick / edit ──────────────────────────────────────────────────────────
    // No-op tick is implicit (deterministic). The voxel edit is the persisted op:
    // fold the tick edits into the full log and fingerprint the post-edit world.
    let mut full_log = initial_log.to_vec();
    full_log.extend_from_slice(tick_edits);
    let voxel_b_prime =
        rule_voxel_edit::persist::replay_edit_log(spec, &full_log).map_err(|e| {
            LoadExecutionError::VoxelReplay {
                detail: format!("{e:?}"),
            }
        })?;
    let voxel_hash_b = Some(voxel_state_fingerprint(&voxel_b_prime));

    // ── Save (real compaction) ─────────────────────────────────────────────────
    let save = compact_voxel_save(spec, &full_log, retain_recent).map_err(|e| {
        LoadExecutionError::VoxelReplay {
            detail: format!("{e:?}"),
        }
    })?;

    // ── Reassemble the saved bundle + reload (state C) ─────────────────────────
    let mut reload_artifacts = BundleArtifacts::new()
        .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
        .with_artifact("scene/scene.json", scene_json)
        .with_artifact("voxel/retained.log", save.retained_log_text.clone())
        .with_voxel_spec(spec);
    let mut snapshot_paths = Vec::new();
    for snap in &save.snapshots {
        reload_artifacts = reload_artifacts.with_artifact(snap.path.clone(), snap.text.clone());
        snapshot_paths.push(snap.path.clone());
    }
    let plan_c = with_voxel_stage(
        scene_plan("scene/scene.json", scene, runtime_session),
        vec!["voxel/retained.log".into()],
        snapshot_paths,
    );
    let result_c = execute_load_plan(&plan_c, &reload_artifacts)?;
    let voxel_hash_c = result_c.voxel.as_ref().map(voxel_state_fingerprint);

    // ── Compare B vs C ─────────────────────────────────────────────────────────
    let mut diagnostics = DiagnosticReportSet::new();
    if result_b.spatial_session_hash != result_c.spatial_session_hash {
        diagnostics.push(mismatch(
            "world-hash",
            format!(
                "scene/entity spatial-session hash changed across save/reload: {:016x} != {:016x}",
                result_b.spatial_session_hash.0, result_c.spatial_session_hash.0
            ),
        ));
    }
    if result_b.spatial_session.entity_count() != result_c.spatial_session.entity_count() {
        diagnostics.push(mismatch(
            "entity-count",
            format!(
                "entity count changed: {} != {}",
                result_b.spatial_session.entity_count(),
                result_c.spatial_session.entity_count()
            ),
        ));
    }
    if result_b.bootstrap.source_trace.len() != result_c.bootstrap.source_trace.len() {
        diagnostics.push(mismatch(
            "source-trace",
            "source-trace length changed across save/reload".to_string(),
        ));
    }
    if voxel_hash_b != voxel_hash_c {
        diagnostics.push(mismatch(
            "voxel",
            format!(
                "voxel content fingerprint changed across save/reload: {} != {}",
                voxel_hash_b.map(|h| h.to_hex()).unwrap_or_default(),
                voxel_hash_c.map(|h| h.to_hex()).unwrap_or_default(),
            ),
        ));
    }

    Ok(BundleEquivalenceReport {
        entities_b: result_b.spatial_session.entity_count(),
        entities_c: result_c.spatial_session.entity_count(),
        source_trace_b: result_b.bootstrap.source_trace.len(),
        source_trace_c: result_c.bootstrap.source_trace.len(),
        spatial_session_hash_b: result_b.spatial_session_hash,
        spatial_session_hash_c: result_c.spatial_session_hash,
        voxel_hash_b,
        voxel_hash_c,
        diagnostics,
    })
}

fn mismatch(reference: &str, message: String) -> DiagnosticReport {
    DiagnosticReport::new(
        DiagnosticCode::RoundTripMismatch,
        reference,
        DiagnosticSourceRef::empty(),
        message,
    )
    .with_remedy(SuggestedRemedy::new(
        RemedyAction::Inspect,
        "save/reload did not preserve authority-equivalent state; inspect the lost facet",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lost facet must surface as a structured `roundTripMismatch` report, not
    /// merely a boolean — the harness's "fail with a useful diagnostic" contract.
    #[test]
    fn a_lost_facet_is_reported_not_just_asserted() {
        let mut diagnostics = DiagnosticReportSet::new();
        diagnostics.push(mismatch(
            "voxel",
            "voxel content fingerprint changed across save/reload".to_string(),
        ));
        let report = BundleEquivalenceReport {
            entities_b: 2,
            entities_c: 2,
            source_trace_b: 2,
            source_trace_c: 2,
            spatial_session_hash_b: SpatialSessionHash(1),
            spatial_session_hash_c: SpatialSessionHash(1),
            voxel_hash_b: Some(BundleHash(10)),
            voxel_hash_c: Some(BundleHash(11)),
            diagnostics,
        };
        assert!(!report.is_equivalent());
        let text = report.to_report_text();
        assert!(text.contains("roundTripMismatch"), "{text}");
        assert!(text.contains("ref=voxel"), "{text}");
    }
}
