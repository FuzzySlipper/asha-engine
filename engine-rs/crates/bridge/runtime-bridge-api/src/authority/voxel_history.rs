use super::*;

use protocol_voxel_edit_history as protocol;
use rule_voxel_edit::history::{
    VoxelEditHistory as AuthorityVoxelEditHistory,
    VoxelEditHistoryCursor as AuthorityVoxelEditHistoryCursor,
    VoxelEditHistoryDiffDiagnostic as AuthorityVoxelEditHistoryDiffDiagnostic,
    VoxelEditHistoryDiffOptions as AuthorityVoxelEditHistoryDiffOptions,
    VoxelEditHistoryDiffSummary as AuthorityVoxelEditHistoryDiffSummary,
    VoxelEditHistoryEntry as AuthorityVoxelEditHistoryEntry,
    VoxelEditHistoryRejection as AuthorityVoxelEditHistoryRejection,
    VoxelEditHistoryRevertReceipt as AuthorityVoxelEditHistoryRevertReceipt,
};
use rule_voxel_edit::{
    execute_transaction, persist::encode_edit_log, preflight_transaction, VoxelEditTransaction,
    VoxelEditTransactionLimits, VoxelEditTransactionRejection,
};

const DEFAULT_VOXEL_EDIT_HISTORY_ID: &str = "history/default";
const MAX_HISTORY_READ_ENTRIES: u64 = 1_000;

impl EngineBridge {
    pub(super) fn reset_voxel_edit_history(&mut self, world: VoxelWorld) {
        self.reset_voxel_edit_history_with_collision_offset(world, [0.0; 3]);
    }

    pub(super) fn reset_voxel_edit_history_with_collision_offset(
        &mut self,
        world: VoxelWorld,
        collision_world_offset: [f64; 3],
    ) {
        self.voxel_edit_history = Some(AuthorityVoxelEditHistory::new(world.clone()));
        self.voxel = Some(world);
        self.collision_world_offset = collision_world_offset;
    }

