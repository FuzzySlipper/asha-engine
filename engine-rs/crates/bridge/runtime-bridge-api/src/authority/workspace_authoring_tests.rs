use super::*;
use protocol_project_bundle::WorkspaceAuthoringOpenRequest;

use core_ids::ProjectId;
use core_space::VoxelCoord;
use core_voxel::VoxelValue;

fn open_request(workspace_id: &str) -> WorkspaceAuthoringOpenRequest {
    WorkspaceAuthoringOpenRequest {
        authoring_id: "workspace-authoring.rust-authority-test".to_owned(),
        seed: 29,
        project: WorkspaceAuthoringProjectIdentity {
            game_id: "authoring-consumer".to_owned(),
            workspace_id: workspace_id.to_owned(),
        },
        project_bundle: WorkspaceAuthoringProjectBundleRef {
            bundle_schema_version: 2,
            protocol_version: 1,
            scene_id: 42,
        },
    }
}

fn initialize_volume(bridge: &mut EngineBridge) {
    let stored_fixture = tests::hand_authored_voxel_volume_asset();
    let receipt = bridge
        .initialize_voxel_volume_authoring(VoxelVolumeAuthoringInitializeRequest {
            grid: 2,
            volume_asset_id: Some("voxel-volume/workspace-authoring".to_owned()),
            seed_chunk: VoxelAssetCoord { x: 0, y: 0, z: 0 },
            material_palette: stored_fixture.material_palette,
            authoring: stored_fixture.authoring,
            max_material_bindings: 8,
        })
        .unwrap();
    assert!(receipt.initialized, "{:?}", receipt.diagnostics);
}

fn projection_request(
    workspace_id: &str,
    generation: u64,
    working_revision: u64,
    cursor: u64,
) -> WorkspaceAuthoringProjectionRequest {
    WorkspaceAuthoringProjectionRequest {
        expected_workspace_id: workspace_id.to_owned(),
        expected_generation: generation,
        expected_working_revision: working_revision,
        cursor,
    }
}

fn procedural_scene(seed: u64) -> core_scene::FlatSceneDocument {
    core_scene::FlatSceneDocument {
        schema_version: 4,
        id: SceneId::new(42),
        metadata: core_scene::SceneMetadata {
            name: Some("Procedural authoring".to_owned()),
            authoring_format_version: 4,
        },
        dependencies: Vec::new(),
        nodes: vec![core_scene::SceneNodeRecord {
            id: SceneNodeId::new(1),
            parent: None,
            child_order: 0,
            transform: core_scene::SceneTransform::IDENTITY,
            kind: core_scene::SceneNodeKind::Bootstrap(core_scene::SceneBootstrapBindings {
                generator: Some(core_scene::SceneGeneratorBinding {
                    provider_id: svc_levelgen::TUNNEL_GENERATOR_ID.to_owned(),
                    preset_id: "tiny-enclosed".to_owned(),
                    seed,
                }),
                catalogs: Vec::new(),
            }),
            metadata: core_scene::NodeMetadata::default(),
        }],
    }
}

