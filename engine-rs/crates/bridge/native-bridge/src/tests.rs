use super::*;
use runtime_bridge_api::VoxelEditHistorySummary;

const WIRED_NAPI_EXPORTS: &[&str] = &[
    "applyFirstPersonCameraInput",
    "applyCollisionConstrainedCameraInput",
    "applyGeneratedTunnelToRuntimeWorld",
    "applySceneObjectCommand",
    "applyEnemyDirectNavMovement",
    "applyFpsEncounterTransition",
    "applyFpsPrimaryFire",
    "applyVoxelConversion",
    "applyVoxelAnnotationEdit",
    "applyVoxelEditRevert",
    "decodeSceneDocument",
    "encodeSceneDocument",
    "exportVoxelConversionEvidence",
    "exportVoxelAnnotationLayer",
    "exportVoxelVolumeAsset",
    "getBuffer",
    "getProjectBundleCompositionStatus",
    "initializeEngine",
    "initializeVoxelVolumeAuthoring",
    "invokeGameExtensionWeaponEffect",
    "importVoxelConversionMeshSource",
    "loadVoxelAnnotationLayer",
    "loadVoxelVolumeAsset",
    "loadProjectBundle",
    "loadFpsRuntimeSession",
    "planVoxelConversion",
    "pickVoxel",
    "previewVoxelEditRevert",
    "readVoxelConversionSourceMetadata",
    "readVoxelAnnotationQuery",
    "readVoxelEditHistory",
    "readFpsEncounterDirector",
    "readProjectionFrame",
    "readRenderDiffs",
    "readFpsRuntimeSession",
    "readCameraProjection",
    "readModelMaterialPreview",
    "readSceneObjectSnapshot",
    "readVoxelMeshEvidence",
    "readVoxelModelInfo",
    "readVoxelModelWindow",
    "redoVoxelEdit",
    "registerVoxelConversionSource",
    "registerVoxelConversionMeshAsset",
    "restartFpsRuntimeSession",
    "saveProjectBundle",
    "saveVoxelVolumeAsset",
    "selectVoxel",
    "stepSimulation",
    "submitCommands",
    "undoVoxelEdit",
    "unloadProjectBundle",
    "unloadVoxelVolumeAsset",
    "updateVoxelVolumeAssetPalette",
    "validateVoxelAnnotationLayer",
];

