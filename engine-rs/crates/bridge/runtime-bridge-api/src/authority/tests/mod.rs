use super::*;

// ── Voxel command submission → Rust authority (launchable-voxel, #2436) ──

use core_space::{LocalVoxelCoord, VoxelCoord};
use core_voxel::VoxelValue;

pub(super) fn init_bridge() -> EngineBridge {
    let mut bridge = EngineBridge::new();
    bridge.initialize_engine(EngineConfig { seed: 1 }).unwrap();
    bridge
}

fn project_voxel_conversion_request(grid: u64) -> VoxelConversionPlanRequest {
    VoxelConversionPlanRequest {
        source: protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: "mesh/import-fixture-a".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 1,
            source_hash: "sha256:import-fixture-a".to_string(),
            mesh_primitive: Some("default".to_string()),
        },
        target: protocol_voxel_conversion::VoxelConversionTargetRef {
            grid,
            volume_asset_id: Some("voxel/generated".to_string()),
            origin: protocol_voxel_conversion::VoxelConversionCoord { x: 0, y: 0, z: 0 },
        },
        settings: protocol_voxel_conversion::VoxelConversionSettings {
            mode: protocol_voxel_conversion::VoxelConversionMode::Surface,
            fit_policy: protocol_voxel_conversion::VoxelConversionFitPolicy::Contain,
            origin_policy: protocol_voxel_conversion::VoxelConversionOriginPolicy::TargetMin,
            resolution: [4, 4, 1],
            voxel_size: 1.0,
            max_output_voxels: 16,
            transform: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            material_map: protocol_voxel_conversion::VoxelConversionMaterialMap {
                entries: vec![protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
                    source_material_slot: 0,
                    source_material_id: Some("material/surface-a".to_string()),
                    voxel_material: 3,
                }],
                texture_assets: Vec::new(),
                texture_bindings: Vec::new(),
                default_voxel_material: None,
            },
        },
    }
}

fn studio_registered_source_request() -> VoxelConversionSourceRegistrationRequest {
    VoxelConversionSourceRegistrationRequest {
        source: protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: "mesh/studio-registered-triangle".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 3,
            source_hash: "sha256:studio-registered-triangle".to_string(),
            mesh_primitive: Some("default".to_string()),
        },
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        triangles: vec![protocol_voxel_conversion::VoxelConversionSourceTriangle {
            indices: [0, 1, 2],
            source_material_slot: 4,
        }],
        material_slots: vec![VoxelConversionSourceMaterialSlot {
            source_material_slot: 4,
            source_material_id: Some("material/studio-copper".to_string()),
        }],
    }
}

fn project_mesh_asset_registration_request(
) -> protocol_voxel_conversion::VoxelConversionMeshAssetRegistrationRequest {
    protocol_voxel_conversion::VoxelConversionMeshAssetRegistrationRequest {
        source: protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: "mesh/project-quad".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 5,
            source_hash: "sha256:project-quad".to_string(),
            mesh_primitive: Some("default".to_string()),
        },
        mesh_asset: protocol_voxel_conversion::VoxelConversionMeshAsset {
            asset_id: "mesh/project-quad".to_string(),
            source_path: Some("assets/meshes/project-quad.mesh.json".to_string()),
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            normals: Vec::new(),
            indices: vec![0, 1, 2, 0, 2, 3],
            groups: vec![protocol_voxel_conversion::VoxelConversionMeshAssetGroup {
                material_slot: 2,
                start: 0,
                count: 6,
            }],
            material_slots: vec![VoxelConversionSourceMaterialSlot {
                source_material_slot: 2,
                source_material_id: Some("material/project-brick".to_string()),
            }],
        },
    }
}

