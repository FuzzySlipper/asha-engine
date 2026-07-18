//! Scene-document diagnostics: map `core-scene` validation (plus optional
//! catalog cross-checks) into stable diagnostic reports.

use core_catalog::Catalog;
use core_scene::document::FlatSceneDocument;
use core_scene::validate::{validate, SceneValidationError};
use protocol_diagnostics::{
    DiagnosticCode, DiagnosticReport, DiagnosticReportSet, DiagnosticSourceRef, RemedyAction,
    SuggestedRemedy,
};

/// Emit diagnostics for a flat scene document.
///
/// Runs `core-scene` validation and maps every classified error to a stable
/// [`DiagnosticReport`]. When `catalog` is supplied, also cross-checks each
/// node's asset reference against the catalog and reports any that are absent
/// ([`DiagnosticCode::SceneAssetMissing`]).
///
/// Read-only: neither the document nor the catalog is mutated.
pub fn scene_diagnostics(
    doc: &FlatSceneDocument,
    catalog: Option<&Catalog>,
) -> DiagnosticReportSet {
    let mut set = DiagnosticReportSet::new();

    for error in validate(doc).errors {
        set.push(map_scene_error(&error));
    }

    // Catalog cross-check: a node asset ref absent from the catalog. The scene
    // validator only checks the ref's kind against the node variant; existence
    // against a catalog is this cross-check's job.
    if let Some(catalog) = catalog {
        for rec in &doc.nodes {
            if let Some(asset) = rec.kind.asset() {
                if !catalog.contains(asset.id()) {
                    set.push(
                        DiagnosticReport::new(
                            DiagnosticCode::SceneAssetMissing,
                            asset.id().as_str(),
                            DiagnosticSourceRef::empty()
                                .with_scene_node(rec.id.raw())
                                .with_asset(asset.id().as_str()),
                            format!(
                                "scene node {} references asset `{}` not present in the catalog",
                                rec.id.raw(),
                                asset.id().as_str()
                            ),
                        )
                        .with_remedy(SuggestedRemedy::new(
                            RemedyAction::ProvideAsset,
                            format!(
                                "add `{}` to the catalog or fix the reference",
                                asset.id().as_str()
                            ),
                        )),
                    );
                }
            }
        }
    }

    set
}

