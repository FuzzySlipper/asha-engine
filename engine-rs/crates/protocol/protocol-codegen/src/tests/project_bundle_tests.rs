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
        interface_coverage_key("projectBundle", "StagedProjectResourceRef"),
        interface_coverage_key("projectBundle", "ProjectResourceBeginRequest"),
        interface_coverage_key("projectBundle", "ProjectResourceTransactionReceipt"),
        interface_coverage_key("projectBundle", "ProjectResourceStageRequest"),
        variant_coverage_key("projectBundle", "ProjectSourceBody", "inline"),
        variant_coverage_key("projectBundle", "ProjectSourceBody", "resource"),
        interface_coverage_key("projectBundle", "RuntimeProjectSourceBatch"),
        interface_coverage_key("projectBundle", "ProjectSourceBatchDiagnostic"),
        interface_coverage_key("projectBundle", "ProjectSourceBatchValidationReceipt"),
        interface_coverage_key("projectBundle", "RuntimeProjectSourceAdapterInput"),
        interface_coverage_key("projectBundle", "RuntimeProjectLifecycleVersion"),
        interface_coverage_key("projectBundle", "RuntimeProjectLoadRequest"),
        interface_coverage_key("projectBundle", "RuntimeProjectDiagnostic"),
        interface_coverage_key("projectBundle", "ActiveRuntimeProjectIdentity"),
        interface_coverage_key("projectBundle", "RuntimeProjectLoadReceipt"),
        interface_coverage_key("projectBundle", "RuntimeProjectCloseRequest"),
        interface_coverage_key("projectBundle", "RuntimeProjectCloseReceipt"),
        interface_coverage_key("projectBundle", "ProjectStoreIdentity"),
        interface_coverage_key("projectBundle", "ProjectArtifactExpectation"),
        interface_coverage_key("projectBundle", "ProjectWriteResourceRef"),
        interface_coverage_key("projectBundle", "CanonicalProjectWrite"),
        interface_coverage_key("projectBundle", "CanonicalProjectMove"),
        interface_coverage_key("projectBundle", "CanonicalProjectDelete"),
        interface_coverage_key("projectBundle", "ProjectWriteCandidate"),
        interface_coverage_key("projectBundle", "ProjectWritePublication"),
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
        scene_instance_id: "instance.zone.exit".to_owned(),
        scope: "zone.exit".to_owned(),
        tags: vec!["door".to_owned(), "exit".to_owned()],
    };
    let value = serde_json::to_value(&trigger).unwrap();
    assert_eq!(value["schemaVersion"], 2);
    assert_eq!(value["sceneInstanceId"], "instance.zone.exit");
    assert_eq!(value["scope"], "zone.exit");
    assert_eq!(value["tags"], serde_json::json!(["door", "exit"]));
    assert_eq!(
        serde_json::from_value::<protocol_project_bundle::GameplayTriggerDefinition>(value)
            .unwrap(),
        trigger
    );
}

