use super::*;

impl EngineBridge {
    pub(super) fn target_for_voxel_conversion(
        &self,
        target: &protocol_voxel_conversion::VoxelConversionTargetRef,
    ) -> Option<VoxelConversionTargetAuthority> {
        self.voxel_conversion_targets
            .get(&(target.grid, target.volume_asset_id.clone()))
            .cloned()
    }

    pub(super) fn voxel_model_key(
        grid: u64,
        volume_asset_id: &Option<String>,
    ) -> (u64, Option<String>) {
        (grid, volume_asset_id.clone())
    }

    pub(super) fn voxel_model_id(grid: u64, volume_asset_id: &Option<String>) -> String {
        match volume_asset_id {
            Some(id) => format!("voxel-model:grid:{grid}:volume:{id}"),
            None => format!("voxel-model:grid:{grid}:volume:none"),
        }
    }

    pub(super) fn voxel_conversion_diagnostic(
        code: VoxelConversionDiagnosticCode,
        reference: impl Into<String>,
        message: impl Into<String>,
    ) -> VoxelConversionDiagnostic {
        VoxelConversionDiagnostic {
            code,
            severity: DiagnosticSeverity::Error,
            reference: reference.into(),
            message: message.into(),
        }
    }

    pub(super) fn rejected_voxel_conversion_receipt(
        plan_id: String,
        diagnostics: Vec<VoxelConversionDiagnostic>,
    ) -> VoxelConversionReceipt {
        VoxelConversionReceipt {
            plan_id,
            applied: false,
            output_hash: None,
            output_voxel_count: 0,
            output_bounds: None,
            diagnostics,
            evidence: Vec::new(),
        }
    }