fn map_scene_error(error: &SceneValidationError) -> DiagnosticReport {
    match error {
        SceneValidationError::DuplicateNodeId { id } => DiagnosticReport::new(
            DiagnosticCode::DuplicateSceneId,
            format!("node:{}", id.raw()),
            DiagnosticSourceRef::empty().with_scene_node(id.raw()),
            format!("two scene nodes share id {}", id.raw()),
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::FixReference,
            "give each node a unique stable id",
        )),
        SceneValidationError::UnknownParent { node, parent } => DiagnosticReport::new(
            DiagnosticCode::InvalidSceneParent,
            format!("node:{}", node.raw()),
            DiagnosticSourceRef::empty().with_scene_node(node.raw()),
            format!(
                "scene node {} names parent {} that is not in the document",
                node.raw(),
                parent.raw()
            ),
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::FixReference,
            "point the node at an existing parent, or make it a root",
        )),
        SceneValidationError::Cycle { path } => {
            let chain = path
                .iter()
                .map(|n| n.raw().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            let first = path.first().map(|n| n.raw()).unwrap_or_default();
            DiagnosticReport::new(
                DiagnosticCode::SceneParentCycle,
                format!("node:{first}"),
                DiagnosticSourceRef::empty().with_scene_node(first),
                format!("scene parent pointers form a cycle: {chain} -> {first}"),
            )
            .with_remedy(SuggestedRemedy::new(
                RemedyAction::BreakCycle,
                "remove one parent edge in the reported cycle",
            ))
        }
        SceneValidationError::InvalidTransform { node, reason } => DiagnosticReport::new(
            DiagnosticCode::InvalidSceneTransform,
            format!("node:{}", node.raw()),
            DiagnosticSourceRef::empty().with_scene_node(node.raw()),
            format!(
                "scene node {} has an invalid transform: {reason:?}",
                node.raw()
            ),
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::Inspect,
            "fix the node transform (finite translation/scale, normalized rotation)",
        )),
        SceneValidationError::InvalidVoxelVolumeTransform { node, reason } => {
            DiagnosticReport::new(
                DiagnosticCode::InvalidSceneTransform,
                format!("node:{}", node.raw()),
                DiagnosticSourceRef::empty().with_scene_node(node.raw()),
                format!(
                    "scene voxel-volume node {} uses unsupported composed transform: {reason}",
                    node.raw()
                ),
            )
            .with_remedy(SuggestedRemedy::new(
                RemedyAction::Inspect,
                "use identity rotation and unit scale on the voxel node and its ancestors",
            ))
        }
        SceneValidationError::AssetKindMismatch {
            node,
            expected,
            actual,
        } => DiagnosticReport::new(
            DiagnosticCode::SceneAssetWrongKind,
            format!("node:{}", node.raw()),
            DiagnosticSourceRef::empty().with_scene_node(node.raw()),
            format!(
                "scene node {} expects a `{expected}` asset but references a `{actual}`",
                node.raw()
            ),
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::FixReference,
            "reference an asset of the kind the node variant requires",
        )),
        SceneValidationError::InvalidLight { node, reason } => DiagnosticReport::new(
            DiagnosticCode::InvalidSceneTransform,
            format!("node:{}", node.raw()),
            DiagnosticSourceRef::empty().with_scene_node(node.raw()),
            format!(
                "scene node {} has an invalid stored light: {}",
                node.raw(),
                reason.label()
            ),
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::Inspect,
            "fix the typed light fields or use scene schema/authoring format version 2",
        )),
        SceneValidationError::DuplicateMarkerId { node, marker_id } => DiagnosticReport::new(
            DiagnosticCode::DuplicateSceneMarkerId,
            format!("marker:{marker_id}"),
            DiagnosticSourceRef::empty().with_scene_node(node.raw()),
            format!(
                "scene node {} reuses durable marker id `{marker_id}`",
                node.raw()
            ),
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::FixReference,
            "give each authored scene marker a unique durable marker id",
        )),
        SceneValidationError::InvalidMarker { node, reason } => DiagnosticReport::new(
            DiagnosticCode::InvalidSceneMarker,
            format!("node:{}", node.raw()),
            DiagnosticSourceRef::empty().with_scene_node(node.raw()),
            format!("scene node {} has an invalid marker: {reason}", node.raw()),
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::Inspect,
            "fix the stored marker id and use scene schema/authoring format version 4",
        )),
        SceneValidationError::DuplicateEntityInstanceId { node, instance_id } => {
            DiagnosticReport::new(
                DiagnosticCode::DuplicateSceneEntityInstanceId,
                format!("entity-instance:{instance_id}"),
                DiagnosticSourceRef::empty().with_scene_node(node.raw()),
                format!(
                    "scene node {} reuses durable entity-instance id `{instance_id}`",
                    node.raw()
                ),
            )
            .with_remedy(SuggestedRemedy::new(
                RemedyAction::FixReference,
                "give each authored entity placement a unique durable instance id",
            ))
        }
        SceneValidationError::InvalidEntityInstance { node, reason } => DiagnosticReport::new(
            DiagnosticCode::InvalidSceneEntityInstance,
            format!("node:{}", node.raw()),
            DiagnosticSourceRef::empty().with_scene_node(node.raw()),
            format!(
                "scene node {} has an invalid entity-instance binding: {reason}",
                node.raw()
            ),
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::Inspect,
            "fix the stored entity definition or prefab instance binding",
        )),
        SceneValidationError::DuplicateBootstrapNode { node } => DiagnosticReport::new(
            DiagnosticCode::DuplicateSceneBootstrap,
            format!("node:{}", node.raw()),
            DiagnosticSourceRef::empty().with_scene_node(node.raw()),
            format!(
                "scene node {} duplicates the scene-wide bootstrap binding",
                node.raw()
            ),
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::FixReference,
            "retain exactly one scene-wide bootstrap node",
        )),
        SceneValidationError::InvalidBootstrap { node, reason } => DiagnosticReport::new(
            DiagnosticCode::InvalidSceneBootstrap,
            format!("node:{}", node.raw()),
            DiagnosticSourceRef::empty().with_scene_node(node.raw()),
            format!(
                "scene node {} has an invalid bootstrap binding: {reason}",
                node.raw()
            ),
        )
        .with_remedy(SuggestedRemedy::new(
            RemedyAction::Inspect,
            "fix the root bootstrap generator and catalog bindings",
        )),
        SceneValidationError::DuplicateCatalogBinding { node, binding_id } => {
            DiagnosticReport::new(
                DiagnosticCode::DuplicateSceneCatalogBinding,
                format!("catalog-binding:{binding_id}"),
                DiagnosticSourceRef::empty().with_scene_node(node.raw()),
                format!(
                    "scene node {} reuses bootstrap catalog binding id `{binding_id}`",
                    node.raw()
                ),
            )
            .with_remedy(SuggestedRemedy::new(
                RemedyAction::FixReference,
                "give each bootstrap catalog input a unique binding id",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_assets::{AssetId, AssetReference, AssetVersionReq};
    use core_ids::{SceneId, SceneNodeId};
    use core_scene::document::{FlatSceneDocument, SceneMetadata, SceneNodeKind, SceneNodeRecord};
    use core_scene::transform::SceneTransform;
    use protocol_diagnostics::DiagnosticSeverity;

    fn aref(s: &str) -> AssetReference {
        AssetReference::new(AssetId::parse(s).unwrap(), AssetVersionReq::Any, None)
    }

    fn node(id: u64, parent: Option<u64>, kind: SceneNodeKind) -> SceneNodeRecord {
        SceneNodeRecord {
            id: SceneNodeId::new(id),
            parent: parent.map(SceneNodeId::new),
            child_order: 0,
            transform: SceneTransform::IDENTITY,
            kind,
            metadata: Default::default(),
        }
    }

    fn doc(nodes: Vec<SceneNodeRecord>) -> FlatSceneDocument {
        FlatSceneDocument {
            id: SceneId::new(1),
            schema_version: 1,
            metadata: SceneMetadata::default(),
            dependencies: Vec::new(),
            nodes,
        }
    }

    #[test]
    fn clean_scene_emits_nothing() {
        let d = doc(vec![node(1, None, SceneNodeKind::EmptyGroup)]);
        assert!(scene_diagnostics(&d, None).is_empty());
    }

    #[test]
    fn duplicate_id_and_unknown_parent_are_classified() {
        let d = doc(vec![
            node(1, None, SceneNodeKind::EmptyGroup),
            node(1, None, SceneNodeKind::EmptyGroup),
            node(2, Some(99), SceneNodeKind::EmptyGroup),
        ]);
        let set = scene_diagnostics(&d, None);
        assert!(set
            .reports
            .iter()
            .any(|r| r.code == DiagnosticCode::DuplicateSceneId));
        assert!(set
            .reports
            .iter()
            .any(|r| r.code == DiagnosticCode::InvalidSceneParent
                && r.source.scene_node_id == Some(2)));
    }

    #[test]
    fn wrong_kind_asset_is_classified_from_scene_validation() {
        // A StaticMesh node pointing at a sprite asset → kind mismatch.
        let d = doc(vec![node(
            1,
            None,
            SceneNodeKind::StaticMesh(aref("sprite/hard-hat")),
        )]);
        let set = scene_diagnostics(&d, None);
        assert!(set
            .reports
            .iter()
            .any(|r| r.code == DiagnosticCode::SceneAssetWrongKind));
    }

    #[test]
    fn missing_asset_needs_catalog_cross_check() {
        let d = doc(vec![node(
            1,
            None,
            SceneNodeKind::StaticMesh(aref("mesh/belt-straight")),
        )]);
        // No catalog → no missing-asset report (kind is fine).
        assert!(scene_diagnostics(&d, None).is_empty());
        // Empty catalog → the asset is missing.
        let empty = Catalog::new();
        let set = scene_diagnostics(&d, Some(&empty));
        let missing = set
            .reports
            .iter()
            .find(|r| r.code == DiagnosticCode::SceneAssetMissing)
            .expect("missing-asset reported");
        assert_eq!(missing.severity, DiagnosticSeverity::Error);
        assert_eq!(
            missing.source.asset_id.as_deref(),
            Some("mesh/belt-straight")
        );
    }

    #[test]
    fn entity_and_bootstrap_errors_have_stable_scene_diagnostics() {
        let cases = [
            (
                SceneValidationError::DuplicateEntityInstanceId {
                    node: SceneNodeId::new(7),
                    instance_id: "actor/player".into(),
                },
                DiagnosticCode::DuplicateSceneEntityInstanceId,
                "entity-instance:actor/player",
            ),
            (
                SceneValidationError::InvalidEntityInstance {
                    node: SceneNodeId::new(8),
                    reason: "invalid-instance-id".into(),
                },
                DiagnosticCode::InvalidSceneEntityInstance,
                "node:8",
            ),
            (
                SceneValidationError::DuplicateBootstrapNode {
                    node: SceneNodeId::new(9),
                },
                DiagnosticCode::DuplicateSceneBootstrap,
                "node:9",
            ),
            (
                SceneValidationError::InvalidBootstrap {
                    node: SceneNodeId::new(10),
                    reason: "bootstrap-must-be-root".into(),
                },
                DiagnosticCode::InvalidSceneBootstrap,
                "node:10",
            ),
            (
                SceneValidationError::DuplicateCatalogBinding {
                    node: SceneNodeId::new(11),
                    binding_id: "materials".into(),
                },
                DiagnosticCode::DuplicateSceneCatalogBinding,
                "catalog-binding:materials",
            ),
        ];

        for (error, expected_code, expected_reference) in cases {
            let report = map_scene_error(&error);
            assert_eq!(report.code, expected_code);
            assert_eq!(report.scope, protocol_diagnostics::DiagnosticScope::Scene);
            assert_eq!(report.reference, expected_reference);
            assert!(report.source.scene_node_id.is_some());
        }
    }
}