fn procedural_request(
    workspace_id: &str,
    generation: u64,
    scene_hash: String,
) -> ProceduralEnvironmentPreviewRequestDto {
    ProceduralEnvironmentPreviewRequestDto {
        expected_workspace_id: workspace_id.to_owned(),
        expected_generation: generation,
        expected_working_revision: 0,
        expected_scene_content_hash: scene_hash,
        provider_id: svc_levelgen::TUNNEL_GENERATOR_ID.to_owned(),
        preset_id: "tiny-enclosed".to_owned(),
        seed: 42,
        target: ProceduralEnvironmentTargetDto {
            scene_id: SceneId::new(42),
            scene_path: "scenes/generated-tunnel.scene.json".to_owned(),
            asset_id: "voxel-volume/generated-tunnel".to_owned(),
            asset_path: "assets/generated-tunnel.avxl.json".to_owned(),
            voxel_node_id: SceneNodeId::new(10),
            voxel_parent_id: None,
            voxel_child_order: 1,
            voxel_label: Some("Generated tunnel".to_owned()),
            voxel_transform: SceneTransformDto {
                translation: [-3.5, -1.0, -5.5],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            },
            marker_targets: vec![
                ProceduralEnvironmentMarkerTargetDto {
                    source_marker_id: "player_start".to_owned(),
                    node_id: SceneNodeId::new(11),
                    marker_id: "spawn/player".to_owned(),
                    child_order: 0,
                },
                ProceduralEnvironmentMarkerTargetDto {
                    source_marker_id: "exit_hint".to_owned(),
                    node_id: SceneNodeId::new(12),
                    marker_id: "navigation/exit".to_owned(),
                    child_order: 1,
                },
            ],
        },
        material_palette: [1u16, 2, 3]
            .into_iter()
            .map(|material| VoxelAssetMaterialBinding {
                voxel_material: material,
                palette_entry_id: format!("voxel-material/tunnel-{material}"),
                display_name: Some(format!("Tunnel {material}")),
                material_asset_id: format!("material/tunnel-{material}"),
                material_catalog_binding_id: Some(format!("catalog-binding/tunnel-{material}")),
            })
            .collect(),
        authoring: VoxelAssetAuthoringMetadata {
            label: Some("Generated tunnel".to_owned()),
            created_by: Some("runtime-bridge-api-test".to_owned()),
            source_tool: Some("workspace-authoring".to_owned()),
        },
        limits: ProceduralEnvironmentLimitsDto {
            max_voxels: 10_000,
            max_sparse_runs: 10_000,
            max_markers: 8,
        },
    }
}

#[test]
fn authoring_cell_is_distinct_from_gameplay_runtime_and_owns_revisions() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring_adapter(open_request("workspace.local"))
        .unwrap();

    assert_eq!(opened.status, "open");
    assert_eq!(opened.identity.generation, 1);
    assert!(bridge.runtime_project.engine.is_none());
    assert_eq!(bridge.time.authority_tick, 0);
    assert_eq!(
        bridge
            .step_simulation(StepInputEnvelope { tick: 1 })
            .unwrap_err()
            .kind,
        RuntimeBridgeErrorKind::NotInitialized
    );

    initialize_volume(&mut bridge);
    let after_initialize = bridge.read_workspace_authoring_state().unwrap();
    assert_eq!(after_initialize.working_revision, 1);
    assert_eq!(after_initialize.stored_revision, 0);
    assert!(after_initialize.dirty);

    let edit = bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(2),
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    assert_eq!(edit.accepted, 1);
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        2
    );

    let rejected_close = bridge
        .close_workspace_authoring(WorkspaceAuthoringCloseRequest {
            expected_workspace_id: "workspace.local".to_owned(),
            expected_generation: 1,
            discard_unsaved_working_state: false,
        })
        .unwrap_err();
    assert_eq!(rejected_close.kind, RuntimeBridgeErrorKind::InvalidInput);
    let closed = bridge
        .close_workspace_authoring(WorkspaceAuthoringCloseRequest {
            expected_workspace_id: "workspace.local".to_owned(),
            expected_generation: 1,
            discard_unsaved_working_state: true,
        })
        .unwrap();
    assert!(closed.closed);

    let reopened = bridge
        .open_workspace_authoring_adapter(open_request("workspace.local"))
        .unwrap();
    assert_eq!(reopened.identity.generation, 2);
    assert_eq!(reopened.working_revision, 0);
}

#[test]
fn projection_rejects_foreign_stale_and_future_bindings_before_drain() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring_adapter(open_request("workspace.local"))
        .unwrap();
    initialize_volume(&mut bridge);

    let first_request = projection_request("workspace.local", opened.identity.generation, 1, 0);
    let first = bridge
        .read_workspace_authoring_projection(first_request.clone())
        .unwrap();
    assert_eq!(first.delivery, "replace");
    assert_eq!(first.next_cursor, 1);
    assert_eq!(
        bridge
            .read_workspace_authoring_projection(first_request)
            .unwrap(),
        first,
        "an exact retry returns the cached receipt"
    );

    let edit = bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(2),
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    assert_eq!(edit.accepted, 1);

    for invalid in [
        projection_request("workspace.foreign", opened.identity.generation, 2, 1),
        projection_request("workspace.local", opened.identity.generation + 1, 2, 1),
        projection_request("workspace.local", opened.identity.generation, 1, 1),
        projection_request("workspace.local", opened.identity.generation, 2, 99),
    ] {
        assert_eq!(
            bridge
                .read_workspace_authoring_projection(invalid)
                .unwrap_err()
                .kind,
            RuntimeBridgeErrorKind::StaleAuthoritySnapshot
        );
    }

    let current_request = projection_request("workspace.local", opened.identity.generation, 2, 1);
    let current = bridge
        .read_workspace_authoring_projection(current_request.clone())
        .unwrap();
    assert_eq!(current.cursor, 1);
    assert_eq!(current.next_cursor, 2);
    assert!(
        current.frame_json.contains("replaceMeshPayload"),
        "rejected reads must not drain the pending edited geometry"
    );
    assert_eq!(
        bridge
            .read_workspace_authoring_projection(current_request)
            .unwrap(),
        current,
        "an accepted cursor retry remains idempotent"
    );
}

