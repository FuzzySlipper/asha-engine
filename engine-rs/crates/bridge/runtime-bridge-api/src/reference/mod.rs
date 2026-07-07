use crate::*;

// ── Tiny in-crate implementation for smoke tests ──────────────────────────────
//
// Proves the boundary types round-trip without any transport. The real native
// body lives in `native-bridge`; this is the deterministic reference the mock and
// native paths must match.

/// A minimal deterministic bridge used for boundary smoke tests. Large payloads
/// are owned by the [`RuntimeBufferProvider`]; the seed buffer is allocated as the
/// first handle (`0`) at init so the boundary `get_buffer`/`release_buffer` verbs
/// exercise the real provider rather than a bespoke `Vec`.
#[derive(Debug, Default)]
pub struct ReferenceBridge {
    engine: Option<EngineHandle>,
    buffers: buffer_provider::RuntimeBufferProvider,
    /// The currently-loaded world's scene identity (the staged/live world).
    loaded_world: Option<u64>,
    /// Live voxel authority for the launch/edit loop (launchable-voxel, #2436).
    /// Present once `initialize_engine` has set up the runtime.
    voxel: Option<VoxelWorld>,
    /// The material catalog voxel edits validate against.
    materials: MaterialCatalog,
    /// Bridge-owned runtime view cameras (view/projection evidence, not gameplay authority).
    cameras: BTreeMap<u64, CameraSnapshot>,
    next_camera: u64,
    /// Minimal authority-owned runtime entity state for bridge-level actor
    /// movement verbs. TypeScript may propose targets, but transform mutation is
    /// applied here through `core-entity`.
    entities: EntityStore,
    /// FPS/ECRP RuntimeSession authority state. Stored definitions seed this
    /// through rule-lifecycle; TS callers only receive typed readouts/receipts.
    fps_session: Option<FpsRuntimeSessionState>,
    fps_seed: Option<FpsRuntimeSessionLoadRequest>,
    fps_epoch: u64,
    /// Last planned voxel conversion. This is bridge-owned authority state used
    /// by preview/apply hash guards; callers cannot provide their own output.
    voxel_conversion_sources: BTreeMap<String, StaticMeshSource>,
    voxel_conversion_targets: BTreeMap<(u64, Option<String>), VoxelConversionTargetAuthority>,
    voxel_conversion_plan: Option<PlannedConversion>,
    voxel_conversion_evidence: Vec<VoxelConversionEvidenceRef>,
}

/// The bundle schema / protocol versions this reference bridge understands.
const REFERENCE_SUPPORTED_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
struct VoxelConversionTargetAuthority {
    spec: VoxelGridSpec,
    volume_asset_id: Option<String>,
}

