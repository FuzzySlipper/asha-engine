use super::*;

impl RuntimeBridge for EngineBridge {
    fn initialize_engine(&mut self, config: EngineConfig) -> BridgeResult<EngineHandle> {
        initialization::initialize(self, config)
    }

    fn read_workspace_authoring_state(&self) -> BridgeResult<WorkspaceAuthoringStateSummary> {
        self.read_workspace_authoring_state_authority()
    }

    fn read_workspace_authoring_projection(
        &mut self,
        request: WorkspaceAuthoringProjectionRequest,
    ) -> BridgeResult<WorkspaceAuthoringProjectionReceipt> {
        self.read_workspace_authoring_projection_authority(request)
    }

    fn confirm_workspace_authoring_stored(
        &mut self,
        request: WorkspaceAuthoringStoredConfirmationRequest,
    ) -> BridgeResult<WorkspaceAuthoringStoredConfirmationReceipt> {
        self.confirm_workspace_authoring_stored_authority(request)
    }

    fn prepare_project_write(
        &mut self,
        request: ProjectWritePrepareRequest,
    ) -> BridgeResult<ProjectWritePrepareReceipt> {
        self.prepare_project_write_authority(request)
    }

    fn confirm_project_write(
        &mut self,
        request: ProjectWriteConfirmRequest,
    ) -> BridgeResult<ProjectWriteConfirmReceipt> {
        self.confirm_project_write_authority(request)
    }

    fn close_workspace_authoring(
        &mut self,
        request: WorkspaceAuthoringCloseRequest,
    ) -> BridgeResult<WorkspaceAuthoringCloseReceipt> {
        self.close_workspace_authoring_authority(request)
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
        let result = self.submit_commands_with_voxel_history(batch)?;
        if result.accepted > 0 {
            self.record_workspace_authoring_mutation();
        }
        if result.rejected > 0 {
            self.record_developer_console(DeveloperConsoleEmission {
                severity: DiagnosticSeverity::Warning,
                category: DeveloperConsoleCategory::Runtime,
                source: DeveloperConsoleSource::Authority,
                message: format!("authority rejected {} voxel command(s)", result.rejected),
                correlation: None,
                authority_tick: Some(self.time.authority_tick),
                detail: DeveloperConsoleDetail {
                    code: "operation_rejected".to_owned(),
                    operation: Some("submit_commands".to_owned()),
                    resource_kind: None,
                    resource_id: None,
                    reason: result.rejections.first().map(ToString::to_string),
                },
            });
        }
        Ok(result)
    }