#[test]
fn stored_confirmation_consumes_only_the_current_rust_save_candidate() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring_adapter(open_request("workspace.local"))
        .unwrap();
    initialize_volume(&mut bridge);
    bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(2),
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();

    let save_candidate = |bridge: &mut EngineBridge| {
        bridge
            .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
                export_request: VoxelVolumeAssetExportRequest {
                    grid: 2,
                    volume_asset_id: Some("voxel-volume/workspace-authoring".to_owned()),
                    target_asset_id: "voxel-volume/workspace-confirmation".to_owned(),
                    label: Some("Workspace confirmation".to_owned()),
                    created_by: Some("runtime-bridge-api-test".to_owned()),
                    source_tool: Some("workspace-authoring".to_owned()),
                    max_sparse_runs: 16,
                    expected_session_hash: None,
                },
                target_project_bundle: "authoring-consumer".to_owned(),
                target_asset_path: "assets/voxels/workspace-confirmation.avxl.json".to_owned(),
                representation_kind: "sparse_runs".to_owned(),
                expected_existing_canonical_json_hash: None,
                expected_canonical_json_hash: None,
                expected_voxel_data_hash: None,
            })
            .unwrap()
            .canonical_json_hash
            .expect("accepted save candidate has a canonical hash")
    };

    let first_hash = save_candidate(&mut bridge);
    bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(2),
                coord: VoxelCoord::new(1, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    let stale = bridge
        .confirm_workspace_authoring_stored(WorkspaceAuthoringStoredConfirmationRequest {
            expected_workspace_id: "workspace.local".to_owned(),
            expected_generation: opened.identity.generation,
            host_path: "/tmp/workspace-confirmation.avxl.json".to_owned(),
            canonical_json_hash: first_hash,
        })
        .unwrap_err();
    assert_eq!(stale.kind, RuntimeBridgeErrorKind::InvalidInput);

    let current_hash = save_candidate(&mut bridge);
    let wrong_hash = bridge
        .confirm_workspace_authoring_stored(WorkspaceAuthoringStoredConfirmationRequest {
            expected_workspace_id: "workspace.local".to_owned(),
            expected_generation: opened.identity.generation,
            host_path: "/tmp/workspace-confirmation.avxl.json".to_owned(),
            canonical_json_hash: "fnv1a64:0000000000000000".to_owned(),
        })
        .unwrap_err();
    assert_eq!(
        wrong_hash.kind,
        RuntimeBridgeErrorKind::StaleAuthoritySnapshot
    );

    let request = WorkspaceAuthoringStoredConfirmationRequest {
        expected_workspace_id: "workspace.local".to_owned(),
        expected_generation: opened.identity.generation,
        host_path: "/tmp/workspace-confirmation.avxl.json".to_owned(),
        canonical_json_hash: current_hash,
    };
    let accepted = bridge
        .confirm_workspace_authoring_stored(request.clone())
        .unwrap();
    assert!(accepted.accepted);
    assert_eq!(accepted.stored_revision, 3);
    assert!(!bridge.read_workspace_authoring_state().unwrap().dirty);

    let replayed = bridge
        .confirm_workspace_authoring_stored(request)
        .unwrap_err();
    assert_eq!(replayed.kind, RuntimeBridgeErrorKind::InvalidInput);
    assert!(!bridge.read_workspace_authoring_state().unwrap().dirty);
}