#[test]
fn project_source_batch_serialization_matches_ir_shape() {
    use protocol_project_bundle::{
        ProjectResourceBeginRequest, ProjectResourceStageRequest,
        ProjectResourceTransactionReceipt, ProjectSourceBatchDiagnostic,
        ProjectSourceBatchErrorCode, ProjectSourceBatchValidationReceipt, ProjectSourceBody,
        RuntimeProjectSourceBatch, StagedProjectResourceRef,
    };

    let project = module("projectBundle");
    let resource = StagedProjectResourceRef {
        handle: 4,
        generation: 2,
        version: 1,
        byte_len: 11,
    };
    let begin = ProjectResourceBeginRequest {
        manifest_json: "{\"bundleSchemaVersion\":2}".into(),
    };
    let transaction = ProjectResourceTransactionReceipt {
        generation: 2,
        manifest_hash: "0123456789abcdef".into(),
    };
    let stage = ProjectResourceStageRequest {
        generation: 2,
        path: "voxel/house.avox".into(),
        bytes: vec![1, 2, 3],
    };
    let inline = ProjectSourceBody::Inline {
        path: "scene/entry.json".into(),
        bytes: vec![123, 125],
    };
    let resource_body = ProjectSourceBody::Resource {
        path: "voxel/house.avox".into(),
        resource,
    };
    let batch = RuntimeProjectSourceBatch {
        manifest_json: begin.manifest_json.clone(),
        resource_generation: Some(2),
        bodies: vec![inline.clone(), resource_body.clone()],
    };
    let diagnostic = ProjectSourceBatchDiagnostic {
        code: ProjectSourceBatchErrorCode::ResourcePathMismatch,
        path: Some("voxel/house.avox".into()),
        message: "staged path mismatch".into(),
    };
    let receipt = ProjectSourceBatchValidationReceipt {
        accepted: false,
        manifest_hash: None,
        paths: Vec::new(),
        diagnostics: vec![diagnostic.clone()],
    };

    let samples = [
        (
            "StagedProjectResourceRef",
            serde_json::to_value(resource).unwrap(),
        ),
        (
            "ProjectResourceBeginRequest",
            serde_json::to_value(&begin).unwrap(),
        ),
        (
            "ProjectResourceTransactionReceipt",
            serde_json::to_value(&transaction).unwrap(),
        ),
        (
            "ProjectResourceStageRequest",
            serde_json::to_value(&stage).unwrap(),
        ),
        (
            "RuntimeProjectSourceBatch",
            serde_json::to_value(&batch).unwrap(),
        ),
        (
            "ProjectSourceBatchDiagnostic",
            serde_json::to_value(&diagnostic).unwrap(),
        ),
        (
            "ProjectSourceBatchValidationReceipt",
            serde_json::to_value(&receipt).unwrap(),
        ),
    ];
    for (name, value) in samples {
        compare_object_to_interface(&project, name, &value).unwrap();
    }
    for (tag, body) in [("inline", &inline), ("resource", &resource_body)] {
        let value = serde_json::to_value(body).unwrap();
        compare_object_to_variant(&project, "ProjectSourceBody", tag, &value).unwrap();
    }

    assert_eq!(
        serde_json::from_value::<ProjectResourceStageRequest>(
            serde_json::to_value(&stage).unwrap()
        )
        .unwrap(),
        stage
    );
    assert_eq!(
        serde_json::from_value::<RuntimeProjectSourceBatch>(serde_json::to_value(&batch).unwrap())
            .unwrap(),
        batch
    );
    assert_eq!(
        serde_json::from_value::<ProjectSourceBatchValidationReceipt>(
            serde_json::to_value(&receipt).unwrap(),
        )
        .unwrap(),
        receipt
    );
}

#[test]
fn runtime_project_public_facade_serialization_matches_ir_shape() {
    use protocol_project_bundle::{
        ActiveRuntimeProjectIdentity, RuntimeProjectCloseReceipt, RuntimeProjectCloseRequest,
        RuntimeProjectDiagnostic, RuntimeProjectDiagnosticPhase, RuntimeProjectLifecycleVersion,
        RuntimeProjectLoadReceipt, RuntimeProjectLoadRequest, RuntimeProjectSourceAdapterInput,
        RuntimeProjectSourceAdapterKind,
    };

    let project = module("projectBundle");
    let lifecycle = RuntimeProjectLifecycleVersion {
        generation: 3,
        revision: 7,
    };
    let source = RuntimeProjectSourceAdapterInput {
        kind: RuntimeProjectSourceAdapterKind::PackagedProject,
        identity: "package:/game.asha".into(),
        materialization_hash: "fnv1a64:0123456789abcdef".into(),
    };
    let diagnostic = RuntimeProjectDiagnostic {
        phase: RuntimeProjectDiagnosticPhase::RuntimeAdmission,
        code: "danglingReference".into(),
        document_id: Some("content/player".into()),
        path: Some("$.target".into()),
        message: "target does not resolve".into(),
    };
    let active = ActiveRuntimeProjectIdentity {
        project_id: 8,
        manifest_hash: "manifest".into(),
        admission_hash: "admission".into(),
        content_set_hash: "content".into(),
        composition_hash: "composition".into(),
        entry_scene_id: 9,
        scene_count: 2,
        entity_count: 11,
        voxel_asset_count: 1,
        lifecycle,
    };
    let request = RuntimeProjectLoadRequest {
        source: source.clone(),
        expected_lifecycle: RuntimeProjectLifecycleVersion::default(),
    };
    let receipt = RuntimeProjectLoadReceipt {
        accepted: true,
        source,
        active_project: Some(active),
        lifecycle,
        diagnostics: Vec::new(),
    };
    let close_request = RuntimeProjectCloseRequest {
        expected_lifecycle: lifecycle,
    };
    let close_receipt = RuntimeProjectCloseReceipt {
        accepted: false,
        closed_project_id: None,
        closed_manifest_hash: None,
        lifecycle,
        diagnostics: vec![diagnostic.clone()],
    };

    for (name, value) in [
        (
            "RuntimeProjectSourceAdapterInput",
            serde_json::to_value(&request.source).unwrap(),
        ),
        (
            "RuntimeProjectLifecycleVersion",
            serde_json::to_value(lifecycle).unwrap(),
        ),
        (
            "RuntimeProjectLoadRequest",
            serde_json::to_value(&request).unwrap(),
        ),
        (
            "RuntimeProjectDiagnostic",
            serde_json::to_value(&diagnostic).unwrap(),
        ),
        (
            "ActiveRuntimeProjectIdentity",
            serde_json::to_value(receipt.active_project.as_ref().unwrap()).unwrap(),
        ),
        (
            "RuntimeProjectLoadReceipt",
            serde_json::to_value(&receipt).unwrap(),
        ),
        (
            "RuntimeProjectCloseRequest",
            serde_json::to_value(close_request).unwrap(),
        ),
        (
            "RuntimeProjectCloseReceipt",
            serde_json::to_value(close_receipt).unwrap(),
        ),
    ] {
        compare_object_to_interface(&project, name, &value).unwrap();
    }
}