    pub(super) fn conversion_commands(
        planned: &PlannedConversion,
    ) -> BridgeResult<Option<CommandBatch>> {
        let Some(output) = &planned.output else {
            return Ok(None);
        };
        let grid = u32::try_from(planned.plan.target.grid).map_err(|_| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel conversion target grid must fit in u32",
            )
        })?;
        let commands = output
            .voxels
            .iter()
            .map(|voxel| {
                let material = voxel.value.material().ok_or_else(|| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::Internal,
                        "voxel conversion output contained a non-solid voxel",
                    )
                })?;
                Ok(set_voxel_command(
                    grid,
                    voxel.coord.x,
                    voxel.coord.y,
                    voxel.coord.z,
                    material.raw(),
                ))
            })
            .collect::<BridgeResult<Vec<_>>>()?;
        Ok(Some(CommandBatch { commands }))
    }

    pub(super) fn apply_command_batch_to_world(
        batch: &CommandBatch,
        world: &mut VoxelWorld,
        materials: &MaterialCatalog,
    ) -> BridgeResult<CommandResult> {
        let mut accepted = 0u32;
        let mut rejections = Vec::new();
        for cmd in &batch.commands {
            // Validate (no mutation), then apply on accept. A rejected command is
            // classified and never touches authority state.
            match rule_voxel_edit::validate(cmd, world, materials) {
                Ok(events) => {
                    for event in &events {
                        rule_voxel_edit::apply(world, event).map_err(|rej| {
                            RuntimeBridgeError::new(
                                RuntimeBridgeErrorKind::Internal,
                                format!("validated voxel command failed to apply: {rej}"),
                            )
                        })?;
                    }
                    accepted += 1;
                }
                Err(rejection) => rejections.push(rejection),
            }
        }

        Ok(CommandResult {
            accepted,
            rejected: rejections.len() as u32,
            rejections,
        })
    }

    pub(super) fn voxel_conversion_target_candidate(
        &self,
        target: &VoxelConversionTargetAuthority,
        planned: &PlannedConversion,
    ) -> BridgeResult<VoxelWorld> {
        self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "apply_voxel_conversion called before initialize_engine",
            )
        })?;
        let should_replace_world = self
            .voxel
            .as_ref()
            .is_none_or(|world| world.grid().id() != target.spec.id());
        let mut candidate = if should_replace_world {
            VoxelWorld::new(target.spec)
        } else {
            self.voxel.as_ref().expect("checked above").clone()
        };

        let Some(output) = &planned.output else {
            return Ok(candidate);
        };
        for voxel in &output.voxels {
            let coord = VoxelCoord::new(voxel.coord.x, voxel.coord.y, voxel.coord.z);
            let chunk = target.spec.voxel_to_chunk(coord);
            if candidate.get(chunk).is_none() {
                candidate.insert(chunk, VoxelChunk::from_spec(&target.spec));
            }
        }
        Ok(candidate)
    }

    pub(super) fn remember_voxel_conversion_evidence(
        &mut self,
        refs: impl IntoIterator<Item = VoxelConversionEvidenceRef>,
    ) {
        for evidence in refs {
            if !self.voxel_conversion_evidence.contains(&evidence) {
                self.voxel_conversion_evidence.push(evidence);
            }
        }
    }

    pub(super) fn remember_voxel_model_info(
        &mut self,
        target: &VoxelConversionTargetAuthority,
        planned: &PlannedConversion,
        receipt: &VoxelConversionReceipt,
    ) {
        let Some(output) = &planned.output else {
            return;
        };
        let Some(output_hash) = receipt.output_hash.clone() else {
            return;
        };
        let grid = target.spec.id().raw() as u64;
        let mut material_count_map = BTreeMap::<u16, u64>::new();
        for voxel in &output.voxels {
            if let Some(material) = voxel.value.material() {
                *material_count_map.entry(material.raw()).or_insert(0) += 1;
            }
        }
        let material_counts = material_count_map
            .into_iter()
            .map(|(material, voxel_count)| VoxelModelMaterialCount {
                material,
                voxel_count,
            })
            .collect::<Vec<_>>();
        let model_id = Self::voxel_model_id(grid, &target.volume_asset_id);
        let mut evidence = self.voxel_conversion_evidence.clone();
        for item in &receipt.evidence {
            if !evidence.contains(item) {
                evidence.push(item.clone());
            }
        }
        let session_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-model-info|session|{}|{}|{}|{}|{:?}",
                model_id,
                planned.plan.plan_id,
                output_hash,
                output.voxels.len(),
                material_counts
            ))
        );
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-model-info|replay|{}|{}|{:?}",
                planned.plan.plan_id, output_hash, evidence
            ))
        );
        self.voxel_model_infos.insert(
            Self::voxel_model_key(grid, &target.volume_asset_id),
            VoxelModelInfoAuthority {
                model_id,
                volume_asset_id: target.volume_asset_id.clone(),
                grid,
                bounds: output.bounds,
                voxel_count: output.voxels.len() as u64,
                material_counts,
                source: planned.plan.source.clone(),
                latest_plan_id: planned.plan.plan_id.clone(),
                latest_output_hash: output_hash,
                session_hash,
                replay_hash,
                evidence,
            },
        );
    }

    pub(super) fn voxel_model_missing_readout(
        request: VoxelModelInfoRequest,
        message: impl Into<String>,
    ) -> VoxelModelInfoReadout {
        let model_id = Self::voxel_model_id(request.grid, &request.volume_asset_id);
        let diagnostic = Self::voxel_conversion_diagnostic(
            VoxelConversionDiagnosticCode::VoxelConversionUnavailable,
            model_id.clone(),
            message,
        );
        let session_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!("voxel-model-info|missing|{:?}", request))
        );
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!("voxel-model-info|missing-replay|{:?}", request))
        );
        VoxelModelInfoReadout {
            request: request.clone(),
            resident: false,
            model_id,
            volume_asset_id: request.volume_asset_id.clone(),
            grid: request.grid,
            bounds: None,
            voxel_count: 0,
            material_counts: Vec::new(),
            source: None,
            latest_plan_id: None,
            latest_output_hash: None,
            session_hash,
            replay_hash,
            evidence: Vec::new(),
            diagnostics: vec![diagnostic],
        }
    }

    pub(super) fn voxel_model_window_missing_readout(
        request: VoxelModelWindowRequest,
        message: impl Into<String>,
    ) -> VoxelModelWindowReadout {
        let model_id = Self::voxel_model_id(request.grid, &request.volume_asset_id);
        let diagnostic = Self::voxel_conversion_diagnostic(
            VoxelConversionDiagnosticCode::VoxelConversionUnavailable,
            model_id.clone(),
            message,
        );
        let info = Self::missing_voxel_model_info(model_id);
        Self::voxel_model_window_readout(request, &info, 0, Vec::new(), vec![diagnostic])
    }

    pub(super) fn missing_voxel_model_info(model_id: String) -> VoxelModelInfoAuthority {
        VoxelModelInfoAuthority {
            model_id,
            volume_asset_id: None,
            grid: 0,
            bounds: None,
            voxel_count: 0,
            material_counts: Vec::new(),
            source: protocol_voxel_conversion::VoxelConversionSourceRef {
                asset_id: "missing".to_string(),
                asset_kind: "voxel_model".to_string(),
                asset_version: 0,
                source_hash: "fnv1a64:missing".to_string(),
                mesh_primitive: None,
            },
            latest_plan_id: "missing".to_string(),
            latest_output_hash: "fnv1a64:missing".to_string(),
            session_hash: "fnv1a64:missing".to_string(),
            replay_hash: "fnv1a64:missing".to_string(),
            evidence: Vec::new(),
        }
    }

    pub(super) fn voxel_model_window_readout(
        request: VoxelModelWindowRequest,
        info: &VoxelModelInfoAuthority,
        scanned_voxel_count: u64,
        samples: Vec<VoxelModelWindowSample>,
        diagnostics: Vec<VoxelConversionDiagnostic>,
    ) -> VoxelModelWindowReadout {
        let session_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-model-window|session|{:?}|{}|{}|{:?}",
                request, info.session_hash, scanned_voxel_count, samples
            ))
        );
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-model-window|replay|{}|{}|{:?}",
                session_hash, info.replay_hash, diagnostics
            ))
        );
        let returned_sample_count = samples.len() as u32;
        VoxelModelWindowReadout {
            request: request.clone(),
            resident: diagnostics.iter().all(|diagnostic| {
                diagnostic.code != VoxelConversionDiagnosticCode::VoxelConversionUnavailable
            }),
            model_id: info.model_id.clone(),
            volume_asset_id: request.volume_asset_id.clone(),
            grid: request.grid,
            requested_bounds: request.bounds,
            model_bounds: info.bounds,
            scanned_voxel_count,
            returned_sample_count,
            samples,
            session_hash,
            replay_hash,
            diagnostics,
        }
    }

    pub(super) fn voxel_model_window_request_diagnostics(
        request: &VoxelModelWindowRequest,
    ) -> Vec<VoxelConversionDiagnostic> {
        let mut diagnostics = Vec::new();
        let Some(volume) = Self::voxel_model_window_volume(request.bounds) else {
            diagnostics.push(Self::voxel_conversion_diagnostic(
                VoxelConversionDiagnosticCode::InvalidQueryBounds,
                "bounds",
                "voxel model window bounds must be ordered and finite",
            ));
            return diagnostics;
        };
        if request.max_samples == 0 {
            diagnostics.push(Self::voxel_conversion_diagnostic(
                VoxelConversionDiagnosticCode::QueryQuotaExceeded,
                "maxSamples",
                "voxel model window maxSamples must be greater than zero",
            ));
        }
        let effective_limit = VOXEL_MODEL_WINDOW_MAX_SAMPLES.min(u64::from(request.max_samples));
        if volume > effective_limit {
            diagnostics.push(Self::voxel_conversion_diagnostic(
                VoxelConversionDiagnosticCode::QueryQuotaExceeded,
                "bounds",
                format!("voxel model window scans {volume} cells; limit is {effective_limit}"),
            ));
        }
        diagnostics
    }

    pub(super) fn voxel_model_window_volume(
        bounds: protocol_voxel_conversion::VoxelConversionBounds,
    ) -> Option<u64> {
        let dx = Self::inclusive_axis_len(bounds.min.x, bounds.max.x)?;
        let dy = Self::inclusive_axis_len(bounds.min.y, bounds.max.y)?;
        let dz = Self::inclusive_axis_len(bounds.min.z, bounds.max.z)?;
        dx.checked_mul(dy)?.checked_mul(dz)
    }

    pub(super) fn inclusive_axis_len(min: i64, max: i64) -> Option<u64> {
        if max < min {
            return None;
        }
        u64::try_from(max.checked_sub(min)?.checked_add(1)?).ok()
    }

    pub(super) fn voxel_value_at(world: &VoxelWorld, coord: VoxelCoord) -> VoxelValue {
        let (chunk, local) = world.grid().voxel_to_chunk_local(coord);
        world
            .get(chunk)
            .and_then(|chunk| chunk.get(local))
            .unwrap_or(VoxelValue::EMPTY)
    }

    pub(super) fn protocol_voxel_coord(
        coord: VoxelCoord,
    ) -> protocol_voxel_conversion::VoxelConversionCoord {
        protocol_voxel_conversion::VoxelConversionCoord {
            x: coord.x,
            y: coord.y,
            z: coord.z,
        }
    }

    pub(super) fn rejected_voxel_volume_asset_export(
        request: VoxelVolumeAssetExportRequest,
        diagnostics: Vec<VoxelAssetDiagnostic>,
    ) -> VoxelVolumeAssetExportReceipt {
        VoxelVolumeAssetExportReceipt {
            request,
            exported: false,
            asset: None,
            canonical_json: None,
            canonical_json_hash: None,
            voxel_data_hash: None,
            diagnostics,
        }
    }

    pub(super) fn rejected_voxel_volume_asset_save(
        request: VoxelVolumeAssetSaveRequest,
        diagnostics: Vec<VoxelAssetDiagnostic>,
    ) -> VoxelVolumeAssetSaveReceipt {
        VoxelVolumeAssetSaveReceipt {
            request,
            saved: false,
            diff: None,
            asset: None,
            canonical_json: None,
            canonical_json_hash: None,
            voxel_data_hash: None,
            diagnostics,
        }
    }

    pub(super) fn voxel_asset_save_request_diagnostics(
        request: &VoxelVolumeAssetSaveRequest,
    ) -> Vec<VoxelAssetDiagnostic> {
        let mut diagnostics = Vec::new();
        if request.target_project_bundle.trim().is_empty() {
            diagnostics.push(Self::voxel_asset_diagnostic(
                VoxelAssetDiagnosticCode::InvalidAssetId,
                "targetProjectBundle",
                "target project bundle must be non-empty",
            ));
        }
        let path = request.target_asset_path.trim();
        if path.is_empty()
            || path.starts_with('/')
            || path.contains('\\')
            || path
                .split('/')
                .any(|segment| segment.is_empty() || segment == "." || segment == "..")
            || !path.ends_with(VOXEL_ASSET_EXTENSION)
        {
            diagnostics.push(Self::voxel_asset_diagnostic(
                VoxelAssetDiagnosticCode::InvalidAssetId,
                "targetAssetPath",
                format!(
                    "target asset path must be a relative ProjectBundle path ending in .{}",
                    VOXEL_ASSET_EXTENSION
                ),
            ));
        }
        if request.representation_kind != VoxelAssetRepresentationKind::SparseRuns.as_str() {
            diagnostics.push(Self::voxel_asset_diagnostic(
                VoxelAssetDiagnosticCode::UnsupportedRepresentation,
                "representationKind",
                "runtime-to-stored voxel asset save currently supports sparse_runs only",
            ));
        }
        diagnostics
    }

    pub(super) fn voxel_asset_diagnostic(
        code: VoxelAssetDiagnosticCode,
        reference: impl Into<String>,
        message: impl Into<String>,
    ) -> VoxelAssetDiagnostic {
        VoxelAssetDiagnostic {
            code,
            severity: DiagnosticSeverity::Error,
            reference: reference.into(),
            message: message.into(),
        }
    }

    pub(super) fn voxel_asset_bounds(
        bounds: protocol_voxel_conversion::VoxelConversionBounds,
    ) -> VoxelAssetBounds {
        VoxelAssetBounds {
            min: Self::voxel_asset_coord(bounds.min),
            max: Self::voxel_asset_coord(bounds.max),
        }
    }

    pub(super) fn voxel_asset_coord(
        coord: protocol_voxel_conversion::VoxelConversionCoord,
    ) -> VoxelAssetCoord {
        VoxelAssetCoord {
            x: coord.x,
            y: coord.y,
            z: coord.z,
        }
    }

    pub(super) fn sparse_runs_for_conversion_output(
        output: &svc_voxel_conversion::ConversionOutput,
    ) -> Vec<VoxelAssetSparseRun> {
        let mut voxels = output.voxels.clone();
        voxels.sort_by_key(|voxel| {
            (
                voxel.coord.z,
                voxel.coord.y,
                voxel.coord.x,
                voxel
                    .value
                    .material()
                    .expect("converted voxels are solid")
                    .raw(),
            )
        });
        let mut runs: Vec<VoxelAssetSparseRun> = Vec::new();
        for voxel in voxels {
            let material = voxel
                .value
                .material()
                .expect("converted voxels are solid")
                .raw();
            if let Some(last) = runs.last_mut() {
                let next_x = last.start.x + i64::from(last.length);
                if last.start.y == voxel.coord.y
                    && last.start.z == voxel.coord.z
                    && last.material == material
                    && next_x == voxel.coord.x
                {
                    last.length += 1;
                    continue;
                }
            }
            runs.push(VoxelAssetSparseRun {
                start: Self::voxel_asset_coord(voxel.coord),
                length: 1,
                material,
            });
        }
        runs
    }

    pub(super) fn material_palette_for_conversion_export(
        planned: &PlannedConversion,
        output: &svc_voxel_conversion::ConversionOutput,
    ) -> Result<Vec<VoxelAssetMaterialBinding>, Vec<VoxelAssetDiagnostic>> {
        let mut used_materials = BTreeSet::new();
        for voxel in &output.voxels {
            if let Some(material) = voxel.value.material() {
                used_materials.insert(material.raw());
            }
        }
        let mut bindings = BTreeMap::<u16, String>::new();
        let mut diagnostics = Vec::new();
        for entry in &planned.plan.settings.material_map.entries {
            if !used_materials.contains(&entry.voxel_material) {
                continue;
            }
            let Some(material_asset_id) = &entry.source_material_id else {
                diagnostics.push(Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::InvalidMaterialReference,
                    format!("materialMap.{}", entry.voxel_material),
                    "export requires every used voxel material to reference a material asset id",
                ));
                continue;
            };
            match bindings.get(&entry.voxel_material) {
                Some(existing) if existing == material_asset_id => {}
                Some(_) => diagnostics.push(Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::DuplicateMaterialBinding,
                    format!("materialMap.{}", entry.voxel_material),
                    "export cannot represent one voxel material with multiple material asset ids",
                )),
                None => {
                    bindings.insert(entry.voxel_material, material_asset_id.clone());
                }
            }
        }
        for material in used_materials {
            if !bindings.contains_key(&material) {
                diagnostics.push(Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::InvalidMaterialReference,
                    format!("material.{material}"),
                    "export could not map a used voxel material to a material asset id",
                ));
            }
        }
        if diagnostics.is_empty() {
            Ok(bindings
                .into_iter()
                .map(
                    |(voxel_material, material_asset_id)| VoxelAssetMaterialBinding {
                        voxel_material,
                        palette_entry_id: Self::voxel_asset_palette_entry_id(&material_asset_id),
                        display_name: None,
                        material_catalog_binding_id: Some(Self::voxel_asset_catalog_binding_id(
                            &material_asset_id,
                        )),
                        material_asset_id,
                    },
                )
                .collect())
        } else {
            Err(diagnostics)
        }
    }

    pub(super) fn voxel_asset_palette_entry_id(material_asset_id: &str) -> String {
        format!(
            "voxel-material/{}",
            material_asset_id
                .strip_prefix("material/")
                .unwrap_or(material_asset_id)
        )
    }

    pub(super) fn voxel_asset_catalog_binding_id(material_asset_id: &str) -> String {
        format!(
            "catalog-binding/{}",
            material_asset_id
                .strip_prefix("material/")
                .unwrap_or(material_asset_id)
        )
    }

    pub(super) fn rejected_voxel_volume_asset_load(
        request: &VoxelVolumeAssetLoadRequest,
        diagnostics: Vec<VoxelAssetDiagnostic>,
    ) -> VoxelVolumeAssetLoadReceipt {
        let volume_asset_id = request
            .target_volume_asset_id
            .clone()
            .or_else(|| Some(request.asset.asset_id.clone()));
        let grid = request.target_grid;
        let model_id = Self::voxel_model_id(grid, &volume_asset_id);
        let session_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-volume-load|rejected|{}|{}|{:?}",
                request.asset.asset_id, grid, diagnostics
            ))
        );
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-volume-load|rejected-replay|{}|{}",
                request.asset.asset_id, session_hash
            ))
        );
        VoxelVolumeAssetLoadReceipt {
            request_asset_id: request.asset.asset_id.clone(),
            loaded: false,
            model_id,
            volume_asset_id,
            grid,
            bounds: None,
            voxel_count: 0,
            material_counts: Vec::new(),
            provenance: request.asset.provenance.clone(),
            canonical_json_hash: None,
            voxel_data_hash: None,
            session_hash,
            replay_hash,
            diagnostics,
        }
    }

    pub(super) fn voxel_asset_load_target(
        &self,
        request: &VoxelVolumeAssetLoadRequest,
    ) -> Result<VoxelConversionTargetAuthority, VoxelAssetDiagnostic> {
        let volume_asset_id = request
            .target_volume_asset_id
            .clone()
            .or_else(|| Some(request.asset.asset_id.clone()));
        if let Some(existing) = self.voxel_conversion_targets.get(&Self::voxel_model_key(
            request.target_grid,
            &volume_asset_id,
        )) {
            if (existing.spec.voxel_size() - request.asset.grid.cell_size).abs() > f64::EPSILON {
                return Err(Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::InvalidGrid,
                    "grid.cellSize",
                    "stored asset cell size does not match the registered runtime target grid",
                ));
            }
            return Ok(existing.clone());
        }
        let grid = u32::try_from(request.target_grid).map_err(|_| {
            Self::voxel_asset_diagnostic(
                VoxelAssetDiagnosticCode::InvalidGrid,
                "targetGrid",
                "target grid id must fit in u32",
            )
        })?;
        let origin = request.asset.grid.origin;
        let spec = VoxelGridSpec::new(
            GridId::new(grid),
            request.asset.grid.cell_size,
            Self::launch_grid().chunk_dims(),
        )
        .map(|spec| spec.with_origin(WorldPos::new(origin[0], origin[1], origin[2])))
        .ok_or_else(|| {
            Self::voxel_asset_diagnostic(
                VoxelAssetDiagnosticCode::InvalidGrid,
                "grid",
                "stored asset grid cannot create a runtime target grid",
            )
        })?;
        Ok(VoxelConversionTargetAuthority {
            spec,
            volume_asset_id,
        })
    }

    pub(super) fn voxel_asset_load_commands(
        asset: &VoxelVolumeAsset,
        grid: GridId,
    ) -> BridgeResult<CommandBatch> {
        let mut commands = Vec::new();
        for run in &asset.representation.sparse_runs {
            for offset in 0..run.length {
                let x = run.start.x + i64::from(offset);
                commands.push(set_voxel_command(
                    grid.raw(),
                    x,
                    run.start.y,
                    run.start.z,
                    run.material,
                ));
            }
        }
        Ok(CommandBatch { commands })
    }

    pub(super) fn voxel_asset_load_candidate(
        &self,
        target: &VoxelConversionTargetAuthority,
        replace_existing: bool,
    ) -> VoxelWorld {
        if replace_existing {
            return VoxelWorld::new(target.spec);
        }
        match &self.voxel {
            Some(world) if world.grid().id() == target.spec.id() => world.clone(),
            _ => VoxelWorld::new(target.spec),
        }
    }

    pub(super) fn ensure_candidate_chunks_for_asset(
        asset: &VoxelVolumeAsset,
        spec: &VoxelGridSpec,
        candidate: &mut VoxelWorld,
    ) {
        for run in &asset.representation.sparse_runs {
            for offset in 0..run.length {
                let coord =
                    VoxelCoord::new(run.start.x + i64::from(offset), run.start.y, run.start.z);
                let chunk = spec.voxel_to_chunk(coord);
                if candidate.get(chunk).is_none() {
                    candidate.insert(chunk, VoxelChunk::from_spec(spec));
                }
            }
        }
    }

    pub(super) fn loaded_voxel_asset_info(
        request: &VoxelVolumeAssetLoadRequest,
        target: &VoxelConversionTargetAuthority,
    ) -> VoxelModelInfoAuthority {
        let asset = &request.asset;
        let volume_asset_id = target.volume_asset_id.clone();
        let grid = target.spec.id().raw() as u64;
        let material_counts = Self::voxel_asset_material_counts(asset)
            .into_iter()
            .map(|count| VoxelModelMaterialCount {
                material: count.material,
                voxel_count: count.voxel_count,
            })
            .collect::<Vec<_>>();
        let model_id = Self::voxel_model_id(grid, &volume_asset_id);
        let evidence = asset
            .provenance
            .iter()
            .map(|provenance| VoxelConversionEvidenceRef {
                kind: protocol_voxel_conversion::VoxelConversionEvidenceKind::OutputSnapshot,
                uri: provenance.uri.clone(),
                content_hash: provenance.content_hash.clone(),
            })
            .collect::<Vec<_>>();
        let session_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-volume-load|session|{}|{}|{}|{}",
                asset.asset_id,
                model_id,
                asset.content_hashes.canonical_json,
                asset.content_hashes.voxel_data
            ))
        );
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-volume-load|replay|{}|{}|{:?}",
                asset.asset_id, session_hash, asset.provenance
            ))
        );
        VoxelModelInfoAuthority {
            model_id,
            volume_asset_id,
            grid,
            bounds: Some(protocol_voxel_conversion::VoxelConversionBounds {
                min: protocol_voxel_conversion::VoxelConversionCoord {
                    x: asset.bounds.min.x,
                    y: asset.bounds.min.y,
                    z: asset.bounds.min.z,
                },
                max: protocol_voxel_conversion::VoxelConversionCoord {
                    x: asset.bounds.max.x,
                    y: asset.bounds.max.y,
                    z: asset.bounds.max.z,
                },
            }),
            voxel_count: material_counts.iter().map(|count| count.voxel_count).sum(),
            material_counts,
            source: protocol_voxel_conversion::VoxelConversionSourceRef {
                asset_id: asset.asset_id.clone(),
                asset_kind: "voxel_volume_asset".to_string(),
                asset_version: u64::from(asset.schema_version),
                source_hash: asset.content_hashes.voxel_data.clone(),
                mesh_primitive: None,
            },
            latest_plan_id: "stored-voxel-volume-load".to_string(),
            latest_output_hash: asset.content_hashes.voxel_data.clone(),
            session_hash,
            replay_hash,
            evidence,
        }
    }

    pub(super) fn voxel_volume_asset_load_receipt(
        request: &VoxelVolumeAssetLoadRequest,
        target: &VoxelConversionTargetAuthority,
        info: &VoxelModelInfoAuthority,
        loaded: bool,
        diagnostics: Vec<VoxelAssetDiagnostic>,
    ) -> VoxelVolumeAssetLoadReceipt {
        VoxelVolumeAssetLoadReceipt {
            request_asset_id: request.asset.asset_id.clone(),
            loaded,
            model_id: info.model_id.clone(),
            volume_asset_id: target.volume_asset_id.clone(),
            grid: info.grid,
            bounds: Some(request.asset.bounds),
            voxel_count: info.voxel_count,
            material_counts: if request.include_material_counts {
                info.material_counts
                    .iter()
                    .map(|count| VoxelAssetMaterialCount {
                        material: count.material,
                        voxel_count: count.voxel_count,
                    })
                    .collect()
            } else {
                Vec::new()
            },
            provenance: request.asset.provenance.clone(),
            canonical_json_hash: Some(request.asset.content_hashes.canonical_json.clone()),
            voxel_data_hash: Some(request.asset.content_hashes.voxel_data.clone()),
            session_hash: info.session_hash.clone(),
            replay_hash: info.replay_hash.clone(),
            diagnostics,
        }
    }
}