#[test]
fn project_content_authoring_is_revision_bound_and_promotes_only_the_rust_candidate() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring_adapter(open_request("workspace.project-content"))
        .unwrap();
    let source = ProjectContentSourceDto {
        source_path: "entities/fixture.json".to_owned(),
        document_id: "fixture.entity.document".to_owned(),
        kind: ProjectContentDocumentKind::EntityDefinition,
        source_text: r#"{
            "kind":"EntityDefinition",
            "stableId":"fixture.entity",
            "displayName":"Fixture Entity",
            "source":{"projectBundle":"fixture","relativePath":"entities/fixture.json"},
            "tags":[],"metadata":[],"capabilities":[]
        }"#
        .to_owned(),
    };
    let decoded = bridge
        .decode_project_content(ProjectContentDecodeRequestDto {
            sources: vec![source.clone()],
        })
        .unwrap();
    assert!(decoded.accepted, "{:?}", decoded.diagnostics);
    let expected_set_hash = decoded.set_hash.clone().unwrap();
    let mut changed = decoded.documents[0].clone();
    let ProjectContentDocumentDto::EntityDefinition { definition, .. } = &mut changed else {
        panic!("fixture decoded as wrong document kind");
    };
    definition.display_name = "Changed Fixture Entity".to_owned();

    let substituted_subset = bridge
        .encode_project_content(ProjectContentEncodeRequestDto {
            documents: Vec::new(),
        })
        .unwrap();
    assert!(substituted_subset.accepted);
    let subset_hash = substituted_subset.set_hash.unwrap();
    let rejected_subset = bridge
        .apply_project_content_authoring(ProjectContentAuthoringRequestDto {
            expected_workspace_id: "workspace.project-content".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 0,
            expected_set_hash: subset_hash,
            command: ProjectContentAuthoringCommandDto::Upsert {
                source_path: "entities/fixture.json".to_owned(),
                document: changed.clone(),
            },
        })
        .unwrap();
    assert!(!rejected_subset.accepted);
    assert_eq!(
        rejected_subset.diagnostics[0].code,
        ProjectContentDiagnosticCode::StaleRevision
    );
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        0
    );

    let accepted_scene = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../harness/fixtures/scenes/sample-flat.json"
            ))
            .to_owned(),
        })
        .unwrap();
    assert!(accepted_scene.accepted);
    let rejected_stale_references = bridge
        .apply_project_content_authoring(ProjectContentAuthoringRequestDto {
            expected_workspace_id: "workspace.project-content".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 0,
            expected_set_hash: expected_set_hash.clone(),
            command: ProjectContentAuthoringCommandDto::Upsert {
                source_path: "entities/fixture.json".to_owned(),
                document: changed.clone(),
            },
        })
        .unwrap();
    assert!(!rejected_stale_references.accepted);
    assert_eq!(
        rejected_stale_references.diagnostics[0].code,
        ProjectContentDiagnosticCode::StaleRevision
    );

    let refreshed = bridge
        .decode_project_content(ProjectContentDecodeRequestDto {
            sources: vec![source],
        })
        .unwrap();
    assert!(refreshed.accepted, "{:?}", refreshed.diagnostics);
    assert_eq!(
        refreshed.set_hash.as_deref(),
        Some(expected_set_hash.as_str())
    );

    let accepted = bridge
        .apply_project_content_authoring(ProjectContentAuthoringRequestDto {
            expected_workspace_id: "workspace.project-content".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 0,
            expected_set_hash,
            command: ProjectContentAuthoringCommandDto::Upsert {
                source_path: "entities/fixture.json".to_owned(),
                document: changed,
            },
        })
        .unwrap();
    assert!(accepted.accepted, "{:?}", accepted.diagnostics);
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        1
    );
    let candidate_hash = accepted.set_hash.clone().unwrap();

    let stale = bridge
        .apply_project_content_authoring(ProjectContentAuthoringRequestDto {
            expected_workspace_id: "workspace.project-content".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 0,
            expected_set_hash: candidate_hash.clone(),
            command: ProjectContentAuthoringCommandDto::Delete {
                document_id: "entities/fixture.json".to_owned(),
                document_kind: ProjectContentDocumentKind::EntityDefinition,
            },
        })
        .unwrap_err();
    assert_eq!(stale.kind, RuntimeBridgeErrorKind::StaleAuthoritySnapshot);
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        1
    );

    let stored = bridge
        .confirm_workspace_authoring_stored(WorkspaceAuthoringStoredConfirmationRequest {
            expected_workspace_id: "workspace.project-content".to_owned(),
            expected_generation: opened.identity.generation,
            host_path: "/tmp/entities/fixture.json".to_owned(),
            canonical_json_hash: candidate_hash,
        })
        .unwrap();
    assert_eq!(stored.stored_revision, 1);
    assert!(stored.accepted);
    assert!(!bridge.read_workspace_authoring_state().unwrap().dirty);
}

