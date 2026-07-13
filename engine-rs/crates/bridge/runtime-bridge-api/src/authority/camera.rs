use super::*;

const MAX_COLLISION_CAMERA_AXIS_TRAVEL: f32 = 256.0;
const MAX_CAMERA_TRANSITION_MILLISECONDS: u32 = 10_000;
const CAMERA_TERRAIN_CLEARANCE: f32 = 0.25;

impl EngineBridge {
    pub(super) fn collision_projection(&self, world: &VoxelWorld) -> CollisionProjection {
        CollisionProjection::build_with_offset(
            world,
            WorldVec::new(
                self.voxel.collision_world_offset[0],
                self.voxel.collision_world_offset[1],
                self.voxel.collision_world_offset[2],
            ),
        )
    }

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

    pub(super) fn initial_camera_controller(snapshot: CameraSnapshot) -> CameraControllerState {
        Self::camera_controller_state(0, CameraMode::FirstPerson, None, None, None, None, snapshot)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn camera_controller_state(
        revision: u64,
        mode: CameraMode,
        pivot: Option<[f32; 3]>,
        distance: Option<f32>,
        min_distance: Option<f32>,
        max_distance: Option<f32>,
        snapshot: CameraSnapshot,
    ) -> CameraControllerState {
        let state_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|{:?}|{:?}|{:?}",
                CAMERA_CONTROLLER_STATE_SCHEMA_VERSION,
                revision,
                mode,
                pivot.map(|value| value.map(f32::to_bits)),
                distance.map(f32::to_bits),
                min_distance.map(f32::to_bits),
                max_distance.map(f32::to_bits),
                snapshot.tick,
                snapshot.pose.position.map(f32::to_bits),
                snapshot.pose.yaw_degrees.to_bits(),
                snapshot.pose.pitch_degrees.to_bits(),
            ))
        );
        CameraControllerState {
            schema_version: CAMERA_CONTROLLER_STATE_SCHEMA_VERSION,
            revision,
            camera: snapshot.camera,
            mode,
            pivot,
            distance,
            min_distance,
            max_distance,
            snapshot,
            state_hash,
        }
    }

    pub(super) fn sync_first_person_controller(
        controller: &CameraControllerState,
        snapshot: CameraSnapshot,
    ) -> Result<CameraControllerState, CameraControllerRejection> {
        if controller.mode != CameraMode::FirstPerson {
            return Err(CameraControllerRejection::IncompatibleMode);
        }
        Ok(Self::camera_controller_state(
            controller.revision.saturating_add(1),
            CameraMode::FirstPerson,
            None,
            None,
            None,
            None,
            snapshot,
        ))
    }

    pub(super) fn validate_transition(
        transition: Option<CameraTransitionSpec>,
    ) -> Result<(), CameraControllerRejection> {
        if transition.is_some_and(|value| {
            value.duration_milliseconds == 0
                || value.duration_milliseconds > MAX_CAMERA_TRANSITION_MILLISECONDS
        }) {
            return Err(CameraControllerRejection::InvalidInput);
        }
        Ok(())
    }

    pub(super) fn resolve_camera_mode_target(
        &self,
        before: &CameraControllerState,
        target: CameraModeTarget,
        tick: u64,
    ) -> Result<(CameraControllerState, bool), CameraControllerRejection> {
        let (mode, pivot, requested_distance, min_distance, max_distance, pose) = match target {
            CameraModeTarget::FirstPerson { pose } => {
                if !Self::valid_pose(pose) {
                    return Err(CameraControllerRejection::InvalidTarget);
                }
                (CameraMode::FirstPerson, None, None, None, None, pose)
            }
            CameraModeTarget::Orbit {
                pivot,
                distance,
                min_distance,
                max_distance,
                yaw_degrees,
                pitch_degrees,
            } => {
                if !Self::valid_pivot_distance(pivot, distance, min_distance, max_distance)
                    || !yaw_degrees.is_finite()
                    || !pitch_degrees.is_finite()
                    || !(-89.0..=89.0).contains(&pitch_degrees)
                {
                    return Err(CameraControllerRejection::InvalidTarget);
                }
                let pose = Self::orbit_pose(pivot, distance, yaw_degrees, pitch_degrees);
                (
                    CameraMode::Orbit,
                    Some(pivot),
                    Some(distance),
                    Some(min_distance),
                    Some(max_distance),
                    pose,
                )
            }
            CameraModeTarget::TopDown {
                pivot,
                height,
                min_height,
                max_height,
                yaw_degrees,
                pitch_degrees,
            } => {
                if !Self::valid_pivot_distance(pivot, height, min_height, max_height)
                    || !yaw_degrees.is_finite()
                    || !pitch_degrees.is_finite()
                    || !(-89.0..=-30.0).contains(&pitch_degrees)
                {
                    return Err(CameraControllerRejection::InvalidTarget);
                }
                let pose = Self::top_down_pose(pivot, height, yaw_degrees, pitch_degrees);
                (
                    CameraMode::TopDown,
                    Some(pivot),
                    Some(height),
                    Some(min_height),
                    Some(max_height),
                    pose,
                )
            }
        };
        let (pose, actual_distance, terrain_constrained) =
            match (pivot, requested_distance, min_distance, mode) {
                (Some(pivot), Some(distance), Some(minimum), mode) => {
                    self.constrain_camera_to_terrain(pivot, pose, distance, minimum, mode)?
                }
                _ => (pose, requested_distance, false),
            };
        let snapshot = CameraSnapshot {
            tick,
            pose,
            basis: Self::basis_from_pose(pose),
            ..before.snapshot
        };
        Ok((
            Self::camera_controller_state(
                before.revision.saturating_add(1),
                mode,
                pivot,
                actual_distance,
                min_distance,
                max_distance,
                snapshot,
            ),
            terrain_constrained,
        ))
    }

    pub(super) fn resolve_camera_navigation(
        &self,
        before: &CameraControllerState,
        input: CameraNavigationInput,
        tick: u64,
    ) -> Result<(CameraControllerState, bool), CameraControllerRejection> {
        if before.mode == CameraMode::FirstPerson {
            return Err(CameraControllerRejection::IncompatibleMode);
        }
        if !Self::valid_navigation_input(input) {
            return Err(CameraControllerRejection::InvalidInput);
        }
        let mut pivot = before
            .pivot
            .ok_or(CameraControllerRejection::InvalidTarget)?;
        let distance = before
            .distance
            .ok_or(CameraControllerRejection::InvalidTarget)?;
        let minimum = before
            .min_distance
            .ok_or(CameraControllerRejection::InvalidTarget)?;
        let maximum = before
            .max_distance
            .ok_or(CameraControllerRejection::InvalidTarget)?;
        let yaw = before.snapshot.pose.yaw_degrees + input.yaw_delta_degrees;
        let pitch = match before.mode {
            CameraMode::Orbit => {
                (before.snapshot.pose.pitch_degrees + input.pitch_delta_degrees).clamp(-89.0, 89.0)
            }
            CameraMode::TopDown => {
                (before.snapshot.pose.pitch_degrees + input.pitch_delta_degrees).clamp(-89.0, -30.0)
            }
            CameraMode::FirstPerson => unreachable!(),
        };
        let pan_basis = Self::basis_from_pose(CameraPose {
            position: pivot,
            yaw_degrees: yaw,
            pitch_degrees: 0.0,
        });
        let pan_distance = input.dt_seconds * input.pan_speed_units_per_second;
        pivot[0] += (pan_basis.right[0] * input.pan_right
            + pan_basis.forward[0] * input.pan_forward)
            * pan_distance;
        pivot[2] += (pan_basis.right[2] * input.pan_right
            + pan_basis.forward[2] * input.pan_forward)
            * pan_distance;
        let requested_distance = (distance - input.zoom_delta).clamp(minimum, maximum);
        let requested_pose = match before.mode {
            CameraMode::Orbit => Self::orbit_pose(pivot, requested_distance, yaw, pitch),
            CameraMode::TopDown => Self::top_down_pose(pivot, requested_distance, yaw, pitch),
            CameraMode::FirstPerson => unreachable!(),
        };
        let (pose, actual_distance, terrain_constrained) = self.constrain_camera_to_terrain(
            pivot,
            requested_pose,
            requested_distance,
            minimum,
            before.mode,
        )?;
        let snapshot = CameraSnapshot {
            tick,
            pose,
            basis: Self::basis_from_pose(pose),
            ..before.snapshot
        };
        Ok((
            Self::camera_controller_state(
                before.revision.saturating_add(1),
                before.mode,
                Some(pivot),
                actual_distance,
                Some(minimum),
                Some(maximum),
                snapshot,
            ),
            terrain_constrained,
        ))
    }

    fn valid_pose(pose: CameraPose) -> bool {
        pose.position.iter().all(|value| value.is_finite())
            && pose.yaw_degrees.is_finite()
            && pose.pitch_degrees.is_finite()
            && (-89.0..=89.0).contains(&pose.pitch_degrees)
    }

    fn valid_pivot_distance(pivot: [f32; 3], value: f32, minimum: f32, maximum: f32) -> bool {
        pivot.iter().all(|item| item.is_finite())
            && value.is_finite()
            && minimum.is_finite()
            && maximum.is_finite()
            && minimum > 0.0
            && minimum <= value
            && value <= maximum
            && maximum <= 10_000.0
    }

    fn valid_navigation_input(input: CameraNavigationInput) -> bool {
        input.pan_right.is_finite()
            && input.pan_forward.is_finite()
            && input.yaw_delta_degrees.is_finite()
            && input.pitch_delta_degrees.is_finite()
            && input.zoom_delta.is_finite()
            && input.dt_seconds.is_finite()
            && input.pan_speed_units_per_second.is_finite()
            && input.dt_seconds >= 0.0
            && input.dt_seconds <= 1.0
            && input.pan_speed_units_per_second >= 0.0
            && input.pan_speed_units_per_second <= 1_000.0
    }

    fn orbit_pose(pivot: [f32; 3], distance: f32, yaw: f32, pitch: f32) -> CameraPose {
        let basis = Self::basis_from_pose(CameraPose {
            position: pivot,
            yaw_degrees: yaw,
            pitch_degrees: pitch,
        });
        CameraPose {
            position: [
                pivot[0] - basis.forward[0] * distance,
                pivot[1] - basis.forward[1] * distance,
                pivot[2] - basis.forward[2] * distance,
            ],
            yaw_degrees: yaw,
            pitch_degrees: pitch,
        }
    }

    fn top_down_pose(pivot: [f32; 3], height: f32, yaw: f32, pitch: f32) -> CameraPose {
        let basis = Self::basis_from_pose(CameraPose {
            position: pivot,
            yaw_degrees: yaw,
            pitch_degrees: pitch,
        });
        let radial_distance = height / (-basis.forward[1]).max(0.001);
        Self::orbit_pose(pivot, radial_distance, yaw, pitch)
    }

    fn constrain_camera_to_terrain(
        &self,
        pivot: [f32; 3],
        desired_pose: CameraPose,
        desired_metric: f32,
        minimum_metric: f32,
        mode: CameraMode,
    ) -> Result<(CameraPose, Option<f32>, bool), CameraControllerRejection> {
        let Some(world) = self.voxel.voxel.as_ref() else {
            return Ok((desired_pose, Some(desired_metric), false));
        };
        let delta = [
            desired_pose.position[0] - pivot[0],
            desired_pose.position[1] - pivot[1],
            desired_pose.position[2] - pivot[2],
        ];
        let radial_distance =
            (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
        if !radial_distance.is_finite() || radial_distance <= 0.0 {
            return Err(CameraControllerRejection::InvalidTarget);
        }
        let direction = [
            delta[0] / radial_distance,
            delta[1] / radial_distance,
            delta[2] / radial_distance,
        ];
        let origin_offset = 0.05_f32.min(radial_distance / 2.0);
        let origin = WorldPos::new(
            f64::from(pivot[0] + direction[0] * origin_offset),
            f64::from(pivot[1] + direction[1] * origin_offset),
            f64::from(pivot[2] + direction[2] * origin_offset),
        );
        let ray = Ray::new(
            origin,
            WorldVec::new(
                f64::from(direction[0]),
                f64::from(direction[1]),
                f64::from(direction[2]),
            ),
        );
        let projection = self.collision_projection(world);
        let Some(hit) = projection.raycast(ray, f64::from(radial_distance - origin_offset)) else {
            return Ok((desired_pose, Some(desired_metric), false));
        };
        let allowed_radial =
            (hit.distance as f32 + origin_offset - CAMERA_TERRAIN_CLEARANCE).max(0.0);
        let forward_down = (-Self::basis_from_pose(desired_pose).forward[1]).max(0.001);
        let allowed_metric = match mode {
            CameraMode::TopDown => allowed_radial * forward_down,
            CameraMode::Orbit | CameraMode::FirstPerson => allowed_radial,
        };
        if allowed_metric < minimum_metric {
            return Err(CameraControllerRejection::TerrainBlocked);
        }
        let pose = CameraPose {
            position: [
                pivot[0] + direction[0] * allowed_radial,
                pivot[1] + direction[1] * allowed_radial,
                pivot[2] + direction[2] * allowed_radial,
            ],
            ..desired_pose
        };
        Ok((pose, Some(allowed_metric), true))
    }

    pub(super) fn camera_transition_readout(
        from: CameraSnapshot,
        to: CameraSnapshot,
        spec: CameraTransitionSpec,
    ) -> CameraTransitionReadout {
        let transition_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{}|{:?}|{:?}|{}|{:?}",
                from.camera.raw(),
                from.tick,
                from.pose,
                to.pose,
                spec.duration_milliseconds,
                spec.easing
            ))
        );
        CameraTransitionReadout {
            from,
            to,
            duration_milliseconds: spec.duration_milliseconds,
            easing: spec.easing,
            transition_hash,
        }
    }

    pub(super) fn camera_mode_receipt(
        before: CameraControllerState,
        after: CameraControllerState,
        transition: Option<CameraTransitionReadout>,
        terrain_constrained: bool,
        rejection: Option<CameraControllerRejection>,
    ) -> CameraModeChangeReceipt {
        let accepted = rejection.is_none();
        let receipt_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{}|{}|{}|{}|{:?}|{:?}",
                accepted,
                before.state_hash,
                after.state_hash,
                transition
                    .as_ref()
                    .map(|value| value.transition_hash.as_str())
                    .unwrap_or("none"),
                terrain_constrained,
                rejection,
                after.mode,
            ))
        );
        CameraModeChangeReceipt {
            accepted,
            before,
            after,
            transition,
            terrain_constrained,
            rejection,
            receipt_hash,
        }
    }

    pub(super) fn camera_navigation_receipt(
        before: CameraControllerState,
        after: CameraControllerState,
        terrain_constrained: bool,
        rejection: Option<CameraControllerRejection>,
    ) -> CameraNavigationReceipt {
        let accepted = rejection.is_none();
        let receipt_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{}|{}|{}|{:?}",
                accepted, before.state_hash, after.state_hash, terrain_constrained, rejection,
            ))
        );
        CameraNavigationReceipt {
            accepted,
            before,
            after,
            terrain_constrained,
            rejection,
            receipt_hash,
        }
    }

    pub(super) fn apply_camera_mode_authority(
        &mut self,
        command: CameraModeCommand,
    ) -> BridgeResult<CameraModeChangeReceipt> {
        self.require_initialized("apply_camera_mode_command")?;
        let before = self
            .camera
            .camera_controllers
            .get(&command.camera.raw())
            .cloned()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::UnknownHandle,
                    "unknown camera handle",
                )
            })?;
        if command.expected_revision != before.revision {
            return Ok(Self::camera_mode_receipt(
                before.clone(),
                before,
                None,
                false,
                Some(CameraControllerRejection::StaleRevision),
            ));
        }
        if let Err(rejection) = Self::validate_transition(command.transition) {
            return Ok(Self::camera_mode_receipt(
                before.clone(),
                before,
                None,
                false,
                Some(rejection),
            ));
        }
        let (after, terrain_constrained) =
            match self.resolve_camera_mode_target(&before, command.target, command.tick) {
                Ok(result) => result,
                Err(rejection) => {
                    return Ok(Self::camera_mode_receipt(
                        before.clone(),
                        before,
                        None,
                        false,
                        Some(rejection),
                    ));
                }
            };
        let transition = command
            .transition
            .map(|spec| Self::camera_transition_readout(before.snapshot, after.snapshot, spec));
        self.camera
            .cameras
            .insert(command.camera.raw(), after.snapshot);
        self.camera
            .camera_controllers
            .insert(command.camera.raw(), after.clone());
        Ok(Self::camera_mode_receipt(
            before,
            after,
            transition,
            terrain_constrained,
            None,
        ))
    }

    pub(super) fn create_camera_authority(
        &mut self,
        request: CameraCreateRequest,
    ) -> BridgeResult<CameraSnapshot> {
        self.require_initialized("create_camera")?;
        Self::validate_create_request(&request)?;
        let camera = protocol_view::CameraHandle::new(self.camera.next_camera);
        self.camera.next_camera += 1;
        let snapshot = CameraSnapshot {
            camera,
            tick: 0,
            pose: request.initial_pose,
            basis: Self::basis_from_pose(request.initial_pose),
            projection: request.projection,
            viewport: request.viewport,
        };
        self.camera.cameras.insert(camera.raw(), snapshot);
        self.camera
            .camera_controllers
            .insert(camera.raw(), Self::initial_camera_controller(snapshot));
        Ok(snapshot)
    }

    pub(super) fn apply_first_person_camera_authority(
        &mut self,
        envelope: FirstPersonCameraInputEnvelope,
    ) -> BridgeResult<CameraSnapshot> {
        self.require_initialized("apply_first_person_camera_input")?;
        let prior = *self
            .camera
            .cameras
            .get(&envelope.camera.raw())
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::UnknownHandle,
                    "unknown camera handle",
                )
            })?;
        let input = envelope.input;
        let controller = self
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
                "first-person camera input requires firstPerson camera mode",
            ));
        }
        Self::validate_camera_input(input)?;
        let snapshot = Self::integrate_camera_snapshot(prior, input, envelope.tick);
        self.camera.cameras.insert(envelope.camera.raw(), snapshot);
        let controller =
            Self::sync_first_person_controller(&controller, snapshot).map_err(|_| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    "first-person camera input requires firstPerson camera mode",
                )
            })?;
        self.camera
            .camera_controllers
            .insert(envelope.camera.raw(), controller);
        Ok(snapshot)
    }

    pub(super) fn apply_camera_navigation_authority(
        &mut self,
        envelope: CameraNavigationInputEnvelope,
    ) -> BridgeResult<CameraNavigationReceipt> {
        self.require_initialized("apply_camera_navigation_input")?;
        let before = self
            .camera
            .camera_controllers
            .get(&envelope.camera.raw())
            .cloned()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::UnknownHandle,
                    "unknown camera handle",
                )
            })?;
        if envelope.expected_revision != before.revision {
            return Ok(Self::camera_navigation_receipt(
                before.clone(),
                before,
                false,
                Some(CameraControllerRejection::StaleRevision),
            ));
        }
        let (after, terrain_constrained) =
            match self.resolve_camera_navigation(&before, envelope.input, envelope.tick) {
                Ok(result) => result,
                Err(rejection) => {
                    return Ok(Self::camera_navigation_receipt(
                        before.clone(),
                        before,
                        false,
                        Some(rejection),
                    ));
                }
            };
        self.camera
            .cameras
            .insert(envelope.camera.raw(), after.snapshot);
        self.camera
            .camera_controllers
            .insert(envelope.camera.raw(), after.clone());
        Ok(Self::camera_navigation_receipt(
            before,
            after,
            terrain_constrained,
            None,
        ))
    }

    pub(super) fn read_camera_controller_authority(
        &self,
        request: CameraControllerReadRequest,
    ) -> BridgeResult<CameraControllerState> {
        self.require_initialized("read_camera_controller_state")?;
        self.camera
            .camera_controllers
            .get(&request.camera.raw())
            .cloned()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::UnknownHandle,
                    "unknown camera handle",
                )
            })
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

    pub(super) fn validate_collision_camera_movement(
        movement_mode: FirstPersonMovementMode,
        input: FirstPersonCameraInput,
    ) -> BridgeResult<()> {
        if movement_mode == FirstPersonMovementMode::Grounded && input.move_up != 0.0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "grounded camera input requires move_up to be zero; select freeFlight for vertical locomotion",
            ));
        }
        Ok(())
    }

    pub(super) fn validate_collision_camera_travel(delta: [f32; 3]) -> BridgeResult<()> {
        if delta
            .iter()
            .any(|component| component.abs() > MAX_COLLISION_CAMERA_AXIS_TRAVEL)
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "collision camera input exceeds the maximum axis travel of {MAX_COLLISION_CAMERA_AXIS_TRAVEL} units per command"
                ),
            ));
        }
        Ok(())
    }

    pub(super) fn resolve_collision_camera_pose(
        projection: &CollisionProjection,
        before: CameraPose,
        attempted: CameraPose,
        shape: CameraCollisionShape,
    ) -> BridgeResult<(CameraPose, Vec<CollisionAxis>)> {
        let delta = [
            attempted.position[0] - before.position[0],
            attempted.position[1] - before.position[1],
            attempted.position[2] - before.position[2],
        ];
        Self::validate_collision_camera_travel(delta)?;
        let mut after = CameraPose {
            position: before.position,
            yaw_degrees: attempted.yaw_degrees,
            pitch_degrees: attempted.pitch_degrees,
        };
        let mut blocked_axes = Vec::new();
        for (idx, axis) in [
            (0usize, CollisionAxis::X),
            (1, CollisionAxis::Y),
            (2, CollisionAxis::Z),
        ] {
            if delta[idx] == 0.0 {
                continue;
            }
            let (min, max) = Self::aabb_for_pose(after, shape);
            let mut translation = [0.0_f64; 3];
            translation[idx] = f64::from(delta[idx]);
            if projection.axis_swept_aabb_overlaps_solid(
                min,
                max,
                WorldVec::new(translation[0], translation[1], translation[2]),
            ) {
                blocked_axes.push(axis);
            } else {
                after.position[idx] += delta[idx];
            }
        }
        Ok((after, blocked_axes))
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

    pub(super) fn integrate_grounded_camera_snapshot(
        prior: CameraSnapshot,
        input: FirstPersonCameraInput,
        tick: u64,
    ) -> CameraSnapshot {
        let distance = input.dt_seconds * input.move_speed_units_per_second;
        let pose = CameraPose {
            position: prior.pose.position,
            yaw_degrees: prior.pose.yaw_degrees + input.yaw_delta_degrees,
            pitch_degrees: (prior.pose.pitch_degrees + input.pitch_delta_degrees)
                .clamp(-89.0, 89.0),
        };
        let movement_basis = Self::basis_from_pose(CameraPose {
            pitch_degrees: 0.0,
            ..pose
        });
        let pose = CameraPose {
            position: [
                prior.pose.position[0]
                    + (movement_basis.forward[0] * input.move_forward
                        + movement_basis.right[0] * input.move_right)
                        * distance,
                prior.pose.position[1],
                prior.pose.position[2]
                    + (movement_basis.forward[2] * input.move_forward
                        + movement_basis.right[2] * input.move_right)
                        * distance,
            ],
            ..pose
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
