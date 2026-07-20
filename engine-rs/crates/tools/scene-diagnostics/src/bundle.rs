//! ProjectBundle diagnostics: manifest validation, durable-artifact integrity,
//! missing optional cache, and terrain generator mismatch.

use std::collections::BTreeMap;

use protocol_diagnostics::{
    DiagnosticCode, DiagnosticReport, DiagnosticReportSet, DiagnosticSeverity, DiagnosticSourceRef,
    RemedyAction, SuggestedRemedy,
};
use rule_project_bundle::{GeneratorMismatch, RegenReplayReport};
use svc_serialization::{ArtifactClass, BundleHash, ManifestError, ProjectBundleManifest};

/// Emit diagnostics for a ProjectBundle manifest by running its fail-closed
/// validation. A version mismatch is `Fatal` (incompatible — stop the load);
/// structural manifest faults (missing/duplicate artifact, unhashed durable) are
/// reported as a corrupt/incomplete bundle. Read-only.
pub fn manifest_diagnostics(manifest: &ProjectBundleManifest) -> DiagnosticReportSet {
    let mut set = DiagnosticReportSet::new();
    if let Err(err) = manifest.validate() {
        set.push(map_manifest_error(&err));
    }
    set
}

fn map_manifest_error(err: &ManifestError) -> DiagnosticReport {
    let message = err.to_string();
    match err {
        ManifestError::UnsupportedSchema { .. } | ManifestError::UnsupportedProtocol { .. } => {
            DiagnosticReport::new(
                DiagnosticCode::ManifestProtocolMismatch,
                "manifest",
                DiagnosticSourceRef::empty(),
                message,
            )
            .with_remedy(SuggestedRemedy::new(
                RemedyAction::Inspect,
                "open the bundle with a build that supports its schema/protocol version",
            ))
        }
        ManifestError::DuplicateArtifact { path }
        | ManifestError::MissingArtifact { path, .. }
        | ManifestError::DurableMissingHash { path }
        | ManifestError::LoadRequiredMissingHash { path }
        | ManifestError::InvalidArtifactPath { path }
        | ManifestError::SceneArtifactMismatch { path, .. }
        | ManifestError::UnreferencedSceneArtifact { path }
        | ManifestError::UnknownArtifactRole { path, .. }
        | ManifestError::ArtifactClassMismatch { path, .. } => DiagnosticReport::new(
            DiagnosticCode::CorruptBundleArtifact,
            path.clone(),
            DiagnosticSourceRef::empty().with_bundle_path(path.clone()),
            message,
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::RestoreArtifact,
            "restore the bundle's artifact table from a known-good copy",
        )),
        ManifestError::DuplicateArtifactRole { role } => DiagnosticReport::new(
            DiagnosticCode::CorruptBundleArtifact,
            role.clone(),
            DiagnosticSourceRef::empty(),
            message,
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::Inspect,
            "retain one authoritative artifact for the singleton bundle role",
        )),
        ManifestError::DuplicateScene { scene } | ManifestError::MissingEntryScene { scene } => {
            DiagnosticReport::new(
                DiagnosticCode::CorruptBundleArtifact,
                format!("scene:{scene}"),
                DiagnosticSourceRef::empty(),
                message,
            )
            .with_remedy(SuggestedRemedy::new(
                RemedyAction::Inspect,
                "repair the manifest scene table and explicit entry-scene identity",
            ))
        }
    }
}