#[test]
fn project_content_authoring_rejects_a_source_path_owned_by_another_document() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring_adapter(open_request("workspace.project-content-paths"))
        .unwrap();
    let presentation_source = |source_path: &str, document_id: &str| ProjectContentSourceDto {
        source_path: source_path.to_owned(),
        document_id: document_id.to_owned(),
        kind: ProjectContentDocumentKind::PresentationCatalog,
        source_text: r#"{"schemaVersion":1,"resources":[],"cues":[]}"#.to_owned(),
    };
    let decoded = bridge
        .decode_project_content(ProjectContentDecodeRequestDto {
            sources: vec![
                presentation_source("presentation/first.json", "presentation.first"),
                presentation_source("presentation/second.json", "presentation.second"),
            ],
        })
        .unwrap();
    assert!(decoded.accepted, "{:?}", decoded.diagnostics);
    let original_set_hash = decoded.set_hash.clone().expect("content set hash");
    let first_document = decoded
        .documents
        .iter()
        .find(|document| document.document_id() == "presentation.first")
        .cloned()
        .expect("first presentation document");

    let rejected = bridge
        .apply_project_content_authoring(ProjectContentAuthoringRequestDto {
            expected_workspace_id: "workspace.project-content-paths".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 0,
            expected_set_hash: original_set_hash.clone(),
            command: ProjectContentAuthoringCommandDto::Upsert {
                source_path: "presentation/second.json".to_owned(),
                document: first_document.clone(),
            },
        })
        .unwrap();
    assert!(!rejected.accepted);
    assert_eq!(
        rejected.diagnostics[0].code,
        ProjectContentDiagnosticCode::DuplicateDocument
    );
    assert_eq!(
        rejected.set_hash.as_deref(),
        Some(original_set_hash.as_str())
    );
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        0
    );

    let accepted = bridge
        .apply_project_content_authoring(ProjectContentAuthoringRequestDto {
            expected_workspace_id: "workspace.project-content-paths".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 0,
            expected_set_hash: original_set_hash,
            command: ProjectContentAuthoringCommandDto::Upsert {
                source_path: "presentation/first.json".to_owned(),
                document: first_document,
            },
        })
        .unwrap();
    assert!(accepted.accepted, "{:?}", accepted.diagnostics);
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        1
    );
}

