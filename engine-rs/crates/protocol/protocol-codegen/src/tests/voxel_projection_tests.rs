use super::*;

pub(super) fn extend_round_trip_coverage(coverage: &mut BTreeSet<String>) {
    coverage.extend(
        [
            "VoxelProjectionInstanceBinding",
            "VoxelProjectionBindingRequest",
            "VoxelProjectionBindingReceipt",
            "VoxelInstancePickHint",
            "VoxelInstancePickRequest",
            "VoxelInstancePickHit",
            "VoxelInstancePickResult",
        ]
        .map(|item| interface_coverage_key("voxel", item)),
    );
    coverage.extend(
        ["hit", "rejected"]
            .map(|tag| variant_coverage_key("voxel", "VoxelInstancePickOutcome", tag)),
    );
}

#[test]
fn voxel_projection_samples_match_generated_ir_shapes() {
    let voxel = module("voxel");
    let transform = json!({
        "translation": [4.0, 1.0, -2.0],
        "rotation": [0.0, 0.70710677, 0.0, 0.70710677],
        "scale": [2.0, 1.0, 0.5]
    });
    let instance = json!({
        "instanceId": "house-a",
        "sceneNodeId": 17,
        "assetId": "voxel-volume:studio/house",
        "transform": transform
    });
    let binding_request = json!({
        "workspaceId": "workspace-1",
        "workspaceGeneration": 3,
        "workingRevision": 9,
        "registryDigest": "fnv1a64:1111111111111111",
        "instances": [instance]
    });
    let binding_receipt = json!({
        "workspaceId": "workspace-1",
        "workspaceGeneration": 3,
        "workingRevision": 9,
        "registryDigest": "fnv1a64:1111111111111111",
        "bindingHash": "fnv1a64:2222222222222222",
        "instanceCount": 1,
        "projectionOpCount": 2
    });
    let hint = json!({
        "localVoxel": { "x": 1, "y": 2, "z": 3 },
        "localFace": "positiveY"
    });
    let pick_request = json!({
        "workspaceId": "workspace-1",
        "workspaceGeneration": 3,
        "workingRevision": 9,
        "registryDigest": "fnv1a64:1111111111111111",
        "bindingHash": "fnv1a64:2222222222222222",
        "instanceId": "house-a",
        "origin": [4.0, 8.0, -2.0],
        "direction": [0.0, -1.0, 0.0],
        "maxDistance": 20.0,
        "rendererHint": hint
    });
    let hit = json!({
        "localVoxel": { "x": 1, "y": 2, "z": 3 },
        "localChunk": { "x": 0, "y": 0, "z": 0 },
        "localFace": "positiveY",
        "localPlaceAnchor": { "x": 1, "y": 3, "z": 3 },
        "worldPoint": [4.0, 3.0, -2.0],
        "worldDistance": 5.0
    });
    let hit_outcome = json!({ "outcome": "hit", "voxelInstancePickHit": hit });
    let rejected_outcome = json!({
        "outcome": "rejected",
        "rejection": "rendererHintMismatch"
    });
    let pick_result = json!({
        "workspaceId": "workspace-1",
        "workspaceGeneration": 3,
        "workingRevision": 9,
        "bindingHash": "fnv1a64:2222222222222222",
        "instanceId": "house-a",
        "outcome": hit_outcome
    });

    for (name, value) in [
        ("VoxelProjectionInstanceBinding", &instance),
        ("VoxelProjectionBindingRequest", &binding_request),
        ("VoxelProjectionBindingReceipt", &binding_receipt),
        ("VoxelInstancePickHint", &hint),
        ("VoxelInstancePickRequest", &pick_request),
        ("VoxelInstancePickHit", &hit),
        ("VoxelInstancePickResult", &pick_result),
    ] {
        compare_object_to_interface(&voxel, name, value).unwrap();
    }
    compare_object_to_variant(&voxel, "VoxelInstancePickOutcome", "hit", &hit_outcome).unwrap();
    compare_object_to_variant(
        &voxel,
        "VoxelInstancePickOutcome",
        "rejected",
        &rejected_outcome,
    )
    .unwrap();
}
