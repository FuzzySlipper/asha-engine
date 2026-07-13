use super::*;

impl RuntimeBridge for EngineBridge {
    fn initialize_engine(&mut self, config: EngineConfig) -> BridgeResult<EngineHandle> {
        initialization::initialize(self, config)
    }

    fn configure_input_session(
        &mut self,
        request: InputSessionConfigureRequest,
    ) -> BridgeResult<InputSessionSnapshot> {
        input::configure(self, request)
    }

    fn apply_input_context_command(
        &mut self,
        command: InputContextCommand,
    ) -> BridgeResult<InputContextChangeReceipt> {
        input::apply_context_command(self, command)
    }

    fn submit_raw_input(&self, sample: RawInputSample) -> BridgeResult<InputResolutionReceipt> {
        input::submit(self, sample)
    }

    fn replay_resolved_input_action(
        &mut self,
        record: RecordedInputAction,
    ) -> BridgeResult<InputActionReplayReceipt> {
        input::replay(self, record)
    }

    fn read_input_context_state(&self) -> BridgeResult<InputContextStackState> {
        input::read_context_state(self)
    }

    fn apply_time_control_command(
        &mut self,
        command: TimeControlCommand,
    ) -> BridgeResult<TimeControlReceipt> {
        time_control::apply(self, command)
    }

    fn read_time_control_state(&self) -> BridgeResult<TimeControlState> {
        time_control::read(self)
    }

    fn submit_commands(&mut self, batch: CommandBatch) -> BridgeResult<CommandResult> {
        self.submit_commands_with_voxel_history(batch)
    }

