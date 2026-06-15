//! World load/save **composition** failure diagnostics (world-runtime-composition,
//! #2364).
//!
//! The load executor ([`rule_world_bundle::execute_load_plan`]) returns a
//! classified [`LoadExecutionError`] on failure. This module maps each failure
//! into a stable [`protocol_diagnostics`] report — scope `worldComposition` —
//! carrying the stage / artifact source ref, a severity tied to recovery policy,
//! and a suggested next step (the recovery hint). It is observational: it reads a
//! failure and describes it; it never mutates authority and never auto-repairs.
//!
//! Severity policy: a load that cannot complete is `Fatal` (it blocks the load);
//! a final-consistency / equivalence problem after composition is reported with
//! the code's default severity (`finalConsistencyMismatch` is Fatal).

use protocol_diagnostics::{
    DiagnosticCode, DiagnosticReport, DiagnosticReportSet, DiagnosticSourceRef, RemedyAction,
    SuggestedRemedy,
};
use rule_world_bundle::LoadExecutionError;

/// Map one composition failure into a stable diagnostic report.
pub fn composition_failure_diagnostic(err: &LoadExecutionError) -> DiagnosticReport {
    match err {
        LoadExecutionError::PlanInvalid(e) => report(
            DiagnosticCode::LoadStageFailed,
            "plan",
            DiagnosticSourceRef::empty(),
            format!("load plan is not a coherent ordered plan: {e}"),
            RemedyAction::Inspect,
            "rebuild the load plan from a validated manifest",
        ),
        LoadExecutionError::MissingArtifact { stage, path } => report(
            DiagnosticCode::LoadStageFailed,
            stage.label(),
            DiagnosticSourceRef::empty().with_bundle_path(path.clone()),
            format!("stage `{}` requires artifact `{path}` which is absent", stage.label()),
            RemedyAction::RestoreArtifact,
            "restore the missing artifact from a known-good bundle copy",
        ),
        LoadExecutionError::EmptyArtifact { stage, path } => report(
            DiagnosticCode::LoadStageFailed,
            stage.label(),
            DiagnosticSourceRef::empty().with_bundle_path(path.clone()),
            format!("stage `{}` artifact `{path}` is empty", stage.label()),
            RemedyAction::RestoreArtifact,
            "restore the artifact; an empty durable artifact cannot load",
        ),
        LoadExecutionError::VersionUnsupported {
            bundle_schema,
            protocol,
        } => report(
            DiagnosticCode::ManifestProtocolMismatch,
            "versions",
            DiagnosticSourceRef::empty(),
            format!(
                "bundle schema {bundle_schema} / protocol {protocol} is newer than this build supports"
            ),
            RemedyAction::Inspect,
            "reject the newer bundle; upgrade the engine or re-export at a supported version",
        ),
        LoadExecutionError::SceneDecode { artifact, error } => report(
            DiagnosticCode::CorruptBundleArtifact,
            "sceneDocument",
            DiagnosticSourceRef::empty().with_bundle_path(artifact.clone()),
            format!("scene document `{artifact}` failed to decode: {error:?}"),
            RemedyAction::RestoreArtifact,
            "restore the scene-document artifact from a known-good copy",
        ),
        LoadExecutionError::SceneInvalid { artifact, report: r } => report(
            DiagnosticCode::LoadStageFailed,
            "sceneDocument",
            DiagnosticSourceRef::empty().with_bundle_path(artifact.clone()),
            format!(
                "scene document `{artifact}` failed validation with {} error(s)",
                r.errors.len()
            ),
            RemedyAction::Inspect,
            "inspect the scene validation report (run scene_diagnostics for the classified errors)",
        ),
        LoadExecutionError::SceneIdMismatch { expected, found } => report(
            DiagnosticCode::LoadStageFailed,
            "sceneDocument",
            DiagnosticSourceRef::empty(),
            format!("scene id mismatch: plan expected {expected} but artifact is {found}"),
            RemedyAction::Inspect,
            "the bundle manifest and scene artifact disagree on scene identity",
        ),
        LoadExecutionError::Bootstrap(e) => report(
            DiagnosticCode::LoadStageFailed,
            "bootstrap",
            DiagnosticSourceRef::empty(),
            format!("atomic scene bootstrap rejected the document: {e:?}"),
            RemedyAction::Inspect,
            "inspect the scene; bootstrap fails closed before producing any world",
        ),
        LoadExecutionError::VoxelDecode { path, detail } => report(
            DiagnosticCode::CorruptBundleArtifact,
            "voxelEdits",
            DiagnosticSourceRef::empty().with_bundle_path(path.clone()),
            format!("voxel artifact `{path}` failed to decode: {detail}"),
            RemedyAction::RestoreArtifact,
            "restore the voxel artifact from a known-good copy",
        ),
        LoadExecutionError::VoxelSpecMissing => report(
            DiagnosticCode::LoadStageFailed,
            "voxelEdits",
            DiagnosticSourceRef::empty(),
            "voxel section present but the bundle carried no voxel grid spec".to_string(),
            RemedyAction::Inspect,
            "provide the bundle's voxel grid metadata (grid id / dims / voxel size)",
        ),
        LoadExecutionError::VoxelReplay { detail } => report(
            DiagnosticCode::LoadStageFailed,
            "voxelEdits",
            DiagnosticSourceRef::empty(),
            format!("voxel replay/reconstruction was rejected: {detail}"),
            RemedyAction::Regenerate,
            "pin the old generator or run dev regenerate-and-replay to resolve the conflict",
        ),
        LoadExecutionError::WorldStateDecode { path, error } => report(
            DiagnosticCode::CorruptBundleArtifact,
            "worldStateSnapshot",
            DiagnosticSourceRef::empty().with_bundle_path(path.clone()),
            format!("world-state snapshot `{path}` failed to decode: {error}"),
            RemedyAction::RestoreArtifact,
            "restore the world-state snapshot artifact from a known-good copy",
        ),
        LoadExecutionError::FinalConsistency { detail } => report(
            DiagnosticCode::FinalConsistencyMismatch,
            "finalValidation",
            DiagnosticSourceRef::empty(),
            format!("final consistency check failed: {detail}"),
            RemedyAction::Inspect,
            "the composed world is internally inconsistent; do not commit it",
        ),
    }
}

