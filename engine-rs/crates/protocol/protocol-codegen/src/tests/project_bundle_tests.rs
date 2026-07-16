use super::*;

pub(super) fn extend_round_trip_coverage(coverage: &mut BTreeSet<String>) {
    coverage.extend([
        interface_coverage_key("projectBundle", "GameplayTriggerDefinition"),
        interface_coverage_key("projectBundle", "WorkspaceAuthoringProjectIdentity"),
        interface_coverage_key("projectBundle", "WorkspaceAuthoringProjectBundleRef"),
        interface_coverage_key("projectBundle", "WorkspaceAuthoringCompositionStatus"),
        interface_coverage_key("projectBundle", "WorkspaceAuthoringOpenRequest"),
        interface_coverage_key("projectBundle", "WorkspaceAuthoringIdentity"),
        interface_coverage_key("projectBundle", "WorkspaceAuthoringStateSummary"),
        interface_coverage_key("projectBundle", "WorkspaceAuthoringProjectionRequest"),
        interface_coverage_key("projectBundle", "WorkspaceAuthoringProjectionReceipt"),
        interface_coverage_key(
            "projectBundle",
            "WorkspaceAuthoringStoredConfirmationRequest",
        ),
        interface_coverage_key(
            "projectBundle",
            "WorkspaceAuthoringStoredConfirmationReceipt",
        ),
        interface_coverage_key("projectBundle", "WorkspaceAuthoringCloseRequest"),
        interface_coverage_key("projectBundle", "WorkspaceAuthoringCloseReceipt"),
    ]);
}

/// ProjectBundle vocabularies and prefab contracts are sourced from Rust and
/// retain the established scene/voxel imports.
#[test]
fn project_bundle_family_emits_vocab_and_shapes() {
    let output = file("projectBundle.ts");
    for class in protocol_project_bundle::ARTIFACT_CLASSES {
        assert!(
            output.contains(&format!("'{class}'")),
            "missing artifact class {class}"
        );
    }
    for stage in protocol_project_bundle::LOAD_STAGES {
        assert!(
            output.contains(&format!("'{stage}'")),
            "missing load stage {stage}"
        );
    }
    for action in protocol_project_bundle::SUGGESTED_ACTIONS {
        assert!(
            output.contains(&format!("'{action}'")),
            "missing suggested action {action}"
        );
    }
    for code in protocol_project_bundle::PREFAB_DIAGNOSTIC_CODES {
        assert!(
            output.contains(&format!("'{code}'")),
            "missing prefab diagnostic code {code}"
        );
    }
    for shape in [
        "ProjectBundleManifest",
        "PrefabDefinition",
        "PrefabRegistry",
        "PrefabPartReference",
        "LoadPlan",
        "SaveSummary",
        "RegenConflictReport",
    ] {
        assert!(
            output.contains(shape),
            "missing ProjectBundle shape {shape}"
        );
    }
    for brand in ["PrefabId", "PrefabPartId", "PrefabInstanceId"] {
        assert!(
            output.contains(&format!("export type {brand} =")),
            "missing brand {brand}"
        );
    }
    assert!(output.contains("'prefabRegistry'"));
    assert!(output.contains("field: 'material'"));
    assert!(output.contains("field: 'activation'"));
    assert!(
        output.contains("import type { ProjectId, RuntimeSessionId, SceneId } from './scene.js';")
    );
    assert!(output.contains("import type { VoxelCoord, VoxelValue } from './voxel.js';"));
}

#[test]
fn gameplay_trigger_definition_serialization_matches_ir_shape() {
    let trigger = protocol_project_bundle::GameplayTriggerDefinition {
        schema_version: protocol_project_bundle::GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
        entity: 10,
        scope: "zone.exit".to_owned(),
        tags: vec!["door".to_owned(), "exit".to_owned()],
    };
    let value = serde_json::to_value(&trigger).unwrap();
    assert_eq!(value["schemaVersion"], 1);
    assert_eq!(value["entity"], 10);
    assert_eq!(value["scope"], "zone.exit");
    assert_eq!(value["tags"], serde_json::json!(["door", "exit"]));
    assert_eq!(
        serde_json::from_value::<protocol_project_bundle::GameplayTriggerDefinition>(value)
            .unwrap(),
        trigger
    );
}
