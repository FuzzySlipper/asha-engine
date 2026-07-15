use super::*;

impl EngineBridge {
    pub(super) fn load_project_bundle_authority(
        &mut self,
        request: ProjectBundleLoadRequest,
    ) -> BridgeResult<CompositionStatus> {
        // Fail closed on a newer bundle; the prior loaded ProjectBundle is left untouched.
        if request.bundle_schema_version > ENGINE_SUPPORTED_VERSION
            || request.protocol_version > ENGINE_SUPPORTED_VERSION
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "unsupported bundle schema {} / protocol {}",
                    request.bundle_schema_version, request.protocol_version
                ),
            ));
        }
        if self.bundle.loaded_project_bundle != Some(request.scene_id) {
            let teardown = self.projection.voxel_projector.clear();
            self.projection.pending_voxel_frame.ops.extend(teardown.ops);
            self.projection.voxel_instance_binding = None;
        }
        self.bundle.loaded_project_bundle = Some(request.scene_id);
        Ok(CompositionStatus {
            loaded_project_bundle: Some(request.scene_id),
            ..CompositionStatus::empty()
        })
    }

    pub(super) fn save_project_bundle_authority(
        &mut self,
    ) -> BridgeResult<ProjectBundleSaveSummary> {
        if self.bundle.loaded_project_bundle.is_none() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "save_project_bundle called with no ProjectBundle loaded",
            ));
        }
        Ok(ProjectBundleSaveSummary {
            artifacts_written: 3,
            compacted_edits: 0,
            retained_edits: 0,
        })
    }

    pub(super) fn project_bundle_composition_status_authority(
        &self,
    ) -> BridgeResult<CompositionStatus> {
        Ok(CompositionStatus {
            loaded_project_bundle: self.bundle.loaded_project_bundle,
            ..CompositionStatus::empty()
        })
    }

    pub(super) fn unload_project_bundle_authority(&mut self) -> BridgeResult<()> {
        let teardown = self.projection.voxel_projector.clear();
        self.projection.pending_voxel_frame.ops.extend(teardown.ops);
        self.projection.voxel_instance_binding = None;
        self.bundle.loaded_project_bundle = None;
        self.input.input_session = None;
        self.scene.entities = EntityStore::new();
        self.gameplay.fps_session = None;
        self.gameplay.fps_seed = None;
        self.gameplay.fps_epoch = 0;
        self.gameplay.static_gameplay_host = None;
        self.gameplay.static_gameplay_reset_checkpoint = None;
        self.gameplay.static_gameplay_base_entities = None;
        self.gameplay.game_rule_modules.clear();
        self.gameplay.game_rule_active_modifiers.clear();
        self.gameplay.game_rule_recent_trace.clear();
        self.evidence.game_rule_recent_replay_hashes.clear();
        Ok(())
    }

    pub(super) fn register_voxel_conversion_mesh_asset_authority(
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
        self.voxel
            .voxel_conversion_sources
            .insert(source.asset_id.clone(), source);
        self.voxel.voxel_conversion_source_metadata.insert(
            request.source.asset_id.clone(),
            Self::source_metadata_from_project_mesh_asset(&request),
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
            material_slots: request.mesh_asset.material_slots,
            diagnostics: Vec::new(),
            evidence,
        })
    }

    pub(super) fn import_voxel_conversion_mesh_source_authority(
        &mut self,
        request: VoxelConversionMeshSourceImportRequest,
    ) -> BridgeResult<VoxelConversionMeshSourceImportReceipt> {
        self.require_initialized("import_voxel_conversion_mesh_source")?;
        let source_byte_count = request.source_bytes.len() as u64;
        let preflight_error = svc_mesh_import::preflight_import_request(&request).err();
        let source_hash = if preflight_error.is_none() {
            svc_mesh_import::source_sha256(&request.source_bytes)
        } else {
            "sha256:not-computed".to_string()
        };
        let rejected_source = protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: request.source_asset_id.clone(),
            asset_kind: "mesh".to_string(),
            asset_version: request.asset_version,
            source_hash,
            mesh_primitive: request.mesh_primitive.clone(),
        };
        if let Some(error) = preflight_error {
            return Ok(VoxelConversionMeshSourceImportReceipt {
                source: rejected_source,
                imported: false,
                source_path: request.source_path,
                format: request.format,
                source_byte_count,
                mesh_asset: None,
                source_bounds: None,
                vertex_count: 0,
                triangle_count: 0,
                groups: Vec::new(),
                material_slots: Vec::new(),
                diagnostics: vec![Self::voxel_conversion_diagnostic(
                    Self::mesh_import_diagnostic_code(error.kind),
                    "meshImportPreflight",
                    error.message,
                )],
                evidence: Vec::new(),
            });
        }
        let imported = match svc_mesh_import::import_static_mesh(&request) {
            Ok(imported) => imported,
            Err(error) => {
                return Ok(VoxelConversionMeshSourceImportReceipt {
                    source: rejected_source,
                    imported: false,
                    source_path: request.source_path,
                    format: request.format,
                    source_byte_count,
                    mesh_asset: None,
                    source_bounds: None,
                    vertex_count: 0,
                    triangle_count: 0,
                    groups: Vec::new(),
                    material_slots: Vec::new(),
                    diagnostics: vec![Self::voxel_conversion_diagnostic(
                        Self::mesh_import_diagnostic_code(error.kind),
                        "sourceBytes",
                        error.message,
                    )],
                    evidence: Vec::new(),
                });
            }
        };
        let registration_request = VoxelConversionMeshAssetRegistrationRequest {
            source: imported.source.clone(),
            mesh_asset: imported.mesh_asset.clone(),
        };
        let source = match Self::static_mesh_source_from_project_mesh_asset(&registration_request) {
            Ok(source) => source,
            Err(message) => {
                return Ok(VoxelConversionMeshSourceImportReceipt {
                    source: imported.source,
                    imported: false,
                    source_path: request.source_path,
                    format: request.format,
                    source_byte_count,
                    mesh_asset: None,
                    source_bounds: None,
                    vertex_count: 0,
                    triangle_count: 0,
                    groups: Vec::new(),
                    material_slots: Vec::new(),
                    diagnostics: vec![Self::voxel_conversion_diagnostic(
                        VoxelConversionDiagnosticCode::UnsupportedSourceAsset,
                        "canonicalGeometry",
                        message,
                    )],
                    evidence: Vec::new(),
                });
            }
        };
        let metadata = Self::source_metadata_from_project_mesh_asset(&registration_request);
        self.voxel
            .voxel_conversion_sources
            .insert(source.asset_id.clone(), source);
        self.voxel
            .voxel_conversion_source_metadata
            .insert(imported.source.asset_id.clone(), metadata.clone());
        self.voxel.voxel_conversion_plan = None;
        self.remember_voxel_conversion_evidence(metadata.evidence.clone());
        Ok(VoxelConversionMeshSourceImportReceipt {
            source: imported.source,
            imported: true,
            source_path: request.source_path,
            format: request.format,
            source_byte_count,
            mesh_asset: Some(imported.mesh_asset),
            source_bounds: metadata.source_bounds,
            vertex_count: metadata.vertex_count,
            triangle_count: metadata.triangle_count,
            groups: metadata.groups,
            material_slots: metadata.material_slots,
            diagnostics: Vec::new(),
            evidence: metadata.evidence,
        })
    }

    pub(super) fn mesh_import_diagnostic_code(
        kind: svc_mesh_import::MeshImportErrorKind,
    ) -> VoxelConversionDiagnosticCode {
        match kind {
            svc_mesh_import::MeshImportErrorKind::QuotaExceeded => {
                VoxelConversionDiagnosticCode::OutputLimitExceeded
            }
            svc_mesh_import::MeshImportErrorKind::InvalidRequest
            | svc_mesh_import::MeshImportErrorKind::UnsupportedFeature
            | svc_mesh_import::MeshImportErrorKind::InvalidGeometry => {
                VoxelConversionDiagnosticCode::UnsupportedSourceAsset
            }
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    /// The default launch grid: id 1, voxel size 1.0, cubic 2×2×2 chunks (matches
    /// the canonical voxel fixture). Chunk dims come from the spec, not a global.
    pub(super) fn launch_grid() -> VoxelGridSpec {
        VoxelGridSpec::new(
            GridId::new(1),
            1.0,
            ChunkDims::cubic(2).expect("nonzero dims"),
        )
        .expect("positive voxel size")
    }

    pub(super) fn material_for_chunk(coord: ChunkCoord) -> u16 {
        const MATERIAL_IDS: [u16; 3] = [1, 2, 3];
        let idx = (coord.x * 2 + coord.y).rem_euclid(MATERIAL_IDS.len() as i64) as usize;
        MATERIAL_IDS[idx]
    }

    pub(super) fn launch_world() -> VoxelWorld {
        let spec = Self::launch_grid();
        let mut world = VoxelWorld::new(spec);
        let dims = spec.chunk_dims();
        for coord in [
            ChunkCoord::new(0, 0, 0),
            ChunkCoord::new(1, 0, 0),
            ChunkCoord::new(0, 1, 0),
            ChunkCoord::new(1, 1, 0),
        ] {
            let mut chunk = VoxelChunk::from_spec(&spec);
            chunk
                .fill_region(
                    core_space::LocalVoxelCoord::new(0, 0, 0),
                    core_space::LocalVoxelCoord::new(dims.x(), dims.y(), 1),
                    VoxelValue::solid_raw(Self::material_for_chunk(coord)),
                )
                .expect("canonical launch chunk fill within bounds");
            world.insert(coord, chunk);
        }
        let _ = world.drain_dirty();
        world
    }

    pub(super) fn fixture_quad_source() -> StaticMeshSource {
        StaticMeshSource {
            asset_id: "mesh/quad".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 1,
            source_hash: "sha256:quad".to_string(),
            mesh_primitive: None,
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            triangles: vec![
                MeshTriangle {
                    indices: [0, 1, 2],
                    source_material_slot: 0,
                },
                MeshTriangle {
                    indices: [0, 2, 3],
                    source_material_slot: 1,
                },
            ],
        }
    }

    pub(super) fn project_triangle_static_mesh_asset() -> StaticMeshAsset {
        StaticMeshAsset {
            asset: "mesh/import-fixture-a".to_string(),
            payload: MeshPayloadDescriptor {
                layout: MeshBufferLayout {
                    vertex_count: 3,
                    index_count: 3,
                    index_width: MeshIndexWidth::U32,
                    attributes: vec![
                        MeshAttribute {
                            name: MeshAttributeName::Position,
                            components: 3,
                            kind: MeshAttributeKind::F32,
                        },
                        MeshAttribute {
                            name: MeshAttributeName::Normal,
                            components: 3,
                            kind: MeshAttributeKind::F32,
                        },
                    ],
                },
                groups: vec![MeshGroupDescriptor {
                    material_slot: 0,
                    start: 0,
                    count: 3,
                }],
                bounds: MeshBoundsDescriptor {
                    min: [0.0, 0.0, 0.0],
                    max: [1.0, 1.0, 0.0],
                },
                source: MeshPayloadSource::Inline {
                    positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                    normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
                    indices: vec![0, 1, 2],
                },
                provenance: MeshProvenance::StaticAsset,
            },
            material_slots: vec![MeshMaterialSlot {
                slot: 0,
                material: "material/surface-a".to_string(),
            }],
            collision: MeshCollisionPolicy::AabbFallback,
        }
    }

    pub(super) fn static_mesh_source_from_asset(
        asset: &StaticMeshAsset,
        asset_version: u64,
        source_hash: impl Into<String>,
        mesh_primitive: Option<String>,
    ) -> BridgeResult<StaticMeshSource> {
        asset.validate().map_err(|err| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("static mesh asset cannot seed voxel conversion authority: {err:?}"),
            )
        })?;
        if asset.payload.provenance != MeshProvenance::StaticAsset {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel conversion source must be an authored static mesh asset",
            ));
        }
        let MeshPayloadSource::Inline {
            positions, indices, ..
        } = &asset.payload.source
        else {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel conversion source requires authority-visible inline mesh geometry",
            ));
        };
        let mut triangles = Vec::new();
        for group in &asset.payload.groups {
            if group.count % 3 != 0 {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    "voxel conversion source mesh group is not a triangle list",
                ));
            }
            let start = group.start as usize;
            let end = start + group.count as usize;
            for tri in indices[start..end].chunks_exact(3) {
                triangles.push(MeshTriangle {
                    indices: [tri[0], tri[1], tri[2]],
                    source_material_slot: group.material_slot as u32,
                });
            }
        }

        Ok(StaticMeshSource {
            asset_id: asset.asset.clone(),
            asset_kind: "mesh".to_string(),
            asset_version,
            source_hash: source_hash.into(),
            mesh_primitive,
            positions: positions
                .chunks_exact(3)
                .map(|position| [position[0], position[1], position[2]])
                .collect(),
            triangles,
        })
    }

    pub(super) fn static_mesh_source_from_project_mesh_asset(
        request: &VoxelConversionMeshAssetRegistrationRequest,
    ) -> Result<StaticMeshSource, String> {
        let asset = &request.mesh_asset;
        if request.source.asset_id != asset.asset_id
            || request.source.asset_kind != "mesh"
            || request.source.asset_version == 0
            || request.source.source_hash.is_empty()
        {
            return Err(
                "mesh asset source ref does not match a supported project mesh asset identity"
                    .to_string(),
            );
        }
        if !matches!(
            request.source.mesh_primitive.as_deref(),
            None | Some("default")
        ) {
            return Err("only the default mesh primitive is supported for project mesh asset voxel conversion".to_string());
        }
        if asset.positions.is_empty() || asset.indices.is_empty() || asset.groups.is_empty() {
            return Err(
                "project mesh asset must contain positions, indices, and triangle groups"
                    .to_string(),
            );
        }
        if !asset.normals.is_empty() && asset.normals.len() != asset.positions.len() {
            return Err(
                "project mesh asset normals must either be omitted or match position count"
                    .to_string(),
            );
        }
        let material_slots = asset
            .material_slots
            .iter()
            .map(|slot| slot.source_material_slot)
            .collect::<BTreeSet<_>>();
        let mut covered = 0u64;
        let mut triangles = Vec::new();
        for group in &asset.groups {
            if group.count % 3 != 0 {
                return Err("project mesh asset group is not a triangle list".to_string());
            }
            if !material_slots.contains(&group.material_slot) {
                return Err(
                    "project mesh asset group references an unbound material slot".to_string(),
                );
            }
            let start = group.start as usize;
            let end = start + group.count as usize;
            if end > asset.indices.len() {
                return Err(
                    "project mesh asset group range is outside the index buffer".to_string()
                );
            }
            covered += u64::from(group.count);
            for tri in asset.indices[start..end].chunks_exact(3) {
                if tri
                    .iter()
                    .any(|index| *index as usize >= asset.positions.len())
                {
                    return Err(
                        "project mesh asset index references a missing position".to_string()
                    );
                }
                triangles.push(MeshTriangle {
                    indices: [tri[0], tri[1], tri[2]],
                    source_material_slot: group.material_slot,
                });
            }
        }
        if covered != asset.indices.len() as u64 {
            return Err(
                "project mesh asset groups must exactly cover the index buffer".to_string(),
            );
        }
        Ok(StaticMeshSource {
            asset_id: request.source.asset_id.clone(),
            asset_kind: "mesh".to_string(),
            asset_version: request.source.asset_version,
            source_hash: request.source.source_hash.clone(),
            mesh_primitive: request.source.mesh_primitive.clone(),
            positions: asset.positions.clone(),
            triangles,
        })
    }

    pub(super) fn source_registration_diagnostic(
        source: &protocol_voxel_conversion::VoxelConversionSourceRef,
        message: impl Into<String>,
    ) -> VoxelConversionSourceRegistration {
        VoxelConversionSourceRegistration {
            source: source.clone(),
            registered: false,
            material_slots: Vec::new(),
            diagnostics: vec![Self::voxel_conversion_diagnostic(
                VoxelConversionDiagnosticCode::UnsupportedSourceAsset,
                source.asset_id.clone(),
                message,
            )],
            evidence: Vec::new(),
        }
    }

    pub(super) fn static_mesh_source_from_registration(
        request: &VoxelConversionSourceRegistrationRequest,
    ) -> Result<StaticMeshSource, String> {
        if request.source.asset_id.trim().is_empty() {
            return Err("voxel conversion source asset id is required".to_string());
        }
        if request.source.asset_kind != "mesh" {
            return Err(format!(
                "voxel conversion source asset kind must be mesh, got {}",
                request.source.asset_kind
            ));
        }
        if request.source.asset_version == 0 {
            return Err("voxel conversion source asset version must be positive".to_string());
        }
        if !request.source.source_hash.starts_with("sha256:") {
            return Err(
                "voxel conversion source hash must be a sha256: authority hash".to_string(),
            );
        }
        if request.positions.is_empty() {
            return Err(
                "voxel conversion source requires at least one vertex position".to_string(),
            );
        }
        if request.triangles.is_empty() {
            return Err("voxel conversion source requires at least one triangle".to_string());
        }

        for (index, position) in request.positions.iter().enumerate() {
            if !position.iter().all(|component| component.is_finite()) {
                return Err(format!(
                    "voxel conversion source position {index} contains a non-finite component"
                ));
            }
        }

        let mut material_slots = BTreeSet::new();
        for slot in &request.material_slots {
            if !material_slots.insert(slot.source_material_slot) {
                return Err(format!(
                    "voxel conversion source material slot {} is duplicated",
                    slot.source_material_slot
                ));
            }
        }
        if material_slots.is_empty() {
            return Err("voxel conversion source requires material slot bindings".to_string());
        }

        let vertex_count = request.positions.len() as u32;
        let mut triangles = Vec::with_capacity(request.triangles.len());
        for (index, triangle) in request.triangles.iter().enumerate() {
            if triangle
                .indices
                .iter()
                .any(|vertex| *vertex >= vertex_count)
            {
                return Err(format!(
                    "voxel conversion source triangle {index} references a missing vertex"
                ));
            }
            if !material_slots.contains(&triangle.source_material_slot) {
                return Err(format!(
                    "voxel conversion source triangle {index} references unbound material slot {}",
                    triangle.source_material_slot
                ));
            }
            triangles.push(MeshTriangle {
                indices: triangle.indices,
                source_material_slot: triangle.source_material_slot,
            });
        }

        Ok(StaticMeshSource {
            asset_id: request.source.asset_id.clone(),
            asset_kind: request.source.asset_kind.clone(),
            asset_version: request.source.asset_version,
            source_hash: request.source.source_hash.clone(),
            mesh_primitive: request.source.mesh_primitive.clone(),
            positions: request.positions.clone(),
            triangles,
        })
    }

    pub(super) fn empty_unsupported_source(
        reference: &protocol_voxel_conversion::VoxelConversionSourceRef,
    ) -> StaticMeshSource {
        StaticMeshSource {
            asset_id: reference.asset_id.clone(),
            asset_kind: reference.asset_kind.clone(),
            asset_version: reference.asset_version,
            source_hash: reference.source_hash.clone(),
            mesh_primitive: reference.mesh_primitive.clone(),
            positions: Vec::new(),
            triangles: Vec::new(),
        }
    }

    pub(super) fn seeded_voxel_conversion_authority() -> BridgeResult<(
        BTreeMap<String, StaticMeshSource>,
        BTreeMap<String, VoxelConversionSourceMetadataAuthority>,
    )> {
        let mut sources = BTreeMap::new();
        let mut metadata = BTreeMap::new();
        let fixture = Self::fixture_quad_source();
        metadata.insert(
            fixture.asset_id.clone(),
            Self::source_metadata_from_static_source(
                &fixture,
                None,
                vec![
                    VoxelConversionSourceMaterialSlot {
                        source_material_slot: 0,
                        source_material_id: Some("mat/a".to_string()),
                    },
                    VoxelConversionSourceMaterialSlot {
                        source_material_slot: 1,
                        source_material_id: Some("mat/b".to_string()),
                    },
                ],
                Self::material_group_metadata_from_source(&fixture),
            ),
        );
        sources.insert(fixture.asset_id.clone(), fixture);

        let project_asset = Self::project_triangle_static_mesh_asset();
        let project_source = Self::static_mesh_source_from_asset(
            &project_asset,
            1,
            "sha256:import-fixture-a",
            Some("default".to_string()),
        )?;
        metadata.insert(
            project_source.asset_id.clone(),
            Self::source_metadata_from_static_mesh_asset(
                &project_source,
                &project_asset,
                Some("asha://fixture/mesh/import-fixture-a".to_string()),
            ),
        );
        sources.insert(project_source.asset_id.clone(), project_source);
        Ok((sources, metadata))
    }

    pub(super) fn seeded_voxel_conversion_targets(
    ) -> BTreeMap<(u64, Option<String>), VoxelConversionTargetAuthority> {
        let launch = Self::launch_grid();
        let studio_grid =
            VoxelGridSpec::new(GridId::new(2), launch.voxel_size(), launch.chunk_dims())
                .expect("positive Studio target voxel size");
        let authored_grid =
            VoxelGridSpec::new(GridId::new(7), launch.voxel_size(), launch.chunk_dims())
                .expect("positive authored target voxel size");
        [
            VoxelConversionTargetAuthority {
                spec: launch,
                volume_asset_id: Some("voxel/generated".to_string()),
            },
            VoxelConversionTargetAuthority {
                spec: studio_grid,
                volume_asset_id: Some("voxel/generated".to_string()),
            },
            VoxelConversionTargetAuthority {
                spec: authored_grid,
                volume_asset_id: Some("voxel/generated".to_string()),
            },
        ]
        .into_iter()
        .map(|target| {
            (
                (
                    target.spec.id().raw() as u64,
                    target.volume_asset_id.clone(),
                ),
                target,
            )
        })
        .collect()
    }

    pub(super) fn source_for_voxel_conversion(
        &self,
        request: &VoxelConversionPlanRequest,
    ) -> StaticMeshSource {
        self.voxel
            .voxel_conversion_sources
            .get(&request.source.asset_id)
            .cloned()
            .unwrap_or_else(|| Self::empty_unsupported_source(&request.source))
    }

    pub(super) fn source_metadata_from_registration(
        request: &VoxelConversionSourceRegistrationRequest,
    ) -> VoxelConversionSourceMetadataAuthority {
        let source = StaticMeshSource {
            asset_id: request.source.asset_id.clone(),
            asset_kind: request.source.asset_kind.clone(),
            asset_version: request.source.asset_version,
            source_hash: request.source.source_hash.clone(),
            mesh_primitive: request.source.mesh_primitive.clone(),
            positions: request.positions.clone(),
            triangles: request
                .triangles
                .iter()
                .map(|triangle| MeshTriangle {
                    indices: triangle.indices,
                    source_material_slot: triangle.source_material_slot,
                })
                .collect(),
        };
        Self::source_metadata_from_static_source(
            &source,
            None,
            request.material_slots.clone(),
            Self::material_group_metadata_from_source(&source),
        )
    }

    pub(super) fn source_metadata_from_project_mesh_asset(
        request: &VoxelConversionMeshAssetRegistrationRequest,
    ) -> VoxelConversionSourceMetadataAuthority {
        let source = StaticMeshSource {
            asset_id: request.source.asset_id.clone(),
            asset_kind: "mesh".to_string(),
            asset_version: request.source.asset_version,
            source_hash: request.source.source_hash.clone(),
            mesh_primitive: request.source.mesh_primitive.clone(),
            positions: request.mesh_asset.positions.clone(),
            triangles: request
                .mesh_asset
                .groups
                .iter()
                .flat_map(|group| {
                    let start = group.start as usize;
                    let end = start + group.count as usize;
                    request.mesh_asset.indices[start..end]
                        .chunks_exact(3)
                        .map(move |tri| MeshTriangle {
                            indices: [tri[0], tri[1], tri[2]],
                            source_material_slot: group.material_slot,
                        })
                })
                .collect(),
        };
        let groups = Self::group_metadata_from_project_mesh_asset(&request.mesh_asset);
        Self::source_metadata_from_static_source(
            &source,
            request.mesh_asset.source_path.clone(),
            request.mesh_asset.material_slots.clone(),
            groups,
        )
    }

    pub(super) fn source_metadata_from_static_mesh_asset(
        source: &StaticMeshSource,
        asset: &StaticMeshAsset,
        source_path: Option<String>,
    ) -> VoxelConversionSourceMetadataAuthority {
        let material_slots = asset
            .material_slots
            .iter()
            .map(|slot| VoxelConversionSourceMaterialSlot {
                source_material_slot: slot.slot as u32,
                source_material_id: Some(slot.material.clone()),
            })
            .collect();
        let groups = Self::group_metadata_from_static_mesh_asset(source, asset);
        Self::source_metadata_from_static_source(source, source_path, material_slots, groups)
    }

    pub(super) fn source_metadata_from_static_source(
        source: &StaticMeshSource,
        source_path: Option<String>,
        material_slots: Vec<VoxelConversionSourceMaterialSlot>,
        groups: Vec<VoxelConversionSourceGroupMetadata>,
    ) -> VoxelConversionSourceMetadataAuthority {
        let source_ref = protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: source.asset_id.clone(),
            asset_kind: source.asset_kind.clone(),
            asset_version: source.asset_version,
            source_hash: source.source_hash.clone(),
            mesh_primitive: source.mesh_primitive.clone(),
        };
        VoxelConversionSourceMetadataAuthority {
            source: source_ref,
            source_path,
            source_bounds: Self::source_bounds(&source.positions),
            vertex_count: Self::saturating_u32(source.positions.len()),
            triangle_count: Self::saturating_u32(source.triangles.len()),
            groups,
            material_slots,
            evidence: vec![VoxelConversionEvidenceRef {
                kind: protocol_voxel_conversion::VoxelConversionEvidenceKind::SourceSnapshot,
                uri: format!("asha://voxel-conversion/source/{}", source.asset_id),
                content_hash: source.source_hash.clone(),
            }],
        }
    }

    pub(super) fn material_group_metadata_from_source(
        source: &StaticMeshSource,
    ) -> Vec<VoxelConversionSourceGroupMetadata> {
        let mut by_slot: BTreeMap<u32, Vec<MeshTriangle>> = BTreeMap::new();
        for triangle in &source.triangles {
            by_slot
                .entry(triangle.source_material_slot)
                .or_default()
                .push(*triangle);
        }
        let mut start = 0u32;
        by_slot
            .into_iter()
            .map(|(slot, triangles)| {
                let count = Self::saturating_u32(triangles.len().saturating_mul(3));
                let bounds = Self::bounds_for_triangles(&source.positions, &triangles);
                let group = VoxelConversionSourceGroupMetadata {
                    group_id: format!("material-slot:{slot}"),
                    label: Some(format!("Material slot {slot}")),
                    material_slot: slot,
                    start,
                    count,
                    bounds,
                };
                start = start.saturating_add(count);
                group
            })
            .collect()
    }

    pub(super) fn group_metadata_from_project_mesh_asset(
        asset: &protocol_voxel_conversion::VoxelConversionMeshAsset,
    ) -> Vec<VoxelConversionSourceGroupMetadata> {
        asset
            .groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                let start = group.start as usize;
                let end = start + group.count as usize;
                let triangles = asset.indices[start..end]
                    .chunks_exact(3)
                    .map(|tri| MeshTriangle {
                        indices: [tri[0], tri[1], tri[2]],
                        source_material_slot: group.material_slot,
                    })
                    .collect::<Vec<_>>();
                VoxelConversionSourceGroupMetadata {
                    group_id: format!("group:{index}:material-slot:{}", group.material_slot),
                    label: Some(format!(
                        "Group {index} / material slot {}",
                        group.material_slot
                    )),
                    material_slot: group.material_slot,
                    start: group.start,
                    count: group.count,
                    bounds: Self::bounds_for_triangles(&asset.positions, &triangles),
                }
            })
            .collect()
    }

    pub(super) fn group_metadata_from_static_mesh_asset(
        source: &StaticMeshSource,
        asset: &StaticMeshAsset,
    ) -> Vec<VoxelConversionSourceGroupMetadata> {
        asset
            .payload
            .groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                let triangles = source
                    .triangles
                    .iter()
                    .filter(|triangle| triangle.source_material_slot == group.material_slot as u32)
                    .copied()
                    .collect::<Vec<_>>();
                VoxelConversionSourceGroupMetadata {
                    group_id: format!("group:{index}:material-slot:{}", group.material_slot),
                    label: Some(format!(
                        "Group {index} / material slot {}",
                        group.material_slot
                    )),
                    material_slot: group.material_slot as u32,
                    start: group.start,
                    count: group.count,
                    bounds: Self::bounds_for_triangles(&source.positions, &triangles),
                }
            })
            .collect()
    }

    pub(super) fn source_bounds(positions: &[[f32; 3]]) -> Option<VoxelConversionSourceBounds> {
        let mut iter = positions.iter();
        let first = *iter.next()?;
        let mut min = first;
        let mut max = first;
        for position in iter {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        Some(VoxelConversionSourceBounds { min, max })
    }

    pub(super) fn bounds_for_triangles(
        positions: &[[f32; 3]],
        triangles: &[MeshTriangle],
    ) -> Option<VoxelConversionSourceBounds> {
        let mut group_positions = Vec::new();
        for triangle in triangles {
            for index in triangle.indices {
                let position = positions.get(index as usize)?;
                group_positions.push(*position);
            }
        }
        Self::source_bounds(&group_positions)
    }

    pub(super) fn missing_voxel_conversion_source_metadata(
        request: VoxelConversionSourceMetadataRequest,
        message: impl Into<String>,
    ) -> VoxelConversionSourceMetadataReadout {
        VoxelConversionSourceMetadataReadout {
            request,
            registered: false,
            source: None,
            source_path: None,
            source_bounds: None,
            vertex_count: 0,
            triangle_count: 0,
            groups: Vec::new(),
            material_slots: Vec::new(),
            latest_plan_id: None,
            latest_plan_transform: None,
            diagnostics: vec![Self::voxel_conversion_diagnostic(
                VoxelConversionDiagnosticCode::VoxelConversionUnavailable,
                "source",
                message,
            )],
            evidence: Vec::new(),
        }
    }

    pub(super) fn saturating_u32(value: usize) -> u32 {
        u32::try_from(value).unwrap_or(u32::MAX)
    }
}