#[test]
fn raw_json_entrypoints_reject_unknown_fields_before_domain_invocation() {
    let camera = apply_camera_mode_command(
        0,
        r#"{"camera":1,"expectedRevision":1,"target":{"mode":"orbit","pivot":[0.0,0.0,0.0],"distance":4.0,"minDistance":1.0,"maxDistance":8.0,"yawDegrees":0.0,"pitchDegrees":-20.0},"transition":null,"tick":1,"unknown":true}"#
            .to_owned(),
    )
    .expect_err("camera request must reject unknown fields before handle lookup");
    assert!(camera.reason.contains("unknown field"));
    assert!(camera.reason.contains("$.unknown"));

    let scene = apply_scene_object_command(
        0,
        r#"{"expectedDocumentHash":0,"command":{"kind":"select","id":null},"unknown":true}"#
            .to_owned(),
    )
    .expect_err("scene request must reject unknown fields before handle lookup");
    assert!(scene.reason.contains("unknown field"));
    assert!(scene
        .reason
        .contains("operation=apply_scene_object_command"));

    let decode = decode_scene_document(0, r#"{"sourceText":"{}","unknown":true}"#.to_owned())
        .expect_err("scene decode request must reject unknown fields before handle lookup");
    assert!(decode.reason.contains("unknown field"));
    assert!(decode.reason.contains("operation=decode_scene_document"));

    let encode = encode_scene_document(
        0,
        r#"{"document":{"schemaVersion":1,"id":1,"metadata":{"name":null,"authoringFormatVersion":1},"dependencies":[],"nodes":[{"id":1,"parent":null,"childOrder":0,"label":null,"tags":[],"transform":{"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1]},"kind":{"kind":"emptyGroup","unknown":true}}]}}"#.to_owned(),
    )
    .expect_err("nested scene encode request must reject unknown fields before handle lookup");
    assert!(encode.reason.contains("unknown field"));
    assert!(encode.reason.contains("operation=encode_scene_document"));

    let authoring = apply_scene_document_authoring(
        0,
        r#"{"currentProjectId":1,"expectedContentHash":"fnv1a64:test","currentDocument":{"schemaVersion":1,"id":1,"metadata":{"name":null,"authoringFormatVersion":1},"dependencies":[],"nodes":[]},"command":{"kind":"refreshProjection","target":{"projectId":1,"sceneId":1},"candidateDocument":{"id":999}}}"#.to_owned(),
    )
    .expect_err("stored scene command must reject candidate payloads before handle lookup");
    assert!(authoring.reason.contains("unknown field"));
    assert!(authoring
        .reason
        .contains("operation=apply_scene_document_authoring"));

    let voxel = read_voxel_mesh_evidence(
        0,
        r#"{"grid":1,"chunks":[{"x":0,"y":0,"z":0}],"unknown":true}"#.to_owned(),
    )
    .expect_err("voxel request must reject unknown fields before handle lookup");
    assert!(voxel.reason.contains("unknown field"));
    assert!(voxel.reason.contains("operation=read_voxel_mesh_evidence"));
}
#[test]
fn wired_export_set_is_explicit_and_bounded() {
    let unique_exports = WIRED_NAPI_EXPORTS
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(WIRED_NAPI_EXPORTS.len(), 55);
    assert_eq!(unique_exports.len(), WIRED_NAPI_EXPORTS.len());
}

fn native_fps_definitions(enemy_health: u32) -> Vec<NativeFpsStoredEntityDefinition> {
    vec![
        NativeFpsStoredEntityDefinition {
            entity: 101,
            stable_id: "actor/custom-player".into(),
            display_name: "Custom Player".into(),
            source_path: "catalogs/actors/player.entity.json".into(),
            tags: vec!["player".into()],
            role: "player".into(),
            transform: Some(NativeFpsTransformCapability {
                translation: NativeVec3 {
                    x: 0.0,
                    y: 1.62,
                    z: 1.5,
                },
                rotation: vec![0.0, 0.0, 0.0, 1.0],
                scale: NativeVec3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            }),
            bounds: Some(NativeFpsBoundsCapability {
                min: NativeVec3 {
                    x: -0.25,
                    y: 0.92,
                    z: 1.25,
                },
                max: NativeVec3 {
                    x: 0.25,
                    y: 2.32,
                    z: 1.75,
                },
            }),
            render_visible: Some(true),
            static_collider: Some(false),
            health: Some(NativeFpsHealth {
                current: 88,
                max: 88,
            }),
            weapon: Some(NativeFpsWeaponMount {
                weapon_id: "weapon.custom.primary".into(),
                damage: 75,
                range_units: 16,
                ammo: 3,
                cooldown_ticks_after_fire: 4,
            }),
            policy_binding: None,
        },
        NativeFpsStoredEntityDefinition {
            entity: 777,
            stable_id: "actor/custom-enemy".into(),
            display_name: "Custom Enemy".into(),
            source_path: "catalogs/actors/enemy.entity.json".into(),
            tags: vec!["enemy".into()],
            role: "enemy".into(),
            transform: Some(NativeFpsTransformCapability {
                translation: NativeVec3 {
                    x: 0.0,
                    y: 0.5,
                    z: -2.6,
                },
                rotation: vec![0.0, 0.0, 0.0, 1.0],
                scale: NativeVec3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            }),
            bounds: Some(NativeFpsBoundsCapability {
                min: NativeVec3 {
                    x: -0.25,
                    y: 0.0,
                    z: -2.85,
                },
                max: NativeVec3 {
                    x: 0.25,
                    y: 1.0,
                    z: -2.35,
                },
            }),
            render_visible: Some(true),
            static_collider: Some(false),
            health: Some(NativeFpsHealth {
                current: enemy_health,
                max: enemy_health,
            }),
            weapon: None,
            policy_binding: Some(NativeFpsPolicyBinding {
                binding_id: "binding.enemy.custom.v0".into(),
                policy_id: "policy.enemy.custom.v0".into(),
                view_kind: "runtime_session.nav_policy_view.v0".into(),
                view_version: "v0".into(),
                allowed_intents: vec!["runtime.intent.primary_fire.v0".into()],
                runtime_moment: "runtime.tick.enemy_policy.v0".into(),
            }),
        },
    ]
}

#[test]
fn native_bridge_stateful_smoke_uses_bounded_operations() {
    let handle = initialize_engine(7).expect("engine initializes");
    assert!(handle > 0);

    let loaded = load_project_bundle(handle, 1, 1, 1001).expect("ProjectBundle loads");
    assert_eq!(loaded.loaded_project_bundle, Some(1001));
    assert_eq!(loaded.fatal_count, 0);
    assert_eq!(loaded.total_count, 0);
    assert!(!loaded.blocks_load);

    let result = submit_commands(
        handle,
        r#"[{"op":"setVoxel","grid":1,"coord":{"x":0,"y":0,"z":0},"value":{"kind":"solid","material":1}}]"#
            .to_string(),
    )
    .expect("bounded command batch submits");
    assert_eq!(result.accepted, 1);
    assert_eq!(result.rejected, 0);
    assert!(result.rejections.is_empty());

    let step = step_simulation(handle, 6).expect("simulation steps");
    assert_eq!(step.tick, 6);
    assert_eq!(step.diff_count, 0);

    let pause: runtime_bridge_api::TimeControlReceipt = serde_json::from_str(
        &apply_time_control_command(handle, r#"{"operation":"pause"}"#.to_string())
            .expect("time control pauses"),
    )
    .expect("pause receipt decodes");
    assert!(pause.accepted);
    let blocked = step_simulation(handle, 7).expect("paused step returns without advancing");
    assert_eq!(blocked.tick, 6);
    assert_eq!(blocked.diff_count, 0);
    let exact: runtime_bridge_api::TimeControlReceipt = serde_json::from_str(
        &apply_time_control_command(
            handle,
            r#"{"operation":"stepTicks","ticks":2}"#.to_string(),
        )
        .expect("time control exact-steps"),
    )
    .expect("exact-step receipt decodes");
    assert_eq!(exact.exact_ticks_advanced, 2);
    assert_eq!(exact.after.authority_tick, 8);
    apply_time_control_command(handle, r#"{"operation":"resume"}"#.to_string())
        .expect("time control resumes");

    let camera = create_camera(
        handle,
        NativeCameraCreateRequest {
            initial_pose: NativeCameraPose {
                position: vec![0.0, 1.6, 0.0],
                yaw_degrees: 0.0,
                pitch_degrees: 0.0,
            },
            projection: NativePerspectiveProjection {
                fov_y_degrees: 60.0,
                near: 0.1,
                far: 500.0,
            },
            viewport: NativeViewportSize {
                width: 1280,
                height: 720,
            },
        },
    )
    .expect("camera creates");
    let camera = apply_first_person_camera_input(
        handle,
        NativeFirstPersonCameraInputEnvelope {
            camera: camera.camera,
            input: NativeFirstPersonCameraInput {
                move_forward: 1.0,
                move_right: 0.0,
                move_up: 0.0,
                yaw_delta_degrees: 0.0,
                pitch_delta_degrees: 0.0,
                dt_seconds: 0.25,
                move_speed_units_per_second: 2.0,
            },
            tick: 9,
        },
    )
    .expect("first-person camera input reaches Rust authority");
    assert_eq!(camera.tick, 9);
    let camera_projection: serde_json::Value = serde_json::from_str(
        &read_camera_projection(
            handle,
            format!(r#"{{"camera":{},"viewport":null}}"#, camera.camera),
        )
        .expect("camera projection reads through native transport"),
    )
    .expect("camera projection JSON decodes");
    assert_eq!(camera_projection["camera"], camera.camera);

    let pick: serde_json::Value = serde_json::from_str(
        &pick_voxel(
            handle,
            r#"{"grid":1,"origin":[-1.0,0.5,0.5],"direction":[1.0,0.0,0.0],"maxDistance":10.0}"#.to_string(),
        )
        .expect("voxel pick reaches Rust authority"),
    )
    .expect("voxel pick JSON decodes");
    assert_eq!(pick["outcome"], "hit");
    let selection: serde_json::Value = serde_json::from_str(
        &select_voxel(
            handle,
            format!(
                r#"{{"camera":{},"grid":1,"viewport":null,"screenPoint":{{"x":0.5,"y":0.5,"space":"normalized_0_1"}},"maxDistance":10.0}}"#,
                camera.camera,
            ),
        )
        .expect("voxel selection reaches Rust authority"),
    )
    .expect("voxel selection JSON decodes");
    assert!(selection["selectionHash"].as_str().is_some());
    let mesh_evidence: serde_json::Value = serde_json::from_str(
        &read_voxel_mesh_evidence(
            handle,
            r#"{"grid":1,"chunks":[{"x":0,"y":0,"z":0}]}"#.to_string(),
        )
        .expect("voxel mesh evidence reaches Rust authority"),
    )
    .expect("voxel mesh evidence JSON decodes");
    assert_eq!(mesh_evidence["chunks"].as_array().map(Vec::len), Some(1));

    let seed_buffer = get_buffer(handle, 0).expect("seed buffer crosses native transport");
    assert_eq!(seed_buffer.bytes, 7_u64.to_le_bytes().to_vec());
    release_buffer(handle, 0).expect("manual buffer release reaches Rust authority");
    assert!(get_buffer(handle, 0).is_err());

    let scene: serde_json::Value = serde_json::from_str(
        &read_scene_object_snapshot(handle).expect("scene hierarchy reads through Rust"),
    )
    .expect("scene hierarchy JSON decodes");
    let document_hash = scene["documentHash"]
        .as_u64()
        .expect("scene hash is an unsigned integer");
    let scene_command: serde_json::Value = serde_json::from_str(
        &apply_scene_object_command(
            handle,
            serde_json::json!({
                "expectedDocumentHash": document_hash,
                "command": { "kind": "select", "id": 1 },
            })
            .to_string(),
        )
        .expect("scene selection reaches Rust authority"),
    )
    .expect("scene command JSON decodes");
    assert_eq!(scene_command["accepted"], true);
    assert_eq!(scene_command["outcome"]["selected"], 1);

    let canonical_scene = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../harness/fixtures/scenes/sample-flat.json"
    ));
    let decoded_scene: serde_json::Value = serde_json::from_str(
        &decode_scene_document(
            handle,
            serde_json::json!({ "sourceText": canonical_scene }).to_string(),
        )
        .expect("stored scene decode reaches Rust authority"),
    )
    .expect("stored scene decode JSON decodes");
    assert_eq!(decoded_scene["accepted"], true);
    assert_eq!(decoded_scene["canonicalJson"], canonical_scene);
    let encoded_scene: serde_json::Value = serde_json::from_str(
        &encode_scene_document(
            handle,
            serde_json::json!({ "document": decoded_scene["document"] }).to_string(),
        )
        .expect("stored scene encode reaches Rust authority"),
    )
    .expect("stored scene encode JSON decodes");
    assert_eq!(encoded_scene["accepted"], true);
    assert_eq!(encoded_scene["canonicalJson"], canonical_scene);

    let light_scene = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../harness/fixtures/scenes/lights-v2.json"
    ));
    let decoded_light_scene: serde_json::Value = serde_json::from_str(
        &decode_scene_document(
            handle,
            serde_json::json!({ "sourceText": light_scene }).to_string(),
        )
        .expect("stored light scene decode reaches Rust authority"),
    )
    .expect("stored light scene decode JSON decodes");
    assert_eq!(decoded_light_scene["accepted"], true);
    let encoded_light_scene: serde_json::Value = serde_json::from_str(
        &encode_scene_document(
            handle,
            serde_json::json!({ "document": decoded_light_scene["document"] }).to_string(),
        )
        .expect("camelCase stored light scene encode reaches Rust authority"),
    )
    .expect("stored light scene encode JSON decodes");
    assert_eq!(encoded_light_scene["accepted"], true);
    assert_eq!(encoded_light_scene["canonicalJson"], light_scene);

    let model_preview: serde_json::Value = serde_json::from_str(
        &read_model_material_preview(handle, scene_preview::model_preview_test_request_json())
            .expect("model material preview reaches Rust authority"),
    )
    .expect("model material preview JSON decodes");
    assert_eq!(model_preview["rendererClassification"], "runtime_readback");
    assert_eq!(
        model_preview["previewDiff"]["ops"].as_array().map(Vec::len),
        Some(3)
    );
    let orbit: runtime_bridge_api::CameraModeChangeReceipt = serde_json::from_str(
        &apply_camera_mode_command(
            handle,
            format!(
                r#"{{"camera":{},"expectedRevision":1,"target":{{"mode":"orbit","pivot":[2.0,1.0,-4.0],"distance":8.0,"minDistance":2.0,"maxDistance":30.0,"yawDegrees":20.0,"pitchDegrees":-25.0}},"transition":{{"durationMilliseconds":250,"easing":"smoothStep"}},"tick":10}}"#,
                camera.camera,
            ),
        )
        .expect("camera switches to orbit"),
    )
    .expect("camera mode receipt decodes");
    assert!(orbit.accepted);
    assert_eq!(orbit.after.mode, runtime_bridge_api::CameraMode::Orbit);
    let navigated: runtime_bridge_api::CameraNavigationReceipt = serde_json::from_str(
        &apply_camera_navigation_input(
            handle,
            format!(
                r#"{{"camera":{},"expectedRevision":2,"input":{{"panRight":0.5,"panForward":0.0,"yawDeltaDegrees":5.0,"pitchDeltaDegrees":-2.0,"zoomDelta":1.0,"dtSeconds":0.25,"panSpeedUnitsPerSecond":4.0}},"tick":11}}"#,
                camera.camera,
            ),
        )
        .expect("orbit camera navigates"),
    )
    .expect("camera navigation receipt decodes");
    assert!(navigated.accepted);
    assert!(navigated.after.distance < orbit.after.distance);
    let controller: runtime_bridge_api::CameraControllerState = serde_json::from_str(
        &read_camera_controller_state(handle, format!(r#"{{"camera":{}}}"#, camera.camera))
            .expect("camera controller reads"),
    )
    .expect("camera controller state decodes");
    assert_eq!(controller, navigated.after);
    let moved = apply_enemy_direct_nav_movement(
        handle,
        777,
        NativeVec3 {
            x: 0.0,
            y: 0.5,
            z: -2.6,
        },
        NativeVec3 {
            x: 0.0,
            y: 1.62,
            z: 1.25,
        },
        0.35,
    )
    .expect("enemy direct-nav movement applies");
    assert_eq!(moved.entity, 777);
    assert_eq!(moved.authority_source, "seeded_from_request");
    assert_eq!(moved.next_waypoint.x, 0.0);
    assert!((moved.next_waypoint.y - 0.598).abs() < 0.0005);
    assert_eq!(moved.path_hash, "fnv1a64:69ed74d692922db7");
    assert!(moved.transform_hash.starts_with("fnv1a64:"));

    let fps_loaded = load_fps_runtime_session(
        handle,
        "custom-demo".into(),
        native_fps_definitions(75),
        "[]".into(),
    )
    .expect("fps runtime session loads");
    assert_eq!(fps_loaded.backend, "engine_bridge_rust");
    assert_eq!(fps_loaded.player_entity, 101);
    assert_eq!(fps_loaded.enemy_entity, 777);
    assert_eq!(fps_loaded.policy_bindings.len(), 1);
    assert!(fps_loaded.replay_hash.starts_with("fnv1a64:"));

    let tunnel =
        apply_generated_tunnel_to_runtime_world(handle, "tiny-enclosed".to_string(), 17)
            .expect("generated tunnel collision authority applies");
    assert_eq!(tunnel.preset_id, "tiny-enclosed");
    assert_eq!(tunnel.grid, 0);
    assert_eq!(tunnel.output_hash, "1471496d88d70647");
    assert!(tunnel.collision_projection_hash.starts_with("fnv1a64:"));
    assert_eq!(tunnel.runtime_frame.world_offset, vec![-3.5, -1.0, -5.5]);
    assert_eq!(tunnel.runtime_frame.playable_min, vec![-2.5, 0.0, -4.5]);
    assert_eq!(tunnel.runtime_frame.playable_max, vec![2.5, 4.0, 4.5]);

    let fps_fire = apply_fps_primary_fire(
        handle,
        9,
        NativeVec3 {
            x: 0.0,
            y: 1.62,
            z: 1.5,
        },
        NativeVec3 {
            x: 0.0,
            y: -1.12,
            z: -4.1,
        },
        None,
        None,
    )
    .expect("fps primary fire applies");
    assert_eq!(fps_fire.target, Some(777));
    assert_eq!(fps_fire.lifecycle_status.state, "enemy_defeated");
    assert_eq!(
        fps_fire
            .target_health_after
            .as_ref()
            .map(|health| health.current),
        Some(0)
    );

    let fps_read = read_fps_runtime_session(handle).expect("fps session reads");
    assert_eq!(fps_read.replay_records.len(), 2);
    assert_eq!(fps_read.replay_hash, fps_fire.replay_hash);

    let projection = read_projection_frame(handle, 0).expect("projection frame reads");
    assert_eq!(projection.schema_version, 1);
    assert_eq!(projection.authority_tick, 9);
    assert_eq!(
        projection.presentation.replay_scope,
        "excludedFromReplayTruth"
    );
    assert_eq!(projection.presentation.ops.len(), 8);
    assert_eq!(projection.presentation.ops[0].domain, "audio");
    assert_eq!(
        projection.presentation.ops[0]
            .audio_op
            .as_ref()
            .map(|op| op.op.as_str()),
        Some("emit")
    );
    assert_eq!(
        projection.presentation.ops[0]
            .audio_op
            .as_ref()
            .expect("audio domain owns audio payload")
            .descriptor
            .as_ref()
            .map(|descriptor| descriptor.clip.asset.as_str()),
        Some("audio/asha-primary-fire-pulse")
    );
    assert_eq!(projection.presentation.ops[1].domain, "particle");
    assert_eq!(
        projection.presentation.ops[1]
            .particle_op
            .as_ref()
            .map(|op| op.op.as_str()),
        Some("emit")
    );
    assert_eq!(projection.presentation.ops[2].domain, "billboard");
    assert_eq!(projection.presentation.ops[3].domain, "billboard");
    assert_eq!(
        projection.presentation.ops[3]
            .billboard_op
            .as_ref()
            .and_then(|op| op.descriptor.as_ref())
            .map(|descriptor| descriptor.visible),
        Some(true)
    );
    assert_eq!(projection.presentation.ops[4].domain, "animation");
    assert_eq!(projection.presentation.ops[5].domain, "animation");
    assert_eq!(projection.presentation.ops[6].domain, "animation");
    let animation = projection.presentation.ops[6]
        .animation_op
        .as_ref()
        .and_then(|operation| operation.controller.as_ref())
        .expect("latest animation update crosses the native border");
    assert_eq!(animation.state_id, "ready");
    assert_eq!(
        animation
            .timing_fact
            .as_ref()
            .map(|fact| fact.to_state_id.as_str()),
        Some("primary_fire")
    );
    assert_eq!(projection.presentation.ops[7].domain, "telemetryOverlay");
    assert_eq!(
        projection.presentation.ops[7]
            .telemetry_overlay_op
            .as_ref()
            .map(|op| op.op.as_str()),
        Some("create")
    );

    let fps_restarted = restart_fps_runtime_session(handle, fps_read.session_epoch)
        .expect("fps session restarts");
    assert_eq!(fps_restarted.session_epoch, fps_read.session_epoch + 1);
    assert_eq!(fps_restarted.lifecycle_status.state, "active");

    let frame = read_render_diffs(handle, 0).expect("render diff read is bounded");
    assert!(frame.contains("replaceMeshPayload"));

    let saved = save_project_bundle(handle).expect("ProjectBundle saves");
    assert_eq!(saved.artifacts_written, 3);
    assert_eq!(saved.compacted_edits, 0);
    assert_eq!(saved.retained_edits, 0);

    let status = get_project_bundle_composition_status(handle).expect("composition reads");
    assert_eq!(status.loaded_project_bundle, Some(1001));
    assert_eq!(status.fatal_count, 0);
    unload_project_bundle(handle).expect("ProjectBundle unload reaches Rust authority");
    let status = get_project_bundle_composition_status(handle).expect("composition reads");
    assert_eq!(status.loaded_project_bundle, None);
}

#[test]
fn native_voxel_command_union_reaches_rust_authority() {
    let handle = initialize_engine(77).expect("engine initializes");
    let result = submit_commands(
        handle,
        r#"[
            {"op":"generateChunk","grid":1,"chunk":{"x":0,"y":0,"z":0},"seed":77,"generatorVersion":1},
            {"op":"fillRegion","grid":1,"min":{"x":0,"y":0,"z":0},"max":{"x":2,"y":2,"z":2},"value":{"kind":"empty"}},
            {"op":"fillRegion","grid":1,"min":{"x":0,"y":0,"z":0},"max":{"x":2,"y":2,"z":2},"value":{"kind":"solid","material":2}},
            {"op":"setVoxel","grid":1,"coord":{"x":1,"y":1,"z":1},"value":{"kind":"empty"}}
        ]"#
            .to_string(),
    )
    .expect("full generated command union submits");
    assert_eq!(result.accepted, 4);
    assert_eq!(result.rejected, 0);

    let history_json = read_voxel_edit_history(
        handle,
        r#"{"historyId":"history/default","cursorId":null,"maxEntries":8,"includeRedoTail":true,"expectedHistoryHash":null}"#
            .to_string(),
    )
    .expect("Rust authority history reads");
    let history: VoxelEditHistorySummary = serde_json::from_str(&history_json).unwrap();
    assert_eq!(history.entries[0].command_count, 4);
    assert_eq!(history.cursor.entry_count, 1);
}

#[test]
fn native_bridge_rejects_invalid_inputs_without_fallback() {
    assert!(initialize_engine(-1).is_err());
    assert!(get_project_bundle_composition_status(-99).is_err());

    let handle = initialize_engine(11).expect("engine initializes");
    assert!(load_project_bundle(handle, -1, 1, 1001).is_err());
    assert!(step_simulation(handle, -1).is_err());
    assert!(submit_commands(handle, r#"[{"op":"deleteEverything"}]"#.to_string()).is_err());
    assert!(submit_commands(
        handle,
        r#"[{"op":"setVoxel","grid":1,"coord":{"x":0,"y":0,"z":0},"value":{"kind":"unknown"}}]"#
            .to_string()
    )
    .is_err());
}
