use super::*;

pub(super) fn apply(
    bridge: &mut EngineBridge,
    envelope: CollisionConstrainedCameraInputEnvelope,
) -> BridgeResult<CameraCollisionSnapshot> {
    let world = bridge.voxel.voxel.as_ref().ok_or_else(|| {
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::NotInitialized,
            "apply_collision_constrained_camera_input called before initialize_engine",
        )
    })?;
    if envelope.grid != world.grid().id().raw() as u64 {
        return Err(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            "collision camera input targets an unknown grid",
        ));
    }
    EngineBridge::validate_camera_input(envelope.input)?;
    EngineBridge::validate_collision_camera_movement(envelope.movement_mode, envelope.input)?;
    EngineBridge::validate_collision_shape(envelope.shape)?;
    if envelope.policy.mode != CameraCollisionPolicyMode::AxisSeparableSlide
        || envelope.policy.max_iterations == 0
        || envelope.policy.max_iterations > 3
    {
        return Err(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            "only axis_separable_slide with max_iterations in 1..=3 is supported",
        ));
    }
    let before = *bridge
        .camera
        .cameras
        .get(&envelope.camera.raw())
        .ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                "unknown camera handle",
            )
        })?;
    let controller = bridge
        .camera
        .camera_controllers
        .get(&envelope.camera.raw())
        .cloned()
        .ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                "unknown camera controller",
            )
        })?;
    if controller.mode != CameraMode::FirstPerson {
        return Err(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            "collision-constrained input requires firstPerson camera mode",
        ));
    }
    let attempted = match envelope.movement_mode {
        FirstPersonMovementMode::Grounded => {
            EngineBridge::integrate_grounded_camera_snapshot(before, envelope.input, envelope.tick)
        }
        FirstPersonMovementMode::FreeFlight => {
            EngineBridge::integrate_camera_snapshot(before, envelope.input, envelope.tick)
        }
    };
    let projection = bridge.collision_projection(world);
    let (mut after_pose, mut blocked_axes) = EngineBridge::resolve_collision_camera_pose(
        &projection,
        before.pose,
        attempted.pose,
        envelope.shape,
    )?;
    let collision_identity = projection.identity(world);
    let collision_projection_hash = collision_identity.projection_hash_label();
    let collision_source_hash = collision_identity.source_hash_hex();
    if bridge.has_static_gameplay_runtime() && bridge.gameplay.fps_session.is_some() {
        let player = bridge
            .fps_session("apply_collision_constrained_camera_input")?
            .role_entity(FpsRuntimeRole::Player)
            .map_err(EngineBridge::fps_runtime_error)?;
        let entity_delta = [
            after_pose.position[0] - before.pose.position[0],
            after_pose.position[1] - before.pose.position[1],
            after_pose.position[2] - before.pose.position[2],
        ];
        let movement = bridge
            .with_static_gameplay_runtime_at_tick(
                "apply_collision_constrained_camera_input.entity_collision",
                envelope.tick,
                |host| host.move_actor_and_reconcile(player, entity_delta, envelope.tick),
            )?
            .expect("FPS project movement requires a static gameplay host");
        let (entity_position, entity_blocked) = match movement.movement.outcome {
            core_entity::MovementOutcome::Moved { to } => (to, [false; 3]),
            core_entity::MovementOutcome::Slid { to, blocked } => (to, blocked),
            core_entity::MovementOutcome::Blocked { at } => (
                at,
                entity_delta.map(|component| component.abs() > f32::EPSILON),
            ),
        };
        after_pose.position = entity_position.to_array();
        for (index, axis) in [CollisionAxis::X, CollisionAxis::Y, CollisionAxis::Z]
            .into_iter()
            .enumerate()
        {
            if entity_blocked[index] && !blocked_axes.contains(&axis) {
                blocked_axes.push(axis);
            }
        }
    }
    let after = CameraSnapshot {
        tick: envelope.tick,
        pose: after_pose,
        basis: EngineBridge::basis_from_pose(after_pose),
        ..before
    };
    bridge.camera.cameras.insert(envelope.camera.raw(), after);
    let accepted_controller = EngineBridge::sync_first_person_controller(&controller, after)
        .map_err(|_| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "collision-constrained input requires firstPerson camera mode",
            )
        })?;
    bridge
        .camera
        .camera_controllers
        .insert(envelope.camera.raw(), accepted_controller);
    let (min, max) = EngineBridge::aabb_for_pose(after.pose, envelope.shape);
    let correction = [
        after.pose.position[0] - attempted.pose.position[0],
        after.pose.position[1] - attempted.pose.position[1],
        after.pose.position[2] - attempted.pose.position[2],
    ];
    let movement_hash = format!(
        "fnv1a64:{}",
        EngineBridge::fnv1a64(&format!(
            "{}|{}|{:?}|{:?}|{:?}|{:?}|{}|{}",
            envelope.camera.raw(),
            envelope.tick,
            envelope.movement_mode,
            before.pose,
            attempted.pose,
            after.pose,
            collision_source_hash,
            collision_projection_hash
        ))
    );
    Ok(CameraCollisionSnapshot {
        camera: envelope.camera,
        tick: envelope.tick,
        before,
        attempted,
        after,
        collision: CameraCollisionEvidence {
            grid: envelope.grid,
            movement_mode: envelope.movement_mode,
            shape: envelope.shape,
            policy: envelope.policy,
            collided: !blocked_axes.is_empty(),
            blocked_axes,
            correction,
            queried_aabb: CollisionAabbEvidence {
                min: [min.x as f32, min.y as f32, min.z as f32],
                max: [max.x as f32, max.y as f32, max.z as f32],
            },
            collision_source_hash,
            collision_projection_hash,
        },
        movement_hash,
    })
}