/// Cross-check the manifest's recorded artifact hashes against the *actual*
/// content hashes of the files on disk. Any durable or generated artifact whose
/// recorded hash does not match its actual bytes is a `Fatal`
/// [`DiagnosticCode::CorruptBundleArtifact`]. Cache artifacts are skipped
/// (disposable). `actual_hashes` maps bundle-relative path → measured hash; a
/// path absent from the map is treated as "not measured" and skipped here
/// (use [`missing_cache_diagnostics`] / manifest validation for presence).
pub fn artifact_integrity_diagnostics(
    manifest: &ProjectBundleManifest,
    actual_hashes: &BTreeMap<String, BundleHash>,
) -> DiagnosticReportSet {
    let mut set = DiagnosticReportSet::new();
    for artifact in &manifest.artifacts {
        if artifact.class == ArtifactClass::Cache {
            continue;
        }
        let (Some(recorded), Some(actual)) = (
            artifact.content_hash,
            actual_hashes.get(&artifact.path).copied(),
        ) else {
            continue;
        };
        if recorded != actual {
            set.push(
                DiagnosticReport::new(
                    DiagnosticCode::CorruptBundleArtifact,
                    artifact.path.clone(),
                    DiagnosticSourceRef::empty().with_bundle_path(artifact.path.clone()),
                    format!(
                        "durable artifact `{}` failed its content hash (recorded {}, actual {})",
                        artifact.path,
                        recorded.to_hex(),
                        actual.to_hex()
                    ),
                )
                .with_remedy(SuggestedRemedy::new(
                    RemedyAction::RestoreArtifact,
                    "restore the artifact from a known-good bundle copy",
                )),
            );
        }
    }
    set
}

/// Warn about optional cache artifacts that the manifest lists but that are
/// absent from `present_paths`. Cache disposal is allowed, so these are
/// `Warning`s, never load-blocking. Read-only.
pub fn missing_cache_diagnostics(
    manifest: &ProjectBundleManifest,
    present_paths: &std::collections::BTreeSet<String>,
) -> DiagnosticReportSet {
    let mut set = DiagnosticReportSet::new();
    for artifact in &manifest.artifacts {
        if artifact.class == ArtifactClass::Cache && !present_paths.contains(&artifact.path) {
            set.push(
                DiagnosticReport::new(
                    DiagnosticCode::MissingCacheWarning,
                    artifact.path.clone(),
                    DiagnosticSourceRef::empty().with_bundle_path(artifact.path.clone()),
                    format!(
                        "optional cache artifact `{}` is absent; it will be rebuilt",
                        artifact.path
                    ),
                )
                .with_remedy(SuggestedRemedy::new(
                    RemedyAction::RefreshCache,
                    "no action required; the cache is reproducible",
                )),
            );
        }
    }
    set
}

/// Emit a `Fatal` diagnostic for a fail-closed terrain generator version
/// mismatch (`rule-project-bundle`'s [`GeneratorMismatch`]).
pub fn generator_mismatch_diagnostic(mismatch: &GeneratorMismatch) -> DiagnosticReport {
    DiagnosticReport::new(
        DiagnosticCode::GeneratorMismatch,
        "generator",
        DiagnosticSourceRef::empty(),
        mismatch.to_string(),
    )
    .with_severity(DiagnosticSeverity::Fatal)
    .with_remedy(SuggestedRemedy::new(
        RemedyAction::Regenerate,
        "regenerate-and-replay in dev to inspect conflicts, or pin the saved generator",
    ))
}

