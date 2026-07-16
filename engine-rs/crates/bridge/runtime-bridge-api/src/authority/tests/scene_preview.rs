use super::*;
use core_ids::ProjectId;

fn material_preview_request() -> ModelMaterialPreviewRequest {
    ModelMaterialPreviewRequest {
        catalog_entry: protocol_assets::CatalogEntry {
            id: "material/copper".to_string(),
            kind: "material".to_string(),
            version: 1,
            hash: Some("sha256-material-copper".to_string()),
            source_path: None,
            label: Some("Copper".to_string()),
            dependencies: Vec::new(),
            material: Some(protocol_assets::MaterialProjection {
                render: protocol_assets::RenderMaterial {
                    color: protocol_assets::Rgba {
                        r: 0.8,
                        g: 0.4,
                        b: 0.2,
                        a: 1.0,
                    },
                    texture: None,
                    roughness: 0.6,
                    texture_tint: protocol_assets::Rgba {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                    emission_color: protocol_assets::Rgba {
                        r: 0.8,
                        g: 0.4,
                        b: 0.2,
                        a: 1.0,
                    },
                    emissive: 0.0,
                    uv_strategy: "flat".to_string(),
                },
                collision: protocol_assets::CollisionMaterial {
                    solid: true,
                    collidable: true,
                    occludes: true,
                    structural_class: "solid".to_string(),
                },
            }),
        },
        mesh_asset: StaticMeshAsset {
            asset: "mesh/preview-cube".to_string(),
            payload: MeshPayloadDescriptor {
                layout: MeshBufferLayout {
                    vertex_count: 3,
                    index_count: 3,
                    index_width: MeshIndexWidth::U32,
                    attributes: vec![MeshAttribute {
                        name: MeshAttributeName::Position,
                        components: 3,
                        kind: MeshAttributeKind::F32,
                    }],
                },
                groups: vec![MeshGroupDescriptor {
                    material_slot: 0,
                    start: 0,
                    count: 3,
                }],
                bounds: MeshBoundsDescriptor {
                    min: [0.0, 0.0, 0.0],
                    max: [1.0, 1.0, 0.0],
                },
                source: MeshPayloadSource::Inline {
                    positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                    normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
                    indices: vec![0, 1, 2],
                },
                provenance: MeshProvenance::StaticAsset,
            },
            material_slots: vec![MeshMaterialSlot {
                slot: 0,
                material: "material/copper".to_string(),
            }],
            collision: MeshCollisionPolicy::AabbFallback,
        },
        instance_handle: protocol_render::RenderHandle::new(7001),
    }
}

#[test]
fn model_material_preview_is_validated_and_projected_by_rust() {
    let bridge = init_bridge();
    let preview = bridge
        .read_model_material_preview(material_preview_request())
        .unwrap();
    assert_eq!(preview.renderer_classification, "runtime_readback");
    assert_eq!(preview.preview_diff.ops.len(), 3);
    assert!(matches!(
        preview.preview_diff.ops[0],
        protocol_render::RenderDiff::DefineMaterial { .. }
    ));
}

#[test]
fn scene_object_commands_are_hash_guarded_and_commit_canonical_state() {
    let mut bridge = init_bridge();
    let before = bridge.read_scene_object_snapshot().unwrap();
    let result = bridge
        .apply_scene_object_command(SceneObjectCommandRequestDto {
            expected_document_hash: before.document_hash,
            command: SceneObjectCommandDto::Rename {
                id: SceneNodeId::new(1),
                label: Some("Playable root".to_string()),
            },
        })
        .unwrap();
    assert!(result.accepted);
    let after = bridge.read_scene_object_snapshot().unwrap();
    assert_ne!(after.document_hash, before.document_hash);
    assert_eq!(after.objects[0].label.as_deref(), Some("Playable root"));

    let stale = bridge
        .apply_scene_object_command(SceneObjectCommandRequestDto {
            expected_document_hash: before.document_hash,
            command: SceneObjectCommandDto::Select { id: None },
        })
        .unwrap();
    assert!(!stale.accepted);
    assert_eq!(
        stale.rejection.unwrap().code,
        SceneObjectCommandRejectionCode::StaleSnapshot
    );
}

#[test]
fn stored_scene_codec_round_trips_the_canonical_golden_without_runtime_mutation() {
    let bridge = init_bridge();
    let before = bridge.read_scene_object_snapshot().unwrap();
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../harness/fixtures/scenes/sample-flat.json"
    ));

    let decoded = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: source.to_string(),
        })
        .unwrap();
    assert!(decoded.accepted);
    assert_eq!(decoded.canonical_json.as_deref(), Some(source));
    assert!(decoded
        .content_hash
        .as_deref()
        .is_some_and(|hash| hash.starts_with("fnv1a64:")));

    let mut document = decoded.document.unwrap();
    document.nodes.reverse();
    let encoded = bridge
        .encode_scene_document(SceneDocumentEncodeRequestDto { document })
        .unwrap();
    assert!(encoded.accepted);
    assert_eq!(encoded.canonical_json.as_deref(), Some(source));
    assert_eq!(encoded.content_hash, decoded.content_hash);
    assert_eq!(bridge.read_scene_object_snapshot().unwrap(), before);
}

