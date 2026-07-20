use super::*;

fn camera_request() -> CameraCreateRequest {
    CameraCreateRequest {
        initial_pose: CameraPose {
            position: [2.0, 3.0, 4.0],
            yaw_degrees: 15.0,
            pitch_degrees: -5.0,
        },
        projection: PerspectiveProjection {
            fov_y_degrees: 60.0,
            near: 0.1,
            far: 500.0,
        },
        viewport: ViewportSize {
            width: 1280,
            height: 720,
        },
    }
}

fn orbit_command(camera: CameraHandle, expected_revision: u64) -> CameraModeCommand {
    CameraModeCommand {
        camera,
        expected_revision,
        target: CameraModeTarget::Orbit {
            pivot: [10.0, 2.0, -4.0],
            distance: 8.0,
            min_distance: 2.0,
            max_distance: 24.0,
            yaw_degrees: 35.0,
            pitch_degrees: -25.0,
        },
        transition: Some(CameraTransitionSpec {
            duration_milliseconds: 400,
            easing: CameraTransitionEasing::SmoothStep,
        }),
        tick: 7,
    }
}

#[test]
fn camera_modes_are_revision_guarded_and_emit_transition_evidence() {
    let mut bridge = init_bridge();
    let created = bridge.create_camera(camera_request()).unwrap();
    let initial = bridge
        .read_camera_controller_state(CameraControllerReadRequest {
            camera: created.camera,
        })
        .unwrap();
    assert_eq!(
        initial.schema_version,
        CAMERA_CONTROLLER_STATE_SCHEMA_VERSION
    );
    assert_eq!(initial.revision, 0);
    assert_eq!(initial.mode, CameraMode::FirstPerson);
    assert_eq!(initial.snapshot, created);

    let changed = bridge
        .apply_camera_mode_command(orbit_command(created.camera, 0))
        .unwrap();
    assert!(changed.accepted);
    assert_eq!(changed.before, initial);
    assert_eq!(changed.after.revision, 1);
    assert_eq!(changed.after.mode, CameraMode::Orbit);
    assert_eq!(changed.after.pivot, Some([10.0, 2.0, -4.0]));
    assert_eq!(changed.after.distance, Some(8.0));
    assert!(!changed.terrain_constrained);
    let transition = changed.transition.as_ref().unwrap();
    assert_eq!(transition.from, created);
    assert_eq!(transition.to, changed.after.snapshot);
    assert_eq!(transition.duration_milliseconds, 400);
    assert_eq!(transition.easing, CameraTransitionEasing::SmoothStep);
    assert_ne!(transition.transition_hash, changed.receipt_hash);

    let stale = bridge
        .apply_camera_mode_command(orbit_command(created.camera, 0))
        .unwrap();
    assert!(!stale.accepted);
    assert_eq!(
        stale.rejection,
        Some(CameraControllerRejection::StaleRevision)
    );
    assert_eq!(stale.before, changed.after);
    assert_eq!(stale.after, changed.after);
    assert_eq!(
        bridge
            .read_camera_controller_state(CameraControllerReadRequest {
                camera: created.camera,
            })
            .unwrap(),
        changed.after
    );

    let legacy_input = bridge
        .apply_first_person_camera_input(FirstPersonCameraInputEnvelope {
            camera: created.camera,
            tick: 8,
            input: FirstPersonCameraInput {
                move_forward: 1.0,
                move_right: 0.0,
                move_up: 0.0,
                yaw_delta_degrees: 0.0,
                pitch_delta_degrees: 0.0,
                dt_seconds: 1.0 / 60.0,
                move_speed_units_per_second: 3.0,
            },
        })
        .unwrap_err();
    assert_eq!(legacy_input.kind, RuntimeBridgeErrorKind::InvalidInput);
}