fn larger_registered_grid_source_request() -> VoxelConversionSourceRegistrationRequest {
    let mut positions = Vec::new();
    for y in 0..3 {
        for x in 0..3 {
            positions.push([x as f32, y as f32, 0.0]);
        }
    }

    let mut triangles = Vec::new();
    for y in 0..2 {
        for x in 0..2 {
            let a = y * 3 + x;
            let b = a + 1;
            let c = a + 3;
            let d = c + 1;
            triangles.push(protocol_voxel_conversion::VoxelConversionSourceTriangle {
                indices: [a, b, d],
                source_material_slot: 0,
            });
            triangles.push(protocol_voxel_conversion::VoxelConversionSourceTriangle {
                indices: [a, d, c],
                source_material_slot: 0,
            });
        }
    }

    VoxelConversionSourceRegistrationRequest {
        source: protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: "mesh/registered-grid-3x3".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 1,
            source_hash: "sha256:registered-grid-3x3".to_string(),
            mesh_primitive: Some("default".to_string()),
        },
        positions,
        triangles,
        material_slots: vec![VoxelConversionSourceMaterialSlot {
            source_material_slot: 0,
            source_material_id: Some("material/grid-stone".to_string()),
        }],
    }
}

fn registered_source_plan_request(
    registration: &VoxelConversionSourceRegistrationRequest,
) -> VoxelConversionPlanRequest {
    let mut request = project_voxel_conversion_request(7);
    request.source = registration.source.clone();
    request.settings.material_map.entries =
        vec![protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
            source_material_slot: 4,
            source_material_id: Some("material/studio-copper".to_string()),
            voxel_material: 9,
        }];
    request.settings.material_map.default_voxel_material = None;
    request
}

fn larger_registered_grid_plan_request(
    registration: &VoxelConversionSourceRegistrationRequest,
) -> VoxelConversionPlanRequest {
    let mut request = project_voxel_conversion_request(7);
    request.source = registration.source.clone();
    request.settings.resolution = [3, 3, 1];
    request.settings.max_output_voxels = 16;
    request.settings.material_map.entries =
        vec![protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
            source_material_slot: 0,
            source_material_id: Some("material/grid-stone".to_string()),
            voxel_material: 3,
        }];
    request.settings.material_map.default_voxel_material = None;
    request
}

fn project_mesh_asset_plan_request(
    registration: &protocol_voxel_conversion::VoxelConversionMeshAssetRegistrationRequest,
) -> VoxelConversionPlanRequest {
    let mut request = project_voxel_conversion_request(7);
    request.source = registration.source.clone();
    request.settings.resolution = [4, 4, 1];
    request.settings.material_map.entries =
        vec![protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
            source_material_slot: 2,
            source_material_id: Some("material/project-brick".to_string()),
            voxel_material: 11,
        }];
    request.settings.material_map.default_voxel_material = None;
    request
}

pub(super) fn hand_authored_voxel_volume_asset() -> VoxelVolumeAsset {
    let asset = VoxelVolumeAsset {
        asset_id: "voxel-volume/hand-authored-room".to_string(),
        schema_version: protocol_voxel_asset::VOXEL_ASSET_SCHEMA_VERSION,
        media_type: protocol_voxel_asset::VOXEL_ASSET_MEDIA_TYPE.to_string(),
        grid: VoxelAssetGrid {
            origin: [0.0, 0.0, 0.0],
            cell_size: 1.0,
            coordinate_system: svc_voxel_asset::VOXEL_ASSET_COORDINATE_SYSTEM.to_string(),
        },
        bounds: VoxelAssetBounds {
            min: VoxelAssetCoord { x: 0, y: 0, z: 0 },
            max: VoxelAssetCoord { x: 1, y: 0, z: 0 },
        },
        representation: VoxelAssetRepresentation {
            kind: VoxelAssetRepresentationKind::SparseRuns,
            sparse_runs: vec![VoxelAssetSparseRun {
                start: VoxelAssetCoord { x: 0, y: 0, z: 0 },
                length: 2,
                material: 1,
            }],
        },
        material_palette: vec![VoxelAssetMaterialBinding {
            voxel_material: 1,
            palette_entry_id: "voxel-material/concrete".to_string(),
            display_name: Some("Concrete".to_string()),
            material_asset_id: "material/concrete".to_string(),
            material_catalog_binding_id: Some("catalog-binding/concrete".to_string()),
        }],
        provenance: vec![VoxelAssetProvenanceRef {
            kind: VoxelAssetProvenanceKind::Authored,
            uri: "asha://project-bundle/assets/voxel-volume/hand-authored-room".to_string(),
            content_hash: "fnv1a64:authored-room".to_string(),
        }],
        authoring: VoxelAssetAuthoringMetadata {
            label: Some("Hand authored room".to_string()),
            created_by: Some("runtime-bridge-api-test".to_string()),
            source_tool: Some("fixture".to_string()),
        },
        validation_diagnostics: Vec::new(),
        content_hashes: VoxelAssetContentHashes {
            canonical_json: String::new(),
            voxel_data: String::new(),
        },
    };
    svc_voxel_asset::with_computed_hashes(&asset)
}

