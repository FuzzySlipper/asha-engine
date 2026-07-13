use super::*;

pub(super) fn initialize(
    bridge: &mut EngineBridge,
    config: EngineConfig,
) -> BridgeResult<EngineHandle> {
    let handle = EngineHandle::new(config.seed);
    bridge.engine = Some(handle);
    bridge.buffers.reset();
    bridge.scene_document = Some(EngineBridge::initial_scene_document());
    let seed_handle = bridge.buffers.create(
        buffer_provider::BufferKind::Opaque,
        buffer_provider::BufferLifetime::Manual,
        None,
        config.seed.to_le_bytes().to_vec(),
    );
    debug_assert_eq!(seed_handle.raw(), 0);
    let world = EngineBridge::launch_world();
    bridge.reset_voxel_edit_history(world);
    bridge.materials = MaterialCatalog::new([1, 2, 3].into_iter().map(VoxelMaterialId::new));
    bridge.cameras.clear();
    bridge.camera_controllers.clear();
    bridge.next_camera = 1;
    bridge.fps_session = None;
    bridge.fps_seed = None;
    bridge.fps_epoch = 0;
    bridge.input_session = None;
    bridge.time_controller = TimeController::default();
    bridge.simulation = SimulationAuthority::new();
    bridge.authority_tick = 0;
    bridge.game_rule_modules.clear();
    bridge.game_rule_active_modifiers.clear();
    bridge.game_rule_recent_trace.clear();
    bridge.game_rule_recent_replay_hashes.clear();
    let presentation_catalog = presentation_catalog::built_in_presentation_catalog();
    bridge.audio_projector = Some(AudioProjector::new(&presentation_catalog));
    bridge.billboard_projector = Some(BillboardProjector::new(&presentation_catalog));
    bridge.particle_projector = Some(ParticleProjector::new(
        &presentation_catalog,
        ParticleProjectionLimits::default(),
    ));
    bridge.telemetry_overlay_projector = Some(TelemetryOverlayProjector::default());
    bridge.projection_frame = Some(RuntimeProjectionFrame::empty(0));
    let (sources, source_metadata) = EngineBridge::seeded_voxel_conversion_authority()?;
    bridge.voxel_conversion_sources = sources;
    bridge.voxel_conversion_source_metadata = source_metadata;
    bridge.voxel_conversion_targets = EngineBridge::seeded_voxel_conversion_targets();
    bridge.voxel_conversion_plan = None;
    bridge.voxel_conversion_evidence.clear();
    bridge.voxel_model_infos.clear();
    bridge.active_voxel_model = None;
    bridge.voxel_annotation_layers.clear();
    Ok(handle)
}