#[test]
fn project_write_candidate_serialization_matches_ir_shape() {
    use protocol_project_bundle::{
        CanonicalProjectDelete, CanonicalProjectMove, CanonicalProjectWrite,
        ProjectArtifactExpectation, ProjectStoreIdentity, ProjectWriteCandidate,
        ProjectWritePublication, ProjectWriteResourceRef,
    };

    let project = module("projectBundle");
    let prior = ProjectStoreIdentity {
        revision: 7,
        manifest_hash: "1111111111111111".into(),
        content_set_hash: "2222222222222222".into(),
        index_hash: None,
    };
    let next = ProjectStoreIdentity {
        revision: 8,
        manifest_hash: "3333333333333333".into(),
        content_set_hash: "4444444444444444".into(),
        index_hash: Some("5555555555555555".into()),
    };
    let expectation = ProjectArtifactExpectation {
        path: "scenes/main.json".into(),
        content_hash: Some("6666666666666666".into()),
    };
    let resource = ProjectWriteResourceRef {
        handle: 9,
        version: 1,
        byte_len: 12,
    };
    let write = CanonicalProjectWrite {
        path: "scenes/main.json".into(),
        content_hash: "7777777777777777".into(),
        resource,
    };
    let movement = CanonicalProjectMove {
        from: "scenes/old.json".into(),
        to: "scenes/archive/old.json".into(),
        expected_content_hash: Some("8888888888888888".into()),
    };
    let deletion = CanonicalProjectDelete {
        path: "scenes/removed.json".into(),
        expected_content_hash: Some("9999999999999999".into()),
    };
    let candidate = ProjectWriteCandidate {
        candidate_hash: "aaaaaaaaaaaaaaaa".into(),
        expected_prior: prior.clone(),
        expected_next: next.clone(),
        expected_prior_artifacts: vec![expectation.clone()],
        expected_next_artifacts: vec![expectation.clone()],
        manifest_json: "{\"bundleSchemaVersion\":2}".into(),
        writes: vec![write.clone()],
        moves: vec![movement.clone()],
        deletes: vec![deletion.clone()],
        index_replacement: Some(CanonicalProjectWrite {
            path: ".asha/project-index.json".into(),
            content_hash: "bbbbbbbbbbbbbbbb".into(),
            resource,
        }),
    };
    let publication = ProjectWritePublication {
        candidate_hash: candidate.candidate_hash.clone(),
        published: next,
    };
    for (name, value) in [
        (
            "ProjectStoreIdentity",
            serde_json::to_value(&prior).unwrap(),
        ),
        (
            "ProjectArtifactExpectation",
            serde_json::to_value(&expectation).unwrap(),
        ),
        (
            "ProjectWriteResourceRef",
            serde_json::to_value(resource).unwrap(),
        ),
        (
            "CanonicalProjectWrite",
            serde_json::to_value(&write).unwrap(),
        ),
        (
            "CanonicalProjectMove",
            serde_json::to_value(&movement).unwrap(),
        ),
        (
            "CanonicalProjectDelete",
            serde_json::to_value(&deletion).unwrap(),
        ),
        (
            "ProjectWriteCandidate",
            serde_json::to_value(&candidate).unwrap(),
        ),
        (
            "ProjectWritePublication",
            serde_json::to_value(&publication).unwrap(),
        ),
    ] {
        compare_object_to_interface(&project, name, &value).unwrap();
    }
    assert_eq!(
        serde_json::from_value::<ProjectWriteCandidate>(serde_json::to_value(&candidate).unwrap())
            .unwrap(),
        candidate
    );
}