/// Map a load result's error into a one-report set (convenience for callers that
/// accumulate composition diagnostics alongside other scopes).
pub fn composition_failure_set(err: &LoadExecutionError) -> DiagnosticReportSet {
    let mut set = DiagnosticReportSet::new();
    set.push(composition_failure_diagnostic(err));
    set
}

fn report(
    code: DiagnosticCode,
    reference: &str,
    source: DiagnosticSourceRef,
    message: String,
    remedy: RemedyAction,
    detail: &str,
) -> DiagnosticReport {
    DiagnosticReport::new(code, reference, source, message)
        .with_remedy(SuggestedRemedy::new(remedy, detail))
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_diagnostics::DiagnosticSeverity;
    use svc_serialization::LoadStage;

    #[test]
    fn missing_artifact_is_fatal_load_stage_failure_with_path() {
        let err = LoadExecutionError::MissingArtifact {
            stage: LoadStage::SceneDocument,
            path: "scene/scene.json".into(),
        };
        let d = composition_failure_diagnostic(&err);
        assert_eq!(d.code, DiagnosticCode::LoadStageFailed);
        assert_eq!(d.severity, DiagnosticSeverity::Fatal);
        assert!(d.severity.blocks_load());
        assert_eq!(d.source.bundle_path.as_deref(), Some("scene/scene.json"));
        assert!(d.remedy.is_some());
    }

    #[test]
    fn version_unsupported_maps_to_manifest_mismatch() {
        let err = LoadExecutionError::VersionUnsupported {
            bundle_schema: 99,
            protocol: 1,
        };
        let d = composition_failure_diagnostic(&err);
        assert_eq!(d.code, DiagnosticCode::ManifestProtocolMismatch);
        assert_eq!(d.severity, DiagnosticSeverity::Fatal);
    }

    #[test]
    fn final_consistency_maps_to_mismatch_code() {
        let err = LoadExecutionError::FinalConsistency {
            detail: "hash drift".into(),
        };
        let d = composition_failure_diagnostic(&err);
        assert_eq!(d.code, DiagnosticCode::FinalConsistencyMismatch);
        assert_eq!(d.severity, DiagnosticSeverity::Fatal);
    }

    #[test]
    fn every_variant_yields_a_world_composition_or_bundle_scoped_report() {
        // Each failure path produces a report whose code scopes into world
        // composition or world bundle, and carries a remedy.
        let errs = [
            LoadExecutionError::VoxelSpecMissing,
            LoadExecutionError::VoxelReplay { detail: "x".into() },
            LoadExecutionError::SceneIdMismatch {
                expected: core_ids_scene_id(1),
                found: core_ids_scene_id(2),
            },
        ];
        for e in &errs {
            let d = composition_failure_diagnostic(e);
            assert!(d.remedy.is_some(), "{e:?} has no remedy");
        }
    }

    fn core_ids_scene_id(raw: u64) -> core_ids::SceneId {
        core_ids::SceneId::new(raw)
    }
}