    fn pick_voxel(&self, ray: PickRay) -> BridgeResult<PickResult> {
        let world = self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "pick_voxel called before initialize_engine",
            )
        })?;
        // Fail closed on a ray that names a grid the runtime is not hosting, rather
        // than silently casting against the wrong (only) grid.
        if ray.grid != world.grid().id().raw() as u64 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "pick_voxel ray targets an unknown grid",
            ));
        }

        // Authority owns the raycast: build the collision projection from authority
        // voxel state and cast. The engine bridge currently rebuilds per pick; a
        // future authority optimization may cache the projection.
        let projection = self.collision_projection(world);
        let origin = WorldPos::new(ray.origin[0], ray.origin[1], ray.origin[2]);
        let dir = WorldVec::new(ray.direction[0], ray.direction[1], ray.direction[2]);
        match projection.raycast(Ray::new(origin, dir), ray.max_distance) {
            Some(hit) => Ok(PickResult::Hit(VoxelHit {
                grid: ray.grid,
                voxel: hit.voxel,
                chunk: hit.chunk,
                face: hit.face,
                point: [hit.point.x, hit.point.y, hit.point.z],
                distance: hit.distance,
            })),
            None => Ok(PickResult::Miss(PickRejection::NoHit)),
        }
    }

    fn apply_collision_constrained_camera_input(
        &mut self,
        envelope: CollisionConstrainedCameraInputEnvelope,
    ) -> BridgeResult<CameraCollisionSnapshot> {
        let world = self.voxel.as_ref().ok_or_else(|| {
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
        Self::validate_camera_input(envelope.input)?;
        Self::validate_collision_camera_movement(envelope.movement_mode, envelope.input)?;
        Self::validate_collision_shape(envelope.shape)?;
        if envelope.policy.mode != CameraCollisionPolicyMode::AxisSeparableSlide
            || envelope.policy.max_iterations == 0
            || envelope.policy.max_iterations > 3
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "only axis_separable_slide with max_iterations in 1..=3 is supported",
            ));
        }
        let before = *self.cameras.get(&envelope.camera.raw()).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                "unknown camera handle",
            )
        })?;
        let controller = self
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
                Self::integrate_grounded_camera_snapshot(before, envelope.input, envelope.tick)
            }
            FirstPersonMovementMode::FreeFlight => {
                Self::integrate_camera_snapshot(before, envelope.input, envelope.tick)
            }
        };
        let projection = self.collision_projection(world);
        let (after_pose, blocked_axes) = Self::resolve_collision_camera_pose(
            &projection,
            before.pose,
            attempted.pose,
            envelope.shape,
        )?;
        let after = CameraSnapshot {
            tick: envelope.tick,
            pose: after_pose,
            basis: Self::basis_from_pose(after_pose),
            ..before
        };
        self.cameras.insert(envelope.camera.raw(), after);
        let controller = Self::sync_first_person_controller(&controller, after).map_err(|_| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "collision-constrained input requires firstPerson camera mode",
            )
        })?;
        self.camera_controllers
            .insert(envelope.camera.raw(), controller);
        let (min, max) = Self::aabb_for_pose(after.pose, envelope.shape);
        let collision_identity = projection.identity(world);
        let collision_projection_hash = collision_identity.projection_hash_label();
        let collision_source_hash = collision_identity.source_hash_hex();
        let correction = [
            after.pose.position[0] - attempted.pose.position[0],
            after.pose.position[1] - attempted.pose.position[1],
            after.pose.position[2] - attempted.pose.position[2],
        ];
        let movement_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
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

    fn apply_generated_tunnel_to_runtime_world(
        &mut self,
        request: GeneratedTunnelRuntimeApplyRequest,
    ) -> BridgeResult<GeneratedTunnelRuntimeApplyReceipt> {
        self.require_initialized("apply_generated_tunnel_to_runtime_world")?;
        self.fps_session("apply_generated_tunnel_to_runtime_world")?;
        let config = match request.preset {
            GeneratedTunnelPreset::TinyEnclosed => {
                svc_levelgen::TunnelGeneratorConfig::tiny_enclosed(request.seed)
            }
        };
        let tunnel = svc_levelgen::generate_tunnel(config).map_err(|error| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("generated tunnel request was rejected: {error}"),
            )
        })?;
        let runtime_frame = tunnel.runtime_frame();
        let collision_world_offset = runtime_frame.world_offset.to_array();
        let projection = CollisionProjection::build_with_offset(
            &tunnel.world,
            WorldVec::new(
                collision_world_offset[0],
                collision_world_offset[1],
                collision_world_offset[2],
            ),
        );
        let collision_identity = projection.identity(&tunnel.world);
        let receipt = GeneratedTunnelRuntimeApplyReceipt {
            preset: request.preset,
            seed: request.seed,
            grid: tunnel.grid.id().raw() as u64,
            config_hash: format!("{:016x}", tunnel.record.config_hash),
            output_hash: format!("{:016x}", tunnel.record.output_hash),
            collision_source_hash: collision_identity.source_hash_hex(),
            collision_projection_hash: collision_identity.projection_hash_label(),
            runtime_frame: GeneratedTunnelRuntimeFrame {
                world_offset: collision_world_offset,
                playable_min: runtime_frame.playable_min.to_array(),
                playable_max: runtime_frame.playable_max.to_array(),
            },
        };
        self.reset_voxel_edit_history_with_collision_offset(tunnel.world, collision_world_offset);
        Ok(receipt)
    }

    fn select_voxel(
        &self,
        request: ScreenPointToPickRayRequest,
    ) -> BridgeResult<VoxelSelectionSnapshot> {
        let snapshot = *self.cameras.get(&request.camera.raw()).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                "unknown camera handle",
            )
        })?;
        let pick_ray = Self::pick_ray_snapshot(snapshot, request)?;
        let ray = PickRay {
            grid: pick_ray.grid,
            origin: pick_ray.origin,
            direction: pick_ray.direction,
            max_distance: pick_ray.max_distance,
        };
        let pick_result = self.pick_voxel(ray)?;
        let outcome = match pick_result {
            PickResult::Hit(_) => VoxelSelectionOutcome::Hit,
            PickResult::Miss(_) => VoxelSelectionOutcome::Miss,
        };
        let (selected_voxel, selected_face, edit_anchor) = match pick_result {
            PickResult::Hit(hit) => {
                let dir = match hit.face {
                    Face::PosX => Direction6::PosX,
                    Face::NegX => Direction6::NegX,
                    Face::PosY => Direction6::PosY,
                    Face::NegY => Direction6::NegY,
                    Face::PosZ => Direction6::PosZ,
                    Face::NegZ => Direction6::NegZ,
                };
                (
                    Some(hit.voxel),
                    Some(hit.face),
                    Some(hit.voxel.neighbor(dir)),
                )
            }
            PickResult::Miss(_) => (None, None, None),
        };
        let selection_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{:?}|{:?}|{:?}",
                pick_ray.ray_hash, pick_result, selected_voxel, edit_anchor
            ))
        );
        Ok(VoxelSelectionSnapshot {
            pick_ray,
            outcome,
            selected_voxel,
            selected_face,
            edit_anchor,
            selection_hash,
        })
    }

    fn read_voxel_mesh_evidence(
        &self,
        request: VoxelMeshEvidenceRequest,
    ) -> BridgeResult<VoxelMeshEvidenceSnapshot> {
        let world = self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "read_voxel_mesh_evidence called before initialize_engine",
            )
        })?;
        if request.grid != world.grid().id().raw() as u64 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "read_voxel_mesh_evidence request targets an unknown grid",
            ));
        }

        let mut coords = if request.chunks.is_empty() {
            world
                .resident_chunks()
                .map(|(coord, _)| coord)
                .collect::<Vec<_>>()
        } else {
            request.chunks
        };
        coords.sort();
        coords.dedup();

        let mut chunks = Vec::with_capacity(coords.len());
        let mut diagnostics = Vec::new();
        for coord in coords {
            let (evidence, mut diag) = Self::mesh_evidence_for(world, coord);
            chunks.push(evidence);
            diagnostics.append(&mut diag);
        }

        Ok(VoxelMeshEvidenceSnapshot {
            grid: request.grid,
            fixture_id: "basic-voxel-landscape-interaction".to_string(),
            voxel_state_hash: Self::voxel_state_hash(world),
            meshing_strategy: "visible-face".to_string(),
            chunks,
            diagnostics,
        })
    }

    fn plan_voxel_conversion(
        &mut self,
        request: VoxelConversionPlanRequest,
    ) -> BridgeResult<VoxelConversionPlan> {
        self.require_initialized("plan_voxel_conversion")?;
        let source = self.source_for_voxel_conversion(&request);
        let planned = svc_voxel_conversion::plan_conversion(&request, &source);
        let plan = planned.plan.clone();
        self.voxel_conversion_plan = Some(planned);
        self.voxel_conversion_evidence.clear();
        self.remember_voxel_conversion_evidence(plan.evidence.clone());
        Ok(plan)
    }

    fn register_voxel_conversion_source(
        &mut self,
        request: VoxelConversionSourceRegistrationRequest,
    ) -> BridgeResult<VoxelConversionSourceRegistration> {
        self.require_initialized("register_voxel_conversion_source")?;
        let source = match Self::static_mesh_source_from_registration(&request) {
            Ok(source) => source,
            Err(message) => {
                return Ok(Self::source_registration_diagnostic(
                    &request.source,
                    message,
                ));
            }
        };
        self.voxel_conversion_sources
            .insert(source.asset_id.clone(), source);
        self.voxel_conversion_source_metadata.insert(
            request.source.asset_id.clone(),
            Self::source_metadata_from_registration(&request),
        );
        self.voxel_conversion_plan = None;
        let evidence = vec![VoxelConversionEvidenceRef {
            kind: protocol_voxel_conversion::VoxelConversionEvidenceKind::SourceSnapshot,
            uri: format!(
                "asha://voxel-conversion/source/{}",
                request.source.asset_id.as_str()
            ),
            content_hash: request.source.source_hash.clone(),
        }];
        self.remember_voxel_conversion_evidence(evidence.clone());
        Ok(VoxelConversionSourceRegistration {
            source: request.source,
            registered: true,
            material_slots: request.material_slots,
            diagnostics: Vec::new(),
            evidence,
        })
    }

    fn register_voxel_conversion_mesh_asset(
        &mut self,
        request: VoxelConversionMeshAssetRegistrationRequest,
    ) -> BridgeResult<VoxelConversionSourceRegistration> {
        self.register_voxel_conversion_mesh_asset_authority(request)
    }

    fn import_voxel_conversion_mesh_source(
        &mut self,
        request: VoxelConversionMeshSourceImportRequest,
    ) -> BridgeResult<VoxelConversionMeshSourceImportReceipt> {
        self.import_voxel_conversion_mesh_source_authority(request)
    }

    fn read_voxel_conversion_source_metadata(
        &self,
        request: VoxelConversionSourceMetadataRequest,
    ) -> BridgeResult<VoxelConversionSourceMetadataReadout> {
        self.require_initialized("read_voxel_conversion_source_metadata")?;
        let Some(metadata) = self
            .voxel_conversion_source_metadata
            .get(&request.source.asset_id)
        else {
            return Ok(Self::missing_voxel_conversion_source_metadata(
                request,
                "voxel conversion source metadata is unavailable in current authority state",
            ));
        };
        if metadata.source != request.source {
            return Ok(Self::missing_voxel_conversion_source_metadata(
                request,
                "voxel conversion source metadata exists, but the requested source identity/hash does not match authority",
            ));
        }
        let latest_plan = self
            .voxel_conversion_plan
            .as_ref()
            .map(|planned| &planned.plan)
            .filter(|plan| plan.source == metadata.source);
        Ok(VoxelConversionSourceMetadataReadout {
            request,
            registered: true,
            source: Some(metadata.source.clone()),
            source_path: metadata.source_path.clone(),
            source_bounds: metadata.source_bounds,
            vertex_count: metadata.vertex_count,
            triangle_count: metadata.triangle_count,
            groups: metadata.groups.clone(),
            material_slots: metadata.material_slots.clone(),
            latest_plan_id: latest_plan.map(|plan| plan.plan_id.clone()),
            latest_plan_transform: latest_plan.map(|plan| plan.settings.transform),
            diagnostics: Vec::new(),
            evidence: metadata.evidence.clone(),
        })
    }

    fn preview_voxel_conversion(
        &mut self,
        request: VoxelConversionPreviewRequest,
    ) -> BridgeResult<VoxelConversionPreview> {
        self.require_initialized("preview_voxel_conversion")?;
        let planned = self.voxel_conversion_plan.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "preview_voxel_conversion called before a conversion plan exists",
            )
        })?;
        let preview = svc_voxel_conversion::preview_conversion(&request, planned);
        self.remember_voxel_conversion_evidence(preview.evidence.clone());
        Ok(preview)
    }

    fn apply_voxel_conversion(
        &mut self,
        request: VoxelConversionApplyRequest,
    ) -> BridgeResult<VoxelConversionReceipt> {
        self.require_initialized("apply_voxel_conversion")?;
        let planned = self.voxel_conversion_plan.clone().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "apply_voxel_conversion called before a conversion plan exists",
            )
        })?;
        let mut receipt = svc_voxel_conversion::apply_conversion(&request, &planned);
        if !receipt.applied {
            self.remember_voxel_conversion_evidence(receipt.evidence.clone());
            return Ok(receipt);
        }

        let target = match self.target_for_voxel_conversion(&planned.plan.target) {
            Some(target) => target,
            None => {
                self.remember_voxel_conversion_evidence(receipt.evidence.clone());
                return Ok(Self::rejected_voxel_conversion_receipt(
                    request.plan_id,
                    vec![Self::voxel_conversion_diagnostic(
                        VoxelConversionDiagnosticCode::ConversionReplayMismatch,
                        "target",
                        "conversion target is not registered in current authority state",
                    )],
                ));
            }
        };

        self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "apply_voxel_conversion called before initialize_engine",
            )
        })?;

        let Some(batch) = Self::conversion_commands(&planned)? else {
            return Ok(Self::rejected_voxel_conversion_receipt(
                request.plan_id,
                vec![Self::voxel_conversion_diagnostic(
                    VoxelConversionDiagnosticCode::ConversionReplayMismatch,
                    "output",
                    "conversion apply had no authority output to commit",
                )],
            ));
        };
        let mut candidate = self.voxel_conversion_target_candidate(&target, &planned)?;
        let prior_world = candidate.clone();
        let expected = batch.commands.len() as u32;
        let command_result =
            Self::apply_command_batch_to_world(&batch, &mut candidate, &self.materials)?;
        if command_result.accepted != expected || command_result.rejected != 0 {
            receipt = Self::rejected_voxel_conversion_receipt(
                request.plan_id,
                vec![Self::voxel_conversion_diagnostic(
                    VoxelConversionDiagnosticCode::ConversionReplayMismatch,
                    "voxel_command_apply",
                    format!(
                        "conversion output command apply accepted {} of {} commands and rejected {}",
                        command_result.accepted, expected, command_result.rejected
                    ),
                )],
            );
        } else {
            self.reset_voxel_edit_history(candidate);
            self.remember_voxel_model_info(&target, &planned, &receipt, &prior_world);
        }
        self.remember_voxel_conversion_evidence(receipt.evidence.clone());
        Ok(receipt)
    }

    fn export_voxel_conversion_evidence(
        &self,
        evidence: Vec<VoxelConversionEvidenceRef>,
    ) -> BridgeResult<Vec<VoxelConversionEvidenceRef>> {
        self.require_initialized("export_voxel_conversion_evidence")?;
        for requested in &evidence {
            if !self.voxel_conversion_evidence.contains(requested) {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "voxel conversion evidence ref is not available from current authority state: {}",
                        requested.uri
                    ),
                ));
            }
        }
        Ok(evidence)
    }

    fn read_voxel_model_info(
        &self,
        request: VoxelModelInfoRequest,
    ) -> BridgeResult<VoxelModelInfoReadout> {
        self.require_initialized("read_voxel_model_info")?;
        let key = Self::voxel_model_key(request.grid, &request.volume_asset_id);
        if !self.voxel_conversion_targets.contains_key(&key) {
            return Ok(Self::voxel_model_missing_readout(
                request,
                "voxel model request targets an unknown conversion target",
            ));
        }
        let Some(info) = self.voxel_model_infos.get(&key) else {
            return Ok(Self::voxel_model_missing_readout(
                request,
                "voxel model is not resident in current authority state; apply a conversion first",
            ));
        };
        Ok(VoxelModelInfoReadout {
            request: request.clone(),
            resident: true,
            model_id: info.model_id.clone(),
            volume_asset_id: info.volume_asset_id.clone(),
            grid: info.grid,
            bounds: info.bounds,
            voxel_count: info.voxel_count,
            material_counts: if request.include_material_counts {
                info.material_counts.clone()
            } else {
                Vec::new()
            },
            source: Some(info.source.clone()),
            latest_plan_id: Some(info.latest_plan_id.clone()),
            latest_output_hash: Some(info.latest_output_hash.clone()),
            session_hash: info.session_hash.clone(),
            replay_hash: info.replay_hash.clone(),
            evidence: info.evidence.clone(),
            diagnostics: Vec::new(),
        })
    }

    fn read_voxel_model_window(
        &self,
        request: VoxelModelWindowRequest,
    ) -> BridgeResult<VoxelModelWindowReadout> {
        self.require_initialized("read_voxel_model_window")?;
        let key = Self::voxel_model_key(request.grid, &request.volume_asset_id);
        if !self.voxel_conversion_targets.contains_key(&key) {
            return Ok(Self::voxel_model_window_missing_readout(
                request,
                "voxel model window request targets an unknown conversion target",
            ));
        }
        let Some(info) = self.voxel_model_infos.get(&key) else {
            return Ok(Self::voxel_model_window_missing_readout(
                request,
                "voxel model is not resident in current authority state; apply a conversion first",
            ));
        };
        let Some(world) = self
            .voxel
            .as_ref()
            .filter(|world| world.grid().id().raw() as u64 == request.grid)
        else {
            return Ok(Self::voxel_model_window_missing_readout(
                request,
                "voxel authority has no resident grid for the requested model window",
            ));
        };
        let diagnostics = Self::voxel_model_window_request_diagnostics(&request);
        if !diagnostics.is_empty() {
            return Ok(Self::voxel_model_window_readout(
                request,
                info,
                0,
                Vec::new(),
                diagnostics,
            ));
        }
        let scanned_voxel_count =
            Self::voxel_model_window_volume(request.bounds).expect("validated window volume");
        let material_filter = request
            .material_filter
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let mut samples = Vec::new();
        for z in request.bounds.min.z..=request.bounds.max.z {
            for y in request.bounds.min.y..=request.bounds.max.y {
                for x in request.bounds.min.x..=request.bounds.max.x {
                    let coord = VoxelCoord::new(x, y, z);
                    let value = Self::voxel_value_at(world, coord);
                    let material = value.material().map(|material| material.raw());
                    if !material_filter.is_empty()
                        && !material.is_some_and(|material| material_filter.contains(&material))
                    {
                        continue;
                    }
                    if material.is_none() && (!request.include_empty || !material_filter.is_empty())
                    {
                        continue;
                    }
                    samples.push(VoxelModelWindowSample {
                        coord: Self::protocol_voxel_coord(coord),
                        occupied: value.is_solid(),
                        material,
                    });
                }
            }
        }
        Ok(Self::voxel_model_window_readout(
            request,
            info,
            scanned_voxel_count,
            samples,
            Vec::new(),
        ))
    }

    fn export_voxel_volume_asset(
        &self,
        request: VoxelVolumeAssetExportRequest,
    ) -> BridgeResult<VoxelVolumeAssetExportReceipt> {
        self.require_initialized("export_voxel_volume_asset")?;
        let key = Self::voxel_model_key(request.grid, &request.volume_asset_id);
        let Some(info) = self.voxel_model_infos.get(&key) else {
            return Ok(Self::rejected_voxel_volume_asset_export(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::RuntimeModelUnavailable,
                    "runtimeModel",
                    "voxel model is not resident in current authority state; apply a conversion before export",
                )],
            ));
        };
        if let Some(expected) = &request.expected_session_hash {
            if expected != &info.session_hash {
                return Ok(Self::rejected_voxel_volume_asset_export(
                    request,
                    vec![Self::voxel_asset_diagnostic(
                        VoxelAssetDiagnosticCode::StaleRuntimeSnapshot,
                        "expectedSessionHash",
                        "export request expected a different runtime model session hash",
                    )],
                ));
            }
        }
        let Some(target) = self.voxel_conversion_targets.get(&key) else {
            return Ok(Self::rejected_voxel_volume_asset_export(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::RuntimeModelUnavailable,
                    "target",
                    "voxel model target is no longer registered in current authority state",
                )],
            ));
        };
        if target.spec.id().raw() as u64 != request.grid
            || target.volume_asset_id != request.volume_asset_id
        {
            return Ok(Self::rejected_voxel_volume_asset_export(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::StaleRuntimeSnapshot,
                    "runtimeModel",
                    "resident model readout does not match the registered runtime target",
                )],
            ));
        }
        let sparse_runs = Self::sparse_runs_for_resident_voxels(&info.resident_voxels);
        if request.max_sparse_runs == 0 || sparse_runs.len() as u64 > request.max_sparse_runs {
            let message = format!(
                "export requires {} sparse run(s), exceeding request limit {}",
                sparse_runs.len(),
                request.max_sparse_runs
            );
            return Ok(Self::rejected_voxel_volume_asset_export(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::ExportLimitExceeded,
                    "maxSparseRuns",
                    message,
                )],
            ));
        }
        let material_palette = match Self::material_palette_for_resident_export(info) {
            Ok(palette) => palette,
            Err(diagnostics) => {
                return Ok(Self::rejected_voxel_volume_asset_export(
                    request,
                    diagnostics,
                ));
            }
        };
        let Some(bounds) = info.bounds else {
            return Ok(Self::rejected_voxel_volume_asset_export(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::RuntimeModelUnavailable,
                    "bounds",
                    "conversion output has no bounds to export",
                )],
            ));
        };

        let origin = target.spec.origin_world().to_array();
        let provenance = Self::voxel_model_export_provenance(info, &request);
        let asset = VoxelVolumeAsset {
            asset_id: request.target_asset_id.clone(),
            schema_version: VOXEL_ASSET_SCHEMA_VERSION,
            media_type: VOXEL_ASSET_MEDIA_TYPE.to_string(),
            grid: VoxelAssetGrid {
                origin,
                cell_size: target.spec.voxel_size(),
                coordinate_system: svc_voxel_asset::VOXEL_ASSET_COORDINATE_SYSTEM.to_string(),
            },
            bounds: Self::voxel_asset_bounds(bounds),
            representation: VoxelAssetRepresentation {
                kind: VoxelAssetRepresentationKind::SparseRuns,
                sparse_runs,
            },
            material_palette,
            provenance,
            authoring: VoxelAssetAuthoringMetadata {
                label: request.label.clone(),
                created_by: request.created_by.clone(),
                source_tool: request.source_tool.clone(),
            },
            validation_diagnostics: Vec::new(),
            content_hashes: VoxelAssetContentHashes {
                canonical_json: String::new(),
                voxel_data: String::new(),
            },
        };
        let asset = svc_voxel_asset::with_computed_hashes(&asset);
        let report = svc_voxel_asset::validate_asset(&asset);
        if !report.is_valid() {
            return Ok(Self::rejected_voxel_volume_asset_export(
                request,
                report.diagnostics,
            ));
        }
        let canonical_json = svc_voxel_asset::encode_asset(&asset).map_err(|report| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!(
                    "validated voxel volume asset failed canonical encode with {} diagnostic(s)",
                    report.diagnostics.len()
                ),
            )
        })?;
        Ok(VoxelVolumeAssetExportReceipt {
            request,
            exported: true,
            canonical_json_hash: Some(asset.content_hashes.canonical_json.clone()),
            voxel_data_hash: Some(asset.content_hashes.voxel_data.clone()),
            asset: Some(asset),
            canonical_json: Some(canonical_json),
            diagnostics: Vec::new(),
        })
    }

    fn save_voxel_volume_asset(
        &self,
        request: VoxelVolumeAssetSaveRequest,
    ) -> BridgeResult<VoxelVolumeAssetSaveReceipt> {
        self.require_initialized("save_voxel_volume_asset")?;
        let diagnostics = Self::voxel_asset_save_request_diagnostics(&request);
        if !diagnostics.is_empty() {
            return Ok(Self::rejected_voxel_volume_asset_save(request, diagnostics));
        }

        let export = self.export_voxel_volume_asset(request.export_request.clone())?;
        if !export.exported {
            return Ok(Self::rejected_voxel_volume_asset_save(
                request,
                export.diagnostics,
            ));
        }

        let Some(asset) = export.asset else {
            return Ok(Self::rejected_voxel_volume_asset_save(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::RuntimeModelUnavailable,
                    "asset",
                    "export reported success but returned no stored voxel asset",
                )],
            ));
        };
        let Some(canonical_json) = export.canonical_json else {
            return Ok(Self::rejected_voxel_volume_asset_save(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::RuntimeModelUnavailable,
                    "canonicalJson",
                    "export reported success but returned no canonical JSON payload",
                )],
            ));
        };
        let canonical_json_hash = asset.content_hashes.canonical_json.clone();
        let voxel_data_hash = asset.content_hashes.voxel_data.clone();
        let mut diagnostics = Vec::new();
        if let Some(expected) = &request.expected_canonical_json_hash {
            if expected != &canonical_json_hash {
                diagnostics.push(Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::ContentHashMismatch,
                    "expectedCanonicalJsonHash",
                    "save request expected a different exported canonical JSON hash",
                ));
            }
        }
        if let Some(expected) = &request.expected_voxel_data_hash {
            if expected != &voxel_data_hash {
                diagnostics.push(Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::ContentHashMismatch,
                    "expectedVoxelDataHash",
                    "save request expected a different exported voxel data hash",
                ));
            }
        }
        if !diagnostics.is_empty() {
            return Ok(Self::rejected_voxel_volume_asset_save(request, diagnostics));
        }

        let key = Self::voxel_model_key(
            request.export_request.grid,
            &request.export_request.volume_asset_id,
        );
        let Some(info) = self.voxel_model_infos.get(&key) else {
            return Ok(Self::rejected_voxel_volume_asset_save(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::RuntimeModelUnavailable,
                    "runtimeModel",
                    "voxel model readback disappeared before save transaction could be packaged",
                )],
            ));
        };
        let voxel_count = asset
            .representation
            .sparse_runs
            .iter()
            .map(|run| u64::from(run.length))
            .sum::<u64>();
        let diff = VoxelVolumeAssetStoredDiff {
            project_bundle: request.target_project_bundle.clone(),
            asset_id: asset.asset_id.clone(),
            asset_path: request.target_asset_path.clone(),
            operation: if request.expected_existing_canonical_json_hash.is_some() {
                "replace".to_string()
            } else {
                "create".to_string()
            },
            previous_canonical_json_hash: request.expected_existing_canonical_json_hash.clone(),
            next_canonical_json_hash: canonical_json_hash.clone(),
            next_voxel_data_hash: voxel_data_hash.clone(),
            representation_kind: asset.representation.kind,
            sparse_run_count: asset.representation.sparse_runs.len() as u64,
            voxel_count,
            material_count: asset.material_palette.len() as u64,
            provenance_count: asset.provenance.len() as u64,
            runtime_session_hash: info.session_hash.clone(),
        };
        Ok(VoxelVolumeAssetSaveReceipt {
            request,
            saved: true,
            diff: Some(diff),
            asset: Some(asset),
            canonical_json: Some(canonical_json),
            canonical_json_hash: Some(canonical_json_hash),
            voxel_data_hash: Some(voxel_data_hash),
            diagnostics: Vec::new(),
        })
    }

    fn update_voxel_volume_asset_palette(
        &self,
        request: VoxelVolumeAssetPaletteUpdateRequest,
    ) -> BridgeResult<VoxelVolumeAssetPaletteUpdateReceipt> {
        self.update_voxel_volume_asset_palette_authority(request)
    }

    fn initialize_voxel_volume_authoring(
        &mut self,
        request: VoxelVolumeAuthoringInitializeRequest,
    ) -> BridgeResult<VoxelVolumeAuthoringInitializeReceipt> {
        self.initialize_voxel_volume_authoring_authority(request)
    }

    fn load_voxel_volume_asset(
        &mut self,
        request: VoxelVolumeAssetLoadRequest,
    ) -> BridgeResult<VoxelVolumeAssetLoadReceipt> {
        self.require_initialized("load_voxel_volume_asset")?;
        let asset = &request.asset;
        let report = svc_voxel_asset::validate_asset(asset);
        if !report.is_valid() {
            return Ok(Self::rejected_voxel_volume_asset_load(
                &request,
                report.diagnostics,
            ));
        }
        let target = match self.voxel_asset_load_target(&request) {
            Ok(target) => target,
            Err(diagnostic) => {
                return Ok(Self::rejected_voxel_volume_asset_load(
                    &request,
                    vec![diagnostic],
                ));
            }
        };
        let batch = Self::voxel_asset_load_commands(asset, target.spec.id())?;
        let mut candidate = self.voxel_asset_load_candidate(&target, request.replace_existing);
        let prior_world = candidate.clone();
        Self::ensure_candidate_chunks_for_asset(asset, &target.spec, &mut candidate);
        let expected = batch.commands.len() as u32;
        let command_result =
            Self::apply_command_batch_to_world(&batch, &mut candidate, &self.materials)?;
        if command_result.accepted != expected || command_result.rejected != 0 {
            return Ok(Self::rejected_voxel_volume_asset_load(
                &request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::RuntimeModelUnavailable,
                    "voxelCommandApply",
                    format!(
                        "stored voxel asset command apply accepted {} of {} commands and rejected {}",
                        command_result.accepted, expected, command_result.rejected
                    ),
                )],
            ));
        }

        let key = Self::voxel_model_key(target.spec.id().raw() as u64, &target.volume_asset_id);
        let existing = if request.replace_existing {
            None
        } else {
            self.voxel_model_infos.get(&key)
        };
        let info = Self::loaded_voxel_asset_info(&request, &target, &prior_world, existing);
        let receipt =
            Self::voxel_volume_asset_load_receipt(&request, &target, &info, true, Vec::new());
        self.reset_voxel_edit_history(candidate);
        self.voxel_conversion_targets.insert(
            Self::voxel_model_key(info.grid, &info.volume_asset_id),
            target,
        );
        self.voxel_model_infos.insert(key.clone(), info);
        self.active_voxel_model = Some(key);
        Ok(receipt)
    }

    fn unload_voxel_volume_asset(
        &mut self,
        request: VoxelVolumeAssetUnloadRequest,
    ) -> BridgeResult<VoxelVolumeAssetUnloadReceipt> {
        self.unload_voxel_volume_asset_authority(request)
    }

    fn validate_voxel_annotation_layer(
        &self,
        request: VoxelAnnotationLayerValidationRequest,
    ) -> BridgeResult<VoxelAnnotationLayerValidationReport> {
        self.validate_voxel_annotation_layer_authority(request)
    }

    fn load_voxel_annotation_layer(
        &mut self,
        request: VoxelAnnotationLayerLoadRequest,
    ) -> BridgeResult<VoxelAnnotationLayerLoadReceipt> {
        self.load_voxel_annotation_layer_authority(request)
    }

    fn read_voxel_annotation_query(
        &self,
        request: VoxelAnnotationQueryRequest,
    ) -> BridgeResult<VoxelAnnotationQueryReadout> {
        self.read_voxel_annotation_query_authority(request)
    }

    fn apply_voxel_annotation_edit(
        &mut self,
        request: VoxelAnnotationEditRequest,
    ) -> BridgeResult<VoxelAnnotationEditReceipt> {
        self.apply_voxel_annotation_edit_authority(request)
    }

    fn export_voxel_annotation_layer(
        &self,
        request: VoxelAnnotationLayerExportRequest,
    ) -> BridgeResult<VoxelAnnotationLayerExportReceipt> {
        self.export_voxel_annotation_layer_authority(request)
    }

    fn read_voxel_edit_history(
        &self,
        request: VoxelEditHistoryReadRequest,
    ) -> BridgeResult<VoxelEditHistorySummary> {
        self.read_voxel_edit_history_authority(request)
    }

    fn preview_voxel_edit_revert(
        &self,
        request: VoxelEditHistoryRevertRequest,
    ) -> BridgeResult<VoxelEditHistoryRevertReceipt> {
        self.preview_voxel_edit_revert_authority(request)
    }

    fn apply_voxel_edit_revert(
        &mut self,
        request: VoxelEditHistoryRevertRequest,
    ) -> BridgeResult<VoxelEditHistoryRevertReceipt> {
        self.apply_voxel_edit_revert_authority(request)
    }

    fn undo_voxel_edit(
        &mut self,
        request: VoxelEditHistoryUndoRequest,
    ) -> BridgeResult<VoxelEditHistoryUndoReceipt> {
        self.undo_voxel_edit_authority(request)
    }

    fn redo_voxel_edit(
        &mut self,
        request: VoxelEditHistoryRedoRequest,
    ) -> BridgeResult<VoxelEditHistoryRedoReceipt> {
        self.redo_voxel_edit_authority(request)
    }

    fn read_model_material_preview(
        &self,
        request: ModelMaterialPreviewRequest,
    ) -> BridgeResult<ModelMaterialPreviewSnapshot> {
        self.read_model_material_preview_authority(request)
    }

    fn read_scene_object_snapshot(&self) -> BridgeResult<SceneObjectSnapshotDto> {
        self.read_scene_object_snapshot_authority()
    }

    fn apply_scene_object_command(
        &mut self,
        request: SceneObjectCommandRequestDto,
    ) -> BridgeResult<SceneObjectCommandResultDto> {
        self.apply_scene_object_command_authority(request)
    }

    fn load_fps_runtime_session(
        &mut self,
        request: FpsRuntimeSessionLoadRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        self.require_initialized("load_fps_runtime_session")?;
        let input = Self::convert_fps_load_request(&request)?;
        let game_rule_modules = Self::verify_game_rule_modules(&request.game_rule_modules)?;
        let loaded = load_fps_project_bundle(input).map_err(Self::fps_runtime_error)?;
        // Commit only after the full authority bootstrap succeeds.
        self.fps_session = Some(loaded);
        self.fps_seed = Some(request);
        self.fps_epoch = self.fps_epoch.saturating_add(1);
        self.game_rule_modules = game_rule_modules;
        self.reset_presentation_projection();
        Self::fps_snapshot(
            self.fps_session.as_ref().expect("just committed"),
            self.fps_epoch,
        )
    }

    fn read_fps_runtime_session(&self) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        self.require_initialized("read_fps_runtime_session")?;
        Self::fps_snapshot(
            self.fps_session("read_fps_runtime_session")?,
            self.fps_epoch,
        )
    }

    fn apply_fps_primary_fire(
        &mut self,
        request: FpsPrimaryFireRequest,
    ) -> BridgeResult<FpsPrimaryFireResult> {
        self.require_initialized("apply_fps_primary_fire")?;
        let tick = request.tick;
        let shooter_role = request
            .shooter_role
            .map(Self::fps_runtime_role)
            .unwrap_or(FpsRuntimeRole::Player);
        let target_role = request
            .target_role
            .map(Self::fps_runtime_role)
            .unwrap_or(FpsRuntimeRole::Enemy);
        let ray = Self::ray_from_primary_fire(request)?;
        let world = self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "apply_fps_primary_fire called before initialize_engine",
            )
        })?;
        let projection = self.collision_projection(world);
        let receipt = self
            .fps_session_mut("apply_fps_primary_fire")?
            .apply_primary_fire_for_roles(&projection, ray, tick, shooter_role, target_role, 0)
            .map_err(Self::fps_runtime_error)?;
        let result = Self::primary_fire_result(receipt);
        self.project_primary_fire_feedback(request, &result)?;
        Ok(result)
    }

    fn read_projection_frame(&self, cursor: u64) -> BridgeResult<RuntimeProjectionFrame> {
        self.require_initialized("read_projection_frame")?;
        let frame = self.projection_frame.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "projection frame is unavailable after initialization",
            )
        })?;
        if cursor > frame.authority_tick {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "projection cursor is ahead of the latest authority tick",
            ));
        }
        Ok(frame.clone())
    }

    fn invoke_game_extension_weapon_effect(
        &mut self,
        request: GameExtensionWeaponEffectInvocationRequest,
    ) -> BridgeResult<GameExtensionWeaponEffectInvocationResult> {
        self.require_initialized("invoke_game_extension_weapon_effect")?;
        let module =
            Self::resolve_weapon_effect_game_rule_module(&self.game_rule_modules, &request.hook)?;
        let transformed = match rule_gameplay_fabric::run_legacy_weapon_effect_transform(
            &module,
            &request.hook,
        ) {
            Ok(outcome) => outcome,
            Err(rule_gameplay_fabric::LegacyWeaponEffectTransformError::ModuleRejected(
                diagnostic,
            )) => {
                let hook_receipt = rejected_receipt(&request.hook, diagnostic);
                let replay_evidence =
                    Self::extension_replay_evidence(&hook_receipt, "rejectedByModule", Vec::new());
                return Ok(GameExtensionWeaponEffectInvocationResult {
                    hook_receipt,
                    replay_evidence,
                    primary_fire: None,
                });
            }
            Err(rule_gameplay_fabric::LegacyWeaponEffectTransformError::DecisionRejected(
                receipt,
            )) => {
                let detail = receipt
                    .diagnostics
                    .first()
                    .map(|diagnostic| diagnostic.message.clone())
                    .unwrap_or_else(|| "compatibility Transform was rejected".to_owned());
                let diagnostic = Self::game_extension_diagnostic(
                    GameExtensionDiagnosticCode::InvalidProposal,
                    "compatibility.transform",
                    detail,
                );
                let hook_receipt = rejected_receipt(&request.hook, diagnostic);
                let replay_evidence =
                    Self::extension_replay_evidence(&hook_receipt, "invalidProposal", Vec::new());
                return Ok(GameExtensionWeaponEffectInvocationResult {
                    hook_receipt,
                    replay_evidence,
                    primary_fire: None,
                });
            }
            Err(error) => {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!("legacy weapon Transform compatibility failed: {error}"),
                ));
            }
        };
        let hook_receipt = proposed_receipt(
            &request.hook,
            transformed.proposal,
            vec![
                GameExtensionTraceEntry {
                    step: 1,
                    code: "module.proposed_damage_modifier".to_string(),
                    message: "resolved Rust game rule module returned a typed damage modifier"
                        .to_string(),
                    refs: vec![
                        module.manifest().module_ref.module_id.clone(),
                        module.manifest().module_ref.version.clone(),
                        module.manifest().module_ref.contract_hash.clone(),
                    ],
                },
                GameExtensionTraceEntry {
                    step: 2,
                    code: "gameplayFabric.transformAccepted".to_string(),
                    message: "legacy weapon proposal passed the common Transform coordinator and combat owner route".to_string(),
                    refs: vec![
                        transformed.decision_receipt.registry_digest.clone(),
                        transformed.decision_receipt.final_workspace_hash.clone(),
                        transformed.decision_receipt.receipt_hash.clone(),
                    ],
                },
            ],
        );
        let damage_delta = transformed.damage_delta;
        let primary_fire = request.primary_fire;
        let shooter_role = primary_fire
            .shooter_role
            .map(Self::fps_runtime_role)
            .unwrap_or(FpsRuntimeRole::Player);
        let target_role = primary_fire
            .target_role
            .map(Self::fps_runtime_role)
            .unwrap_or(FpsRuntimeRole::Enemy);
        let ray = Self::ray_from_primary_fire(primary_fire)?;
        let world = self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "invoke_game_extension_weapon_effect called before initialize_engine",
            )
        })?;
        let projection = self.collision_projection(world);
        let receipt = self
            .fps_session_mut("invoke_game_extension_weapon_effect")?
            .apply_primary_fire_for_roles(
                &projection,
                ray,
                primary_fire.tick,
                shooter_role,
                target_role,
                damage_delta,
            )
            .map_err(Self::fps_runtime_error)?;
        let primary_fire = Self::primary_fire_result(receipt);
        self.project_primary_fire_feedback(request.primary_fire, &primary_fire)?;
        let replay_evidence = Self::extension_replay_evidence(
            &hook_receipt,
            "accepted",
            vec![format!("fnv1a64:{:016x}", primary_fire.replay_hash)],
        );
        Ok(GameExtensionWeaponEffectInvocationResult {
            hook_receipt,
            replay_evidence,
            primary_fire: Some(primary_fire),
        })
    }

    fn validate_game_rule_catalog(
        &mut self,
        catalog: GameRuleCatalog,
    ) -> BridgeResult<GameRuleCatalogValidationReceipt> {
        self.require_initialized("validate_game_rule_catalog")?;
        let report = validate_catalog(&catalog);
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{}|catalogValidation",
                catalog.catalog.catalog_id, report.catalog_hash
            ))
        );
        self.game_rule_recent_trace = report.trace.clone();
        self.game_rule_recent_replay_hashes
            .push(replay_hash.clone());
        Ok(GameRuleCatalogValidationReceipt {
            accepted: report.accepted(),
            catalog_hash: report.catalog_hash,
            diagnostics: report.diagnostics,
            trace: report.trace,
            evidence: vec![GameRuleEvidenceRef {
                kind: GameRuleEvidenceKind::CatalogValidation,
                uri: format!(
                    "asha://game-rules/catalog-validation/{}",
                    catalog.catalog.catalog_id
                ),
                content_hash: replay_hash,
            }],
        })
    }

    fn submit_game_rule_effect_intent(
        &mut self,
        input: GameRuleEffectIntentRequest,
    ) -> BridgeResult<GameRuleResolutionReceipt> {
        self.require_initialized("submit_game_rule_effect_intent")?;
        let receipt = resolve_protocol_request(&input.request, &input.catalog);
        self.game_rule_recent_trace = receipt.trace.clone();
        self.game_rule_recent_replay_hashes
            .push(receipt.replay_hash.clone());
        if receipt.accepted {
            for modifier in &receipt.applied_modifiers {
                if let Some(existing) = self.game_rule_active_modifiers.iter_mut().find(|active| {
                    active.modifier_id == modifier.modifier_id
                        && active.source == modifier.source
                        && active.target == modifier.target
                }) {
                    *existing = modifier.clone();
                } else {
                    self.game_rule_active_modifiers.push(modifier.clone());
                }
            }
        }
        Ok(receipt)
    }

    fn read_game_rule_runtime_readout(&self) -> BridgeResult<GameRuleRuntimeReadout> {
        self.require_initialized("read_game_rule_runtime_readout")?;
        Ok(GameRuleRuntimeReadout {
            backend: "engine_bridge_rust".to_string(),
            authority_surface: "runtime_session.game_rules.v0".to_string(),
            active_modifiers: self.game_rule_active_modifiers.clone(),
            recent_trace: self.game_rule_recent_trace.clone(),
            recent_replay_hashes: self.game_rule_recent_replay_hashes.clone(),
            latest_replay_hash: self.game_rule_recent_replay_hashes.last().cloned(),
        })
    }

    fn restart_fps_runtime_session(
        &mut self,
        request: FpsRuntimeSessionRestartRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        self.require_initialized("restart_fps_runtime_session")?;
        if request.expected_epoch != self.fps_epoch {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "restart expected epoch {} but current epoch is {}",
                    request.expected_epoch, self.fps_epoch
                ),
            ));
        }
        let seed = self.fps_seed.clone().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "restart_fps_runtime_session called before load_fps_runtime_session",
            )
        })?;
        let input = Self::convert_fps_load_request(&seed)?;
        let loaded = load_fps_project_bundle(input).map_err(Self::fps_runtime_error)?;
        self.fps_session = Some(loaded);
        self.fps_epoch = self.fps_epoch.saturating_add(1);
        self.reset_presentation_projection();
        Self::fps_snapshot(
            self.fps_session.as_ref().expect("just restarted"),
            self.fps_epoch,
        )
    }

    fn read_fps_encounter_director(
        &self,
        lifecycle: FpsEncounterLifecycleInput,
    ) -> BridgeResult<FpsEncounterDirectorSnapshot> {
        self.require_initialized("read_fps_encounter_director")?;
        Ok(Self::encounter_snapshot(
            self.fps_session("read_fps_encounter_director")?,
            lifecycle,
        ))
    }

    fn apply_fps_encounter_transition(
        &mut self,
        request: FpsEncounterTransitionRequest,
    ) -> BridgeResult<FpsEncounterTransitionResult> {
        self.require_initialized("apply_fps_encounter_transition")?;
        let action = Self::encounter_action(&request.action)?;
        let lifecycle = request.lifecycle;
        let rule_lifecycle = Self::bridge_encounter_lifecycle(lifecycle.clone());
        let receipt = self
            .fps_session_mut("apply_fps_encounter_transition")?
            .apply_encounter_transition(&request.preset_id, action, &rule_lifecycle)
            .map_err(Self::fps_runtime_error)?;
        Ok(Self::encounter_transition_result(receipt, lifecycle))
    }

    fn step_simulation(&mut self, input: StepInputEnvelope) -> BridgeResult<StepResult> {
        time_control::step(self, input)
    }

    fn create_camera(&mut self, request: CameraCreateRequest) -> BridgeResult<CameraSnapshot> {
        self.create_camera_authority(request)
    }

    fn apply_camera_mode_command(
        &mut self,
        command: CameraModeCommand,
    ) -> BridgeResult<CameraModeChangeReceipt> {
        self.apply_camera_mode_authority(command)
    }

    fn apply_camera_navigation_input(
        &mut self,
        envelope: CameraNavigationInputEnvelope,
    ) -> BridgeResult<CameraNavigationReceipt> {
        self.apply_camera_navigation_authority(envelope)
    }

    fn read_camera_controller_state(
        &self,
        request: CameraControllerReadRequest,
    ) -> BridgeResult<CameraControllerState> {
        self.read_camera_controller_authority(request)
    }

    fn apply_first_person_camera_input(
        &mut self,
        envelope: FirstPersonCameraInputEnvelope,
    ) -> BridgeResult<CameraSnapshot> {
        self.apply_first_person_camera_authority(envelope)
    }

    fn apply_enemy_direct_nav_movement(
        &mut self,
        request: EnemyDirectNavMovementRequest,
    ) -> BridgeResult<EnemyDirectNavMovementResult> {
        self.require_initialized("apply_enemy_direct_nav_movement")?;
        let entity = Self::enemy_entity_id(request.entity)?;
        if self.fps_session.is_some() {
            let receipt = self
                .fps_session_mut("apply_enemy_direct_nav_movement")?
                .apply_autonomous_enemy_direct_nav_movement(
                    entity,
                    request.target.to_array(),
                    request.max_step_units,
                )
                .map_err(Self::fps_runtime_error)?;
            return Ok(EnemyDirectNavMovementResult {
                entity: receipt.entity.raw(),
                authority_source: EnemyDirectNavAuthoritySource::RustEntityStore,
                from: receipt.navigation.from,
                target: receipt.navigation.target,
                next_waypoint: receipt.navigation.next_waypoint,
                distance_units: receipt.navigation.distance_units,
                reached: receipt.navigation.reached,
                path_hash: receipt.navigation.path_hash,
                transform_hash: Self::transform_hash(receipt.entity, receipt.transform),
                projection_changed: receipt.projection_changed,
            });
        }

        let entities = &mut self.entities;
        let (authority_source, current_transform) =
            Self::seed_or_read_enemy_transform(entities, entity, request.seed_position)?;
        let from = current_transform.translation;
        let nav = propose_direct_nav_movement(DirectNavMovementRequest {
            from,
            target: request.target,
            max_step_units: request.max_step_units,
        })
        .map_err(|err| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "enemy direct-nav movement rejected by svc-pathfinding: {}",
                    EnemyDirectNavMovementError::Navigation(err).label()
                ),
            )
        })?;
        let next_transform = EntityTransform {
            translation: nav.next_waypoint,
            ..current_transform
        };
        let transform_event = entities
            .apply_transform(TransformCommand::Set {
                id: entity,
                transform: next_transform,
            })
            .map_err(|err| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "enemy direct-nav movement rejected by core-entity: {}",
                        EnemyDirectNavMovementError::Transform(err).label()
                    ),
                )
            })?;
        Ok(EnemyDirectNavMovementResult {
            entity: entity.raw(),
            authority_source,
            from,
            target: nav.target,
            next_waypoint: nav.next_waypoint,
            distance_units: nav.distance_units,
            reached: nav.reached,
            path_hash: nav.path_hash,
            transform_hash: Self::transform_hash(entity, transform_event.transform),
            projection_changed: transform_event.projection_changed,
        })
    }

    fn read_camera_projection(
        &self,
        request: CameraProjectionRequest,
    ) -> BridgeResult<CameraProjectionSnapshot> {
        self.require_initialized("read_camera_projection")?;
        let snapshot = *self.cameras.get(&request.camera.raw()).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                "unknown camera handle",
            )
        })?;
        let viewport = request.viewport.unwrap_or(snapshot.viewport);
        Self::validate_viewport(viewport)?;
        Ok(Self::projection_snapshot(snapshot, viewport))
    }

    fn get_buffer(&self, handle: RuntimeBufferHandle) -> BridgeResult<RuntimeBufferView<'_>> {
        self.buffers.view(handle)
    }

    fn release_buffer(&mut self, handle: RuntimeBufferHandle) -> BridgeResult<()> {
        self.buffers.dispose(handle)
    }

    fn load_project_bundle(
        &mut self,
        request: ProjectBundleLoadRequest,
    ) -> BridgeResult<CompositionStatus> {
        self.load_project_bundle_authority(request)
    }

    fn save_project_bundle(&mut self) -> BridgeResult<ProjectBundleSaveSummary> {
        self.save_project_bundle_authority()
    }

    fn get_project_bundle_composition_status(&self) -> BridgeResult<CompositionStatus> {
        self.project_bundle_composition_status_authority()
    }

    fn unload_project_bundle(&mut self) -> BridgeResult<()> {
        self.unload_project_bundle_authority()
    }
}