#[test]
fn stored_scene_codec_preserves_v2_lights_and_v1_without_migration() {
    let bridge = init_bridge();
    let lights = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../harness/fixtures/scenes/lights-v2.json"
    ));
    let decoded = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: lights.into(),
        })
        .unwrap();
    assert!(decoded.accepted);
    assert_eq!(decoded.canonical_json.as_deref(), Some(lights));
    assert!(matches!(
        decoded.document.as_ref().unwrap().nodes[0].kind,
        SceneNodeKindDto::Light(SceneLightDto::Ambient { .. })
    ));

    let v1 = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../harness/fixtures/scenes/sample-flat.json"
    ));
    let legacy = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: v1.into(),
        })
        .unwrap();
    assert!(legacy.accepted);
    assert_eq!(legacy.document.unwrap().schema_version, 1);
    assert_eq!(legacy.canonical_json.as_deref(), Some(v1));
}

fn stored_target(
    project_id: ProjectId,
    document: &FlatSceneDocumentDto,
) -> SceneDocumentAuthoringTargetDto {
    SceneDocumentAuthoringTargetDto {
        project_id,
        scene_id: document.id,
    }
}

fn apply_stored_command(
    bridge: &EngineBridge,
    project_id: ProjectId,
    current: FlatSceneDocumentDto,
    current_hash: String,
    command: SceneDocumentAuthoringCommandDto,
) -> SceneDocumentAuthoringResultDto {
    bridge
        .apply_scene_document_authoring(SceneDocumentAuthoringRequestDto {
            current_project_id: project_id,
            expected_content_hash: current_hash,
            current_document: current,
            command,
        })
        .unwrap()
}

