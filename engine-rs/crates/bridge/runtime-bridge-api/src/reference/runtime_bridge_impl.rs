use super::*;

impl RuntimeBridge for ReferenceBridge {
    fn initialize_engine(&mut self, config: EngineConfig) -> BridgeResult<EngineHandle> {
        let handle = EngineHandle::new(config.seed);
        self.engine = Some(handle);
        // Deterministic: seed buffer is the first provider handle (0), so the
        // boundary buffer verbs below operate on the real lifetime model.
        self.buffers.reset();
        let seed_handle = self.buffers.create(
            buffer_provider::BufferKind::Opaque,
            buffer_provider::BufferLifetime::Manual,
            None,
            config.seed.to_le_bytes().to_vec(),
        );
        debug_assert_eq!(seed_handle.raw(), 0);

        // Stand up the voxel authority for the launch/edit loop: a resident origin
        // chunk so edits land, plus the launch material catalog. Start clean so a
        // later submit's dirty marking is observable.
        let world = Self::launch_world();
        self.reset_voxel_edit_history(world);
        self.materials = MaterialCatalog::new([1, 2, 3].into_iter().map(VoxelMaterialId::new));
        self.cameras.clear();
        self.next_camera = 1;
        self.fps_session = None;
        self.fps_seed = None;
        self.fps_epoch = 0;
        self.game_rule_modules.clear();
        self.game_rule_active_modifiers.clear();
        self.game_rule_recent_trace.clear();
        self.game_rule_recent_replay_hashes.clear();
        let (sources, source_metadata) = Self::seeded_voxel_conversion_authority()?;
        self.voxel_conversion_sources = sources;
        self.voxel_conversion_source_metadata = source_metadata;
        self.voxel_conversion_targets = Self::seeded_voxel_conversion_targets();
        self.voxel_conversion_plan = None;
        self.voxel_conversion_evidence.clear();
        self.voxel_model_infos.clear();
        self.voxel_annotation_layers.clear();

        Ok(handle)
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
        // voxel state and cast. (The reference bridge rebuilds per pick; a native
        // bridge can cache the projection — this stays the correctness reference.)
        let projection = CollisionProjection::build(world);
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
        let attempted = Self::integrate_camera_snapshot(before, envelope.input, envelope.tick);
        let projection = CollisionProjection::build(world);
        let mut after_pose = CameraPose {
            position: before.pose.position,
            yaw_degrees: attempted.pose.yaw_degrees,
            pitch_degrees: attempted.pose.pitch_degrees,
        };
        let delta = [
            attempted.pose.position[0] - before.pose.position[0],
            attempted.pose.position[1] - before.pose.position[1],
            attempted.pose.position[2] - before.pose.position[2],
        ];
        let mut blocked_axes = Vec::new();
        for (idx, axis) in [
            (0usize, CollisionAxis::X),
            (1, CollisionAxis::Y),
            (2, CollisionAxis::Z),
        ] {
            if delta[idx] == 0.0 {
                continue;
            }
            let mut candidate = after_pose;
            candidate.position[idx] += delta[idx];
            let (min, max) = Self::aabb_for_pose(candidate, envelope.shape);
            if projection.aabb_overlaps_solid(min, max) {
                blocked_axes.push(axis);
            } else {
                after_pose.position[idx] = candidate.position[idx];
            }
        }
        let after = CameraSnapshot {
            tick: envelope.tick,
            pose: after_pose,
            basis: Self::basis_from_pose(after_pose),
            ..before
        };
        self.cameras.insert(envelope.camera.raw(), after);
        let (min, max) = Self::aabb_for_pose(after.pose, envelope.shape);
        let collision_projection_hash = Self::collision_projection_hash(world, &projection);
        let collision_source_hash = Self::voxel_state_hash(world);
        let correction = [
            after.pose.position[0] - attempted.pose.position[0],
            after.pose.position[1] - attempted.pose.position[1],
            after.pose.position[2] - attempted.pose.position[2],
        ];
        let movement_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{}|{:?}|{:?}|{:?}|{}|{}",
                envelope.camera.raw(),
                envelope.tick,
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
        self.require_initialized("register_voxel_conversion_mesh_asset")?;
        let source = match Self::static_mesh_source_from_project_mesh_asset(&request) {
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
            Self::source_metadata_from_project_mesh_asset(&request),
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
            material_slots: request.mesh_asset.material_slots,
            diagnostics: Vec::new(),
            evidence,
        })
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
            self.remember_voxel_model_info(&target, &planned, &receipt);
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
        let Some(planned) = &self.voxel_conversion_plan else {
            return Ok(Self::rejected_voxel_volume_asset_export(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::RuntimeModelUnavailable,
                    "conversionOutput",
                    "current authority state no longer has complete conversion output for export",
                )],
            ));
        };
        let Some(output) = &planned.output else {
            return Ok(Self::rejected_voxel_volume_asset_export(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::RuntimeModelUnavailable,
                    "conversionOutput",
                    "conversion output is incomplete and cannot be exported as a stored asset",
                )],
            ));
        };
        let Some(target) = self.target_for_voxel_conversion(&planned.plan.target) else {
            return Ok(Self::rejected_voxel_volume_asset_export(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::RuntimeModelUnavailable,
                    "target",
                    "conversion target is no longer registered in current authority state",
                )],
            ));
        };
        if target.spec.id().raw() as u64 != request.grid
            || target.volume_asset_id != request.volume_asset_id
            || info.latest_plan_id != planned.plan.plan_id
            || info.latest_output_hash != output.output_hash
        {
            return Ok(Self::rejected_voxel_volume_asset_export(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::StaleRuntimeSnapshot,
                    "runtimeModel",
                    "resident model readout does not match the current conversion output snapshot",
                )],
            ));
        }
        let sparse_runs = Self::sparse_runs_for_conversion_output(output);
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
        let material_palette = match Self::material_palette_for_conversion_export(planned, output) {
            Ok(palette) => palette,
            Err(diagnostics) => {
                return Ok(Self::rejected_voxel_volume_asset_export(
                    request,
                    diagnostics,
                ));
            }
        };
        let Some(bounds) = output.bounds else {
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
        let mut provenance = info
            .evidence
            .iter()
            .map(|evidence| VoxelAssetProvenanceRef {
                kind: VoxelAssetProvenanceKind::Converted,
                uri: evidence.uri.clone(),
                content_hash: evidence.content_hash.clone(),
            })
            .collect::<Vec<_>>();
        provenance.push(VoxelAssetProvenanceRef {
            kind: VoxelAssetProvenanceKind::RuntimeExport,
            uri: format!(
                "asha://runtime-session/voxel-volume-export/{}",
                request.target_asset_id
            ),
            content_hash: format!(
                "fnv1a64:{}",
                Self::fnv1a64(&format!(
                    "voxel-volume-export|{}|{}|{}|{}",
                    request.target_asset_id,
                    info.session_hash,
                    info.replay_hash,
                    output.output_hash
                ))
            ),
        });
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

        let info = Self::loaded_voxel_asset_info(&request, &target);
        let receipt =
            Self::voxel_volume_asset_load_receipt(&request, &target, &info, true, Vec::new());
        self.reset_voxel_edit_history(candidate);
        self.voxel_conversion_targets.insert(
            Self::voxel_model_key(info.grid, &info.volume_asset_id),
            target,
        );
        self.voxel_model_infos.insert(
            Self::voxel_model_key(info.grid, &info.volume_asset_id),
            info,
        );
        Ok(receipt)
    }

    fn validate_voxel_annotation_layer(
        &self,
        request: VoxelAnnotationLayerValidationRequest,
    ) -> BridgeResult<VoxelAnnotationLayerValidationReport> {
        self.validate_voxel_annotation_layer_reference(request)
    }

    fn load_voxel_annotation_layer(
        &mut self,
        request: VoxelAnnotationLayerLoadRequest,
    ) -> BridgeResult<VoxelAnnotationLayerLoadReceipt> {
        self.load_voxel_annotation_layer_reference(request)
    }

    fn read_voxel_annotation_query(
        &self,
        request: VoxelAnnotationQueryRequest,
    ) -> BridgeResult<VoxelAnnotationQueryReadout> {
        self.read_voxel_annotation_query_reference(request)
    }

    fn apply_voxel_annotation_edit(
        &mut self,
        request: VoxelAnnotationEditRequest,
    ) -> BridgeResult<VoxelAnnotationEditReceipt> {
        self.apply_voxel_annotation_edit_reference(request)
    }

    fn export_voxel_annotation_layer(
        &self,
        request: VoxelAnnotationLayerExportRequest,
    ) -> BridgeResult<VoxelAnnotationLayerExportReceipt> {
        self.export_voxel_annotation_layer_reference(request)
    }

    fn read_voxel_edit_history(
        &self,
        request: VoxelEditHistoryReadRequest,
    ) -> BridgeResult<VoxelEditHistorySummary> {
        self.read_voxel_edit_history_reference(request)
    }

    fn preview_voxel_edit_revert(
        &self,
        request: VoxelEditHistoryRevertRequest,
    ) -> BridgeResult<VoxelEditHistoryRevertReceipt> {
        self.preview_voxel_edit_revert_reference(request)
    }

    fn apply_voxel_edit_revert(
        &mut self,
        request: VoxelEditHistoryRevertRequest,
    ) -> BridgeResult<VoxelEditHistoryRevertReceipt> {
        self.apply_voxel_edit_revert_reference(request)
    }

    fn undo_voxel_edit(
        &mut self,
        request: VoxelEditHistoryUndoRequest,
    ) -> BridgeResult<VoxelEditHistoryUndoReceipt> {
        self.undo_voxel_edit_reference(request)
    }

    fn redo_voxel_edit(
        &mut self,
        request: VoxelEditHistoryRedoRequest,
    ) -> BridgeResult<VoxelEditHistoryRedoReceipt> {
        self.redo_voxel_edit_reference(request)
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
        let projection = CollisionProjection::build(world);
        let receipt = self
            .fps_session_mut("apply_fps_primary_fire")?
            .apply_primary_fire_for_roles(&projection, ray, tick, shooter_role, target_role, 0)
            .map_err(Self::fps_runtime_error)?;
        Ok(Self::primary_fire_result(receipt))
    }

    fn invoke_game_extension_weapon_effect(
        &mut self,
        request: GameExtensionWeaponEffectInvocationRequest,
    ) -> BridgeResult<GameExtensionWeaponEffectInvocationResult> {
        self.require_initialized("invoke_game_extension_weapon_effect")?;
        let module =
            Self::resolve_weapon_effect_game_rule_module(&self.game_rule_modules, &request.hook)?;
        let proposal = match module.evaluate_weapon_effect(&request.hook) {
            Ok(proposal) => proposal,
            Err(diagnostic) => {
                let hook_receipt = rejected_receipt(&request.hook, diagnostic);
                let replay_evidence =
                    Self::extension_replay_evidence(&hook_receipt, "rejectedByModule", Vec::new());
                return Ok(GameExtensionWeaponEffectInvocationResult {
                    hook_receipt,
                    replay_evidence,
                    primary_fire: None,
                });
            }
        };
        let hook_receipt = proposed_receipt(
            &request.hook,
            proposal,
            vec![GameExtensionTraceEntry {
                step: 1,
                code: "module.proposed_damage_modifier".to_string(),
                message: "resolved Rust game rule module returned a typed damage modifier"
                    .to_string(),
                refs: vec![
                    module.manifest().module_ref.module_id.clone(),
                    module.manifest().module_ref.version.clone(),
                    module.manifest().module_ref.contract_hash.clone(),
                ],
            }],
        );
        let damage_delta = match Self::validated_damage_modifier_delta(&request.hook, &hook_receipt)
        {
            Ok(delta) => delta,
            Err(diagnostic) => {
                let mut rejected = hook_receipt;
                rejected.status = GameExtensionReceiptStatus::RejectedByModule;
                rejected.diagnostics.push(diagnostic);
                let replay_evidence =
                    Self::extension_replay_evidence(&rejected, "invalidProposal", Vec::new());
                return Ok(GameExtensionWeaponEffectInvocationResult {
                    hook_receipt: rejected,
                    replay_evidence,
                    primary_fire: None,
                });
            }
        };
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
        let projection = CollisionProjection::build(world);
        let receipt = self
            .fps_session_mut("invoke_game_extension_weapon_effect")?
            .apply_targeted_primary_fire_with_damage_delta(
                &projection,
                ray.origin,
                primary_fire.tick,
                shooter_role,
                target_role,
                damage_delta,
            )
            .map_err(Self::fps_runtime_error)?;
        let primary_fire = Self::primary_fire_result(receipt);
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
            backend: "reference_bridge_rust".to_string(),
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
        if self.engine.is_none() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "step_simulation called before initialize_engine",
            ));
        }
        Ok(StepResult {
            tick: input.tick,
            diff_count: (input.tick % 4) as u32,
        })
    }

    fn create_camera(&mut self, request: CameraCreateRequest) -> BridgeResult<CameraSnapshot> {
        self.require_initialized("create_camera")?;
        Self::validate_create_request(&request)?;
        let camera = protocol_view::CameraHandle::new(self.next_camera);
        self.next_camera += 1;
        let snapshot = CameraSnapshot {
            camera,
            tick: 0,
            pose: request.initial_pose,
            basis: Self::basis_from_pose(request.initial_pose),
            projection: request.projection,
            viewport: request.viewport,
        };
        self.cameras.insert(camera.raw(), snapshot);
        Ok(snapshot)
    }

    fn apply_first_person_camera_input(
        &mut self,
        envelope: FirstPersonCameraInputEnvelope,
    ) -> BridgeResult<CameraSnapshot> {
        self.require_initialized("apply_first_person_camera_input")?;
        let prior = *self.cameras.get(&envelope.camera.raw()).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                "unknown camera handle",
            )
        })?;
        let input = envelope.input;
        Self::validate_camera_input(input)?;
        let snapshot = Self::integrate_camera_snapshot(prior, input, envelope.tick);
        self.cameras.insert(envelope.camera.raw(), snapshot);
        Ok(snapshot)
    }

    fn apply_enemy_direct_nav_movement(
        &mut self,
        request: EnemyDirectNavMovementRequest,
    ) -> BridgeResult<EnemyDirectNavMovementResult> {
        self.require_initialized("apply_enemy_direct_nav_movement")?;
        let entity = Self::enemy_entity_id(request.entity)?;
        let (authority_source, current_transform) =
            Self::seed_or_read_enemy_transform(&mut self.entities, entity, request.seed_position)?;
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
        let transform_event = self
            .entities
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
        // Fail closed on a newer bundle; the prior loaded ProjectBundle is left untouched
        // (we only mutate `loaded_project_bundle` on success — the staged commit/swap).
        if request.bundle_schema_version > REFERENCE_SUPPORTED_VERSION
            || request.protocol_version > REFERENCE_SUPPORTED_VERSION
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "unsupported bundle schema {} / protocol {}",
                    request.bundle_schema_version, request.protocol_version
                ),
            ));
        }
        self.loaded_project_bundle = Some(request.scene_id);
        Ok(CompositionStatus {
            loaded_project_bundle: Some(request.scene_id),
            ..CompositionStatus::empty()
        })
    }

    fn save_project_bundle(&mut self) -> BridgeResult<ProjectBundleSaveSummary> {
        if self.loaded_project_bundle.is_none() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "save_project_bundle called with no ProjectBundle loaded",
            ));
        }
        // Deterministic stand-in for the real save/compaction summary.
        Ok(ProjectBundleSaveSummary {
            artifacts_written: 3,
            compacted_edits: 0,
            retained_edits: 0,
        })
    }

    fn get_project_bundle_composition_status(&self) -> BridgeResult<CompositionStatus> {
        Ok(CompositionStatus {
            loaded_project_bundle: self.loaded_project_bundle,
            ..CompositionStatus::empty()
        })
    }

    fn unload_project_bundle(&mut self) -> BridgeResult<()> {
        self.loaded_project_bundle = None;
        Ok(())
    }
}