    pub(super) fn submit_commands_with_voxel_history(
        &mut self,
        batch: CommandBatch,
    ) -> BridgeResult<CommandResult> {
        self.require_initialized("submit_commands")?;
        let current_grid = self.voxel.as_ref().map(VoxelWorld::grid).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "submit_commands called before voxel authority was initialized",
            )
        })?;
        preflight_transaction(
            &batch.commands,
            current_grid,
            VoxelEditTransactionLimits::default(),
        )
        .map_err(Self::voxel_command_batch_preflight_error)?;

        let current_world = self
            .voxel
            .as_ref()
            .expect("voxel authority checked before resource preflight")
            .clone();
        let current_history = self.voxel_edit_history()?.clone();

        let mut validation_world = current_world.clone();
        let mut accepted_commands = Vec::new();
        let mut rejections = Vec::new();
        for command in &batch.commands {
            match rule_voxel_edit::validate(command, &validation_world, &self.materials) {
                Ok(events) => {
                    for event in &events {
                        rule_voxel_edit::apply(&mut validation_world, event).map_err(|rejection| {
                            RuntimeBridgeError::new(
                                RuntimeBridgeErrorKind::Internal,
                                format!(
                                    "validated voxel command failed to apply during history staging: {rejection}"
                                ),
                            )
                        })?;
                    }
                    accepted_commands.push(*command);
                }
                Err(rejection) => rejections.push(rejection),
            }
        }

        let result = CommandResult {
            accepted: accepted_commands.len().min(u32::MAX as usize) as u32,
            rejected: rejections.len().min(u32::MAX as usize) as u32,
            rejections,
        };
        if accepted_commands.is_empty() {
            return Ok(result);
        }

        let mut next_world = current_world;
        let transaction = VoxelEditTransaction::apply(&accepted_commands);
        let receipt = execute_transaction(&mut next_world, &self.materials, &transaction);
        if !receipt.applied
            || receipt.rejected != 0
            || receipt.accepted != result.accepted
            || receipt.after_hash != rule_voxel_edit::voxel_world_hash(&validation_world)
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "accepted voxel command subset did not reproduce during history commit",
            ));
        }

        let mut next_history = current_history;
        next_history
            .append_accepted(receipt)
            .map_err(Self::voxel_edit_history_error)?;
        if next_history.current_world_hash() != rule_voxel_edit::voxel_world_hash(&next_world) {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "voxel edit history and live voxel authority diverged during commit",
            ));
        }

        self.voxel = Some(next_world);
        self.voxel_edit_history = Some(next_history);
        Ok(result)
    }

    fn voxel_command_batch_preflight_error(
        rejection: VoxelEditTransactionRejection,
    ) -> RuntimeBridgeError {
        let detail = match rejection {
            VoxelEditTransactionRejection::CommandQuotaExceeded { limit, actual } => {
                format!("command limit {limit} (actual {actual})")
            }
            VoxelEditTransactionRejection::EventQuotaExceeded { limit, actual } => {
                format!("event limit {limit} (upper bound {actual})")
            }
            VoxelEditTransactionRejection::TouchedVoxelQuotaExceeded { limit, actual } => {
                format!("expanded touched-voxel limit {limit} (actual {actual})")
            }
            other => format!("unexpected preflight rejection {other:?}"),
        };
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("voxel command batch resource preflight rejected: {detail}"),
        )
    }

    pub(super) fn read_voxel_edit_history_authority(
        &self,
        request: VoxelEditHistoryReadRequest,
    ) -> BridgeResult<VoxelEditHistorySummary> {
        self.require_initialized("read_voxel_edit_history")?;
        Self::validate_history_id(&request.history_id)?;
        if request.max_entries == 0 || request.max_entries > MAX_HISTORY_READ_ENTRIES {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("voxel edit history max_entries must be in 1..={MAX_HISTORY_READ_ENTRIES}"),
            ));
        }

        let history = self.voxel_edit_history()?;
        Self::validate_expected_history_hash(request.expected_history_hash.as_deref(), history)?;
        let cursor = match request.cursor_id.as_deref() {
            Some(cursor_id) => Self::find_history_cursor(history, cursor_id)?,
            None => history.cursor(),
        };
        let material_catalog_hash = self.voxel_material_catalog_hash();
        let entry_limit = usize::try_from(request.max_entries).unwrap_or(usize::MAX);
        let entry_end = if request.include_redo_tail {
            history.entries().len()
        } else {
            cursor.index
        };
        let entry_start = entry_end.saturating_sub(entry_limit);
        let entries = history.entries()[entry_start..entry_end]
            .iter()
            .map(|entry| self.protocol_voxel_edit_history_entry(entry, &material_catalog_hash))
            .collect();
        let retained_redo_transaction_ids = if request.include_redo_tail {
            history.entries()[cursor.index..]
                .iter()
                .map(|entry| Self::protocol_transaction_id(entry.transaction_id))
                .collect()
        } else {
            Vec::new()
        };
        let cursor =
            self.protocol_voxel_edit_history_cursor(history, cursor, &material_catalog_hash)?;

        Ok(VoxelEditHistorySummary {
            history_id: DEFAULT_VOXEL_EDIT_HISTORY_ID.to_string(),
            schema_version: protocol::VOXEL_EDIT_HISTORY_SCHEMA_VERSION,
            media_type: protocol::VOXEL_EDIT_HISTORY_MEDIA_TYPE.to_string(),
            target_grid: history.current_world().grid().id().raw() as u64,
            target_voxel_volume_asset_id: None,
            base_voxel_hash: Self::protocol_authority_hash(history.base_world_hash()),
            material_catalog_hash,
            history_hash: cursor.history_hash.clone(),
            cursor,
            entries,
            retained_redo_transaction_ids,
            diagnostics: Vec::new(),
        })
    }

    pub(super) fn preview_voxel_edit_revert_authority(
        &self,
        request: VoxelEditHistoryRevertRequest,
    ) -> BridgeResult<VoxelEditHistoryRevertReceipt> {
        self.require_initialized("preview_voxel_edit_revert")?;
        if request.mode != protocol::VoxelEditHistoryRevertMode::PreviewRevert {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "preview_voxel_edit_revert requires mode preview_revert",
            ));
        }
        let history = self.voxel_edit_history()?;
        self.validate_revert_request(&request, history)?;
        let target_cursor = Self::resolve_revert_target(history, &request.target)?;
        Self::validate_replay_budget(target_cursor, request.max_replay_steps)?;
        let options =
            Self::history_diff_options(request.max_diff_voxels, request.include_sample_window)?;
        let receipt = history
            .preview_revert_to_cursor_with_options(target_cursor, options)
            .map_err(Self::voxel_edit_history_error)?;
        self.protocol_voxel_edit_history_revert_receipt(request, receipt, history)
    }

    pub(super) fn apply_voxel_edit_revert_authority(
        &mut self,
        request: VoxelEditHistoryRevertRequest,
    ) -> BridgeResult<VoxelEditHistoryRevertReceipt> {
        self.require_initialized("apply_voxel_edit_revert")?;
        if request.mode != protocol::VoxelEditHistoryRevertMode::ApplyRevert {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "apply_voxel_edit_revert requires mode apply_revert",
            ));
        }
        let mut history = self.voxel_edit_history()?.clone();
        self.validate_revert_request(&request, &history)?;
        let target_cursor = Self::resolve_revert_target(&history, &request.target)?;
        Self::validate_replay_budget(target_cursor, request.max_replay_steps)?;
        let options =
            Self::history_diff_options(request.max_diff_voxels, request.include_sample_window)?;
        let receipt = history
            .apply_revert_to_cursor_with_options(target_cursor, options)
            .map_err(Self::voxel_edit_history_error)?;
        let protocol_receipt =
            self.protocol_voxel_edit_history_revert_receipt(request, receipt, &history)?;
        self.commit_voxel_edit_history(history);
        Ok(protocol_receipt)
    }

    pub(super) fn undo_voxel_edit_authority(
        &mut self,
        request: VoxelEditHistoryUndoRequest,
    ) -> BridgeResult<VoxelEditHistoryUndoReceipt> {
        self.require_initialized("undo_voxel_edit")?;
        let mut history = self.voxel_edit_history()?.clone();
        Self::validate_history_id(&request.history_id)?;
        Self::validate_current_history_hashes(
            &request.expected_history_hash,
            &request.expected_cursor_hash,
            &history,
        )?;
        let target_cursor = history.cursor().index.checked_sub(1).ok_or_else(|| {
            Self::voxel_edit_history_error(AuthorityVoxelEditHistoryRejection::EmptyUndoStack)
        })?;
        Self::validate_replay_budget(target_cursor, request.max_replay_steps)?;
        let options = Self::history_diff_options(request.max_diff_voxels, false)?;
        let receipt = history
            .undo_one_with_options(options)
            .map_err(Self::voxel_edit_history_error)?;
        let revert_request = Self::convenience_revert_request(
            &request.history_id,
            protocol::VoxelEditHistoryRevertMode::Undo,
            target_cursor,
            &request.expected_history_hash,
            &request.expected_cursor_hash,
            request.max_replay_steps,
            request.max_diff_voxels,
        );
        let protocol_receipt =
            self.protocol_voxel_edit_history_revert_receipt(revert_request, receipt, &history)?;
        self.commit_voxel_edit_history(history);
        Ok(VoxelEditHistoryUndoReceipt {
            request,
            receipt: protocol_receipt,
        })
    }

    pub(super) fn redo_voxel_edit_authority(
        &mut self,
        request: VoxelEditHistoryRedoRequest,
    ) -> BridgeResult<VoxelEditHistoryRedoReceipt> {
        self.require_initialized("redo_voxel_edit")?;
        let mut history = self.voxel_edit_history()?.clone();
        Self::validate_history_id(&request.history_id)?;
        Self::validate_current_history_hashes(
            &request.expected_history_hash,
            &request.expected_cursor_hash,
            &history,
        )?;
        let target_cursor = history.cursor().index.saturating_add(1);
        if target_cursor > history.entries().len() {
            return Err(Self::voxel_edit_history_error(
                AuthorityVoxelEditHistoryRejection::EmptyRedoStack,
            ));
        }
        Self::validate_replay_budget(target_cursor, request.max_replay_steps)?;
        let options = Self::history_diff_options(request.max_diff_voxels, false)?;
        let receipt = history
            .redo_one_with_options(options)
            .map_err(Self::voxel_edit_history_error)?;
        let revert_request = Self::convenience_revert_request(
            &request.history_id,
            protocol::VoxelEditHistoryRevertMode::Redo,
            target_cursor,
            &request.expected_history_hash,
            &request.expected_cursor_hash,
            request.max_replay_steps,
            request.max_diff_voxels,
        );
        let protocol_receipt =
            self.protocol_voxel_edit_history_revert_receipt(revert_request, receipt, &history)?;
        self.commit_voxel_edit_history(history);
        Ok(VoxelEditHistoryRedoReceipt {
            request,
            receipt: protocol_receipt,
        })
    }

    fn voxel_edit_history(&self) -> BridgeResult<&AuthorityVoxelEditHistory> {
        self.voxel_edit_history.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "voxel edit history authority is not loaded in the engine bridge",
            )
        })
    }

    fn commit_voxel_edit_history(&mut self, history: AuthorityVoxelEditHistory) {
        self.voxel = Some(history.current_world().clone());
        self.voxel_edit_history = Some(history);
    }

    fn validate_history_id(history_id: &str) -> BridgeResult<()> {
        if history_id != DEFAULT_VOXEL_EDIT_HISTORY_ID {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "unknown voxel edit history id {history_id:?}; expected {DEFAULT_VOXEL_EDIT_HISTORY_ID}"
                ),
            ));
        }
        Ok(())
    }

    fn validate_expected_history_hash(
        expected: Option<&str>,
        history: &AuthorityVoxelEditHistory,
    ) -> BridgeResult<()> {
        if let Some(expected) = expected {
            let actual = Self::protocol_authority_hash(history.cursor().history_hash);
            if expected != actual {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!("stale voxel edit history hash: expected {expected}, current {actual}"),
                ));
            }
        }
        Ok(())
    }

    fn validate_current_history_hashes(
        expected_history_hash: &str,
        expected_cursor_hash: &str,
        history: &AuthorityVoxelEditHistory,
    ) -> BridgeResult<()> {
        let actual = Self::protocol_authority_hash(history.cursor().history_hash);
        if expected_history_hash != actual {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "stale voxel edit history hash: expected {expected_history_hash}, current {actual}"
                ),
            ));
        }
        if expected_cursor_hash != actual {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "stale voxel edit cursor hash: expected {expected_cursor_hash}, current {actual}"
                ),
            ));
        }
        Ok(())
    }

    fn validate_revert_request(
        &self,
        request: &VoxelEditHistoryRevertRequest,
        history: &AuthorityVoxelEditHistory,
    ) -> BridgeResult<()> {
        Self::validate_history_id(&request.history_id)?;
        Self::validate_current_history_hashes(
            &request.expected_history_hash,
            &request.expected_cursor_hash,
            history,
        )
    }

    fn validate_replay_budget(cursor_index: usize, max_replay_steps: u64) -> BridgeResult<()> {
        if max_replay_steps == 0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel edit history max_replay_steps must be positive",
            ));
        }
        if u64::try_from(cursor_index).unwrap_or(u64::MAX) > max_replay_steps {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "voxel edit history replay target requires {cursor_index} steps, request allows {max_replay_steps}"
                ),
            ));
        }
        Ok(())
    }

    fn history_diff_options(
        max_diff_voxels: u64,
        include_sample_window: bool,
    ) -> BridgeResult<AuthorityVoxelEditHistoryDiffOptions> {
        let max_changed_voxels = usize::try_from(max_diff_voxels).map_err(|_| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel edit history max_diff_voxels does not fit this runtime",
            )
        })?;
        if max_changed_voxels == 0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel edit history max_diff_voxels must be positive",
            ));
        }
        Ok(AuthorityVoxelEditHistoryDiffOptions::new(
            max_changed_voxels,
            include_sample_window,
        ))
    }

    fn resolve_revert_target(
        history: &AuthorityVoxelEditHistory,
        target: &protocol::VoxelEditHistoryRevertTarget,
    ) -> BridgeResult<usize> {
        let selected = usize::from(target.transaction_id.is_some())
            + usize::from(target.cursor_id.is_some())
            + usize::from(target.cursor_index.is_some());
        if selected != 1 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel edit revert target must select exactly one transaction, cursor, or cursor index",
            ));
        }
        if let Some(transaction_id) = target.transaction_id.as_deref() {
            return history
                .entries()
                .iter()
                .position(|entry| {
                    Self::protocol_transaction_id(entry.transaction_id) == transaction_id
                })
                .map(|index| index + 1)
                .ok_or_else(|| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::InvalidInput,
                        format!("unknown voxel edit transaction id {transaction_id:?}"),
                    )
                });
        }
        if let Some(cursor_id) = target.cursor_id.as_deref() {
            return Ok(Self::find_history_cursor(history, cursor_id)?.index);
        }
        let cursor_index =
            usize::try_from(target.cursor_index.unwrap_or_default()).map_err(|_| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    "voxel edit cursor index does not fit this runtime",
                )
            })?;
        history
            .cursor_at_index(cursor_index)
            .map_err(Self::voxel_edit_history_error)?;
        Ok(cursor_index)
    }

    fn find_history_cursor(
        history: &AuthorityVoxelEditHistory,
        cursor_id: &str,
    ) -> BridgeResult<AuthorityVoxelEditHistoryCursor> {
        for index in 0..=history.entries().len() {
            let cursor = history
                .cursor_at_index(index)
                .map_err(Self::voxel_edit_history_error)?;
            if Self::protocol_cursor_id(cursor.cursor_id) == cursor_id {
                return Ok(cursor);
            }
        }
        Err(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("unknown voxel edit cursor id {cursor_id:?}"),
        ))
    }

    fn voxel_material_catalog_hash(&self) -> String {
        let key = self
            .materials
            .ids()
            .map(|material| material.raw().to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("fnv1a64:{}", Self::fnv1a64(&key))
    }

    fn protocol_voxel_edit_history_cursor(
        &self,
        history: &AuthorityVoxelEditHistory,
        cursor: AuthorityVoxelEditHistoryCursor,
        material_catalog_hash: &str,
    ) -> BridgeResult<protocol::VoxelEditHistoryCursor> {
        let parent_cursor_id = if cursor.index == 0 {
            None
        } else {
            Some(Self::protocol_cursor_id(
                history
                    .cursor_at_index(cursor.index - 1)
                    .map_err(Self::voxel_edit_history_error)?
                    .cursor_id,
            ))
        };
        Ok(protocol::VoxelEditHistoryCursor {
            cursor_id: Self::protocol_cursor_id(cursor.cursor_id),
            cursor_kind: protocol::VoxelEditHistoryCursorKind::Applied,
            applied_transaction_id: cursor
                .applied_transaction_id
                .map(Self::protocol_transaction_id),
            parent_cursor_id,
            history_hash: Self::protocol_authority_hash(cursor.history_hash),
            voxel_state_hash: Self::protocol_authority_hash(cursor.world_hash),
            material_catalog_hash: material_catalog_hash.to_string(),
            undo_depth: cursor.undo_depth as u64,
            redo_depth: cursor.redo_depth as u64,
            entry_count: history.entries().len() as u64,
            checkpoint_count: 0,
        })
    }

    fn protocol_voxel_edit_history_entry(
        &self,
        entry: &AuthorityVoxelEditHistoryEntry,
        material_catalog_hash: &str,
    ) -> protocol::VoxelEditHistoryEntry {
        let command_log = encode_edit_log(&entry.receipt.events);
        protocol::VoxelEditHistoryEntry {
            transaction_id: Self::protocol_transaction_id(entry.transaction_id),
            parent_transaction_id: entry
                .parent_transaction_id
                .map(Self::protocol_transaction_id),
            cursor_id: Self::protocol_cursor_id(entry.cursor_id),
            parent_cursor_id: Some(Self::protocol_cursor_id(entry.parent_cursor_id)),
            entry_kind: protocol::VoxelEditHistoryEntryKind::AcceptedTransaction,
            operation_label: "submit_commands".to_string(),
            provenance: "runtime_bridge.submit_commands".to_string(),
            command_hash: format!("fnv1a64:{}", Self::fnv1a64(&command_log)),
            receipt_hash: Self::protocol_authority_hash(entry.receipt.transaction_hash),
            before_voxel_hash: Self::protocol_authority_hash(entry.receipt.before_hash),
            after_voxel_hash: Self::protocol_authority_hash(entry.receipt.after_hash),
            projected_voxel_hash: Some(Self::protocol_authority_hash(entry.receipt.projected_hash)),
            material_catalog_hash: material_catalog_hash.to_string(),
            command_count: entry.receipt.accepted as u64,
            event_count: entry.receipt.event_count as u64,
            touched_bounds: None,
            touched_voxel_count: entry.receipt.touched_voxels,
            checkpoint: None,
            diff_summary: None,
            diagnostics: Vec::new(),
        }
    }

    fn protocol_voxel_edit_history_revert_receipt(
        &self,
        request: VoxelEditHistoryRevertRequest,
        receipt: AuthorityVoxelEditHistoryRevertReceipt,
        history: &AuthorityVoxelEditHistory,
    ) -> BridgeResult<VoxelEditHistoryRevertReceipt> {
        let material_catalog_hash = self.voxel_material_catalog_hash();
        let cursor_before = self.protocol_voxel_edit_history_cursor(
            history,
            receipt.cursor_before,
            &material_catalog_hash,
        )?;
        let cursor_after = self.protocol_voxel_edit_history_cursor(
            history,
            receipt.cursor_after,
            &material_catalog_hash,
        )?;
        let diff_summary = Self::protocol_voxel_edit_history_diff(&receipt.diff);
        let replay_hash = Self::protocol_authority_hash(receipt.replay_hash);
        let preview_evidence = if receipt.preview {
            Some(protocol::VoxelEditHistoryPreviewEvidence {
                request_mode: request.mode,
                target: request.target.clone(),
                projected_cursor: Some(cursor_after.clone()),
                diff_summary: Some(diff_summary.clone()),
                replay_hash: Some(replay_hash.clone()),
                diagnostics: Vec::new(),
            })
        } else {
            None
        };
        Ok(VoxelEditHistoryRevertReceipt {
            applied: receipt.applied,
            preview: receipt.preview,
            history_id: DEFAULT_VOXEL_EDIT_HISTORY_ID.to_string(),
            history_hash_before: cursor_before.history_hash.clone(),
            history_hash_after: Some(cursor_after.history_hash.clone()),
            request,
            cursor_before,
            cursor_after: Some(cursor_after),
            durable_entry: None,
            preview_evidence,
            diff_summary: Some(diff_summary),
            replay_hash: Some(replay_hash),
            diagnostics: Vec::new(),
        })
    }

    fn protocol_voxel_edit_history_diff(
        diff: &AuthorityVoxelEditHistoryDiffSummary,
    ) -> protocol::VoxelEditHistoryDiffSummary {
        let diagnostics = diff
            .diagnostics
            .iter()
            .map(|diagnostic| match diagnostic {
                AuthorityVoxelEditHistoryDiffDiagnostic::DiffTruncated { limit, observed } => {
                    protocol::VoxelEditHistoryDiagnostic {
                        code: protocol::VoxelEditHistoryDiagnosticCode::DiffTruncated,
                        severity: DiagnosticSeverity::Warning,
                        reference: "voxel_edit_history.diff".to_string(),
                        message: format!(
                            "voxel history diff exceeded limit {limit}; observed at least {observed} changed voxels"
                        ),
                    }
                }
            })
            .collect();
        protocol::VoxelEditHistoryDiffSummary {
            diff_level: if diff.partial {
                protocol::VoxelEditHistoryDiffLevel::Partial
            } else if diff.sample_window_ref.is_some() {
                protocol::VoxelEditHistoryDiffLevel::BoundedSamples
            } else {
                protocol::VoxelEditHistoryDiffLevel::Summary
            },
            partial: diff.partial,
            changed_voxel_count: diff.changed_voxel_count,
            touched_bounds: diff
                .touched_bounds
                .map(|bounds| protocol::VoxelEditHistoryBounds {
                    min: protocol::VoxelEditHistoryCoord {
                        x: bounds.min.x,
                        y: bounds.min.y,
                        z: bounds.min.z,
                    },
                    max: protocol::VoxelEditHistoryCoord {
                        x: bounds.max.x,
                        y: bounds.max.y,
                        z: bounds.max.z,
                    },
                }),
            material_deltas: diff
                .material_deltas
                .iter()
                .filter_map(|delta| {
                    delta
                        .material
                        .map(|material| protocol::VoxelEditHistoryMaterialDelta {
                            material,
                            before_count: delta.before_count,
                            after_count: delta.target_count,
                            delta: delta.delta,
                        })
                })
                .collect(),
            included_transaction_ids: diff
                .included_transaction_ids
                .iter()
                .copied()
                .map(Self::protocol_transaction_id)
                .collect(),
            before_voxel_hash: Self::protocol_authority_hash(diff.before_hash),
            current_voxel_hash: Self::protocol_authority_hash(diff.current_hash),
            target_voxel_hash: Self::protocol_authority_hash(diff.target_hash),
            projected_voxel_hash: diff.projected_hash.map(Self::protocol_authority_hash),
            sample_window_ref: diff.sample_window_ref.clone(),
            diagnostics,
        }
    }

    fn convenience_revert_request(
        history_id: &str,
        mode: protocol::VoxelEditHistoryRevertMode,
        cursor_index: usize,
        expected_history_hash: &str,
        expected_cursor_hash: &str,
        max_replay_steps: u64,
        max_diff_voxels: u64,
    ) -> VoxelEditHistoryRevertRequest {
        VoxelEditHistoryRevertRequest {
            history_id: history_id.to_string(),
            mode,
            target: protocol::VoxelEditHistoryRevertTarget {
                transaction_id: None,
                cursor_id: None,
                cursor_index: Some(cursor_index as u64),
            },
            expected_history_hash: expected_history_hash.to_string(),
            expected_cursor_hash: expected_cursor_hash.to_string(),
            max_replay_steps,
            max_diff_voxels,
            include_sample_window: false,
        }
    }

    fn protocol_authority_hash(hash: u64) -> String {
        format!("fnv1a64:{hash:016x}")
    }

    fn protocol_transaction_id(transaction_id: u64) -> String {
        format!("transaction/{transaction_id}")
    }

    fn protocol_cursor_id(cursor_id: u64) -> String {
        format!("cursor/{cursor_id:016x}")
    }

    fn voxel_edit_history_error(error: AuthorityVoxelEditHistoryRejection) -> RuntimeBridgeError {
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("voxel edit history authority rejected request: {error:?}"),
        )
    }
}