    fn pick_voxel(&self, ray: PickRay) -> BridgeResult<PickResult> {
        let world = self.voxel.voxel.as_ref().ok_or_else(|| {
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

    fn configure_voxel_projection_instances(
        &mut self,
        request: VoxelProjectionBindingRequest,
    ) -> BridgeResult<VoxelProjectionBindingReceipt> {
        self.configure_voxel_projection_instances_authority(request)
    }

    fn pick_voxel_instance(
        &self,
        request: VoxelInstancePickRequest,
    ) -> BridgeResult<VoxelInstancePickResult> {
        self.pick_voxel_instance_authority(request)
    }

    fn apply_collision_constrained_camera_input(
        &mut self,
        envelope: CollisionConstrainedCameraInputEnvelope,
    ) -> BridgeResult<CameraCollisionSnapshot> {
        collision_camera::apply(self, envelope)
    }

    fn select_voxel(
        &self,
        request: ScreenPointToPickRayRequest,
    ) -> BridgeResult<VoxelSelectionSnapshot> {
        let snapshot = *self
            .camera
            .cameras
            .get(&request.camera.raw())
            .ok_or_else(|| {
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
        let world = self.voxel.voxel.as_ref().ok_or_else(|| {
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

    fn read_voxel_update_telemetry(
        &self,
        request: VoxelUpdateTelemetryRequest,
    ) -> BridgeResult<VoxelUpdateTelemetryReadout> {
        self.require_runtime_or_workspace_authoring("read_voxel_update_telemetry")?;
        let readout = self
            .projection
            .voxel_update_telemetry
            .latest
            .as_ref()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "voxel update telemetry is unavailable before a projection read",
                )
            })?;
        if request.grid != readout.grid {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel update telemetry request targets an unknown grid",
            ));
        }
        if request.projection_cursor != readout.projection_cursor {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel update telemetry cursor is stale or from the future",
            ));
        }
        Ok(readout.clone())
    }

    fn plan_voxel_conversion(
        &mut self,
        request: VoxelConversionPlanRequest,
    ) -> BridgeResult<VoxelConversionPlan> {
        self.require_runtime_or_workspace_authoring("plan_voxel_conversion")?;
        let source = self.source_for_voxel_conversion(&request);
        let planned = svc_voxel_conversion::plan_conversion(&request, &source);
        let plan = planned.plan.clone();
        self.voxel.voxel_conversion_plan = Some(planned);
        self.evidence.voxel_conversion_evidence.clear();
        self.remember_voxel_conversion_evidence(plan.evidence.clone());
        Ok(plan)
    }

    fn register_voxel_conversion_source(
        &mut self,
        request: VoxelConversionSourceRegistrationRequest,
    ) -> BridgeResult<VoxelConversionSourceRegistration> {
        self.require_runtime_or_workspace_authoring("register_voxel_conversion_source")?;
        let source = match Self::static_mesh_source_from_registration(&request) {
            Ok(source) => source,
            Err(message) => {
                return Ok(Self::source_registration_diagnostic(
                    &request.source,
                    message,
                ));
            }
        };
        self.voxel
            .voxel_conversion_sources
            .insert(source.asset_id.clone(), source);
        self.voxel.voxel_conversion_source_metadata.insert(
            request.source.asset_id.clone(),
            Self::source_metadata_from_registration(&request),
        );
        self.voxel.voxel_conversion_plan = None;
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
        self.require_runtime_or_workspace_authoring("read_voxel_conversion_source_metadata")?;
        let Some(metadata) = self
            .voxel
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
            .voxel
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
        self.require_runtime_or_workspace_authoring("preview_voxel_conversion")?;
        let planned = self.voxel.voxel_conversion_plan.as_ref().ok_or_else(|| {
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
        self.require_runtime_or_workspace_authoring("apply_voxel_conversion")?;
        let planned = self.voxel.voxel_conversion_plan.clone().ok_or_else(|| {
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

        self.voxel.voxel.as_ref().ok_or_else(|| {
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
            Self::apply_command_batch_to_world(&batch, &mut candidate, &self.voxel.materials)?;
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
        if receipt.applied {
            self.record_workspace_authoring_mutation();
        }
        Ok(receipt)
    }

    fn export_voxel_conversion_evidence(
        &self,
        evidence: Vec<VoxelConversionEvidenceRef>,
    ) -> BridgeResult<Vec<VoxelConversionEvidenceRef>> {
        self.require_runtime_or_workspace_authoring("export_voxel_conversion_evidence")?;
        for requested in &evidence {
            if !self.evidence.voxel_conversion_evidence.contains(requested) {
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
        self.require_runtime_or_workspace_authoring("read_voxel_model_info")?;
        let key = Self::voxel_model_key(request.grid, &request.volume_asset_id);
        if !self.voxel.voxel_conversion_targets.contains_key(&key) {
            return Ok(Self::voxel_model_missing_readout(
                request,
                "voxel model request targets an unknown conversion target",
            ));
        }
        let Some(info) = self.voxel.voxel_model_infos.get(&key) else {
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
        self.require_runtime_or_workspace_authoring("read_voxel_model_window")?;
        let key = Self::voxel_model_key(request.grid, &request.volume_asset_id);
        if !self.voxel.voxel_conversion_targets.contains_key(&key) {
            return Ok(Self::voxel_model_window_missing_readout(
                request,
                "voxel model window request targets an unknown conversion target",
            ));
        }
        let Some(info) = self.voxel.voxel_model_infos.get(&key) else {
            return Ok(Self::voxel_model_window_missing_readout(
                request,
                "voxel model is not resident in current authority state; apply a conversion first",
            ));
        };
        let Some(world) = self
            .voxel
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
        self.require_runtime_or_workspace_authoring("export_voxel_volume_asset")?;
        let key = Self::voxel_model_key(request.grid, &request.volume_asset_id);
        let Some(info) = self.voxel.voxel_model_infos.get(&key) else {
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
        let Some(target) = self.voxel.voxel_conversion_targets.get(&key) else {
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
        &mut self,
        request: VoxelVolumeAssetSaveRequest,
    ) -> BridgeResult<VoxelVolumeAssetSaveReceipt> {
        self.require_runtime_or_workspace_authoring("save_voxel_volume_asset")?;
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
        let Some(info) = self.voxel.voxel_model_infos.get(&key) else {
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
        let authored_asset = asset.clone();
        let authored_path = request.target_asset_path.clone();
        let receipt = VoxelVolumeAssetSaveReceipt {
            request,
            saved: true,
            diff: Some(diff),
            asset: Some(asset),
            canonical_json: Some(canonical_json),
            canonical_json_hash: Some(canonical_json_hash.clone()),
            voxel_data_hash: Some(voxel_data_hash),
            diagnostics: Vec::new(),
        };
        self.remember_workspace_authoring_voxel_write(authored_path, authored_asset);
        self.remember_workspace_authoring_save_candidate(canonical_json_hash);
        Ok(receipt)
    }

    fn update_voxel_volume_asset_palette(
        &mut self,
        request: VoxelVolumeAssetPaletteUpdateRequest,
    ) -> BridgeResult<VoxelVolumeAssetPaletteUpdateReceipt> {
        let receipt = self.update_voxel_volume_asset_palette_authority(request)?;
        if receipt.updated {
            self.record_workspace_authoring_mutation();
        }
        Ok(receipt)
    }

    fn initialize_voxel_volume_authoring(
        &mut self,
        request: VoxelVolumeAuthoringInitializeRequest,
    ) -> BridgeResult<VoxelVolumeAuthoringInitializeReceipt> {
        let receipt = self.initialize_voxel_volume_authoring_authority(request)?;
        if receipt.initialized {
            self.record_workspace_authoring_mutation();
        }
        Ok(receipt)
    }

    fn load_voxel_volume_asset(
        &mut self,
        request: VoxelVolumeAssetLoadRequest,
    ) -> BridgeResult<VoxelVolumeAssetLoadReceipt> {
        self.require_runtime_or_workspace_authoring("load_voxel_volume_asset")?;
        let loaded_asset = request.asset.clone();
        let asset = &request.asset;
        let scene_instance = self.voxel_asset_scene_instance(&asset.asset_id)?;
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
            Self::apply_command_batch_to_world(&batch, &mut candidate, &self.voxel.materials)?;
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
            self.voxel.voxel_model_infos.get(&key)
        };
        let info = Self::loaded_voxel_asset_info(&request, &target, &prior_world, existing);
        let receipt =
            Self::voxel_volume_asset_load_receipt(&request, &target, &info, true, Vec::new());
        let collision_offset = scene_instance
            .as_ref()
            .map(|(_, transform)| transform.translation.to_array().map(f64::from))
            .unwrap_or([0.0; 3]);
        self.reset_voxel_edit_history_with_collision_offset(candidate.clone(), collision_offset);
        if let Some((node_id, transform)) = scene_instance {
            let frame = self
                .projection
                .voxel_projector
                .set_instances(
                    &candidate,
                    vec![VoxelProjectionInstance {
                        instance_id: format!("scene-node/{}", node_id.raw()),
                        asset_id: asset.asset_id.clone(),
                        transform,
                    }],
                )
                .map_err(|error| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::InvalidInput,
                        format!("stored voxel scene projection was rejected: {error:?}"),
                    )
                })?;
            self.projection.pending_voxel_frame.ops.extend(frame.ops);
        }
        self.voxel.voxel_conversion_targets.insert(
            Self::voxel_model_key(info.grid, &info.volume_asset_id),
            target,
        );
        self.voxel.voxel_model_infos.insert(key.clone(), info);
        self.voxel.active_voxel_model = Some(key);
        if receipt.loaded {
            self.record_workspace_authoring_loaded_asset(loaded_asset);
            self.queue_current_project_voxel_materials()?;
        }
        Ok(receipt)
    }

    fn unload_voxel_volume_asset(
        &mut self,
        request: VoxelVolumeAssetUnloadRequest,
    ) -> BridgeResult<VoxelVolumeAssetUnloadReceipt> {
        let receipt = self.unload_voxel_volume_asset_authority(request)?;
        if receipt.unloaded {
            self.record_workspace_authoring_mutation();
            self.clear_workspace_authoring_loaded_assets();
        }
        Ok(receipt)
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
        let receipt = self.load_voxel_annotation_layer_authority(request)?;
        if receipt.loaded {
            self.record_workspace_authoring_mutation();
        }
        Ok(receipt)
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
        let receipt = self.apply_voxel_annotation_edit_authority(request)?;
        if receipt.edited {
            self.record_workspace_authoring_mutation();
        }
        Ok(receipt)
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
        let receipt = self.apply_voxel_edit_revert_authority(request)?;
        if receipt.applied {
            self.record_workspace_authoring_mutation();
        }
        Ok(receipt)
    }

    fn undo_voxel_edit(
        &mut self,
        request: VoxelEditHistoryUndoRequest,
    ) -> BridgeResult<VoxelEditHistoryUndoReceipt> {
        let receipt = self.undo_voxel_edit_authority(request)?;
        if receipt.receipt.applied {
            self.record_workspace_authoring_mutation();
        }
        Ok(receipt)
    }

    fn redo_voxel_edit(
        &mut self,
        request: VoxelEditHistoryRedoRequest,
    ) -> BridgeResult<VoxelEditHistoryRedoReceipt> {
        let receipt = self.redo_voxel_edit_authority(request)?;
        if receipt.receipt.applied {
            self.record_workspace_authoring_mutation();
        }
        Ok(receipt)
    }

    fn read_model_material_preview(
        &self,
        request: ModelMaterialPreviewRequest,
    ) -> BridgeResult<ModelMaterialPreviewSnapshot> {
        self.read_model_material_preview_authority(request)
    }

    fn decode_scene_document(
        &mut self,
        request: SceneDocumentDecodeRequestDto,
    ) -> BridgeResult<SceneDocumentCodecResultDto> {
        self.decode_scene_document_authority(request)
    }

    fn encode_scene_document(
        &self,
        request: SceneDocumentEncodeRequestDto,
    ) -> BridgeResult<SceneDocumentCodecResultDto> {
        self.encode_scene_document_authority(request)
    }

    fn apply_scene_document_authoring(
        &mut self,
        request: SceneDocumentAuthoringRequestDto,
    ) -> BridgeResult<SceneDocumentAuthoringResultDto> {
        self.apply_scene_document_authoring_authority(request)
    }

    fn decode_project_content(
        &mut self,
        request: ProjectContentDecodeRequestDto,
    ) -> BridgeResult<ProjectContentCodecResultDto> {
        let authority = self.require_open_workspace_authoring_mut("decode_project_content")?;
        let scenes = authority
            .project_content_scenes
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let outcome = svc_project_content::decode_project_content(
            request,
            svc_project_content::ProjectContentValidationContext {
                scenes: &scenes,
                gameplay: &authority.project_content_admission,
                reference_revision: authority.project_content_reference_revision,
            },
        );
        let mut installed = false;
        if let Some(validated) = outcome.validated {
            if let Some(current) = &authority.project_content_current {
                if current.set_hash() != validated.set_hash() {
                    return Ok(ProjectContentCodecResultDto {
                        accepted: false,
                        documents: Vec::new(),
                        canonical_files: Vec::new(),
                        set_hash: Some(current.set_hash().to_owned()),
                        provider_schemas: svc_project_content::ProjectContentGameplayAdmission::configuration_schemas(
                            &authority.project_content_admission,
                        )
                        .to_vec(),
                        field_metadata: Vec::new(),
                        diagnostics: vec![ProjectContentDiagnosticDto {
                            code: ProjectContentDiagnosticCode::StaleRevision,
                            document_id: None,
                            path: "sources".to_owned(),
                            message: "a different project-content set is already loaded in this workspace generation"
                                .to_owned(),
                        }],
                    });
                }
            }
            authority.project_content_current = Some(validated);
            installed = true;
        }
        let result = outcome.result;
        if installed {
            self.queue_current_project_voxel_materials()?;
        }
        Ok(result)
    }

    fn encode_project_content(
        &self,
        request: ProjectContentEncodeRequestDto,
    ) -> BridgeResult<ProjectContentCodecResultDto> {
        let authority = self
            .workspace_authoring
            .as_ref()
            .filter(|authority| authority.open)
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "encode_project_content called before workspace authoring open",
                )
            })?;
        let scenes = authority
            .project_content_scenes
            .values()
            .cloned()
            .collect::<Vec<_>>();
        Ok(svc_project_content::encode_project_content(
            request,
            svc_project_content::ProjectContentValidationContext {
                scenes: &scenes,
                gameplay: &authority.project_content_admission,
                reference_revision: authority.project_content_reference_revision,
            },
        ))
    }

    fn apply_project_content_authoring(
        &mut self,
        request: ProjectContentAuthoringRequestDto,
    ) -> BridgeResult<ProjectContentAuthoringResultDto> {
        self.require_runtime_or_workspace_authoring("apply_project_content_authoring")?;
        self.require_workspace_authoring_revision(
            "apply_project_content_authoring",
            &request.expected_workspace_id,
            request.expected_generation,
            request.expected_working_revision,
        )?;
        let (result, validated) = {
            let authority =
                self.require_open_workspace_authoring_mut("apply_project_content_authoring")?;
            let scenes = authority
                .project_content_scenes
                .values()
                .cloned()
                .collect::<Vec<_>>();
            let current = authority.project_content_current.as_ref().ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "apply_project_content_authoring requires a decoded Engine-owned current set",
                )
            })?;
            svc_project_content::apply_project_content_authoring(
                current,
                request,
                svc_project_content::ProjectContentValidationContext {
                    scenes: &scenes,
                    gameplay: &authority.project_content_admission,
                    reference_revision: authority.project_content_reference_revision,
                },
            )
        };
        if result.accepted {
            let validated = validated.ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "accepted project-content authoring omitted its validated set artifact",
                )
            })?;
            let authority =
                self.require_open_workspace_authoring_mut("apply_project_content_authoring")?;
            authority.project_content_current = Some(validated);
            self.record_workspace_authoring_mutation();
            if let Some(set_hash) = result.set_hash.clone() {
                self.remember_workspace_authoring_save_candidate(set_hash);
            }
            self.queue_current_project_voxel_materials()?;
        }
        Ok(result)
    }

    fn preview_procedural_environment(
        &mut self,
        request: ProceduralEnvironmentPreviewRequestDto,
    ) -> BridgeResult<ProceduralEnvironmentPreviewResultDto> {
        self.preview_procedural_environment_authority(request)
    }

    fn apply_procedural_environment(
        &mut self,
        request: ProceduralEnvironmentApplyRequestDto,
    ) -> BridgeResult<ProceduralEnvironmentApplyResultDto> {
        self.apply_procedural_environment_authority(request)
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

    fn read_fps_runtime_session(&self) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        self.require_initialized("read_fps_runtime_session")?;
        Self::fps_snapshot(
            self.fps_session("read_fps_runtime_session")?,
            &self.scene.entities,
            self.gameplay.fps_epoch,
        )
    }

    fn apply_fps_primary_fire(
        &mut self,
        request: FpsPrimaryFireRequest,
    ) -> BridgeResult<FpsPrimaryFireResult> {
        self.apply_fps_primary_fire_authority(request)
    }

    fn read_composed_runtime_session(&mut self) -> BridgeResult<ComposedRuntimeSessionReadout> {
        EngineBridge::read_composed_runtime_session(self)
    }

    fn read_gameplay_module_view(
        &mut self,
        request: GameplayModuleViewRequest,
    ) -> BridgeResult<GameplayModuleViewSnapshot> {
        EngineBridge::read_gameplay_module_view(self, request)
    }

    fn apply_gameplay_prefab_part_interaction(
        &mut self,
        request: GameplayPrefabPartInteractionRequest,
    ) -> BridgeResult<GameplayPrefabPartInteractionReceipt> {
        EngineBridge::apply_gameplay_prefab_part_interaction(self, request)
    }

    fn read_projection_frame(&self, cursor: u64) -> BridgeResult<RuntimeProjectionFrame> {
        self.require_initialized("read_projection_frame")?;
        let frame = self.projection.projection_frame.as_ref().ok_or_else(|| {
            self.record_developer_console(DeveloperConsoleEmission {
                severity: DiagnosticSeverity::Error,
                category: DeveloperConsoleCategory::Capability,
                source: DeveloperConsoleSource::RuntimeHost,
                message: "render projection capability is unavailable".to_owned(),
                correlation: Some(format!("projection-cursor:{cursor}")),
                authority_tick: Some(self.time.authority_tick),
                detail: DeveloperConsoleDetail {
                    code: "capability_unavailable".to_owned(),
                    operation: Some("read_projection_frame".to_owned()),
                    resource_kind: None,
                    resource_id: None,
                    reason: Some("projection frame missing after initialization".to_owned()),
                },
            });
            self.record_developer_console(DeveloperConsoleEmission {
                severity: DiagnosticSeverity::Error,
                category: DeveloperConsoleCategory::Resource,
                source: DeveloperConsoleSource::Projection,
                message: "render projection is unavailable".to_owned(),
                correlation: Some(format!("projection-cursor:{cursor}")),
                authority_tick: Some(self.time.authority_tick),
                detail: DeveloperConsoleDetail {
                    code: "resource_degraded".to_owned(),
                    operation: Some("read_projection_frame".to_owned()),
                    resource_kind: Some("render_projection".to_owned()),
                    resource_id: None,
                    reason: Some("projection frame missing after initialization".to_owned()),
                },
            });
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

    fn read_developer_console(&self) -> BridgeResult<DeveloperConsoleSnapshot> {
        self.require_runtime_or_workspace_authoring("read_developer_console")?;
        Ok(self.developer_console_snapshot())
    }

    fn invoke_game_extension_weapon_effect(
        &mut self,
        request: GameExtensionWeaponEffectInvocationRequest,
    ) -> BridgeResult<GameExtensionWeaponEffectInvocationResult> {
        self.require_initialized("invoke_game_extension_weapon_effect")?;
        let module = Self::resolve_weapon_effect_game_rule_module(
            &self.gameplay.game_rule_modules,
            &request.hook,
        )?;
        let transformed = match rule_gameplay_fabric::compatibility::run_legacy_weapon_effect_transform(
            &module,
            &request.hook,
        ) {
            Ok(outcome) => outcome,
            Err(rule_gameplay_fabric::compatibility::LegacyWeaponEffectTransformError::ModuleRejected(
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
            Err(rule_gameplay_fabric::compatibility::LegacyWeaponEffectTransformError::DecisionRejected(
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
                    format!(
                        "{}: legacy weapon Transform compatibility failed: {error}",
                        rule_gameplay_fabric::compatibility::LEGACY_WEAPON_EFFECT_COMPATIBILITY_DIAGNOSTIC
                    ),
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
        let world = self.voxel.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "invoke_game_extension_weapon_effect called before initialize_engine",
            )
        })?;
        let projection = self.collision_projection(world);
        let fps_before = self
            .fps_session("invoke_game_extension_weapon_effect")?
            .clone();
        let entities_before = self.scene.entities.clone();
        let session = self.gameplay.fps_session.as_mut().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "invoke_game_extension_weapon_effect called before canonical FPS project activation",
            )
        })?;
        let receipt = session
            .apply_primary_fire_for_roles_with_entities(FpsPrimaryFireAuthorityInput {
                entities: &mut self.scene.entities,
                projection: &projection,
                ray,
                tick: primary_fire.tick,
                shooter_role,
                target_role,
                damage_delta,
            })
            .map_err(Self::fps_runtime_error)?;
        let gameplay_events = receipt.gameplay_events.clone();
        if let Err(error) = self.deliver_static_gameplay_owner_events(gameplay_events) {
            self.gameplay.fps_session = Some(fps_before);
            self.scene.entities = entities_before;
            return Err(error);
        }
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
        self.gameplay.game_rule_recent_trace = report.trace.clone();
        self.evidence
            .game_rule_recent_replay_hashes
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
        self.gameplay.game_rule_recent_trace = receipt.trace.clone();
        self.evidence
            .game_rule_recent_replay_hashes
            .push(receipt.replay_hash.clone());
        if receipt.accepted {
            for modifier in &receipt.applied_modifiers {
                if let Some(existing) =
                    self.gameplay
                        .game_rule_active_modifiers
                        .iter_mut()
                        .find(|active| {
                            active.modifier_id == modifier.modifier_id
                                && active.source == modifier.source
                                && active.target == modifier.target
                        })
                {
                    *existing = modifier.clone();
                } else {
                    self.gameplay
                        .game_rule_active_modifiers
                        .push(modifier.clone());
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
            active_modifiers: self.gameplay.game_rule_active_modifiers.clone(),
            recent_trace: self.gameplay.game_rule_recent_trace.clone(),
            recent_replay_hashes: self.evidence.game_rule_recent_replay_hashes.clone(),
            latest_replay_hash: self.evidence.game_rule_recent_replay_hashes.last().cloned(),
        })
    }

    fn restart_fps_runtime_session(
        &mut self,
        request: FpsRuntimeSessionRestartRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        self.restart_fps_runtime_session_authority(request)
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
        self.apply_fps_encounter_transition_authority(request)
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
        self.apply_enemy_direct_nav_movement_authority(request)
    }
    fn read_render_diffs(&mut self, cursor: u64) -> BridgeResult<RenderFrameDiff> {
        self.read_render_diffs_authority(cursor)
    }
    fn read_camera_projection(
        &self,
        request: CameraProjectionRequest,
    ) -> BridgeResult<CameraProjectionSnapshot> {
        self.require_initialized("read_camera_projection")?;
        let snapshot = *self
            .camera
            .cameras
            .get(&request.camera.raw())
            .ok_or_else(|| {
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
        self.voxel.buffers.view(handle)
    }

    fn release_buffer(&mut self, handle: RuntimeBufferHandle) -> BridgeResult<()> {
        self.voxel.buffers.dispose(handle)
    }

    fn begin_runtime_project_source_resources(
        &mut self,
        request: ProjectResourceBeginRequest,
    ) -> BridgeResult<ProjectResourceTransactionReceipt> {
        let transaction =
            EngineBridge::begin_runtime_project_source_resources(self, &request.manifest_json)?;
        Ok(ProjectResourceTransactionReceipt {
            generation: transaction.generation(),
            manifest_hash: transaction.manifest_hash().to_hex(),
        })
    }

    fn stage_runtime_project_source_resource(
        &mut self,
        request: ProjectResourceStageRequest,
    ) -> BridgeResult<StagedProjectResourceRef> {
        let staged = self.stage_runtime_project_source_resource_generation(
            request.generation,
            &request.path,
            request.bytes,
        )?;
        Ok(StagedProjectResourceRef {
            handle: staged.handle.raw(),
            generation: staged.generation,
            version: staged.version,
            byte_len: staged.byte_len,
        })
    }

    fn admit_runtime_project_source_batch(
        &mut self,
        request: RuntimeProjectSourceBatch,
    ) -> BridgeResult<ProjectSourceBatchValidationReceipt> {
        EngineBridge::admit_runtime_project_source_batch(self, request)
    }

    fn load_runtime_project(
        &mut self,
        request: RuntimeProjectLoadRequest,
    ) -> BridgeResult<RuntimeProjectLoadReceipt> {
        Ok(self.load_runtime_project_authority(request))
    }

    fn read_active_runtime_project_content(
        &self,
    ) -> BridgeResult<ActiveRuntimeProjectContentReadoutDto> {
        self.read_active_runtime_project_content_authority()
    }

    fn close_runtime_project(
        &mut self,
        request: RuntimeProjectCloseRequest,
    ) -> BridgeResult<RuntimeProjectCloseReceipt> {
        Ok(self.close_runtime_project_authority(request))
    }
}