#[test]
fn project_write_is_rust_derived_revision_bound_and_single_use() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring_adapter(open_request("workspace.project-write"))
        .unwrap();
    let scene_bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../harness/fixtures/scenes/sample-flat.json"
    ));
    let decoded_scene = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: String::from_utf8(scene_bytes.to_vec()).unwrap(),
        })
        .unwrap();
    assert!(decoded_scene.accepted, "{:?}", decoded_scene.diagnostics);
    let content_source = r#"{
        "kind":"EntityDefinition",
        "stableId":"fixture.entity",
        "displayName":"Fixture Entity",
        "source":{"projectBundle":"fixture","relativePath":"entities/fixture.json"},
        "tags":[],"metadata":[],"capabilities":[]
    }"#;
    let decoded_content = bridge
        .decode_project_content(ProjectContentDecodeRequestDto {
            sources: vec![ProjectContentSourceDto {
                source_path: "entities/fixture.json".to_owned(),
                document_id: "fixture.entity.document".to_owned(),
                kind: ProjectContentDocumentKind::EntityDefinition,
                source_text: content_source.to_owned(),
            }],
        })
        .unwrap();
    assert!(
        decoded_content.accepted,
        "{:?}",
        decoded_content.diagnostics
    );
    let canonical_content = decoded_content.canonical_files[0].canonical_json.clone();
    let authored_content = bridge
        .apply_project_content_authoring(ProjectContentAuthoringRequestDto {
            expected_workspace_id: "workspace.project-write".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 0,
            expected_set_hash: decoded_content.set_hash.clone().expect("content set hash"),
            command: ProjectContentAuthoringCommandDto::Upsert {
                source_path: "content/moved-fixture.json".to_owned(),
                document: decoded_content.documents[0].clone(),
            },
        })
        .unwrap();
    assert!(
        authored_content.accepted,
        "{:?}",
        authored_content.diagnostics
    );
    assert_eq!(
        authored_content.canonical_files[0].source_path.as_deref(),
        Some("content/moved-fixture.json")
    );

    let asset_lock = br#"{"assets":[]}"#;
    let prior = svc_serialization::ProjectBundleManifest {
        bundle_schema_version: svc_serialization::BUNDLE_SCHEMA_VERSION,
        protocol_version: svc_serialization::SUPPORTED_PROTOCOL_VERSION,
        project: svc_serialization::ProjectSection {
            id: ProjectId::new(9),
            name: Some("Project write authority".to_owned()),
        },
        entry_scene: SceneId::new(100),
        scenes: vec![svc_serialization::SceneSection {
            id: SceneId::new(100),
            schema_version: 1,
            artifact: "scenes/sample-flat.json".to_owned(),
        }],
        asset_lock: svc_serialization::AssetLockSection {
            artifact: "assets/lock.json".to_owned(),
            asset_count: 0,
        },
        generation_provenance: None,
        artifacts: vec![
            svc_serialization::ArtifactEntry::durable(
                "assets/lock.json",
                svc_serialization::ArtifactRole::AssetLock,
                asset_lock,
            ),
            svc_serialization::ArtifactEntry::durable(
                "entities/fixture.json",
                svc_serialization::ArtifactRole::ProjectContent,
                canonical_content.as_bytes(),
            ),
            svc_serialization::ArtifactEntry::durable(
                "scenes/sample-flat.json",
                svc_serialization::ArtifactRole::SceneDocument,
                scene_bytes,
            ),
        ],
    }
    .canonical();
    let observed =
        svc_serialization::ProjectStoreIdentity::from_manifest(12, &prior, None).unwrap();
    let request = ProjectWritePrepareRequest {
        expected_workspace_id: "workspace.project-write".to_owned(),
        expected_generation: opened.identity.generation,
        expected_working_revision: 1,
        observed_prior: ProjectStoreIdentity {
            revision: observed.revision,
            manifest_hash: observed.manifest_hash.to_hex(),
            content_set_hash: observed.content_set_hash.to_hex(),
            index_hash: None,
        },
        prior_manifest_json: svc_serialization::encode(&prior),
        relocations: vec![ProjectArtifactRelocation {
            from: "scenes/sample-flat.json".to_owned(),
            to: "scenes/archive/sample-flat.json".to_owned(),
        }],
    };
    let prepared = bridge.prepare_project_write(request).unwrap();
    assert!(prepared.accepted, "{:?}", prepared.diagnostics);
    let candidate = prepared.candidate.unwrap();
    assert_eq!(candidate.expected_prior.revision, 12);
    assert!(candidate
        .expected_next_artifacts
        .iter()
        .any(|artifact| artifact.path == "scenes/archive/sample-flat.json"));
    assert!(candidate
        .expected_next_artifacts
        .iter()
        .any(|artifact| artifact.path == "content/moved-fixture.json"));
    assert!(candidate
        .writes
        .iter()
        .any(|write| write.path == "content/moved-fixture.json"));
    assert!(candidate
        .deletes
        .iter()
        .any(|deletion| deletion.path == "entities/fixture.json"));
    assert!(
        candidate.moves.iter().any(|movement| {
            movement.from == "scenes/sample-flat.json"
                && movement.to == "scenes/archive/sample-flat.json"
        }) || candidate
            .deletes
            .iter()
            .any(|artifact| artifact.path == "scenes/sample-flat.json")
    );
    for write in &candidate.writes {
        let view = bridge
            .get_buffer(RuntimeBufferHandle::new(write.resource.handle))
            .unwrap();
        assert_eq!(view.bytes.len() as u64, write.resource.byte_len);
        assert!(!view.bytes.is_empty());
    }

    let wrong = bridge
        .confirm_project_write(ProjectWriteConfirmRequest {
            expected_workspace_id: "workspace.project-write".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 1,
            publication: ProjectWritePublication {
                candidate_hash: "0000000000000000".to_owned(),
                published: candidate.expected_next.clone(),
            },
        })
        .unwrap();
    assert!(!wrong.accepted);
    assert_eq!(wrong.diagnostics[0].code, "staleCandidate");

    let confirmation = ProjectWriteConfirmRequest {
        expected_workspace_id: "workspace.project-write".to_owned(),
        expected_generation: opened.identity.generation,
        expected_working_revision: 1,
        publication: ProjectWritePublication {
            candidate_hash: candidate.candidate_hash.clone(),
            published: candidate.expected_next.clone(),
        },
    };
    let stored = bridge.confirm_project_write(confirmation.clone()).unwrap();
    assert!(stored.accepted, "{:?}", stored.diagnostics);
    assert_eq!(stored.stored, Some(candidate.expected_next));
    assert!(!bridge.read_workspace_authoring_state().unwrap().dirty);

    let replay = bridge.confirm_project_write(confirmation).unwrap();
    assert!(!replay.accepted);
    assert_eq!(replay.diagnostics[0].code, "missingCandidate");
}

