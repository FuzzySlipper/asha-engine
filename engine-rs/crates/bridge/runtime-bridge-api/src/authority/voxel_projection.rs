use super::*;
use protocol_render::RenderDiff;

impl EngineBridge {
    pub(super) fn read_render_diffs_authority(
        &mut self,
        cursor: u64,
    ) -> BridgeResult<RenderFrameDiff> {
        self.require_initialized("read_render_diffs")?;
        if self.voxel.voxel.is_none() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "read_render_diffs called before voxel authority was initialized",
            ));
        }
        self.drain_voxel_projection_frame(cursor)
    }

    pub(super) fn drain_voxel_projection_frame(
        &mut self,
        cursor: u64,
    ) -> BridgeResult<RenderFrameDiff> {
        if let Some(latest) = &self.projection.voxel_update_telemetry.latest {
            let telemetry = &self.projection.voxel_update_telemetry;
            let has_pending_work = telemetry.pending_committed_command_batches > 0
                || telemetry.pending_accepted_commands > 0
                || telemetry.pending_touched_voxels > 0
                || !self.projection.pending_voxel_frame.is_empty()
                || self
                    .voxel
                    .voxel
                    .as_ref()
                    .is_some_and(|world| world.dirty_count() > 0);
            if cursor == latest.projection_cursor && !has_pending_work {
                return Ok(RenderFrameDiff::default());
            }
            if cursor <= latest.projection_cursor {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    "frame cursor is stale or already identifies a completed projection",
                ));
            }
        }
        let mut frame = std::mem::take(&mut self.projection.pending_voxel_frame);
        let Some(world) = self.voxel.voxel.as_mut() else {
            return Ok(frame);
        };
        let grid = world.grid().id().raw() as u64;
        let resident_chunk_count = world.resident_chunks().count() as u64;
        let chunks_dirtied = world.dirty_count() as u64;
        let projected = self
            .projection
            .voxel_projector
            .project_dirty_with_work(world);
        let chunks_projected = projected.projected_chunk_count;
        let chunks_remeshed = projected.remeshed_chunk_count;
        frame.ops.extend(projected.frame.ops);
        let pending_dirty_chunk_count = world.dirty_count() as u64;
        let emitted_mesh_count = frame
            .ops
            .iter()
            .filter(|op| matches!(op, RenderDiff::ReplaceMeshPayload { .. }))
            .count() as u64;
        let emitted_render_op_count = frame.ops.len() as u64;
        if !self.projection.voxel_projector.diagnostics().is_empty() {
            self.record_developer_console(DeveloperConsoleEmission {
                severity: DiagnosticSeverity::Error,
                category: DeveloperConsoleCategory::Resource,
                source: DeveloperConsoleSource::Projection,
                message: "one or more voxel chunks could not be projected".to_owned(),
                correlation: Some(format!("projection-cursor:{cursor}")),
                authority_tick: Some(self.time.authority_tick),
                detail: DeveloperConsoleDetail {
                    code: "voxel_projection_failed".to_owned(),
                    operation: Some("read_workspace_authoring_projection".to_owned()),
                    resource_kind: Some("voxel_chunk_mesh".to_owned()),
                    resource_id: None,
                    reason: Some("voxel mesh exceeded the supported projection limits".to_owned()),
                },
            });
        }
        let telemetry = &mut self.projection.voxel_update_telemetry;
        telemetry.latest = Some(VoxelUpdateTelemetryReadout {
            schema_version: VOXEL_UPDATE_TELEMETRY_SCHEMA_VERSION,
            compatibility_version: VOXEL_UPDATE_TELEMETRY_COMPATIBILITY_VERSION.to_owned(),
            grid,
            projection_cursor: cursor,
            authority_tick: self.time.authority_tick,
            committed_command_batch_count: std::mem::take(
                &mut telemetry.pending_committed_command_batches,
            ),
            accepted_command_count: std::mem::take(&mut telemetry.pending_accepted_commands),
            touched_voxel_count: std::mem::take(&mut telemetry.pending_touched_voxels),
            resident_chunk_count,
            chunks_dirtied,
            chunks_projected,
            chunks_remeshed,
            emitted_mesh_count,
            emitted_render_op_count,
            pending_dirty_chunk_count,
        });
        Ok(frame)
    }
}
