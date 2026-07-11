use super::*;

impl EngineBridge {
    pub(super) fn initialize_voxel_volume_authoring_authority(
        &mut self,
        request: VoxelVolumeAuthoringInitializeRequest,
    ) -> BridgeResult<VoxelVolumeAuthoringInitializeReceipt> {
        self.require_initialized("initialize_voxel_volume_authoring")?;
        let diagnostics = self.voxel_volume_authoring_initialize_diagnostics(&request);
        if !diagnostics.is_empty() {
            return Ok(Self::rejected_voxel_volume_authoring_initialize(
                request,
                diagnostics,
            ));
        }
        let key = Self::voxel_model_key(request.grid, &request.volume_asset_id);
        if self.voxel_model_infos.contains_key(&key) {
            return Ok(Self::rejected_voxel_volume_authoring_initialize(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::StaleRuntimeSnapshot,
                    "runtimeModel",
                    "voxel authoring initialization requires an absent runtime model",
                )],
            ));
        }
        let Some(mut target) = self
            .voxel_conversion_targets
            .values()
            .find(|target| target.spec.id().raw() as u64 == request.grid)
            .cloned()
        else {
            return Ok(Self::rejected_voxel_volume_authoring_initialize(
                request,
                vec![Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::InvalidGrid,
                    "grid",
                    "voxel authoring initialization requires a registered runtime grid",
                )],
            ));
        };
        target.volume_asset_id = request.volume_asset_id.clone();
        let mut candidate = self
            .voxel
            .as_ref()
            .filter(|world| world.grid().id() == target.spec.id())
            .cloned()
            .unwrap_or_else(|| VoxelWorld::new(target.spec));
        let seed_chunk = ChunkCoord::new(
            request.seed_chunk.x,
            request.seed_chunk.y,
            request.seed_chunk.z,
        );
        if candidate.get(seed_chunk).is_none() {
            candidate.insert(seed_chunk, VoxelChunk::from_spec(&target.spec));
        }
        let model_id = Self::voxel_model_id(request.grid, &request.volume_asset_id);
        let latest_output_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-authoring-initialize|{}|{:?}|{:?}|{:?}",
                model_id, request.seed_chunk, request.material_palette, request.authoring
            ))
        );
        let session_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-authoring-initialize|session|{}|{}",
                model_id, latest_output_hash
            ))
        );
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-authoring-initialize|replay|{}|{}",
                model_id, session_hash
            ))
        );
        let info = VoxelModelInfoAuthority {
            model_id: model_id.clone(),
            volume_asset_id: request.volume_asset_id.clone(),
            grid: request.grid,
            bounds: None,
            voxel_count: 0,
            material_counts: Vec::new(),
            source: protocol_voxel_conversion::VoxelConversionSourceRef {
                asset_id: request
                    .volume_asset_id
                    .clone()
                    .unwrap_or_else(|| model_id.clone()),
                asset_kind: "voxel_volume_authoring".to_string(),
                asset_version: 1,
                source_hash: latest_output_hash.clone(),
                mesh_primitive: None,
            },
            latest_plan_id: "voxel-volume-authoring-initialize".to_string(),
            latest_output_hash,
            session_hash: session_hash.clone(),
            replay_hash: replay_hash.clone(),
            evidence: Vec::new(),
            authoring_edit_count: 0,
            material_palette: request.material_palette.clone(),
            authoring: request.authoring.clone(),
            resident_voxels: BTreeMap::new(),
            prior_voxels: BTreeMap::new(),
        };
        self.reset_voxel_edit_history(candidate);
        self.voxel_conversion_targets.insert(key.clone(), target);
        self.voxel_model_infos.insert(key.clone(), info);
        self.active_voxel_model = Some(key);
        Ok(VoxelVolumeAuthoringInitializeReceipt {
            volume_asset_id: request.volume_asset_id.clone(),
            grid: request.grid,
            request,
            initialized: true,
            model_id,
            session_hash,
            replay_hash,
            diagnostics: Vec::new(),
        })
    }

    fn voxel_volume_authoring_initialize_diagnostics(
        &self,
        request: &VoxelVolumeAuthoringInitializeRequest,
    ) -> Vec<VoxelAssetDiagnostic> {
        let mut diagnostics = Vec::new();
        if request
            .volume_asset_id
            .as_deref()
            .is_none_or(|asset_id| asset_id.trim().is_empty())
        {
            diagnostics.push(Self::voxel_asset_diagnostic(
                VoxelAssetDiagnosticCode::InvalidAssetId,
                "volumeAssetId",
                "voxel authoring initialization requires a non-empty volume asset id",
            ));
        }
        if request.max_material_bindings == 0
            || request.max_material_bindings > VOXEL_PALETTE_UPDATE_MAX_MATERIAL_BINDINGS
            || request.material_palette.len() as u64 > request.max_material_bindings
        {
            diagnostics.push(Self::voxel_asset_diagnostic(
                VoxelAssetDiagnosticCode::ExportLimitExceeded,
                "materialPalette",
                format!(
                    "material palette must fit a request limit in 1..={VOXEL_PALETTE_UPDATE_MAX_MATERIAL_BINDINGS}"
                ),
            ));
        }
        if request
            .authoring
            .label
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
            || request
                .authoring
                .created_by
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            || request
                .authoring
                .source_tool
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            diagnostics.push(Self::voxel_asset_diagnostic(
                VoxelAssetDiagnosticCode::InvalidAssetId,
                "authoring",
                "authoring label, creator, and source tool must be non-empty",
            ));
        }
        let mut materials = BTreeSet::new();
        for binding in &request.material_palette {
            if !materials.insert(binding.voxel_material) {
                diagnostics.push(Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::DuplicateMaterialBinding,
                    format!("materialPalette.{}", binding.voxel_material),
                    "material palette contains a duplicate voxel material binding",
                ));
            }
            if binding.material_asset_id.trim().is_empty() {
                diagnostics.push(Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::InvalidMaterialReference,
                    format!("materialPalette.{}", binding.voxel_material),
                    "material palette binding requires a material asset id",
                ));
            }
            if self
                .materials
                .validate(VoxelValue::solid_raw(binding.voxel_material))
                .is_err()
            {
                diagnostics.push(Self::voxel_asset_diagnostic(
                    VoxelAssetDiagnosticCode::UnknownVoxelMaterial,
                    format!("materialPalette.{}", binding.voxel_material),
                    "material palette references a material absent from runtime authority",
                ));
            }
        }
        diagnostics
    }

    fn rejected_voxel_volume_authoring_initialize(
        request: VoxelVolumeAuthoringInitializeRequest,
        diagnostics: Vec<VoxelAssetDiagnostic>,
    ) -> VoxelVolumeAuthoringInitializeReceipt {
        let model_id = Self::voxel_model_id(request.grid, &request.volume_asset_id);
        let session_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-authoring-initialize|rejected|{}|{:?}",
                model_id, diagnostics
            ))
        );
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-authoring-initialize|rejected-replay|{}|{}",
                model_id, session_hash
            ))
        );
        VoxelVolumeAuthoringInitializeReceipt {
            grid: request.grid,
            volume_asset_id: request.volume_asset_id.clone(),
            request,
            initialized: false,
            model_id,
            session_hash,
            replay_hash,
            diagnostics,
        }
    }
}
