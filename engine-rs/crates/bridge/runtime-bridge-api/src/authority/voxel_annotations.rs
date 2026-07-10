use super::*;

impl EngineBridge {
    pub(super) fn validate_voxel_annotation_layer_authority(
        &self,
        request: VoxelAnnotationLayerValidationRequest,
    ) -> BridgeResult<VoxelAnnotationLayerValidationReport> {
        self.require_initialized("validate_voxel_annotation_layer")?;
        Ok(svc_voxel_annotation::validate_layer(&request))
    }

    pub(super) fn load_voxel_annotation_layer_authority(
        &mut self,
        request: VoxelAnnotationLayerLoadRequest,
    ) -> BridgeResult<VoxelAnnotationLayerLoadReceipt> {
        self.require_initialized("load_voxel_annotation_layer")?;
        let key = (
            request.target_grid,
            Some(request.layer.target_voxel_volume_asset_id.clone()),
        );
        let Some(model) = self.voxel_model_infos.get(&key) else {
            return Ok(Self::voxel_annotation_load_receipt(
                &request,
                None,
                false,
                vec![Self::voxel_annotation_diagnostic(
                    VoxelAnnotationDiagnosticCode::LayerNotLoaded,
                    "targetVoxelVolumeAssetId",
                    "target voxel-volume asset is not loaded in runtime authority",
                )],
            ));
        };
        if request
            .expected_session_hash
            .as_deref()
            .is_some_and(|expected| expected != model.session_hash)
        {
            return Ok(Self::voxel_annotation_load_receipt(
                &request,
                Some(model.session_hash.clone()),
                false,
                vec![Self::voxel_annotation_diagnostic(
                    VoxelAnnotationDiagnosticCode::TargetVoxelHashMismatch,
                    "expectedSessionHash",
                    "annotation load expected a different target voxel-volume runtime session hash",
                )],
            ));
        }
        let validation =
            svc_voxel_annotation::validate_layer(&VoxelAnnotationLayerValidationRequest {
                input: VoxelAnnotationLayerValidationInput::Finalized {
                    layer: request.layer.clone(),
                },
                expected_target_voxel_volume_asset_id: Some(
                    request.layer.target_voxel_volume_asset_id.clone(),
                ),
                expected_target_voxel_data_hash: Some(model.latest_output_hash.clone()),
                max_regions: svc_voxel_annotation::DEFAULT_MAX_REGIONS,
                max_sparse_runs_per_region:
                    svc_voxel_annotation::DEFAULT_MAX_SPARSE_RUNS_PER_REGION,
                max_total_assigned_cells: svc_voxel_annotation::DEFAULT_MAX_TOTAL_ASSIGNED_CELLS,
            });
        if !validation.valid {
            return Ok(Self::voxel_annotation_load_receipt(
                &request,
                Some(model.session_hash.clone()),
                false,
                validation.diagnostics,
            ));
        }
        let runtime_layer_id = Self::voxel_annotation_runtime_layer_id(&request.layer);
        if !request.replace_existing && self.voxel_annotation_layers.contains_key(&runtime_layer_id)
        {
            return Ok(Self::voxel_annotation_load_receipt(
                &request,
                Some(model.session_hash.clone()),
                false,
                vec![Self::voxel_annotation_diagnostic(
                    VoxelAnnotationDiagnosticCode::EditConflict,
                    "runtimeLayerId",
                    "annotation layer is already loaded; set replaceExisting to replace it",
                )],
            ));
        }
        let layer = svc_voxel_annotation::with_computed_hashes(&request.layer);
        self.voxel_annotation_layers
            .insert(runtime_layer_id, layer.clone());
        Ok(Self::voxel_annotation_load_receipt(
            &VoxelAnnotationLayerLoadRequest { layer, ..request },
            Some(model.session_hash.clone()),
            true,
            Vec::new(),
        ))
    }