impl ReferenceBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// The default launch grid: id 1, voxel size 1.0, cubic 2×2×2 chunks (matches
    /// the canonical voxel fixture). Chunk dims come from the spec, not a global.
    fn launch_grid() -> VoxelGridSpec {
        VoxelGridSpec::new(
            GridId::new(1),
            1.0,
            ChunkDims::cubic(2).expect("nonzero dims"),
        )
        .expect("positive voxel size")
    }

    fn material_for_chunk(coord: ChunkCoord) -> u16 {
        const MATERIAL_IDS: [u16; 3] = [1, 2, 3];
        let idx = (coord.x * 2 + coord.y).rem_euclid(MATERIAL_IDS.len() as i64) as usize;
        MATERIAL_IDS[idx]
    }

    fn launch_world() -> VoxelWorld {
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

    fn fixture_quad_source() -> StaticMeshSource {
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

    fn project_triangle_static_mesh_asset() -> StaticMeshAsset {
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

    fn static_mesh_source_from_asset(
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

    fn source_registration_diagnostic(
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

    fn static_mesh_source_from_registration(
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

    fn empty_unsupported_source(
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

    fn seeded_voxel_conversion_sources() -> BridgeResult<BTreeMap<String, StaticMeshSource>> {
        let mut sources = BTreeMap::new();
        let fixture = Self::fixture_quad_source();
        sources.insert(fixture.asset_id.clone(), fixture);

        let project_source = Self::static_mesh_source_from_asset(
            &Self::project_triangle_static_mesh_asset(),
            1,
            "sha256:import-fixture-a",
            Some("default".to_string()),
        )?;
        sources.insert(project_source.asset_id.clone(), project_source);
        Ok(sources)
    }

    fn seeded_voxel_conversion_targets(
    ) -> BTreeMap<(u64, Option<String>), VoxelConversionTargetAuthority> {
        let launch = Self::launch_grid();
        let authored_grid =
            VoxelGridSpec::new(GridId::new(7), launch.voxel_size(), launch.chunk_dims())
                .expect("positive authored target voxel size");
        [
            VoxelConversionTargetAuthority {
                spec: launch,
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

    fn source_for_voxel_conversion(
        &self,
        request: &VoxelConversionPlanRequest,
    ) -> StaticMeshSource {
        self.voxel_conversion_sources
            .get(&request.source.asset_id)
            .cloned()
            .unwrap_or_else(|| Self::empty_unsupported_source(&request.source))
    }

    fn target_for_voxel_conversion(
        &self,
        target: &protocol_voxel_conversion::VoxelConversionTargetRef,
    ) -> Option<VoxelConversionTargetAuthority> {
        self.voxel_conversion_targets
            .get(&(target.grid, target.volume_asset_id.clone()))
            .cloned()
    }

    fn voxel_conversion_diagnostic(
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

    fn rejected_voxel_conversion_receipt(
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

    fn conversion_commands(planned: &PlannedConversion) -> BridgeResult<Option<CommandBatch>> {
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

    fn apply_command_batch_to_world(
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

    fn voxel_conversion_target_candidate(
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

    fn remember_voxel_conversion_evidence(
        &mut self,
        refs: impl IntoIterator<Item = VoxelConversionEvidenceRef>,
    ) {
        for evidence in refs {
            if !self.voxel_conversion_evidence.contains(&evidence) {
                self.voxel_conversion_evidence.push(evidence);
            }
        }
    }

    fn world_hash(world: &VoxelWorld) -> String {
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

    fn mesh_payload_hash(mesh: &svc_mesh::MeshPayload) -> String {
        format!("fnv1a64:{}", Self::fnv1a64(&mesh.to_fixture_string()))
    }

    fn mesh_evidence_for(
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

    fn require_initialized(&self, op: &str) -> BridgeResult<()> {
        if self.engine.is_none() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{op} called before initialize_engine"),
            ));
        }
        Ok(())
    }

    fn fps_runtime_error(error: FpsRuntimeError) -> RuntimeBridgeError {
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("FPS RuntimeSession authority rejected request: {error:?}"),
        )
    }

    fn fps_session(&self, op: &str) -> BridgeResult<&FpsRuntimeSessionState> {
        self.fps_session.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{op} called before load_fps_runtime_session"),
            )
        })
    }

    fn fps_session_mut(&mut self, op: &str) -> BridgeResult<&mut FpsRuntimeSessionState> {
        self.fps_session.as_mut().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{op} called before load_fps_runtime_session"),
            )
        })
    }

    fn convert_fps_load_request(
        request: &FpsRuntimeSessionLoadRequest,
    ) -> BridgeResult<FpsProjectBundleLoadInput> {
        let mut definitions = Vec::with_capacity(request.definitions.len());
        for entry in &request.definitions {
            let entity = EntityId::new(entry.entity);
            let mut capabilities = Vec::new();
            if let Some(transform) = &entry.transform {
                capabilities.push(EntityDefinitionCapability::Transform {
                    transform: AuthoringTransform {
                        translation: transform.translation,
                        rotation: transform.rotation,
                        scale: transform.scale,
                    },
                });
            }
            if let Some(bounds) = entry.bounds {
                capabilities.push(EntityDefinitionCapability::Bounds {
                    min: bounds.min,
                    max: bounds.max,
                });
            }
            if let Some(visible) = entry.render_visible {
                capabilities.push(EntityDefinitionCapability::Render { visible });
            }
            if let Some(static_collider) = entry.static_collider {
                capabilities.push(EntityDefinitionCapability::Collision { static_collider });
            }

            definitions.push(FpsStoredEntityDefinition {
                entity,
                definition: EntityDefinition {
                    stable_id: entry.stable_id.clone(),
                    display_name: entry.display_name.clone(),
                    source: EntityDefinitionSourceTrace {
                        project_bundle: request.project_bundle.clone(),
                        relative_path: entry.source_path.clone(),
                    },
                    tags: Vec::new(),
                    metadata: Vec::new(),
                    capabilities,
                },
                role: match entry.role {
                    FpsBridgeRole::Player => FpsRuntimeRole::Player,
                    FpsBridgeRole::Enemy => FpsRuntimeRole::Enemy,
                    FpsBridgeRole::Neutral => FpsRuntimeRole::Neutral,
                },
                health: entry
                    .health
                    .map(|health| HealthState::new(health.current, health.max)),
                weapon: entry.weapon.as_ref().map(|weapon| FpsWeaponMount {
                    weapon_id: weapon.weapon_id.clone(),
                    damage: weapon.damage,
                    range_units: weapon.range_units,
                    ammo: weapon.ammo,
                    cooldown_ticks_after_fire: weapon.cooldown_ticks_after_fire,
                }),
                render_projection: entry
                    .render_visible
                    .map(|visible| FpsRenderProjectionState {
                        projection: match entry.role {
                            FpsBridgeRole::Player => "first_person_camera",
                            FpsBridgeRole::Enemy => "target_actor",
                            FpsBridgeRole::Neutral => "neutral_actor",
                        }
                        .to_string(),
                        visible,
                    }),
                policy_binding: entry
                    .policy_binding
                    .as_ref()
                    .map(|binding| FpsPolicyBinding {
                        binding_id: binding.binding_id.clone(),
                        policy_id: binding.policy_id.clone(),
                        view_kind: binding.view_kind.clone(),
                        view_version: binding.view_version.clone(),
                        allowed_intents: binding.allowed_intents.clone(),
                        runtime_moment: binding.runtime_moment.clone(),
                    }),
            });
        }

        Ok(FpsProjectBundleLoadInput {
            project_bundle: request.project_bundle.clone(),
            definitions,
        })
    }

    fn fps_lifecycle_status(status: FpsLifecycleStatus) -> FpsBridgeLifecycleStatus {
        match status {
            FpsLifecycleStatus::Active => FpsBridgeLifecycleStatus::Active,
            FpsLifecycleStatus::EnemyDefeated { entity, tick } => {
                FpsBridgeLifecycleStatus::EnemyDefeated {
                    entity: entity.raw(),
                    tick,
                }
            }
        }
    }

    fn fps_read_sets() -> Vec<FpsReadSetEvidence> {
        vec![
            FpsReadSetEvidence {
                view_kind: "runtime_session.lifecycle.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec![
                    "EntityStore.lifecycle".to_string(),
                    "FpsRuntimeSessionState.lifecycle_status".to_string(),
                ],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.health.v0".to_string(),
                owner: "svc-combat".to_string(),
                read_set: vec![
                    "CombatState.health".to_string(),
                    "CombatState.health_hash".to_string(),
                ],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.policy_binding.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec!["FpsStoredEntityDefinition.policy_binding".to_string()],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.replay.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec!["FpsRuntimeSessionState.replay_records".to_string()],
            },
        ]
    }

    fn fps_encounter_read_sets() -> Vec<FpsReadSetEvidence> {
        vec![
            FpsReadSetEvidence {
                view_kind: "runtime_session.encounter_director.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec![
                    "FpsRuntimeSessionState.encounter".to_string(),
                    "FpsRuntimeSessionState.lifecycle_status".to_string(),
                ],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.encounter_replay.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec!["FpsRuntimeSessionState.replay_records".to_string()],
            },
        ]
    }

    fn bridge_encounter_lifecycle(
        lifecycle: FpsEncounterLifecycleInput,
    ) -> RuleFpsEncounterLifecycleInput {
        RuleFpsEncounterLifecycleInput {
            outcome_kind: lifecycle.outcome_kind,
            terminal: lifecycle.terminal,
            enemy_dead: lifecycle.enemy_dead,
            player_dead: lifecycle.player_dead,
            lifecycle_hash: lifecycle.lifecycle_hash,
        }
    }

    fn bridge_encounter_state(state: &FpsEncounterState) -> FpsEncounterStateReadout {
        FpsEncounterStateReadout {
            preset_id: state.preset_id.clone(),
            status: Self::encounter_status_label(state.status).to_string(),
            spawned_enemy_ids: state.spawned_enemy_ids.clone(),
            defeated_enemy_ids: state.defeated_enemy_ids.clone(),
            revision: state.revision,
            last_transition: Self::encounter_last_transition_label(state.last_transition)
                .to_string(),
        }
    }

    fn encounter_status_label(status: FpsEncounterStatus) -> &'static str {
        match status {
            FpsEncounterStatus::Pending => "pending",
            FpsEncounterStatus::Active => "active",
            FpsEncounterStatus::Cleared => "cleared",
            FpsEncounterStatus::Failed => "failed",
        }
    }

    fn encounter_last_transition_label(transition: FpsEncounterLastTransition) -> &'static str {
        match transition {
            FpsEncounterLastTransition::Initialized => "initialized",
            FpsEncounterLastTransition::Activated => "activated",
            FpsEncounterLastTransition::Cleared => "cleared",
            FpsEncounterLastTransition::Failed => "failed",
            FpsEncounterLastTransition::Reset => "reset",
        }
    }

    fn encounter_action(action: &str) -> BridgeResult<FpsEncounterTransitionAction> {
        match action {
            "activate" => Ok(FpsEncounterTransitionAction::Activate),
            "sync_lifecycle" => Ok(FpsEncounterTransitionAction::SyncLifecycle),
            "reset" => Ok(FpsEncounterTransitionAction::Reset),
            other => Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("unknown FPS encounter transition action '{other}'"),
            )),
        }
    }

    fn encounter_hash(state: &FpsEncounterState, lifecycle: &FpsEncounterLifecycleInput) -> u64 {
        let key = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            state.preset_id,
            Self::encounter_status_label(state.status),
            state.spawned_enemy_ids.join(","),
            state.defeated_enemy_ids.join(","),
            state.revision,
            Self::encounter_last_transition_label(state.last_transition),
            lifecycle.outcome_kind,
            lifecycle.terminal,
            lifecycle.enemy_dead,
            lifecycle.player_dead,
            lifecycle.lifecycle_hash
        );
        u64::from_str_radix(&Self::fnv1a64(&key), 16).expect("fnv1a64 emits hex")
    }

    fn encounter_snapshot(
        session: &FpsRuntimeSessionState,
        lifecycle: FpsEncounterLifecycleInput,
    ) -> FpsEncounterDirectorSnapshot {
        let latest = session.replay_records.last();
        let encounter_hash = Self::encounter_hash(&session.encounter, &lifecycle);
        FpsEncounterDirectorSnapshot {
            backend: "reference_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.encounter_director.v0".to_string(),
            mutation_owner: "rule-lifecycle".to_string(),
            workspace_trace: vec!["projected encounter state from rule-lifecycle".to_string()],
            state: Self::bridge_encounter_state(&session.encounter),
            lifecycle,
            read_sets: Self::fps_encounter_read_sets(),
            encounter_hash,
            replay_hash: latest
                .map(|record| record.record_hash)
                .unwrap_or(encounter_hash),
        }
    }

    fn encounter_transition_result(
        receipt: FpsEncounterTransitionReceipt,
        lifecycle: FpsEncounterLifecycleInput,
    ) -> FpsEncounterTransitionResult {
        FpsEncounterTransitionResult {
            backend: "reference_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.encounter_transition.v0".to_string(),
            mutation_owner: "rule-lifecycle".to_string(),
            workspace_trace: vec![
                "validated encounter transition against rule-lifecycle".to_string(),
                "serialized accepted encounter transition into replay evidence".to_string(),
            ],
            accepted: receipt.accepted,
            rejection_reason: receipt.rejection_reason.map(str::to_string),
            event_kind: receipt.event_kind.map(str::to_string),
            state: Self::bridge_encounter_state(&receipt.state),
            lifecycle,
            encounter_hash: receipt.encounter_hash,
            replay_hash: receipt.replay_hash,
        }
    }

    fn fps_snapshot(
        session: &FpsRuntimeSessionState,
        epoch: u64,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        let player = session
            .role_entity(FpsRuntimeRole::Player)
            .map_err(Self::fps_runtime_error)?;
        let enemy = session
            .role_entity(FpsRuntimeRole::Enemy)
            .map_err(Self::fps_runtime_error)?;
        let mut health = Vec::new();
        let mut policy_bindings = Vec::new();
        for (entity, definition) in &session.definitions {
            if let Some(state) = session.health(*entity) {
                health.push(FpsEntityHealthReadout {
                    entity: entity.raw(),
                    current: state.current,
                    max: state.max,
                });
            }
            if let Some(binding) = &definition.policy_binding {
                policy_bindings.push(FpsPolicyBindingReadout {
                    entity: entity.raw(),
                    binding_id: binding.binding_id.clone(),
                    policy_id: binding.policy_id.clone(),
                    view_kind: binding.view_kind.clone(),
                    view_version: binding.view_version.clone(),
                    allowed_intents: binding.allowed_intents.clone(),
                    runtime_moment: binding.runtime_moment.clone(),
                });
            }
        }
        let replay_records = session
            .replay_records
            .iter()
            .map(|record| FpsReplayEvidence {
                replay_unit: record.kind.to_string(),
                entity_hash: record.entity_hash,
                health_hash: record.health_hash,
                record_hash: record.record_hash,
            })
            .collect::<Vec<_>>();
        let latest = session.replay_records.last();
        Ok(FpsRuntimeSessionSnapshot {
            backend: "reference_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.authority.v0".to_string(),
            project_bundle: session.project_bundle.clone(),
            session_epoch: epoch,
            lifecycle_status: Self::fps_lifecycle_status(session.lifecycle_status),
            player_entity: player.raw(),
            enemy_entity: enemy.raw(),
            health,
            policy_bindings,
            replay_records,
            read_sets: Self::fps_read_sets(),
            entity_hash: session.entities.hash().0,
            health_hash: session.combat.health_hash(),
            replay_hash: latest.map(|record| record.record_hash).unwrap_or(0),
        })
    }

    fn bridge_health(state: HealthState) -> FpsBridgeHealth {
        FpsBridgeHealth {
            current: state.current,
            max: state.max,
        }
    }

    fn primary_fire_result(receipt: FpsPrimaryFireReceipt) -> FpsPrimaryFireResult {
        FpsPrimaryFireResult {
            backend: "reference_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.primary_fire.v0".to_string(),
            mutation_owner: "rule-lifecycle + svc-combat".to_string(),
            workspace_trace: vec![
                "validated FireIntentCommand against svc-combat".to_string(),
                "serialized accepted combat/lifecycle outcome into replay evidence".to_string(),
            ],
            shooter: receipt.shooter.raw(),
            target: receipt.target.map(EntityId::raw),
            target_health_before: receipt.target_health_before.map(Self::bridge_health),
            target_health_after: receipt.target_health_after.map(Self::bridge_health),
            lifecycle_status: Self::fps_lifecycle_status(receipt.lifecycle_status),
            target_render_visible: receipt.target_render_visible,
            entity_hash: receipt.entity_hash,
            health_hash: receipt.health_hash,
            replay_hash: receipt.replay_hash,
        }
    }

    fn ray_from_primary_fire(request: FpsPrimaryFireRequest) -> BridgeResult<Ray> {
        if !request.origin.iter().all(|value| value.is_finite())
            || !request.direction.iter().all(|value| value.is_finite())
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "primary fire origin/direction must be finite",
            ));
        }
        Ok(Ray::new(
            WorldPos::new(request.origin[0], request.origin[1], request.origin[2]),
            WorldVec::new(
                request.direction[0],
                request.direction[1],
                request.direction[2],
            ),
        ))
    }

    fn enemy_entity_id(raw: u64) -> BridgeResult<EntityId> {
        if raw == 0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                EnemyDirectNavMovementError::InvalidEntity.label(),
            ));
        }
        Ok(EntityId::new(raw))
    }

    fn seed_or_read_enemy_transform(
        entities: &mut EntityStore,
        entity: EntityId,
        seed_position: Vec3,
    ) -> BridgeResult<(EnemyDirectNavAuthoritySource, EntityTransform)> {
        if let Some(transform) = entities.transform(entity) {
            return Ok((
                EnemyDirectNavAuthoritySource::RustEntityStore,
                transform.transform,
            ));
        }
        entities
            .apply(EntityLifecycleCommand::Create {
                id: entity,
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .map_err(|err| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!("enemy direct-nav entity seed rejected: {err}"),
                )
            })?;
        let transform = EntityTransform::at(seed_position);
        let attached = entities.attach_transform(entity, transform);
        debug_assert!(attached);
        Ok((EnemyDirectNavAuthoritySource::SeededFromRequest, transform))
    }

    fn transform_hash(entity: EntityId, transform: EntityTransform) -> u64 {
        let key = format!(
            "{}|{:.3},{:.3},{:.3}|{:.3},{:.3},{:.3},{:.3}|{:.3},{:.3},{:.3}",
            entity.raw(),
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
            transform.rotation.w,
            transform.scale.x,
            transform.scale.y,
            transform.scale.z
        );
        u64::from_str_radix(&Self::fnv1a64(&key), 16).expect("fnv1a64 emits hex")
    }

    fn basis_from_pose(pose: protocol_view::CameraPose) -> protocol_view::CameraBasis {
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

    fn validate_viewport(viewport: protocol_view::ViewportSize) -> BridgeResult<()> {
        if viewport.width == 0 || viewport.height == 0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "viewport dimensions must be positive",
            ));
        }
        Ok(())
    }

    fn validate_create_request(request: &CameraCreateRequest) -> BridgeResult<()> {
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

    fn matrix_key(values: &[f32]) -> String {
        values
            .iter()
            .map(|v| format!("{v:.3}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn fnv1a64(text: &str) -> String {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in text.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }

    fn multiply_matrix4(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
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

    fn projection_snapshot(
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

    fn validate_camera_input(input: FirstPersonCameraInput) -> BridgeResult<()> {
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

    fn integrate_camera_snapshot(
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

    fn aabb_for_pose(pose: CameraPose, shape: CameraCollisionShape) -> (WorldPos, WorldPos) {
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

    fn validate_collision_shape(shape: CameraCollisionShape) -> BridgeResult<()> {
        if !shape.half_extents.iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "collision shape half_extents must be finite positive values",
            ));
        }
        Ok(())
    }

    fn collision_projection_hash(world: &VoxelWorld, projection: &CollisionProjection) -> String {
        let chunks = projection
            .collider_chunks()
            .map(|coord| format!("{},{},{}", coord.x, coord.y, coord.z))
            .collect::<Vec<_>>()
            .join(";");
        let key = format!(
            "{}|v{}|n{}|{}",
            Self::world_hash(world),
            projection.version(),
            projection.collider_count(),
            chunks
        );
        format!("fnv1a64:{}", Self::fnv1a64(&key))
    }

    fn screen_point_to_normalized(
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

    fn pick_ray_snapshot(
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

mod runtime_bridge_impl;

#[cfg(test)]
mod tests;
