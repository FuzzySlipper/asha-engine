use super::*;

pub(super) fn extend_round_trip_coverage(coverage: &mut BTreeSet<String>) {
    coverage.extend([
        interface_coverage_key("render", "MaterialInstanceParameters"),
        variant_coverage_key("render", "RenderDiff", "setMaterialInstanceParameters"),
    ]);
}

#[test]
fn material_feedback_fixture_matches_render_ir_shape() {
    let render = module("render");
    let fixture_path = repo_root().join("harness/fixtures/render-diffs/material-feedback.json");
    let fixture: Value = serde_json::from_str(
        &std::fs::read_to_string(&fixture_path).unwrap_or_else(|err| {
            panic!(
                "failed to read material feedback render-diff fixture {}: {err}",
                fixture_path.display()
            )
        }),
    )
    .unwrap();
    let op = fixture["ops"]
        .as_array()
        .and_then(|ops| ops.last())
        .expect("material feedback fixture should end with an operation");
    compare_object_to_variant(&render, "RenderDiff", "setMaterialInstanceParameters", op).unwrap();
    compare_object_to_interface(&render, "MaterialInstanceParameters", &op["parameters"]).unwrap();
}

#[test]
fn animated_mesh_fixture_matches_render_ir_shape() {
    use protocol_render::{AnimatedMeshRuntimeFormat, AnimationLoopMode};

    let render = module("render");
    assert_eq!(
        string_enum_values(&render, "AnimatedMeshRuntimeFormat"),
        BTreeSet::from([AnimatedMeshRuntimeFormat::Glb.label().to_string()])
    );
    assert_eq!(
        string_enum_values(&render, "AnimationLoopMode"),
        BTreeSet::from([
            AnimationLoopMode::Once.label().to_string(),
            AnimationLoopMode::Repeat.label().to_string(),
            AnimationLoopMode::PingPong.label().to_string(),
        ])
    );

    let fixture_path = repo_root().join("harness/fixtures/render-diffs/animated-mesh.json");
    let fixture: Value = serde_json::from_str(
        &std::fs::read_to_string(&fixture_path).unwrap_or_else(|err| {
            panic!(
                "failed to read animated mesh render-diff fixture {}: {err}",
                fixture_path.display()
            )
        }),
    )
    .unwrap();
    let ops = fixture["ops"]
        .as_array()
        .expect("animated mesh fixture should contain ops array");
    assert_eq!(ops.len(), 3);

    compare_object_to_variant(&render, "RenderDiff", "defineAnimatedMesh", &ops[0]).unwrap();
    compare_object_to_interface(&render, "AnimatedMeshAsset", &ops[0]["asset"]).unwrap();
    compare_object_to_interface(
        &render,
        "AnimationClipDescriptor",
        &ops[0]["asset"]["clips"][0],
    )
    .unwrap();
    assert_eq!(
        ops[0]["asset"]["runtimeFormat"],
        json!(AnimatedMeshRuntimeFormat::Glb.label())
    );
    assert_eq!(ops[0]["asset"]["defaultClip"], json!("idle"));

    compare_object_to_variant(&render, "RenderDiff", "createAnimatedMeshInstance", &ops[1])
        .unwrap();
    compare_object_to_interface(
        &render,
        "AnimatedMeshInstanceDescriptor",
        &ops[1]["instance"],
    )
    .unwrap();
    assert_eq!(ops[1]["instance"]["playback"], Value::Null);

    compare_object_to_variant(&render, "RenderDiff", "setAnimatedMeshPlayback", &ops[2]).unwrap();
    compare_object_to_variant(
        &render,
        "AnimatedMeshPlaybackCommand",
        "play",
        &ops[2]["playback"],
    )
    .unwrap();
    assert_eq!(
        ops[2]["playback"]["loop"],
        json!(AnimationLoopMode::Repeat.label())
    );

    let stop = json!({ "action": "stop", "fadeSeconds": 0.125 });
    compare_object_to_variant(&render, "AnimatedMeshPlaybackCommand", "stop", &stop).unwrap();
    let pause = json!({ "action": "pause" });
    compare_object_to_variant(&render, "AnimatedMeshPlaybackCommand", "pause", &pause).unwrap();
    let resume = json!({ "action": "resume" });
    compare_object_to_variant(&render, "AnimatedMeshPlaybackCommand", "resume", &resume).unwrap();
}