    pub(super) fn read_voxel_annotation_query_authority(
        &self,
        request: VoxelAnnotationQueryRequest,
    ) -> BridgeResult<VoxelAnnotationQueryReadout> {
        self.require_initialized("read_voxel_annotation_query")?;
        let Some(layer) = self.voxel_annotation_layer(&request.runtime_layer_id, &request.layer_id)
        else {
            return Ok(VoxelAnnotationQueryReadout {
                request,
                matched_regions: Vec::new(),
                region_count: 0,
                truncated: false,
                layer_hash: None,
                diagnostics: vec![Self::voxel_annotation_diagnostic(
                    VoxelAnnotationDiagnosticCode::LayerNotLoaded,
                    "layerId",
                    "annotation layer is not loaded",
                )],
            });
        };
        Ok(svc_voxel_annotation::query_layer(layer, &request))
    }

    pub(super) fn apply_voxel_annotation_edit_authority(
        &mut self,
        request: VoxelAnnotationEditRequest,
    ) -> BridgeResult<VoxelAnnotationEditReceipt> {
        self.require_initialized("apply_voxel_annotation_edit")?;
        let Some(runtime_layer_id) =
            self.voxel_annotation_layer_key(&request.runtime_layer_id, &request.layer_id)
        else {
            return Ok(Self::voxel_annotation_edit_receipt(
                request,
                String::new(),
                None,
                0,
                0,
                vec![Self::voxel_annotation_diagnostic(
                    VoxelAnnotationDiagnosticCode::LayerNotLoaded,
                    "layerId",
                    "annotation layer is not loaded",
                )],
            ));
        };
        let layer = self
            .voxel_annotation_layers
            .get(&runtime_layer_id)
            .expect("key came from map")
            .clone();
        let layer_hash_before = layer.content_hashes.canonical_json.clone();
        if request.expected_layer_hash.as_str() != layer_hash_before.as_str() {
            return Ok(Self::voxel_annotation_edit_receipt(
                request,
                layer_hash_before,
                None,
                layer.regions.len() as u64,
                Self::voxel_annotation_assigned_cells(&layer),
                vec![Self::voxel_annotation_diagnostic(
                    VoxelAnnotationDiagnosticCode::StaleLayerHash,
                    "expectedLayerHash",
                    "edit expected a different annotation layer hash",
                )],
            ));
        }
        let mut candidate = layer.clone();
        if let Some(diagnostic) =
            Self::apply_voxel_annotation_edit_to_layer(&mut candidate, &request)
        {
            return Ok(Self::voxel_annotation_edit_receipt(
                request,
                layer_hash_before,
                None,
                layer.regions.len() as u64,
                Self::voxel_annotation_assigned_cells(&layer),
                vec![diagnostic],
            ));
        }
        candidate = svc_voxel_annotation::with_computed_hashes(&candidate);
        let validation =
            svc_voxel_annotation::validate_layer(&VoxelAnnotationLayerValidationRequest {
                input: VoxelAnnotationLayerValidationInput::Finalized {
                    layer: candidate.clone(),
                },
                expected_target_voxel_volume_asset_id: Some(
                    candidate.target_voxel_volume_asset_id.clone(),
                ),
                expected_target_voxel_data_hash: Some(candidate.target_voxel_data_hash.clone()),
                max_regions: svc_voxel_annotation::DEFAULT_MAX_REGIONS,
                max_sparse_runs_per_region:
                    svc_voxel_annotation::DEFAULT_MAX_SPARSE_RUNS_PER_REGION,
                max_total_assigned_cells: svc_voxel_annotation::DEFAULT_MAX_TOTAL_ASSIGNED_CELLS,
            });
        if !validation.valid {
            return Ok(Self::voxel_annotation_edit_receipt(
                request,
                layer_hash_before,
                None,
                layer.regions.len() as u64,
                Self::voxel_annotation_assigned_cells(&layer),
                validation.diagnostics,
            ));
        }
        let layer_hash_after = candidate.content_hashes.canonical_json.clone();
        let region_count = candidate.regions.len() as u64;
        let assigned_cell_count = Self::voxel_annotation_assigned_cells(&candidate);
        self.voxel_annotation_layers
            .insert(runtime_layer_id, candidate);
        Ok(Self::voxel_annotation_edit_receipt(
            request,
            layer_hash_before,
            Some(layer_hash_after),
            region_count,
            assigned_cell_count,
            Vec::new(),
        ))
    }