#[test]
fn procedural_materialization_is_preview_pure_candidate_bound_and_combined_saveable() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring_adapter(open_request("workspace.procedural"))
        .unwrap();
    let decoded = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: core_scene::encode(&procedural_scene(42)),
        })
        .unwrap();
    assert!(decoded.accepted, "{:?}", decoded.diagnostics);
    let scene_hash = decoded.content_hash.expect("scene hash");

    let preview = bridge
        .preview_procedural_environment(procedural_request(
            "workspace.procedural",
            opened.identity.generation,
            scene_hash,
        ))
        .unwrap();
    assert!(preview.accepted, "{:?}", preview.diagnostics);
    assert!(preview.preview_diff_count > 0);
    assert!(
        bridge.voxel.voxel.is_none(),
        "preview must not install authority"
    );
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        0
    );
    let candidate = preview.candidate.expect("accepted candidate");
    assert_eq!(candidate.asset.grid.origin, [0.0; 3]);
    assert_eq!(candidate.markers.len(), 2);
    assert!(candidate.scene.nodes.iter().any(|record| {
        record.id == SceneNodeId::new(10)
            && record.transform.translation == [-3.5, -1.0, -5.5]
            && matches!(record.kind, SceneNodeKindDto::VoxelVolume(_))
    }));
    let candidate_hash = candidate.candidate_hash.clone();

    let wrong = bridge
        .apply_procedural_environment(ProceduralEnvironmentApplyRequestDto {
            expected_workspace_id: "workspace.procedural".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 0,
            candidate_hash: "fnv1a64:wrong".to_owned(),
        })
        .unwrap();
    assert!(!wrong.accepted);
    assert!(bridge.voxel.voxel.is_none());
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        0
    );

    let applied = bridge
        .apply_procedural_environment(ProceduralEnvironmentApplyRequestDto {
            expected_workspace_id: "workspace.procedural".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 0,
            candidate_hash: candidate_hash.clone(),
        })
        .unwrap();
    assert!(applied.accepted, "{:?}", applied.diagnostics);
    assert_eq!(applied.working_revision, 1);
    assert!(bridge.voxel.voxel.is_some());
    let save_hash = applied
        .save_candidate_hash
        .clone()
        .expect("combined save hash");
    assert_eq!(save_hash, candidate.artifact_set_hash);

    let replay = bridge
        .apply_procedural_environment(ProceduralEnvironmentApplyRequestDto {
            expected_workspace_id: "workspace.procedural".to_owned(),
            expected_generation: opened.identity.generation,
            expected_working_revision: 1,
            candidate_hash,
        })
        .unwrap();
    assert!(!replay.accepted);
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        1
    );

    let stored = bridge
        .confirm_workspace_authoring_stored(WorkspaceAuthoringStoredConfirmationRequest {
            expected_workspace_id: "workspace.procedural".to_owned(),
            expected_generation: opened.identity.generation,
            host_path: "/tmp/generated-tunnel.artifact-set".to_owned(),
            canonical_json_hash: save_hash,
        })
        .unwrap();
    assert_eq!(stored.stored_revision, 1);
    assert!(!bridge.read_workspace_authoring_state().unwrap().dirty);
}