#[test]
fn orbit_navigation_and_mode_return_are_deterministic() {
    let run = || {
        let mut bridge = init_bridge();
        let camera = bridge.create_camera(camera_request()).unwrap().camera;
        let orbit = bridge
            .apply_camera_mode_command(orbit_command(camera, 0))
            .unwrap();
        let navigation = bridge
            .apply_camera_navigation_input(CameraNavigationInputEnvelope {
                camera,
                expected_revision: orbit.after.revision,
                tick: 8,
                input: CameraNavigationInput {
                    pan_right: 0.5,
                    pan_forward: -0.25,
                    yaw_delta_degrees: 20.0,
                    pitch_delta_degrees: -10.0,
                    zoom_delta: 3.0,
                    dt_seconds: 0.5,
                    pan_speed_units_per_second: 4.0,
                },
            })
            .unwrap();
        assert!(navigation.accepted);
        assert_eq!(navigation.after.revision, 2);
        assert_eq!(navigation.after.mode, CameraMode::Orbit);
        assert_eq!(navigation.after.distance, Some(5.0));
        assert_ne!(navigation.after.pivot, orbit.after.pivot);

        let top_down = bridge
            .apply_camera_mode_command(CameraModeCommand {
                camera,
                expected_revision: navigation.after.revision,
                target: CameraModeTarget::TopDown {
                    pivot: navigation.after.pivot.unwrap(),
                    height: 12.0,
                    min_height: 3.0,
                    max_height: 30.0,
                    yaw_degrees: 0.0,
                    pitch_degrees: -80.0,
                },
                transition: None,
                tick: 9,
            })
            .unwrap();
        assert!(top_down.accepted);
        assert_eq!(top_down.after.mode, CameraMode::TopDown);

        let returned = bridge
            .apply_camera_mode_command(CameraModeCommand {
                camera,
                expected_revision: top_down.after.revision,
                target: CameraModeTarget::FirstPerson {
                    pose: camera_request().initial_pose,
                },
                transition: Some(CameraTransitionSpec {
                    duration_milliseconds: 250,
                    easing: CameraTransitionEasing::Linear,
                }),
                tick: 10,
            })
            .unwrap();
        assert!(returned.accepted);
        assert_eq!(returned.after.mode, CameraMode::FirstPerson);
        assert_eq!(returned.after.pivot, None);
        assert_eq!(returned.after.distance, None);
        (orbit, navigation, top_down, returned)
    };

    assert_eq!(run(), run());
}

#[test]
fn invalid_camera_controller_inputs_fail_atomically() {
    let mut bridge = init_bridge();
    let camera = bridge.create_camera(camera_request()).unwrap().camera;

    let invalid_target = bridge
        .apply_camera_mode_command(CameraModeCommand {
            camera,
            expected_revision: 0,
            target: CameraModeTarget::Orbit {
                pivot: [0.0, 0.0, 0.0],
                distance: 1.0,
                min_distance: 2.0,
                max_distance: 10.0,
                yaw_degrees: 0.0,
                pitch_degrees: 0.0,
            },
            transition: None,
            tick: 1,
        })
        .unwrap();
    assert!(!invalid_target.accepted);
    assert_eq!(
        invalid_target.rejection,
        Some(CameraControllerRejection::InvalidTarget)
    );
    assert_eq!(invalid_target.before, invalid_target.after);

    let invalid_transition = bridge
        .apply_camera_mode_command(CameraModeCommand {
            transition: Some(CameraTransitionSpec {
                duration_milliseconds: 0,
                easing: CameraTransitionEasing::Linear,
            }),
            ..orbit_command(camera, 0)
        })
        .unwrap();
    assert!(!invalid_transition.accepted);
    assert_eq!(
        invalid_transition.rejection,
        Some(CameraControllerRejection::InvalidInput)
    );

    let first_person_navigation = bridge
        .apply_camera_navigation_input(CameraNavigationInputEnvelope {
            camera,
            expected_revision: 0,
            tick: 2,
            input: CameraNavigationInput {
                pan_right: 0.0,
                pan_forward: 0.0,
                yaw_delta_degrees: 0.0,
                pitch_delta_degrees: 0.0,
                zoom_delta: 0.0,
                dt_seconds: 1.0 / 60.0,
                pan_speed_units_per_second: 1.0,
            },
        })
        .unwrap();
    assert!(!first_person_navigation.accepted);
    assert_eq!(
        first_person_navigation.rejection,
        Some(CameraControllerRejection::IncompatibleMode)
    );
    assert_eq!(
        bridge
            .read_camera_controller_state(CameraControllerReadRequest { camera })
            .unwrap()
            .revision,
        0
    );
}