#[test]
fn stored_scene_authoring_applies_bounded_commands_and_projects_hierarchical_lights() {
    let bridge = init_bridge();
    let runtime_before = bridge.read_scene_object_snapshot().unwrap();
    let project_id = ProjectId::new(41);
    let decoded = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../harness/fixtures/scenes/lights-v2.json"
            ))
            .into(),
        })
        .unwrap();
    let mut current_hash = decoded.content_hash.unwrap();
    let mut current = decoded.document.unwrap();

    let target = stored_target(project_id, &current);
    let renamed = apply_stored_command(
        &bridge,
        project_id,
        current,
        current_hash,
        SceneDocumentAuthoringCommandDto::Rename {
            target,
            id: SceneNodeId::new(4),
            label: Some("Key spot".to_string()),
        },
    );
    assert!(renamed.accepted);
    current = renamed.document.unwrap();
    current_hash = renamed.content_hash.unwrap();

    let target = stored_target(project_id, &current);
    let reparented = apply_stored_command(
        &bridge,
        project_id,
        current,
        current_hash,
        SceneDocumentAuthoringCommandDto::Reparent {
            target,
            id: SceneNodeId::new(4),
            parent: Some(SceneNodeId::new(2)),
            child_order: 0,
        },
    );
    assert!(reparented.accepted);
    current = reparented.document.unwrap();
    current_hash = reparented.content_hash.unwrap();

    let target = stored_target(project_id, &current);
    let transformed = apply_stored_command(
        &bridge,
        project_id,
        current,
        current_hash,
        SceneDocumentAuthoringCommandDto::SetTransform {
            target,
            id: SceneNodeId::new(4),
            transform: SceneTransformDto {
                translation: [3.0, 4.0, 5.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            },
        },
    );
    assert!(transformed.accepted);
    current = transformed.document.unwrap();
    current_hash = transformed.content_hash.unwrap();

    let target = stored_target(project_id, &current);
    let updated = apply_stored_command(
        &bridge,
        project_id,
        current,
        current_hash,
        SceneDocumentAuthoringCommandDto::UpdateLight {
            target,
            id: SceneNodeId::new(4),
            scene_light: SceneLightDto::Spot {
                color: [0.2, 0.4, 1.0],
                intensity: 9.0,
                enabled: true,
                range: Some(16.0),
                decay: 1.0,
                outer_angle_radians: 0.7,
                penumbra: 0.25,
                shadow_intent: SceneLightShadowIntentDto::Disabled,
            },
        },
    );
    assert!(updated.accepted);
    assert!(updated.authored_light_frame.as_ref().is_some_and(|frame| {
        frame.ops.iter().any(|op| {
            matches!(
                op,
                protocol_render::RenderDiff::CreateLight {
                    light: protocol_render::LightDescriptor::Spot { intensity: 9.0, .. },
                    ..
                }
            )
        })
    }));
    current = updated.document.unwrap();
    current_hash = updated.content_hash.unwrap();

    let voxel_id = SceneNodeId::new(10);
    let target = stored_target(project_id, &current);
    let created = apply_stored_command(
        &bridge,
        project_id,
        current,
        current_hash,
        SceneDocumentAuthoringCommandDto::Create {
            target,
            record: SceneNodeRecordDto {
                id: voxel_id,
                parent: None,
                child_order: 4,
                label: Some("Voxel".to_string()),
                tags: vec!["source:a".to_string()],
                transform: SceneTransformDto {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
                kind: SceneNodeKindDto::VoxelVolume(AssetReferenceDto {
                    id: "voxel-volume/house-a".to_string(),
                    version: AssetVersionReqDto::Any,
                    hash: None,
                }),
            },
        },
    );
    assert!(created.accepted);
    current = created.document.unwrap();
    current_hash = created.content_hash.unwrap();
    assert_eq!(current.dependencies[0].id, "voxel-volume/house-a");

    let target = stored_target(project_id, &current);
    let retargeted = apply_stored_command(
        &bridge,
        project_id,
        current,
        current_hash,
        SceneDocumentAuthoringCommandDto::RetargetVoxelAsset {
            target,
            id: voxel_id,
            asset: AssetReferenceDto {
                id: "voxel-volume/house-b".to_string(),
                version: AssetVersionReqDto::Any,
                hash: None,
            },
            tags: vec!["source:b".to_string()],
        },
    );
    assert!(retargeted.accepted);
    current = retargeted.document.unwrap();
    current_hash = retargeted.content_hash.unwrap();
    assert_eq!(current.dependencies.len(), 1);
    assert_eq!(current.dependencies[0].id, "voxel-volume/house-b");

    let target = stored_target(project_id, &current);
    let deleted = apply_stored_command(
        &bridge,
        project_id,
        current,
        current_hash,
        SceneDocumentAuthoringCommandDto::Delete {
            target,
            id: voxel_id,
        },
    );
    assert!(deleted.accepted);
    assert!(deleted.document.unwrap().dependencies.is_empty());
    assert_eq!(bridge.read_scene_object_snapshot().unwrap(), runtime_before);
}

#[test]
fn stored_scene_authoring_rejections_return_no_document_or_projection() {
    let bridge = init_bridge();
    let project_id = ProjectId::new(51);
    let decoded = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../harness/fixtures/scenes/lights-v2.json"
            ))
            .into(),
        })
        .unwrap();
    let current_hash = decoded.content_hash.unwrap();
    let current = decoded.document.unwrap();
    let target = stored_target(project_id, &current);

    let cases = [
        (
            format!("{current_hash}:stale"),
            SceneDocumentAuthoringCommandDto::RefreshProjection { target },
            SceneDocumentAuthoringRejectionCode::StaleDocument,
        ),
        (
            current_hash.clone(),
            SceneDocumentAuthoringCommandDto::SetTransform {
                target,
                id: SceneNodeId::new(4),
                transform: SceneTransformDto {
                    translation: [f32::NAN, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
            },
            SceneDocumentAuthoringRejectionCode::InvalidResultingDocument,
        ),
        (
            current_hash.clone(),
            SceneDocumentAuthoringCommandDto::Rename {
                target: SceneDocumentAuthoringTargetDto {
                    project_id: ProjectId::new(999),
                    scene_id: current.id,
                },
                id: SceneNodeId::new(4),
                label: Some("Foreign".to_string()),
            },
            SceneDocumentAuthoringRejectionCode::ForeignDocumentIdentity,
        ),
        (
            current_hash.clone(),
            SceneDocumentAuthoringCommandDto::Delete {
                target,
                id: SceneNodeId::new(999),
            },
            SceneDocumentAuthoringRejectionCode::MissingTarget,
        ),
    ];

    for (hash, command, expected_code) in cases {
        let rejected = apply_stored_command(&bridge, project_id, current.clone(), hash, command);
        assert!(!rejected.accepted);
        assert!(rejected.document.is_none());
        assert!(rejected.content_hash.is_none());
        assert!(rejected.authored_light_frame.is_none());
        assert_eq!(rejected.rejection.unwrap().code, expected_code);
    }
}

#[test]
fn blank_runtime_scene_accepts_typed_light_create_and_update_commands() {
    let mut bridge = init_bridge();
    let before = bridge.read_scene_object_snapshot().unwrap();

    let light_id = SceneNodeId::new(2);
    let created = bridge
        .apply_scene_object_command(SceneObjectCommandRequestDto {
            expected_document_hash: before.document_hash,
            command: SceneObjectCommandDto::Create {
                record: SceneNodeRecordDto {
                    id: light_id,
                    parent: Some(SceneNodeId::new(1)),
                    child_order: 0,
                    transform: SceneTransformDto {
                        translation: [0.0, 2.0, 0.0],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [1.0, 1.0, 1.0],
                    },
                    kind: SceneNodeKindDto::Light(SceneLightDto::Point {
                        color: [1.0, 0.8, 0.6],
                        intensity: 4.0,
                        enabled: true,
                        range: Some(12.0),
                        decay: 2.0,
                        shadow_intent: SceneLightShadowIntentDto::Disabled,
                    }),
                    label: Some("Key light".to_string()),
                    tags: Vec::new(),
                },
            },
        })
        .unwrap();
    assert!(created.accepted);

    let updated = bridge
        .apply_scene_object_command(SceneObjectCommandRequestDto {
            expected_document_hash: created.outcome.as_ref().unwrap().snapshot.document_hash,
            command: SceneObjectCommandDto::UpdateLight {
                id: light_id,
                scene_light: SceneLightDto::Point {
                    color: [0.5, 0.7, 1.0],
                    intensity: 7.0,
                    enabled: true,
                    range: Some(18.0),
                    decay: 2.0,
                    shadow_intent: SceneLightShadowIntentDto::Requested,
                },
            },
        })
        .unwrap();
    assert!(updated.accepted);

    let outcome = updated.outcome.unwrap();
    assert_eq!(outcome.document.schema_version, 2);
    assert!(matches!(
        outcome
            .document
            .nodes
            .iter()
            .find(|node| node.id == light_id)
            .unwrap()
            .kind,
        SceneNodeKindDto::Light(SceneLightDto::Point {
            intensity: 7.0,
            range: Some(18.0),
            shadow_intent: SceneLightShadowIntentDto::Requested,
            ..
        })
    ));
}

#[test]
fn stored_scene_codec_classifies_structural_semantic_and_version_rejections() {
    let bridge = init_bridge();
    let before = bridge.read_scene_object_snapshot().unwrap();

    let malformed = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: "{not-json".to_string(),
        })
        .unwrap();
    assert!(!malformed.accepted);
    assert_eq!(
        malformed.diagnostics[0].code,
        SceneDocumentCodecDiagnosticCode::InvalidJson
    );

    let cycle = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../harness/fixtures/scenes/invalid-cycle.json"
    ));
    let invalid = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: cycle.to_string(),
        })
        .unwrap();
    assert!(!invalid.accepted);
    assert!(invalid.diagnostics.is_empty());
    assert_eq!(
        invalid.validation.errors[0].code,
        SceneValidationCode::Cycle
    );

    let mut unsupported = bridge
        .decode_scene_document(SceneDocumentDecodeRequestDto {
            source_text: include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../harness/fixtures/scenes/sample-flat.json"
            ))
            .to_string(),
        })
        .unwrap()
        .document
        .unwrap();
    unsupported.schema_version = 99;
    let unsupported = bridge
        .encode_scene_document(SceneDocumentEncodeRequestDto {
            document: unsupported,
        })
        .unwrap();
    assert!(!unsupported.accepted);
    assert_eq!(
        unsupported.diagnostics[0].code,
        SceneDocumentCodecDiagnosticCode::UnsupportedSchema
    );
    assert_eq!(bridge.read_scene_object_snapshot().unwrap(), before);
}