    pub(super) fn export_voxel_annotation_layer_authority(
        &self,
        request: VoxelAnnotationLayerExportRequest,
    ) -> BridgeResult<VoxelAnnotationLayerExportReceipt> {
        self.require_initialized("export_voxel_annotation_layer")?;
        let Some(layer) = self.voxel_annotation_layer(&request.runtime_layer_id, &request.layer_id)
        else {
            return Ok(Self::rejected_voxel_annotation_export(
                request,
                vec![Self::voxel_annotation_diagnostic(
                    VoxelAnnotationDiagnosticCode::LayerNotLoaded,
                    "layerId",
                    "annotation layer is not loaded",
                )],
            ));
        };
        if request.expected_layer_hash.as_str() != layer.content_hashes.canonical_json.as_str() {
            return Ok(Self::rejected_voxel_annotation_export(
                request,
                vec![Self::voxel_annotation_diagnostic(
                    VoxelAnnotationDiagnosticCode::StaleLayerHash,
                    "expectedLayerHash",
                    "export expected a different annotation layer hash",
                )],
            ));
        }
        let validation =
            svc_voxel_annotation::validate_layer(&VoxelAnnotationLayerValidationRequest {
                input: VoxelAnnotationLayerValidationInput::Finalized {
                    layer: layer.clone(),
                },
                expected_target_voxel_volume_asset_id: Some(
                    layer.target_voxel_volume_asset_id.clone(),
                ),
                expected_target_voxel_data_hash: Some(layer.target_voxel_data_hash.clone()),
                max_regions: svc_voxel_annotation::DEFAULT_MAX_REGIONS,
                max_sparse_runs_per_region:
                    svc_voxel_annotation::DEFAULT_MAX_SPARSE_RUNS_PER_REGION,
                max_total_assigned_cells: svc_voxel_annotation::DEFAULT_MAX_TOTAL_ASSIGNED_CELLS,
            });
        if !validation.valid {
            return Ok(Self::rejected_voxel_annotation_export(
                request,
                validation.diagnostics,
            ));
        }
        let canonical_json = svc_voxel_annotation::encode_layer(
            &VoxelAnnotationLayerValidationRequest {
                input: VoxelAnnotationLayerValidationInput::Finalized {
                    layer: layer.clone(),
                },
                expected_target_voxel_volume_asset_id: Some(
                    layer.target_voxel_volume_asset_id.clone(),
                ),
                expected_target_voxel_data_hash: Some(layer.target_voxel_data_hash.clone()),
                max_regions: svc_voxel_annotation::DEFAULT_MAX_REGIONS,
                max_sparse_runs_per_region:
                    svc_voxel_annotation::DEFAULT_MAX_SPARSE_RUNS_PER_REGION,
                max_total_assigned_cells: svc_voxel_annotation::DEFAULT_MAX_TOTAL_ASSIGNED_CELLS,
            },
        )
        .map_err(|report| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!(
                    "validated voxel annotation layer failed canonical encode with {} diagnostic(s)",
                    report.diagnostics.len()
                ),
            )
        })?;
        let mut exported = layer.clone();
        if !request.include_diagnostics {
            exported.validation_diagnostics.clear();
        }
        Ok(VoxelAnnotationLayerExportReceipt {
            request,
            exported: true,
            layer: Some(exported),
            canonical_json: Some(canonical_json),
            canonical_json_hash: validation.canonical_json_hash,
            membership_data_hash: validation.membership_data_hash,
            diagnostics: Vec::new(),
        })
    }

    pub(super) fn voxel_annotation_layer_key(
        &self,
        runtime_layer_id: &Option<String>,
        layer_id: &str,
    ) -> Option<String> {
        if let Some(runtime_layer_id) = runtime_layer_id {
            return self
                .voxel_annotation_layers
                .contains_key(runtime_layer_id)
                .then(|| runtime_layer_id.clone());
        }
        self.voxel_annotation_layers
            .iter()
            .find_map(|(key, layer)| (layer.layer_id == layer_id).then(|| key.clone()))
    }

    pub(super) fn voxel_annotation_layer(
        &self,
        runtime_layer_id: &Option<String>,
        layer_id: &str,
    ) -> Option<&VoxelAnnotationLayer> {
        let key = self.voxel_annotation_layer_key(runtime_layer_id, layer_id)?;
        self.voxel_annotation_layers.get(&key)
    }

    pub(super) fn voxel_annotation_runtime_layer_id(layer: &VoxelAnnotationLayer) -> String {
        format!("runtime-annotation/{}", layer.layer_id)
    }

    pub(super) fn voxel_annotation_assigned_cells(layer: &VoxelAnnotationLayer) -> u64 {
        layer
            .regions
            .iter()
            .flat_map(|region| &region.selection.sparse_runs)
            .map(|run| u64::from(run.length))
            .sum()
    }

    pub(super) fn voxel_annotation_diagnostic(
        code: VoxelAnnotationDiagnosticCode,
        reference: impl Into<String>,
        message: impl Into<String>,
    ) -> VoxelAnnotationDiagnostic {
        VoxelAnnotationDiagnostic {
            code,
            severity: DiagnosticSeverity::Error,
            reference: reference.into(),
            message: message.into(),
        }
    }

    pub(super) fn voxel_annotation_load_receipt(
        request: &VoxelAnnotationLayerLoadRequest,
        target_session_hash: Option<String>,
        loaded: bool,
        diagnostics: Vec<VoxelAnnotationDiagnostic>,
    ) -> VoxelAnnotationLayerLoadReceipt {
        let layer_hash = loaded.then(|| request.layer.content_hashes.canonical_json.clone());
        let runtime_layer_id =
            loaded.then(|| Self::voxel_annotation_runtime_layer_id(&request.layer));
        let session_hash = target_session_hash.unwrap_or_else(|| {
            format!(
                "fnv1a64:{}",
                Self::fnv1a64(&format!(
                    "voxel-annotation-load|{}|{}|{}|{:?}",
                    request.layer.layer_id,
                    request.layer.target_voxel_volume_asset_id,
                    request.target_grid,
                    diagnostics
                ))
            )
        });
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-annotation-load|{}|{}|{}|{}|{:?}",
                request.layer.layer_id,
                loaded,
                request.layer.content_hashes.canonical_json,
                session_hash,
                diagnostics
            ))
        );
        VoxelAnnotationLayerLoadReceipt {
            request_layer_id: request.layer.layer_id.clone(),
            loaded,
            runtime_layer_id,
            target_voxel_volume_asset_id: request.layer.target_voxel_volume_asset_id.clone(),
            target_voxel_data_hash: request.layer.target_voxel_data_hash.clone(),
            region_count: request.layer.regions.len() as u64,
            assigned_cell_count: Self::voxel_annotation_assigned_cells(&request.layer),
            layer_hash,
            session_hash,
            replay_hash,
            diagnostics,
        }
    }

    pub(super) fn voxel_annotation_edit_receipt(
        request: VoxelAnnotationEditRequest,
        layer_hash_before: String,
        layer_hash_after: Option<String>,
        region_count: u64,
        assigned_cell_count: u64,
        diagnostics: Vec<VoxelAnnotationDiagnostic>,
    ) -> VoxelAnnotationEditReceipt {
        let edited = diagnostics.is_empty() && layer_hash_after.is_some();
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "voxel-annotation-edit|{}|{}|{}|{:?}|{:?}",
                request.layer_id, edited, layer_hash_before, layer_hash_after, diagnostics
            ))
        );
        VoxelAnnotationEditReceipt {
            request,
            edited,
            layer_hash_before,
            layer_hash_after,
            region_count,
            assigned_cell_count,
            diagnostics,
            replay_hash,
        }
    }

    pub(super) fn rejected_voxel_annotation_export(
        request: VoxelAnnotationLayerExportRequest,
        diagnostics: Vec<VoxelAnnotationDiagnostic>,
    ) -> VoxelAnnotationLayerExportReceipt {
        VoxelAnnotationLayerExportReceipt {
            request,
            exported: false,
            layer: None,
            canonical_json: None,
            canonical_json_hash: None,
            membership_data_hash: None,
            diagnostics,
        }
    }

    pub(super) fn apply_voxel_annotation_edit_to_layer(
        layer: &mut VoxelAnnotationLayer,
        request: &VoxelAnnotationEditRequest,
    ) -> Option<VoxelAnnotationDiagnostic> {
        match request.operation {
            VoxelAnnotationEditOperation::UpsertRegion => {
                let Some(region) = request.region.clone() else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "region",
                        "upsert_region requires a region payload",
                    ));
                };
                match Self::voxel_annotation_region_index(layer, &region.region_id) {
                    Some(index) => layer.regions[index] = region,
                    None => layer.regions.push(region),
                }
            }
            VoxelAnnotationEditOperation::RemoveRegion => {
                let Some(region_id) = request.region_id.as_deref() else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "regionId",
                        "remove_region requires regionId",
                    ));
                };
                let Some(index) = Self::voxel_annotation_region_index(layer, region_id) else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "regionId",
                        "remove_region targeted an unknown region",
                    ));
                };
                layer.regions.remove(index);
                for region in &mut layer.regions {
                    if region.parent_region_id.as_deref() == Some(region_id) {
                        region.parent_region_id = None;
                    }
                }
            }
            VoxelAnnotationEditOperation::AddRuns => {
                let Some(region) = Self::voxel_annotation_region_mut(layer, request) else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "regionId",
                        "add_runs targeted an unknown region",
                    ));
                };
                if request.sparse_runs.is_empty() {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidSparseRun,
                        "sparseRuns",
                        "add_runs requires at least one sparse run",
                    ));
                }
                region
                    .selection
                    .sparse_runs
                    .extend(request.sparse_runs.iter().cloned());
                Self::sort_voxel_annotation_runs(&mut region.selection.sparse_runs);
            }
            VoxelAnnotationEditOperation::RemoveRuns => {
                let Some(region) = Self::voxel_annotation_region_mut(layer, request) else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "regionId",
                        "remove_runs targeted an unknown region",
                    ));
                };
                if request.sparse_runs.is_empty() {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidSparseRun,
                        "sparseRuns",
                        "remove_runs requires at least one sparse run",
                    ));
                }
                region
                    .selection
                    .sparse_runs
                    .retain(|run| !request.sparse_runs.iter().any(|removal| removal == run));
            }
            VoxelAnnotationEditOperation::ReplaceSelection => {
                let Some(region) = Self::voxel_annotation_region_mut(layer, request) else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "regionId",
                        "replace_selection targeted an unknown region",
                    ));
                };
                region.selection.sparse_runs = request.sparse_runs.clone();
                Self::sort_voxel_annotation_runs(&mut region.selection.sparse_runs);
            }
            VoxelAnnotationEditOperation::SetParent => {
                let Some(region) = Self::voxel_annotation_region_mut(layer, request) else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "regionId",
                        "set_parent targeted an unknown region",
                    ));
                };
                region.parent_region_id = request.parent_region_id.clone();
            }
            VoxelAnnotationEditOperation::SetTags => {
                let Some(region) = Self::voxel_annotation_region_mut(layer, request) else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "regionId",
                        "set_tags targeted an unknown region",
                    ));
                };
                let mut tags = request.tags.clone();
                tags.sort();
                tags.dedup();
                region.tags = tags;
            }
            VoxelAnnotationEditOperation::SetLabel => {
                let Some(label) = request.label.clone() else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "label",
                        "set_label requires label",
                    ));
                };
                let Some(region) = Self::voxel_annotation_region_mut(layer, request) else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "regionId",
                        "set_label targeted an unknown region",
                    ));
                };
                region.label = label;
            }
            VoxelAnnotationEditOperation::SetKind => {
                let Some(kind) = request.kind else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::UnsupportedAnnotationKind,
                        "kind",
                        "set_kind requires kind",
                    ));
                };
                let Some(region) = Self::voxel_annotation_region_mut(layer, request) else {
                    return Some(Self::voxel_annotation_diagnostic(
                        VoxelAnnotationDiagnosticCode::InvalidRegionId,
                        "regionId",
                        "set_kind targeted an unknown region",
                    ));
                };
                region.kind = kind;
            }
        }
        None
    }

    pub(super) fn voxel_annotation_region_mut<'a>(
        layer: &'a mut VoxelAnnotationLayer,
        request: &VoxelAnnotationEditRequest,
    ) -> Option<&'a mut VoxelAnnotationRegion> {
        let region_id = request.region_id.as_deref()?;
        layer
            .regions
            .iter_mut()
            .find(|region| region.region_id == region_id)
    }

    pub(super) fn voxel_annotation_region_index(
        layer: &VoxelAnnotationLayer,
        region_id: &str,
    ) -> Option<usize> {
        layer
            .regions
            .iter()
            .position(|region| region.region_id == region_id)
    }

    pub(super) fn sort_voxel_annotation_runs(runs: &mut [VoxelAnnotationSparseRun]) {
        runs.sort_by_key(|run| (run.start.z, run.start.y, run.start.x));
    }

    pub(super) fn voxel_asset_material_counts(
        asset: &VoxelVolumeAsset,
    ) -> Vec<VoxelAssetMaterialCount> {
        let mut counts = BTreeMap::<u16, u64>::new();
        for run in &asset.representation.sparse_runs {
            *counts.entry(run.material).or_insert(0) += u64::from(run.length);
        }
        counts
            .into_iter()
            .map(|(material, voxel_count)| VoxelAssetMaterialCount {
                material,
                voxel_count,
            })
            .collect()
    }

    pub(super) fn voxel_state_hash(world: &VoxelWorld) -> String {
        let mut buf = String::new();
        for (coord, chunk) in world.resident_chunks() {
            buf.push_str(&format!(
                "{},{},{}={:016x};",
                coord.x,
                coord.y,
                coord.z,
                chunk.content_hash().0
            ));
        }
        BundleHash::of_str(&buf).to_hex()
    }

    pub(super) fn mesh_payload_hash(mesh: &svc_mesh::MeshPayload) -> String {
        format!("fnv1a64:{}", Self::fnv1a64(&mesh.to_fixture_string()))
    }

    pub(super) fn mesh_evidence_for(
        world: &VoxelWorld,
        coord: ChunkCoord,
    ) -> (VoxelMeshChunkEvidence, Vec<String>) {
        let Some(chunk) = world.get(coord) else {
            return (
                VoxelMeshChunkEvidence {
                    coord,
                    resident: false,
                    visible: false,
                    content_hash: None,
                    mesh_hash: None,
                    stats: None,
                    bounds: None,
                    material_slots: Vec::new(),
                },
                Vec::new(),
            );
        };

        match mesh_chunk_in_world(world, coord) {
            Some(Ok(mesh)) if !mesh.indices.is_empty() => {
                let stats = mesh.stats;
                (
                    VoxelMeshChunkEvidence {
                        coord,
                        resident: true,
                        visible: true,
                        content_hash: Some(format!("{:016x}", chunk.content_hash().0)),
                        mesh_hash: Some(Self::mesh_payload_hash(&mesh)),
                        stats: Some(VoxelMeshStatsEvidence {
                            vertices: stats.vertices,
                            indices: stats.indices,
                            quads: stats.quads,
                            faces_emitted: stats.faces_emitted,
                            faces_culled: stats.faces_culled,
                        }),
                        bounds: Some(VoxelMeshBoundsEvidence {
                            min: mesh.bounds.min,
                            max: mesh.bounds.max,
                        }),
                        material_slots: mesh.groups.iter().map(|g| g.material_slot).collect(),
                    },
                    Vec::new(),
                )
            }
            Some(Ok(_)) => (
                VoxelMeshChunkEvidence {
                    coord,
                    resident: true,
                    visible: false,
                    content_hash: Some(format!("{:016x}", chunk.content_hash().0)),
                    mesh_hash: None,
                    stats: None,
                    bounds: None,
                    material_slots: Vec::new(),
                },
                Vec::new(),
            ),
            Some(Err(err)) => (
                VoxelMeshChunkEvidence {
                    coord,
                    resident: true,
                    visible: false,
                    content_hash: Some(format!("{:016x}", chunk.content_hash().0)),
                    mesh_hash: None,
                    stats: None,
                    bounds: None,
                    material_slots: Vec::new(),
                },
                vec![format!(
                    "mesh error for chunk {},{},{}: {err}",
                    coord.x, coord.y, coord.z
                )],
            ),
            None => unreachable!("world.get(coord) already proved resident"),
        }
    }
}