/// Map a development regenerate-and-replay diagnostic into reports: one
/// `Warning` [`DiagnosticCode::GeneratorMismatch`] per edit whose authored
/// generated context changed under the new generator. A clean replay yields no
/// reports.
pub fn regen_conflict_diagnostics(report: &RegenReplayReport) -> DiagnosticReportSet {
    let mut set = DiagnosticReportSet::new();
    for c in &report.conflicts {
        set.push(
            DiagnosticReport::new(
                DiagnosticCode::GeneratorMismatch,
                format!("edit:{}", c.event_id),
                DiagnosticSourceRef::empty().with_chunk([c.coord.x, c.coord.y, c.coord.z]),
                format!(
                    "edit {} at {:?} sits on changed generated context (was {:?}, now {:?}); {}",
                    c.event_id,
                    c.coord.to_array(),
                    c.old_generated,
                    c.new_generated,
                    c.suggested.label()
                ),
            )
            // A regenerate-and-replay conflict is a dev diagnostic, not a hard
            // load failure (the fail-closed path is the Fatal one above).
            .with_severity(DiagnosticSeverity::Warning)
            .with_remedy(SuggestedRemedy::new(
                RemedyAction::Inspect,
                "review whether to reapply, drop, or pin the old generator",
            )),
        );
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::{ProjectId, SceneId};
    use svc_serialization::artifact::ArtifactRole;
    use svc_serialization::{
        ArtifactEntry, AssetLockSection, GeneratorMetadata, ProjectSection, SceneSection,
    };

    fn manifest() -> ProjectBundleManifest {
        let scene_bytes = b"scene-doc";
        let lock_bytes = b"asset-lock";
        ProjectBundleManifest {
            bundle_schema_version: 2,
            protocol_version: 1,
            project: ProjectSection {
                id: ProjectId::new(1),
                name: None,
            },
            entry_scene: SceneId::new(1),
            scenes: vec![SceneSection {
                id: SceneId::new(1),
                schema_version: 1,
                artifact: "scene/scene.json".to_string(),
            }],
            asset_lock: AssetLockSection {
                artifact: "scene/asset-lock.json".to_string(),
                asset_count: 0,
            },
            generation_provenance: Some(GeneratorMetadata {
                provider: "asha.environment.test".to_string(),
                seed: 7,
                version: 1,
                params: "p".to_string(),
            }),
            artifacts: vec![
                ArtifactEntry::durable(
                    "scene/scene.json",
                    ArtifactRole::SceneDocument,
                    scene_bytes,
                ),
                ArtifactEntry::durable(
                    "scene/asset-lock.json",
                    ArtifactRole::AssetLock,
                    lock_bytes,
                ),
            ],
        }
    }

    #[test]
    fn valid_manifest_emits_nothing() {
        assert!(manifest_diagnostics(&manifest()).is_empty());
    }

    #[test]
    fn unsupported_schema_is_fatal_mismatch() {
        let mut m = manifest();
        m.bundle_schema_version = 99;
        let set = manifest_diagnostics(&m);
        assert!(set.blocks_load());
        assert_eq!(
            set.reports[0].code,
            DiagnosticCode::ManifestProtocolMismatch
        );
    }

    #[test]
    fn tampered_durable_artifact_is_corrupt() {
        let m = manifest();
        let mut actual: BTreeMap<String, BundleHash> = BTreeMap::new();
        // Correct hash for the lock, wrong hash for the scene doc.
        actual.insert(
            "scene/asset-lock.json".to_string(),
            BundleHash::of(b"asset-lock"),
        );
        actual.insert("scene/scene.json".to_string(), BundleHash::of(b"TAMPERED"));
        let set = artifact_integrity_diagnostics(&m, &actual);
        assert_eq!(set.reports.len(), 1);
        assert_eq!(set.reports[0].code, DiagnosticCode::CorruptBundleArtifact);
        assert!(set.blocks_load());
        assert_eq!(
            set.reports[0].source.bundle_path.as_deref(),
            Some("scene/scene.json")
        );
    }

    #[test]
    fn absent_cache_is_a_warning_not_a_block() {
        let mut m = manifest();
        m.artifacts
            .push(ArtifactEntry::cache("cache/mesh.bin", ArtifactRole::Cache));
        let present = std::collections::BTreeSet::new(); // nothing present on disk
        let set = missing_cache_diagnostics(&m, &present);
        assert_eq!(set.reports.len(), 1);
        assert_eq!(set.reports[0].code, DiagnosticCode::MissingCacheWarning);
        assert!(!set.blocks_load());
    }

    #[test]
    fn generator_mismatch_is_fatal() {
        let r = generator_mismatch_diagnostic(&GeneratorMismatch {
            saved_version: 1,
            current_version: 2,
        });
        assert_eq!(r.code, DiagnosticCode::GeneratorMismatch);
        assert_eq!(r.severity, DiagnosticSeverity::Fatal);
    }
}
