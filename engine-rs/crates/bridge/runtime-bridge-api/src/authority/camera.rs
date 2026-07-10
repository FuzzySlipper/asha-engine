use super::*;

impl EngineBridge {
    pub(super) fn basis_from_pose(pose: protocol_view::CameraPose) -> protocol_view::CameraBasis {
        let yaw = pose.yaw_degrees.to_radians();
        let pitch = pose.pitch_degrees.to_radians();
        let cp = pitch.cos();
        let sp = pitch.sin();
        let sy = yaw.sin();
        let cy = yaw.cos();
        protocol_view::CameraBasis {
            forward: [sy * cp, sp, -cy * cp],
            right: [cy, 0.0, sy],
            up: [-sy * sp, cp, cy * sp],
        }
    }

    pub(super) fn validate_viewport(viewport: protocol_view::ViewportSize) -> BridgeResult<()> {
        if viewport.width == 0 || viewport.height == 0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "viewport dimensions must be positive",
            ));
        }
        Ok(())
    }

    pub(super) fn validate_create_request(request: &CameraCreateRequest) -> BridgeResult<()> {
        Self::validate_viewport(request.viewport)?;
        if !(request.projection.fov_y_degrees.is_finite()
            && request.projection.near.is_finite()
            && request.projection.far.is_finite())
            || request.projection.fov_y_degrees <= 0.0
            || request.projection.fov_y_degrees >= 180.0
            || request.projection.near <= 0.0
            || request.projection.far <= request.projection.near
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "invalid perspective projection parameters",
            ));
        }
        if !request.initial_pose.position.iter().all(|v| v.is_finite())
            || !request.initial_pose.yaw_degrees.is_finite()
            || !request.initial_pose.pitch_degrees.is_finite()
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "camera pose values must be finite",
            ));
        }
        Ok(())
    }

    pub(super) fn matrix_key(values: &[f32]) -> String {
        values
            .iter()
            .map(|v| format!("{v:.3}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub(super) fn fnv1a64(text: &str) -> String {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in text.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }

    pub(super) fn multiply_matrix4(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
        let mut out = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += a[k * 4 + row] * b[col * 4 + k];
                }
                out[col * 4 + row] = sum;
            }
        }
        out
    }

    pub(super) fn projection_snapshot(
        snapshot: CameraSnapshot,
        viewport: protocol_view::ViewportSize,
    ) -> CameraProjectionSnapshot {
        let right = snapshot.basis.right;
        let up = snapshot.basis.up;
        let forward = snapshot.basis.forward;
        let position = snapshot.pose.position;
        let dot_right = right[0] * position[0] + right[1] * position[1] + right[2] * position[2];
        let dot_up = up[0] * position[0] + up[1] * position[1] + up[2] * position[2];
        let dot_forward =
            forward[0] * position[0] + forward[1] * position[1] + forward[2] * position[2];
        let view_matrix = [
            right[0],
            up[0],
            -forward[0],
            0.0,
            right[1],
            up[1],
            -forward[1],
            0.0,
            right[2],
            up[2],
            -forward[2],
            0.0,
            -dot_right,
            -dot_up,
            dot_forward,
            1.0,
        ];
        let aspect = viewport.width as f32 / viewport.height as f32;
        let f = 1.0 / (snapshot.projection.fov_y_degrees.to_radians() / 2.0).tan();
        let near = snapshot.projection.near;
        let far = snapshot.projection.far;
        let projection_matrix = [
            f / aspect,
            0.0,
            0.0,
            0.0,
            0.0,
            f,
            0.0,
            0.0,
            0.0,
            0.0,
            (far + near) / (near - far),
            -1.0,
            0.0,
            0.0,
            (2.0 * far * near) / (near - far),
            0.0,
        ];
        let view_projection_matrix = Self::multiply_matrix4(projection_matrix, view_matrix);
        let mut hash_values = Vec::with_capacity(48);
        hash_values.extend_from_slice(&view_matrix);
        hash_values.extend_from_slice(&projection_matrix);
        hash_values.extend_from_slice(&view_projection_matrix);
        let projection_hash = format!("fnv1a64:{}", Self::fnv1a64(&Self::matrix_key(&hash_values)));
        CameraProjectionSnapshot {
            camera: snapshot.camera,
            tick: snapshot.tick,
            pose: snapshot.pose,
            basis: snapshot.basis,
            projection: snapshot.projection,
            viewport,
            view_matrix,
            projection_matrix,
            view_projection_matrix,
            projection_hash,
        }
    }

    pub(super) fn validate_camera_input(input: FirstPersonCameraInput) -> BridgeResult<()> {
        let finite = input.move_forward.is_finite()
            && input.move_right.is_finite()
            && input.move_up.is_finite()
            && input.yaw_delta_degrees.is_finite()
            && input.pitch_delta_degrees.is_finite()
            && input.dt_seconds.is_finite()
            && input.move_speed_units_per_second.is_finite();
        if !finite || input.dt_seconds < 0.0 || input.move_speed_units_per_second < 0.0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "camera input values must be finite; dt_seconds and move_speed_units_per_second must be non-negative",
            ));
        }
        Ok(())
    }

    pub(super) fn integrate_camera_snapshot(
        prior: CameraSnapshot,
        input: FirstPersonCameraInput,
        tick: u64,
    ) -> CameraSnapshot {
        let distance = input.dt_seconds * input.move_speed_units_per_second;
        let basis = prior.basis;
        let pose = CameraPose {
            position: [
                prior.pose.position[0]
                    + (basis.forward[0] * input.move_forward
                        + basis.right[0] * input.move_right
                        + basis.up[0] * input.move_up)
                        * distance,
                prior.pose.position[1]
                    + (basis.forward[1] * input.move_forward
                        + basis.right[1] * input.move_right
                        + basis.up[1] * input.move_up)
                        * distance,
                prior.pose.position[2]
                    + (basis.forward[2] * input.move_forward
                        + basis.right[2] * input.move_right
                        + basis.up[2] * input.move_up)
                        * distance,
            ],
            yaw_degrees: prior.pose.yaw_degrees + input.yaw_delta_degrees,
            pitch_degrees: (prior.pose.pitch_degrees + input.pitch_delta_degrees)
                .clamp(-89.0, 89.0),
        };
        CameraSnapshot {
            tick,
            pose,
            basis: Self::basis_from_pose(pose),
            ..prior
        }
    }

    pub(super) fn aabb_for_pose(
        pose: CameraPose,
        shape: CameraCollisionShape,
    ) -> (WorldPos, WorldPos) {
        let p = pose.position;
        let h = shape.half_extents;
        (
            WorldPos::new(
                (p[0] - h[0]) as f64,
                (p[1] - h[1]) as f64,
                (p[2] - h[2]) as f64,
            ),
            WorldPos::new(
                (p[0] + h[0]) as f64,
                (p[1] + h[1]) as f64,
                (p[2] + h[2]) as f64,
            ),
        )
    }

    pub(super) fn validate_collision_shape(shape: CameraCollisionShape) -> BridgeResult<()> {
        if !shape.half_extents.iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "collision shape half_extents must be finite positive values",
            ));
        }
        Ok(())
    }

    pub(super) fn collision_projection_hash(
        world: &VoxelWorld,
        projection: &CollisionProjection,
    ) -> String {
        let chunks = projection
            .collider_chunks()
            .map(|coord| format!("{},{},{}", coord.x, coord.y, coord.z))
            .collect::<Vec<_>>()
            .join(";");
        let key = format!(
            "{}|v{}|n{}|{}",
            Self::voxel_state_hash(world),
            projection.version(),
            projection.collider_count(),
            chunks
        );
        format!("fnv1a64:{}", Self::fnv1a64(&key))
    }

    pub(super) fn screen_point_to_normalized(
        point: ScreenPoint,
        viewport: ViewportSize,
    ) -> BridgeResult<(f32, f32)> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "screen point coordinates must be finite",
            ));
        }
        match point.space {
            ScreenPointSpace::Normalized01 => Ok((point.x, point.y)),
            ScreenPointSpace::Pixel => Ok((
                point.x / viewport.width as f32,
                point.y / viewport.height as f32,
            )),
        }
    }

    pub(super) fn pick_ray_snapshot(
        snapshot: CameraSnapshot,
        request: ScreenPointToPickRayRequest,
    ) -> BridgeResult<PickRaySnapshot> {
        let viewport = request.viewport.unwrap_or(snapshot.viewport);
        Self::validate_viewport(viewport)?;
        if !request.max_distance.is_finite() || request.max_distance <= 0.0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "max_distance must be finite and positive",
            ));
        }
        let (sx, sy) = Self::screen_point_to_normalized(request.screen_point, viewport)?;
        if !(0.0..=1.0).contains(&sx) || !(0.0..=1.0).contains(&sy) {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "screen point must be inside the viewport",
            ));
        }
        let ndc_x = sx * 2.0 - 1.0;
        let ndc_y = 1.0 - sy * 2.0;
        let aspect = viewport.width as f32 / viewport.height as f32;
        let tan_y = (snapshot.projection.fov_y_degrees.to_radians() / 2.0).tan();
        let tan_x = tan_y * aspect;
        let f = snapshot.basis.forward;
        let r = snapshot.basis.right;
        let u = snapshot.basis.up;
        let raw = [
            f[0] + r[0] * ndc_x * tan_x + u[0] * ndc_y * tan_y,
            f[1] + r[1] * ndc_x * tan_x + u[1] * ndc_y * tan_y,
            f[2] + r[2] * ndc_x * tan_x + u[2] * ndc_y * tan_y,
        ];
        let len = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2]).sqrt();
        if !len.is_finite() || len <= 0.0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "derived pick ray direction is invalid",
            ));
        }
        let dir = [raw[0] / len, raw[1] / len, raw[2] / len];
        let ray = PickRay {
            grid: request.grid,
            origin: [
                snapshot.pose.position[0] as f64,
                snapshot.pose.position[1] as f64,
                snapshot.pose.position[2] as f64,
            ],
            direction: [dir[0] as f64, dir[1] as f64, dir[2] as f64],
            max_distance: request.max_distance,
        };
        let projection_hash = Self::projection_snapshot(snapshot, viewport).projection_hash;
        let ray_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{}|{:.6},{:.6},{:.6}|{:.6},{:.6},{:.6}|{:.6}|{}",
                snapshot.camera.raw(),
                request.grid,
                ray.origin[0],
                ray.origin[1],
                ray.origin[2],
                ray.direction[0],
                ray.direction[1],
                ray.direction[2],
                ray.max_distance,
                projection_hash
            ))
        );
        Ok(PickRaySnapshot {
            camera: snapshot.camera,
            tick: snapshot.tick,
            grid: request.grid,
            screen_point: request.screen_point,
            origin: ray.origin,
            direction: ray.direction,
            max_distance: ray.max_distance,
            camera_projection_hash: projection_hash,
            ray_hash,
        })
    }
}
