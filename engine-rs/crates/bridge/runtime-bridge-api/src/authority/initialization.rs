use super::*;

pub(super) fn initialize(
    bridge: &mut EngineBridge,
    config: EngineConfig,
) -> BridgeResult<EngineHandle> {
    let handle = EngineHandle::new(config.seed);
    bridge.bundle.engine = Some(handle);
    bridge.voxel.buffers.reset();
    bridge.scene.scene_document = Some(EngineBridge::initial_scene_document());
    let seed_handle = bridge.voxel.buffers.create(
        buffer_provider::BufferKind::Opaque,
        buffer_provider::BufferLifetime::Manual,
        None,
        config.seed.to_le_bytes().to_vec(),
    );
    debug_assert_eq!(seed_handle.raw(), 0);
    let world = EngineBridge::launch_world();
    bridge.reset_voxel_edit_history(world);
    bridge.voxel.materials = MaterialCatalog::new([1, 2, 3].into_iter().map(VoxelMaterialId::new));
    bridge.camera.cameras.clear();
    bridge.camera.camera_controllers.clear();
    bridge.camera.next_camera = 1;
    bridge.gameplay.fps_session = None;
    bridge.gameplay.fps_seed = None;
    bridge.gameplay.fps_epoch = 0;
    bridge.input.input_session = None;
    bridge.time.time_controller = TimeController::default();
    bridge.time.simulation = SimulationAuthority::new();
    bridge.time.authority_tick = 0;
    bridge.gameplay.game_rule_modules.clear();
    bridge.gameplay.game_rule_active_modifiers.clear();
    bridge.gameplay.game_rule_recent_trace.clear();
    bridge.evidence.game_rule_recent_replay_hashes.clear();
    let presentation_catalog = presentation_catalog::built_in_presentation_catalog();
    bridge.projection.audio_projector = Some(AudioProjector::new(&presentation_catalog));
    bridge.projection.billboard_projector = Some(BillboardProjector::new(&presentation_catalog));
    bridge.projection.particle_projector = Some(ParticleProjector::new(
        &presentation_catalog,
        ParticleProjectionLimits::default(),
    ));
    bridge.projection.telemetry_overlay_projector = Some(TelemetryOverlayProjector::default());
    bridge.projection.projection_frame = Some(RuntimeProjectionFrame::empty(0));
    let (sources, source_metadata) = EngineBridge::seeded_voxel_conversion_authority()?;
    bridge.voxel.voxel_conversion_sources = sources;
    bridge.voxel.voxel_conversion_source_metadata = source_metadata;
    bridge.voxel.voxel_conversion_targets = EngineBridge::seeded_voxel_conversion_targets();
    bridge.voxel.voxel_conversion_plan = None;
    bridge.evidence.voxel_conversion_evidence.clear();
    bridge.voxel.voxel_model_infos.clear();
    bridge.voxel.active_voxel_model = None;
    bridge.voxel.voxel_annotation_layers.clear();
    Ok(handle)
}
