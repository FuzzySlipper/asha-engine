use super::*;
use protocol_project_bundle::WorkspaceAuthoringOpenRequest;

impl EngineBridge {
    /// Private transport adapter used by canonical project-source authoring.
    ///
    /// This is deliberately absent from [`RuntimeBridge`], the bridge manifest,
    /// and generated consumer contracts. Native transport calls it only through
    /// the package-private adapter owned by `WorkspaceAuthoring.openProject`.
    #[doc(hidden)]
    pub fn open_workspace_authoring_adapter(
        &mut self,
        request: WorkspaceAuthoringOpenRequest,
    ) -> BridgeResult<WorkspaceAuthoringStateSummary> {
        self.open_workspace_authoring_authority(request)
    }

    pub(super) fn open_workspace_authoring_authority(
        &mut self,
        request: WorkspaceAuthoringOpenRequest,
    ) -> BridgeResult<WorkspaceAuthoringStateSummary> {
        if self.runtime_project.engine.is_some() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "workspace authoring cannot open inside a gameplay runtime authority cell",
            ));
        }
        if self
            .workspace_authoring
            .as_ref()
            .is_some_and(|authority| authority.open)
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "workspace authoring is already open",
            ));
        }
        Self::validate_workspace_authoring_open(&request)?;

        self.workspace_authoring_epoch =
            self.workspace_authoring_epoch
                .checked_add(1)
                .ok_or_else(|| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::Internal,
                        "workspace authoring generation overflowed",
                    )
                })?;
        let generation = self.workspace_authoring_epoch;
        let identity = WorkspaceAuthoringIdentity {
            kind: "workspace_authoring.identity.v0".to_owned(),
            authoring_id: request.authoring_id,
            mode: "rust".to_owned(),
            generation,
            seed: request.seed,
            project: request.project,
            project_bundle: request.project_bundle.clone(),
            non_claims: vec![
                "not_gameplay_runtime_session".to_owned(),
                "not_simulation_loop".to_owned(),
                "not_stored_truth".to_owned(),
                "not_renderer_authority".to_owned(),
            ],
        };

        // Construct only the resources required by authoring. In particular,
        // do not call `initialize`: that path owns launch-world, simulation,
        // camera, gameplay, and runtime ProjectBundle lifecycle state.
        self.reset_developer_console();
        self.voxel.buffers.reset();
        let seed_handle = self.voxel.buffers.create(
            buffer_provider::BufferKind::Opaque,
            buffer_provider::BufferLifetime::Manual,
            None,
            request.seed.to_le_bytes().to_vec(),
        );
        debug_assert_eq!(seed_handle.raw(), 0);
        self.voxel.voxel = None;
        self.voxel.voxel_edit_history = None;
        self.voxel.materials =
            MaterialCatalog::new([1, 2, 3].into_iter().map(VoxelMaterialId::new));
        let (sources, source_metadata) = Self::seeded_voxel_conversion_authority()?;
        self.voxel.voxel_conversion_sources = sources;
        self.voxel.voxel_conversion_source_metadata = source_metadata;
        self.voxel.voxel_conversion_targets = Self::seeded_voxel_conversion_targets();
        self.voxel.voxel_conversion_plan = None;
        self.voxel.voxel_model_infos.clear();
        self.voxel.active_voxel_model = None;
        self.voxel.voxel_annotation_layers.clear();
        self.evidence.voxel_conversion_evidence.clear();
        self.projection.voxel_projector = VoxelChunkProjector::default();
        self.projection.pending_voxel_frame = RenderFrameDiff::default();
        self.projection.voxel_instance_binding = None;

        let project_content_admission = self
            .gameplay
            .static_project_content_admission
            .as_ref()
            .cloned()
            .unwrap_or_default();

        self.workspace_authoring = Some(WorkspaceAuthoringAuthority {
            identity,
            composition: WorkspaceAuthoringCompositionStatus {
                loaded_project_bundle: Some(request.project_bundle.scene_id),
                fatal_count: 0,
                total_count: 0,
                blocks_load: false,
            },
            open: true,
            working_revision: 0,
            stored_revision: 0,
            last_stored_canonical_json_hash: None,
            pending_save_candidate: None,
            pending_project_write: None,
            pending_procedural_environment: None,
            next_projection_cursor: 0,
            projection_initialized: false,
            last_projection_receipt: None,
            loaded_voxel_assets: BTreeMap::new(),
            project_content_scenes: BTreeMap::new(),
            project_content_reference_revision: 0,
            project_content_current: None,
            project_content_admission,
        });
        self.read_workspace_authoring_state_authority()
    }

    pub(super) fn read_workspace_authoring_state_authority(
        &self,
    ) -> BridgeResult<WorkspaceAuthoringStateSummary> {
        let authority = self.workspace_authoring.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "workspace authoring state requested before open",
            )
        })?;
        Ok(Self::workspace_authoring_state_summary(authority))
    }

    pub(super) fn confirm_workspace_authoring_stored_authority(
        &mut self,
        request: WorkspaceAuthoringStoredConfirmationRequest,
    ) -> BridgeResult<WorkspaceAuthoringStoredConfirmationReceipt> {
        Self::validate_nonempty(&request.host_path, "hostPath")?;
        Self::validate_nonempty(&request.canonical_json_hash, "canonicalJsonHash")?;
        let authority = self.require_bound_workspace_authoring_mut(
            "confirm_workspace_authoring_stored",
            &request.expected_workspace_id,
            request.expected_generation,
        )?;
        let candidate = authority.pending_save_candidate.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "storage confirmation requires a current Rust save candidate",
            )
        })?;
        if candidate.working_revision != authority.working_revision
            || candidate.canonical_json_hash != request.canonical_json_hash
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::StaleAuthoritySnapshot,
                "storage confirmation does not match the current Rust save candidate",
            ));
        }
        let stored_revision = candidate.working_revision;
        authority.stored_revision = stored_revision;
        authority.last_stored_canonical_json_hash = Some(candidate.canonical_json_hash.clone());
        authority.pending_save_candidate = None;
        authority.pending_project_write = None;
        authority.pending_procedural_environment = None;
        let lifecycle_hash = Self::workspace_authoring_lifecycle_hash(authority);
        Ok(WorkspaceAuthoringStoredConfirmationReceipt {
            kind: "workspace_authoring.stored_confirmation.v0".to_owned(),
            accepted: true,
            workspace_id: authority.identity.project.workspace_id.clone(),
            generation: authority.identity.generation,
            host_path: request.host_path,
            canonical_json_hash: request.canonical_json_hash,
            stored_revision,
            lifecycle_hash,
        })
    }

    pub(super) fn close_workspace_authoring_authority(
        &mut self,
        request: WorkspaceAuthoringCloseRequest,
    ) -> BridgeResult<WorkspaceAuthoringCloseReceipt> {
        let authority = self.require_bound_workspace_authoring_mut(
            "close_workspace_authoring",
            &request.expected_workspace_id,
            request.expected_generation,
        )?;
        let dirty = authority.working_revision != authority.stored_revision;
        if dirty && !request.discard_unsaved_working_state {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "workspace authoring has unsaved working state",
            ));
        }
        authority.open = false;
        authority.pending_save_candidate = None;
        authority.pending_project_write = None;
        authority.pending_procedural_environment = None;
        authority.last_projection_receipt = None;
        let workspace_id = authority.identity.project.workspace_id.clone();
        let generation = authority.identity.generation;
        let lifecycle_hash = Self::workspace_authoring_lifecycle_hash(authority);

        self.voxel = BridgeVoxelAssetBufferState::default();
        self.projection.voxel_projector = VoxelChunkProjector::default();
        self.projection.pending_voxel_frame = RenderFrameDiff::default();
        self.projection.voxel_instance_binding = None;
        Ok(WorkspaceAuthoringCloseReceipt {
            kind: "workspace_authoring.close_receipt.v0".to_owned(),
            closed: true,
            workspace_id,
            generation,
            discarded_unsaved_working_state: dirty,
            lifecycle_hash,
        })
    }

    pub(super) fn read_workspace_authoring_projection_authority(
        &mut self,
        request: WorkspaceAuthoringProjectionRequest,
    ) -> BridgeResult<WorkspaceAuthoringProjectionReceipt> {
        {
            let authority = self.require_bound_workspace_authoring(
                "read_workspace_authoring_projection",
                &request.expected_workspace_id,
                request.expected_generation,
            )?;
            if let Some(receipt) = &authority.last_projection_receipt {
                if receipt.cursor == request.cursor
                    && receipt.working_revision == request.expected_working_revision
                {
                    return Ok(receipt.clone());
                }
            }
            if request.expected_working_revision != authority.working_revision {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::StaleAuthoritySnapshot,
                    "projection request targeted a stale working revision",
                ));
            }
            if request.cursor != authority.next_projection_cursor {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::StaleAuthoritySnapshot,
                    "projection request cursor is stale or from the future",
                ));
            }
        }

        let frame = self.drain_voxel_projection_frame(request.cursor)?;
        let authority = self.require_bound_workspace_authoring_mut(
            "read_workspace_authoring_projection",
            &request.expected_workspace_id,
            request.expected_generation,
        )?;
        let delivery = if authority.projection_initialized {
            "apply"
        } else {
            "replace"
        };
        let next_cursor = request.cursor.checked_add(1).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "workspace authoring projection cursor overflowed",
            )
        })?;
        let frame_json = render_bridge::json::encode_frame(&frame);
        let projection_key = serde_json::to_string(&(
            &authority.identity.project.workspace_id,
            authority.identity.generation,
            authority.working_revision,
            request.cursor,
            next_cursor,
            delivery,
            &frame_json,
        ))
        .map_err(|error| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("failed to hash workspace authoring projection: {error}"),
            )
        })?;
        let receipt = WorkspaceAuthoringProjectionReceipt {
            kind: "workspace_authoring.projection.v0".to_owned(),
            workspace_id: authority.identity.project.workspace_id.clone(),
            generation: authority.identity.generation,
            working_revision: authority.working_revision,
            cursor: request.cursor,
            next_cursor,
            delivery: delivery.to_owned(),
            render_diff_count: frame.ops.len() as u64,
            frame_json,
            projection_hash: format!("fnv1a64:{}", Self::fnv1a64(&projection_key)),
        };
        authority.next_projection_cursor = next_cursor;
        authority.projection_initialized = true;
        authority.last_projection_receipt = Some(receipt.clone());
        Ok(receipt)
    }

    pub(super) fn require_runtime_or_workspace_authoring(
        &self,
        operation: &str,
    ) -> BridgeResult<()> {
        if self.runtime_project.engine.is_some()
            || self
                .workspace_authoring
                .as_ref()
                .is_some_and(|authority| authority.open)
        {
            Ok(())
        } else {
            Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{operation} requires runtime or workspace authoring authority"),
            ))
        }
    }

    pub(super) fn record_workspace_authoring_mutation(&mut self) {
        if let Some(authority) = self.workspace_authoring.as_mut().filter(|value| value.open) {
            authority.working_revision = authority
                .working_revision
                .checked_add(1)
                .expect("workspace authoring revision overflow is unreachable in one process");
            authority.pending_save_candidate = None;
            authority.pending_project_write = None;
            authority.pending_procedural_environment = None;
            self.projection.voxel_instance_binding = None;
        }
    }

    pub(super) fn clear_workspace_authoring_loaded_assets(&mut self) {
        if let Some(authority) = self.workspace_authoring.as_mut().filter(|value| value.open) {
            authority.loaded_voxel_assets.clear();
        }
    }

    pub(super) fn record_workspace_authoring_loaded_asset(&mut self, asset: VoxelVolumeAsset) {
        let canonical_json_hash = asset.content_hashes.canonical_json.clone();
        self.record_workspace_authoring_mutation();
        if let Some(authority) = self.workspace_authoring.as_mut().filter(|value| value.open) {
            authority.stored_revision = authority.working_revision;
            authority.last_stored_canonical_json_hash = Some(canonical_json_hash);
            authority.pending_save_candidate = None;
            authority.pending_project_write = None;
            authority
                .loaded_voxel_assets
                .insert(asset.asset_id.clone(), asset);
        }
    }

    pub(super) fn remember_workspace_authoring_save_candidate(
        &mut self,
        canonical_json_hash: String,
    ) {
        if let Some(authority) = self.workspace_authoring.as_mut().filter(|value| value.open) {
            authority.pending_save_candidate = Some(WorkspaceAuthoringSaveCandidate {
                canonical_json_hash,
                working_revision: authority.working_revision,
            });
        }
    }

    pub(super) fn require_workspace_authoring_revision(
        &self,
        operation: &str,
        expected_workspace_id: &str,
        expected_generation: u64,
        expected_working_revision: u64,
    ) -> BridgeResult<()> {
        let authority = self.require_bound_workspace_authoring(
            operation,
            expected_workspace_id,
            expected_generation,
        )?;
        if authority.working_revision != expected_working_revision {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::StaleAuthoritySnapshot,
                format!("{operation} targeted a stale working revision"),
            ));
        }
        Ok(())
    }

    pub(super) fn require_open_workspace_authoring_mut(
        &mut self,
        operation: &str,
    ) -> BridgeResult<&mut WorkspaceAuthoringAuthority> {
        self.workspace_authoring
            .as_mut()
            .filter(|authority| authority.open)
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    format!("{operation} called before workspace authoring open"),
                )
            })
    }

    fn require_bound_workspace_authoring(
        &self,
        operation: &str,
        expected_workspace_id: &str,
        expected_generation: u64,
    ) -> BridgeResult<&WorkspaceAuthoringAuthority> {
        let authority = self.workspace_authoring.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{operation} called before workspace authoring open"),
            )
        })?;
        Self::validate_workspace_authoring_binding(
            authority,
            operation,
            expected_workspace_id,
            expected_generation,
        )?;
        Ok(authority)
    }

    fn require_bound_workspace_authoring_mut(
        &mut self,
        operation: &str,
        expected_workspace_id: &str,
        expected_generation: u64,
    ) -> BridgeResult<&mut WorkspaceAuthoringAuthority> {
        let authority = self.workspace_authoring.as_mut().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{operation} called before workspace authoring open"),
            )
        })?;
        Self::validate_workspace_authoring_binding(
            authority,
            operation,
            expected_workspace_id,
            expected_generation,
        )?;
        Ok(authority)
    }

    fn validate_workspace_authoring_binding(
        authority: &WorkspaceAuthoringAuthority,
        operation: &str,
        expected_workspace_id: &str,
        expected_generation: u64,
    ) -> BridgeResult<()> {
        if !authority.open {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{operation} requires an open workspace authoring authority"),
            ));
        }
        if authority.identity.project.workspace_id != expected_workspace_id
            || authority.identity.generation != expected_generation
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::StaleAuthoritySnapshot,
                format!("{operation} targeted a foreign workspace or stale generation"),
            ));
        }
        Ok(())
    }

    fn workspace_authoring_state_summary(
        authority: &WorkspaceAuthoringAuthority,
    ) -> WorkspaceAuthoringStateSummary {
        WorkspaceAuthoringStateSummary {
            kind: "workspace_authoring.state.v0".to_owned(),
            status: if authority.open { "open" } else { "closed" }.to_owned(),
            identity: authority.identity.clone(),
            composition: authority.composition,
            working_revision: authority.working_revision,
            stored_revision: authority.stored_revision,
            dirty: authority.working_revision != authority.stored_revision,
            last_stored_canonical_json_hash: authority.last_stored_canonical_json_hash.clone(),
            authority_snapshot_hash: Self::workspace_authoring_authority_hash(authority),
            lifecycle_hash: Self::workspace_authoring_lifecycle_hash(authority),
        }
    }

    fn workspace_authoring_authority_hash(authority: &WorkspaceAuthoringAuthority) -> String {
        let key = format!(
            "workspace-authoring-authority|{}|{}|{}|{}|{}|{}",
            authority.identity.authoring_id,
            authority.identity.project.workspace_id,
            authority.identity.generation,
            authority.working_revision,
            authority.stored_revision,
            authority.open
        );
        format!("fnv1a64:{}", Self::fnv1a64(&key))
    }

    fn workspace_authoring_lifecycle_hash(authority: &WorkspaceAuthoringAuthority) -> String {
        let key = format!(
            "workspace-authoring-lifecycle|{}|{}|{}|{}|{}|{}|{}",
            authority.identity.authoring_id,
            authority.identity.project.workspace_id,
            authority.identity.generation,
            authority.working_revision,
            authority.stored_revision,
            authority.open,
            authority
                .last_stored_canonical_json_hash
                .as_deref()
                .unwrap_or("none")
        );
        format!("fnv1a64:{}", Self::fnv1a64(&key))
    }

    fn validate_workspace_authoring_open(
        request: &WorkspaceAuthoringOpenRequest,
    ) -> BridgeResult<()> {
        Self::validate_nonempty(&request.authoring_id, "authoringId")?;
        Self::validate_nonempty(&request.project.game_id, "project.gameId")?;
        Self::validate_nonempty(&request.project.workspace_id, "project.workspaceId")?;
        if request.project_bundle.bundle_schema_version > ENGINE_SUPPORTED_BUNDLE_VERSION
            || request.project_bundle.protocol_version > ENGINE_SUPPORTED_PROTOCOL_VERSION
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "workspace authoring ProjectBundle version is unsupported",
            ));
        }
        Ok(())
    }

    fn validate_nonempty(value: &str, field: &str) -> BridgeResult<()> {
        if value.trim().is_empty() {
            Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("{field} must be non-empty"),
            ))
        } else {
            Ok(())
        }
    }
}
