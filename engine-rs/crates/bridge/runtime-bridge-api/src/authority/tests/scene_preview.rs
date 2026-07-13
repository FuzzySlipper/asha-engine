use super::*;

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