fn hand_authored_voxel_annotation_layer(asset: &VoxelVolumeAsset) -> VoxelAnnotationLayer {
    let layer = VoxelAnnotationLayer {
        layer_id: "voxel-annotation/hand-authored-room".to_string(),
        schema_version: protocol_voxel_annotation::VOXEL_ANNOTATION_SCHEMA_VERSION,
        media_type: protocol_voxel_annotation::VOXEL_ANNOTATION_MEDIA_TYPE.to_string(),
        target_voxel_volume_asset_id: asset.asset_id.clone(),
        target_voxel_data_hash: asset.content_hashes.voxel_data.clone(),
        target_bounds: protocol_voxel_annotation::VoxelAnnotationBounds {
            min: protocol_voxel_annotation::VoxelAnnotationCoord { x: 0, y: 0, z: 0 },
            max: protocol_voxel_annotation::VoxelAnnotationCoord { x: 1, y: 0, z: 0 },
        },
        regions: vec![VoxelAnnotationRegion {
            region_id: "region/entry-room".to_string(),
            label: "Entry room".to_string(),
            kind: protocol_voxel_annotation::VoxelAnnotationKind::Room,
            tags: vec!["entry".to_string(), "runtime".to_string()],
            parent_region_id: None,
            bounds: protocol_voxel_annotation::VoxelAnnotationBounds {
                min: protocol_voxel_annotation::VoxelAnnotationCoord { x: 0, y: 0, z: 0 },
                max: protocol_voxel_annotation::VoxelAnnotationCoord { x: 1, y: 0, z: 0 },
            },
            selection: VoxelAnnotationSelection {
                sparse_runs: vec![VoxelAnnotationSparseRun {
                    start: protocol_voxel_annotation::VoxelAnnotationCoord { x: 0, y: 0, z: 0 },
                    length: 2,
                }],
            },
        }],
        provenance: vec![protocol_voxel_annotation::VoxelAnnotationProvenanceRef {
            kind: protocol_voxel_annotation::VoxelAnnotationProvenanceKind::Authored,
            uri: "asha://runtime-bridge-api/tests/hand-authored-room.annotation".to_string(),
            content_hash: "fnv1a64:annotation-source".to_string(),
        }],
        content_hashes: protocol_voxel_annotation::VoxelAnnotationContentHashes {
            canonical_json: String::new(),
            membership_data: String::new(),
        },
        validation_diagnostics: Vec::new(),
    };
    svc_voxel_annotation::with_computed_hashes(&layer)
}

mod camera_modes;
mod core;
mod developer_console;
mod input;
mod mesh_import;
mod project_source;
mod runtime;
mod runtime_project_activation;
mod scene_preview;
mod time_control;
mod voxel;
mod voxel_instances;
mod voxel_unload;