#[test]
fn reopened_procedural_environment_replaces_from_loaded_asset_provenance() {
    let mut source_bridge = EngineBridge::new();
    let source_opened = source_bridge
        .open_workspace_authoring_adapter(open_request("workspace.procedural-source"))
        .unwrap();
    let source_scene = source_bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: core_scene::encode(&procedural_scene(42)),
        })
        .unwrap();
    let source_preview = source_bridge
        .preview_procedural_environment(procedural_request(
            "workspace.procedural-source",
            source_opened.identity.generation,
            source_scene.content_hash.unwrap(),
        ))
        .unwrap();
    let stored = source_preview.candidate.expect("source candidate");

    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring_adapter(open_request("workspace.procedural-reopen"))
        .unwrap();
    let decoded = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: stored.scene_file.canonical_json,
        })
        .unwrap();
    assert!(decoded.accepted, "{:?}", decoded.diagnostics);
    let scene_hash = decoded.content_hash.expect("stored scene hash");
    let loaded = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: stored.asset,
            target_grid: 1,
            target_volume_asset_id: Some("voxel/generated-tunnel".to_owned()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(loaded.loaded, "{:?}", loaded.diagnostics);
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        1
    );

    // An unrelated scene/content mutation advances the same authoring cell but
    // does not unload the canonical asset that was explicitly loaded into it.
    bridge.record_workspace_authoring_mutation();
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        2
    );

    let mut changed_seed = procedural_request(
        "workspace.procedural-reopen",
        opened.identity.generation,
        scene_hash.clone(),
    );
    changed_seed.expected_working_revision = 2;
    changed_seed.seed = 43;
    let changed = bridge.preview_procedural_environment(changed_seed).unwrap();
    assert!(changed.accepted, "{:?}", changed.diagnostics);
    assert_eq!(
        changed
            .candidate
            .expect("changed candidate")
            .provenance
            .seed,
        43
    );

    let mut replacement = procedural_request(
        "workspace.procedural-reopen",
        opened.identity.generation,
        scene_hash,
    );
    replacement.expected_working_revision = 2;
    let preview = bridge.preview_procedural_environment(replacement).unwrap();
    assert!(preview.accepted, "{:?}", preview.diagnostics);
    let scene = preview.candidate.expect("replacement candidate").scene;
    assert_eq!(
        scene
            .nodes
            .iter()
            .filter(|record| matches!(record.kind, SceneNodeKindDto::VoxelVolume(_)))
            .count(),
        1
    );
}

#[test]
fn procedural_materialization_rejects_stale_unresolved_and_oversized_without_mutation() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring_adapter(open_request("workspace.procedural-rejection"))
        .unwrap();
    let decoded = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: core_scene::encode(&procedural_scene(42)),
        })
        .unwrap();
    let scene_hash = decoded.content_hash.unwrap();

    let mut stale = procedural_request(
        "workspace.procedural-rejection",
        opened.identity.generation,
        "fnv1a64:stale".to_owned(),
    );
    let stale_result = bridge
        .preview_procedural_environment(stale.clone())
        .unwrap();
    assert!(!stale_result.accepted);
    assert_eq!(
        stale_result.diagnostics[0].code,
        ProceduralEnvironmentDiagnosticCode::StaleScene
    );

    stale.expected_scene_content_hash = scene_hash.clone();
    stale.provider_id = "unknown.provider".to_owned();
    let unresolved = bridge.preview_procedural_environment(stale).unwrap();
    assert!(!unresolved.accepted);
    assert!(unresolved
        .diagnostics
        .iter()
        .any(|entry| entry.code == ProceduralEnvironmentDiagnosticCode::UnknownProvider));

    let mut oversized = procedural_request(
        "workspace.procedural-rejection",
        opened.identity.generation,
        scene_hash,
    );
    oversized.limits.max_voxels = 1;
    let bounded = bridge.preview_procedural_environment(oversized).unwrap();
    assert!(!bounded.accepted);
    assert!(bounded
        .diagnostics
        .iter()
        .any(|entry| entry.code == ProceduralEnvironmentDiagnosticCode::LimitExceeded));
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        0
    );
    assert!(bridge.voxel.voxel.is_none());
}
