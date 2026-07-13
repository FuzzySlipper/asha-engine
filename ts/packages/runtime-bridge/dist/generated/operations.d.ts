export type BridgeSurface = 'stable' | 'quarantined';
export type BridgeErrorFamily = 'not_initialized' | 'invalid_input' | 'unknown_handle' | 'buffer_expired' | 'native_unavailable' | 'voxel_conversion_unavailable' | 'unsupported_source_asset' | 'source_hash_mismatch' | 'invalid_material_map' | 'output_limit_exceeded' | 'stale_authority_snapshot' | 'conversion_replay_mismatch' | 'operation_unimplemented' | 'internal';
export interface BridgeOperation {
    readonly capability: string;
    readonly errors: string;
    readonly facadeMethod: string;
    readonly input: string;
    readonly inputWire: BridgeWireTypeRef;
    readonly manifestName: string;
    readonly maxInputBytes: number;
    readonly maxOutputBytes: number;
    readonly nativeWired: boolean;
    readonly output: string;
    readonly outputWire: BridgeWireTypeRef;
    readonly surface: BridgeSurface;
}
export interface BridgeWireTypeRef {
    readonly name: string;
    readonly owner: 'custom' | 'generated' | 'handle' | 'unit';
    readonly repeated: boolean;
}
declare const BRIDGE_OPERATION_DESCRIPTORS: readonly [{
    readonly capability: "bundle_lifecycle";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "initializeEngine";
    readonly input: "protocol_runtime::EngineConfig";
    readonly inputWire: {
        readonly name: "EngineConfig";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "initialize_engine";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "EngineHandle";
    readonly outputWire: {
        readonly name: "EngineHandle";
        readonly owner: "handle";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "time_simulation";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "stepSimulation";
    readonly input: "protocol_runtime::StepInputEnvelope";
    readonly inputWire: {
        readonly name: "StepInputEnvelope";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "step_simulation";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::StepResult";
    readonly outputWire: {
        readonly name: "StepResult";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "time_simulation";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyTimeControlCommand";
    readonly input: "protocol_time_control::TimeControlCommand";
    readonly inputWire: {
        readonly name: "timeControl.TimeControlCommand";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_time_control_command";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_time_control::TimeControlReceipt";
    readonly outputWire: {
        readonly name: "timeControl.TimeControlReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "time_simulation";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readTimeControlState";
    readonly input: "Unit";
    readonly inputWire: {
        readonly name: "Unit";
        readonly owner: "unit";
        readonly repeated: false;
    };
    readonly manifestName: "read_time_control_state";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_time_control::TimeControlState";
    readonly outputWire: {
        readonly name: "timeControl.TimeControlState";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "submitCommands";
    readonly input: "protocol_voxel::CommandBatch";
    readonly inputWire: {
        readonly name: "voxel.CommandBatch";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "submit_commands";
    readonly maxInputBytes: 2097152;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel::CommandResult";
    readonly outputWire: {
        readonly name: "voxel.CommandResult";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "pickVoxel";
    readonly input: "protocol_voxel::PickRay";
    readonly inputWire: {
        readonly name: "voxel.PickRay";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "pick_voxel";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel::PickResult";
    readonly outputWire: {
        readonly name: "voxel.PickResult";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "camera";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyCollisionConstrainedCameraInput";
    readonly input: "protocol_view::CollisionConstrainedCameraInputEnvelope";
    readonly inputWire: {
        readonly name: "view.CollisionConstrainedCameraInputEnvelope";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_collision_constrained_camera_input";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_view::CameraCollisionSnapshot";
    readonly outputWire: {
        readonly name: "view.CameraCollisionSnapshot";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyGeneratedTunnelToRuntimeWorld";
    readonly input: "protocol_view::GeneratedTunnelRuntimeApplyRequest";
    readonly inputWire: {
        readonly name: "view.GeneratedTunnelRuntimeApplyRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_generated_tunnel_to_runtime_world";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_view::GeneratedTunnelRuntimeApplyReceipt";
    readonly outputWire: {
        readonly name: "view.GeneratedTunnelRuntimeApplyReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "selectVoxel";
    readonly input: "protocol_view::ScreenPointToPickRayRequest";
    readonly inputWire: {
        readonly name: "view.ScreenPointToPickRayRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "select_voxel";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_view::VoxelSelectionSnapshot";
    readonly outputWire: {
        readonly name: "view.VoxelSelectionSnapshot";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readVoxelMeshEvidence";
    readonly input: "protocol_render::VoxelMeshEvidenceRequest";
    readonly inputWire: {
        readonly name: "VoxelMeshEvidenceRequest";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "read_voxel_mesh_evidence";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_render::VoxelMeshEvidenceSnapshot";
    readonly outputWire: {
        readonly name: "VoxelMeshEvidenceSnapshot";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "planVoxelConversion";
    readonly input: "protocol_voxel_conversion::VoxelConversionPlanRequest";
    readonly inputWire: {
        readonly name: "voxelConversion.VoxelConversionPlanRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "plan_voxel_conversion";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_conversion::VoxelConversionPlan";
    readonly outputWire: {
        readonly name: "voxelConversion.VoxelConversionPlan";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "registerVoxelConversionSource";
    readonly input: "protocol_voxel_conversion::VoxelConversionSourceRegistrationRequest";
    readonly inputWire: {
        readonly name: "voxelConversion.VoxelConversionSourceRegistrationRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "register_voxel_conversion_source";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_conversion::VoxelConversionSourceRegistration";
    readonly outputWire: {
        readonly name: "voxelConversion.VoxelConversionSourceRegistration";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "registerVoxelConversionMeshAsset";
    readonly input: "protocol_voxel_conversion::VoxelConversionMeshAssetRegistrationRequest";
    readonly inputWire: {
        readonly name: "voxelConversion.VoxelConversionMeshAssetRegistrationRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "register_voxel_conversion_mesh_asset";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_conversion::VoxelConversionSourceRegistration";
    readonly outputWire: {
        readonly name: "voxelConversion.VoxelConversionSourceRegistration";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "importVoxelConversionMeshSource";
    readonly input: "protocol_voxel_conversion::VoxelConversionMeshSourceImportRequest";
    readonly inputWire: {
        readonly name: "voxelConversion.VoxelConversionMeshSourceImportRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "import_voxel_conversion_mesh_source";
    readonly maxInputBytes: 268468224;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_conversion::VoxelConversionMeshSourceImportReceipt";
    readonly outputWire: {
        readonly name: "voxelConversion.VoxelConversionMeshSourceImportReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readVoxelConversionSourceMetadata";
    readonly input: "protocol_voxel_conversion::VoxelConversionSourceMetadataRequest";
    readonly inputWire: {
        readonly name: "voxelConversion.VoxelConversionSourceMetadataRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "read_voxel_conversion_source_metadata";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_conversion::VoxelConversionSourceMetadataReadout";
    readonly outputWire: {
        readonly name: "voxelConversion.VoxelConversionSourceMetadataReadout";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "previewVoxelConversion";
    readonly input: "protocol_voxel_conversion::VoxelConversionPreviewRequest";
    readonly inputWire: {
        readonly name: "voxelConversion.VoxelConversionPreviewRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "preview_voxel_conversion";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_conversion::VoxelConversionPreview";
    readonly outputWire: {
        readonly name: "voxelConversion.VoxelConversionPreview";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyVoxelConversion";
    readonly input: "protocol_voxel_conversion::VoxelConversionApplyRequest";
    readonly inputWire: {
        readonly name: "voxelConversion.VoxelConversionApplyRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_voxel_conversion";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_conversion::VoxelConversionReceipt";
    readonly outputWire: {
        readonly name: "voxelConversion.VoxelConversionReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "replay_evidence";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "exportVoxelConversionEvidence";
    readonly input: "protocol_voxel_conversion::VoxelConversionEvidenceRef[]";
    readonly inputWire: {
        readonly name: "voxelConversion.VoxelConversionEvidenceRef";
        readonly owner: "generated";
        readonly repeated: true;
    };
    readonly manifestName: "export_voxel_conversion_evidence";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_conversion::VoxelConversionEvidenceRef[]";
    readonly outputWire: {
        readonly name: "voxelConversion.VoxelConversionEvidenceRef";
        readonly owner: "generated";
        readonly repeated: true;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readVoxelModelInfo";
    readonly input: "protocol_voxel_conversion::VoxelModelInfoRequest";
    readonly inputWire: {
        readonly name: "voxelConversion.VoxelModelInfoRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "read_voxel_model_info";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_conversion::VoxelModelInfoReadout";
    readonly outputWire: {
        readonly name: "voxelConversion.VoxelModelInfoReadout";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readVoxelModelWindow";
    readonly input: "protocol_voxel_conversion::VoxelModelWindowRequest";
    readonly inputWire: {
        readonly name: "voxelConversion.VoxelModelWindowRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "read_voxel_model_window";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_conversion::VoxelModelWindowReadout";
    readonly outputWire: {
        readonly name: "voxelConversion.VoxelModelWindowReadout";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "exportVoxelVolumeAsset";
    readonly input: "protocol_voxel_asset::VoxelVolumeAssetExportRequest";
    readonly inputWire: {
        readonly name: "voxelAsset.VoxelVolumeAssetExportRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "export_voxel_volume_asset";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_asset::VoxelVolumeAssetExportReceipt";
    readonly outputWire: {
        readonly name: "voxelAsset.VoxelVolumeAssetExportReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "saveVoxelVolumeAsset";
    readonly input: "protocol_voxel_asset::VoxelVolumeAssetSaveRequest";
    readonly inputWire: {
        readonly name: "voxelAsset.VoxelVolumeAssetSaveRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "save_voxel_volume_asset";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_asset::VoxelVolumeAssetSaveReceipt";
    readonly outputWire: {
        readonly name: "voxelAsset.VoxelVolumeAssetSaveReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "updateVoxelVolumeAssetPalette";
    readonly input: "protocol_voxel_asset::VoxelVolumeAssetPaletteUpdateRequest";
    readonly inputWire: {
        readonly name: "voxelAsset.VoxelVolumeAssetPaletteUpdateRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "update_voxel_volume_asset_palette";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_asset::VoxelVolumeAssetPaletteUpdateReceipt";
    readonly outputWire: {
        readonly name: "voxelAsset.VoxelVolumeAssetPaletteUpdateReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "initializeVoxelVolumeAuthoring";
    readonly input: "protocol_voxel_asset::VoxelVolumeAuthoringInitializeRequest";
    readonly inputWire: {
        readonly name: "voxelAsset.VoxelVolumeAuthoringInitializeRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "initialize_voxel_volume_authoring";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_asset::VoxelVolumeAuthoringInitializeReceipt";
    readonly outputWire: {
        readonly name: "voxelAsset.VoxelVolumeAuthoringInitializeReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "loadVoxelVolumeAsset";
    readonly input: "protocol_voxel_asset::VoxelVolumeAssetLoadRequest";
    readonly inputWire: {
        readonly name: "voxelAsset.VoxelVolumeAssetLoadRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "load_voxel_volume_asset";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_asset::VoxelVolumeAssetLoadReceipt";
    readonly outputWire: {
        readonly name: "voxelAsset.VoxelVolumeAssetLoadReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "unloadVoxelVolumeAsset";
    readonly input: "protocol_voxel_asset::VoxelVolumeAssetUnloadRequest";
    readonly inputWire: {
        readonly name: "voxelAsset.VoxelVolumeAssetUnloadRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "unload_voxel_volume_asset";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_asset::VoxelVolumeAssetUnloadReceipt";
    readonly outputWire: {
        readonly name: "voxelAsset.VoxelVolumeAssetUnloadReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "validateVoxelAnnotationLayer";
    readonly input: "protocol_voxel_annotation::VoxelAnnotationLayerValidationRequest";
    readonly inputWire: {
        readonly name: "voxelAnnotation.VoxelAnnotationLayerValidationRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "validate_voxel_annotation_layer";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_annotation::VoxelAnnotationLayerValidationReport";
    readonly outputWire: {
        readonly name: "voxelAnnotation.VoxelAnnotationLayerValidationReport";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "loadVoxelAnnotationLayer";
    readonly input: "protocol_voxel_annotation::VoxelAnnotationLayerLoadRequest";
    readonly inputWire: {
        readonly name: "voxelAnnotation.VoxelAnnotationLayerLoadRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "load_voxel_annotation_layer";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_annotation::VoxelAnnotationLayerLoadReceipt";
    readonly outputWire: {
        readonly name: "voxelAnnotation.VoxelAnnotationLayerLoadReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readVoxelAnnotationQuery";
    readonly input: "protocol_voxel_annotation::VoxelAnnotationQueryRequest";
    readonly inputWire: {
        readonly name: "voxelAnnotation.VoxelAnnotationQueryRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "read_voxel_annotation_query";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_annotation::VoxelAnnotationQueryReadout";
    readonly outputWire: {
        readonly name: "voxelAnnotation.VoxelAnnotationQueryReadout";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyVoxelAnnotationEdit";
    readonly input: "protocol_voxel_annotation::VoxelAnnotationEditRequest";
    readonly inputWire: {
        readonly name: "voxelAnnotation.VoxelAnnotationEditRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_voxel_annotation_edit";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_annotation::VoxelAnnotationEditReceipt";
    readonly outputWire: {
        readonly name: "voxelAnnotation.VoxelAnnotationEditReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "exportVoxelAnnotationLayer";
    readonly input: "protocol_voxel_annotation::VoxelAnnotationLayerExportRequest";
    readonly inputWire: {
        readonly name: "voxelAnnotation.VoxelAnnotationLayerExportRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "export_voxel_annotation_layer";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_annotation::VoxelAnnotationLayerExportReceipt";
    readonly outputWire: {
        readonly name: "voxelAnnotation.VoxelAnnotationLayerExportReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readVoxelEditHistory";
    readonly input: "protocol_voxel_edit_history::VoxelEditHistoryReadRequest";
    readonly inputWire: {
        readonly name: "voxelEditHistory.VoxelEditHistoryReadRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "read_voxel_edit_history";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_edit_history::VoxelEditHistorySummary";
    readonly outputWire: {
        readonly name: "voxelEditHistory.VoxelEditHistorySummary";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "previewVoxelEditRevert";
    readonly input: "protocol_voxel_edit_history::VoxelEditHistoryRevertRequest";
    readonly inputWire: {
        readonly name: "voxelEditHistory.VoxelEditHistoryRevertRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "preview_voxel_edit_revert";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_edit_history::VoxelEditHistoryRevertReceipt";
    readonly outputWire: {
        readonly name: "voxelEditHistory.VoxelEditHistoryRevertReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyVoxelEditRevert";
    readonly input: "protocol_voxel_edit_history::VoxelEditHistoryRevertRequest";
    readonly inputWire: {
        readonly name: "voxelEditHistory.VoxelEditHistoryRevertRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_voxel_edit_revert";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_edit_history::VoxelEditHistoryRevertReceipt";
    readonly outputWire: {
        readonly name: "voxelEditHistory.VoxelEditHistoryRevertReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "undoVoxelEdit";
    readonly input: "protocol_voxel_edit_history::VoxelEditHistoryUndoRequest";
    readonly inputWire: {
        readonly name: "voxelEditHistory.VoxelEditHistoryUndoRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "undo_voxel_edit";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_edit_history::VoxelEditHistoryUndoReceipt";
    readonly outputWire: {
        readonly name: "voxelEditHistory.VoxelEditHistoryUndoReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "redoVoxelEdit";
    readonly input: "protocol_voxel_edit_history::VoxelEditHistoryRedoRequest";
    readonly inputWire: {
        readonly name: "voxelEditHistory.VoxelEditHistoryRedoRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "redo_voxel_edit";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_voxel_edit_history::VoxelEditHistoryRedoReceipt";
    readonly outputWire: {
        readonly name: "voxelEditHistory.VoxelEditHistoryRedoReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "loadFpsRuntimeSession";
    readonly input: "protocol_runtime::FpsRuntimeSessionLoadRequest";
    readonly inputWire: {
        readonly name: "FpsRuntimeSessionLoadRequest";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "load_fps_runtime_session";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::FpsRuntimeSessionSnapshot";
    readonly outputWire: {
        readonly name: "FpsRuntimeSessionSnapshot";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readFpsRuntimeSession";
    readonly input: "Unit";
    readonly inputWire: {
        readonly name: "Unit";
        readonly owner: "unit";
        readonly repeated: false;
    };
    readonly manifestName: "read_fps_runtime_session";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::FpsRuntimeSessionSnapshot";
    readonly outputWire: {
        readonly name: "FpsRuntimeSessionSnapshot";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyFpsPrimaryFire";
    readonly input: "protocol_runtime::FpsPrimaryFireRequest";
    readonly inputWire: {
        readonly name: "FpsPrimaryFireRequest";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "apply_fps_primary_fire";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::FpsPrimaryFireResult";
    readonly outputWire: {
        readonly name: "FpsPrimaryFireResult";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "invokeGameExtensionWeaponEffect";
    readonly input: "protocol_runtime::GameExtensionWeaponEffectInvocationRequest";
    readonly inputWire: {
        readonly name: "GameExtensionWeaponEffectInvocationRequest";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "invoke_game_extension_weapon_effect";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::GameExtensionWeaponEffectInvocationResult";
    readonly outputWire: {
        readonly name: "GameExtensionWeaponEffectInvocationResult";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "validateGameRuleCatalog";
    readonly input: "protocol_game_rules::GameRuleCatalog";
    readonly inputWire: {
        readonly name: "gameRules.GameRuleCatalog";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "validate_game_rule_catalog";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::GameRuleCatalogValidationReceipt";
    readonly outputWire: {
        readonly name: "GameRuleCatalogValidationReceipt";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "submitGameRuleEffectIntent";
    readonly input: "protocol_runtime::GameRuleEffectIntentRequest";
    readonly inputWire: {
        readonly name: "GameRuleEffectIntentRequest";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "submit_game_rule_effect_intent";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_game_rules::GameRuleResolutionReceipt";
    readonly outputWire: {
        readonly name: "gameRules.GameRuleResolutionReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readGameRuleRuntimeReadout";
    readonly input: "Unit";
    readonly inputWire: {
        readonly name: "Unit";
        readonly owner: "unit";
        readonly repeated: false;
    };
    readonly manifestName: "read_game_rule_runtime_readout";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::GameRuleRuntimeReadout";
    readonly outputWire: {
        readonly name: "GameRuleRuntimeReadout";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "restartFpsRuntimeSession";
    readonly input: "protocol_runtime::FpsRuntimeSessionRestartRequest";
    readonly inputWire: {
        readonly name: "FpsRuntimeSessionRestartRequest";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "restart_fps_runtime_session";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::FpsRuntimeSessionSnapshot";
    readonly outputWire: {
        readonly name: "FpsRuntimeSessionSnapshot";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readFpsEncounterDirector";
    readonly input: "protocol_runtime::FpsEncounterLifecycleInput";
    readonly inputWire: {
        readonly name: "FpsEncounterLifecycleInput";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "read_fps_encounter_director";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::FpsEncounterDirectorSnapshot";
    readonly outputWire: {
        readonly name: "FpsEncounterDirectorSnapshot";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "gameplay";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyFpsEncounterTransition";
    readonly input: "protocol_runtime::FpsEncounterTransitionRequest";
    readonly inputWire: {
        readonly name: "FpsEncounterTransitionRequest";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "apply_fps_encounter_transition";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::FpsEncounterTransitionResult";
    readonly outputWire: {
        readonly name: "FpsEncounterTransitionResult";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "projection";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readProjectionFrame";
    readonly input: "FrameCursor";
    readonly inputWire: {
        readonly name: "FrameCursor";
        readonly owner: "handle";
        readonly repeated: false;
    };
    readonly manifestName: "read_projection_frame";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_presentation::RuntimeProjectionFrame";
    readonly outputWire: {
        readonly name: "presentation.RuntimeProjectionFrame";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "projection";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readRenderDiffs";
    readonly input: "FrameCursor";
    readonly inputWire: {
        readonly name: "FrameCursor";
        readonly owner: "handle";
        readonly repeated: false;
    };
    readonly manifestName: "read_render_diffs";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_render::RenderFrameDiffDescriptor";
    readonly outputWire: {
        readonly name: "render.RenderFrameDiff";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "scene_entities";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readModelMaterialPreview";
    readonly input: "protocol_render::ModelMaterialPreviewRequest";
    readonly inputWire: {
        readonly name: "render.ModelMaterialPreviewRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "read_model_material_preview";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_render::ModelMaterialPreviewSnapshot";
    readonly outputWire: {
        readonly name: "render.ModelMaterialPreviewSnapshot";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "scene_entities";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readSceneObjectSnapshot";
    readonly input: "Unit";
    readonly inputWire: {
        readonly name: "Unit";
        readonly owner: "unit";
        readonly repeated: false;
    };
    readonly manifestName: "read_scene_object_snapshot";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_scene::SceneObjectSnapshot";
    readonly outputWire: {
        readonly name: "scene.SceneObjectSnapshot";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "scene_entities";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applySceneObjectCommand";
    readonly input: "protocol_scene::SceneObjectCommandRequest";
    readonly inputWire: {
        readonly name: "scene.SceneObjectCommandRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_scene_object_command";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_scene::SceneObjectCommandResult";
    readonly outputWire: {
        readonly name: "scene.SceneObjectCommandResult";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "input";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "configureInputSession";
    readonly input: "protocol_input::InputSessionConfigureRequest";
    readonly inputWire: {
        readonly name: "input.InputSessionConfigureRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "configure_input_session";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_input::InputSessionSnapshot";
    readonly outputWire: {
        readonly name: "input.InputSessionSnapshot";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "input";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyInputContextCommand";
    readonly input: "protocol_input::InputContextCommand";
    readonly inputWire: {
        readonly name: "input.InputContextCommand";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_input_context_command";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_input::InputContextChangeReceipt";
    readonly outputWire: {
        readonly name: "input.InputContextChangeReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "input";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "submitRawInput";
    readonly input: "protocol_input::RawInputSample";
    readonly inputWire: {
        readonly name: "input.RawInputSample";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "submit_raw_input";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_input::InputResolutionReceipt";
    readonly outputWire: {
        readonly name: "input.InputResolutionReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "input";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "replayResolvedInputAction";
    readonly input: "protocol_input::RecordedInputAction";
    readonly inputWire: {
        readonly name: "input.RecordedInputAction";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "replay_resolved_input_action";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_input::InputActionReplayReceipt";
    readonly outputWire: {
        readonly name: "input.InputActionReplayReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "input";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readInputContextState";
    readonly input: "Unit";
    readonly inputWire: {
        readonly name: "Unit";
        readonly owner: "unit";
        readonly repeated: false;
    };
    readonly manifestName: "read_input_context_state";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_input::InputContextStackState";
    readonly outputWire: {
        readonly name: "input.InputContextStackState";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "camera";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "createCamera";
    readonly input: "protocol_view::CameraCreateRequest";
    readonly inputWire: {
        readonly name: "view.CameraCreateRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "create_camera";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_view::CameraSnapshot";
    readonly outputWire: {
        readonly name: "view.CameraSnapshot";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "camera";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyCameraModeCommand";
    readonly input: "protocol_view::CameraModeCommand";
    readonly inputWire: {
        readonly name: "view.CameraModeCommand";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_camera_mode_command";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_view::CameraModeChangeReceipt";
    readonly outputWire: {
        readonly name: "view.CameraModeChangeReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "camera";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyCameraNavigationInput";
    readonly input: "protocol_view::CameraNavigationInputEnvelope";
    readonly inputWire: {
        readonly name: "view.CameraNavigationInputEnvelope";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_camera_navigation_input";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_view::CameraNavigationReceipt";
    readonly outputWire: {
        readonly name: "view.CameraNavigationReceipt";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "camera";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readCameraControllerState";
    readonly input: "protocol_view::CameraControllerReadRequest";
    readonly inputWire: {
        readonly name: "view.CameraControllerReadRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "read_camera_controller_state";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_view::CameraControllerState";
    readonly outputWire: {
        readonly name: "view.CameraControllerState";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "camera";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyFirstPersonCameraInput";
    readonly input: "protocol_view::FirstPersonCameraInputEnvelope";
    readonly inputWire: {
        readonly name: "view.FirstPersonCameraInputEnvelope";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "apply_first_person_camera_input";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_view::CameraSnapshot";
    readonly outputWire: {
        readonly name: "view.CameraSnapshot";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "scene_entities";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "applyEnemyDirectNavMovement";
    readonly input: "protocol_runtime::EnemyDirectNavMovementRequest";
    readonly inputWire: {
        readonly name: "EnemyDirectNavMovementRequest";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "apply_enemy_direct_nav_movement";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_runtime::EnemyDirectNavMovementResult";
    readonly outputWire: {
        readonly name: "EnemyDirectNavMovementResult";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "camera";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "readCameraProjection";
    readonly input: "protocol_view::CameraProjectionRequest";
    readonly inputWire: {
        readonly name: "view.CameraProjectionRequest";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly manifestName: "read_camera_projection";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_view::CameraProjectionSnapshot";
    readonly outputWire: {
        readonly name: "view.CameraProjectionSnapshot";
        readonly owner: "generated";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "getBuffer";
    readonly input: "RuntimeBufferHandle";
    readonly inputWire: {
        readonly name: "RuntimeBufferHandle";
        readonly owner: "handle";
        readonly repeated: false;
    };
    readonly manifestName: "get_buffer";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "RuntimeBufferView";
    readonly outputWire: {
        readonly name: "RuntimeBufferView";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "voxel_assets_buffers";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "releaseBuffer";
    readonly input: "RuntimeBufferHandle";
    readonly inputWire: {
        readonly name: "RuntimeBufferHandle";
        readonly owner: "handle";
        readonly repeated: false;
    };
    readonly manifestName: "release_buffer";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "Unit";
    readonly outputWire: {
        readonly name: "Unit";
        readonly owner: "unit";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "bundle_lifecycle";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "loadProjectBundle";
    readonly input: "protocol_project_bundle::ProjectBundleManifest";
    readonly inputWire: {
        readonly name: "ProjectBundleLoadRequest";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "load_project_bundle";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_diagnostics::DiagnosticReportSet";
    readonly outputWire: {
        readonly name: "CompositionStatus";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "bundle_lifecycle";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "saveProjectBundle";
    readonly input: "Unit";
    readonly inputWire: {
        readonly name: "Unit";
        readonly owner: "unit";
        readonly repeated: false;
    };
    readonly manifestName: "save_project_bundle";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_project_bundle::SaveSummary";
    readonly outputWire: {
        readonly name: "ProjectBundleSaveSummary";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "bundle_lifecycle";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "getProjectBundleCompositionStatus";
    readonly input: "Unit";
    readonly inputWire: {
        readonly name: "Unit";
        readonly owner: "unit";
        readonly repeated: false;
    };
    readonly manifestName: "get_project_bundle_composition_status";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "protocol_diagnostics::DiagnosticReportSet";
    readonly outputWire: {
        readonly name: "CompositionStatus";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "bundle_lifecycle";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "unloadProjectBundle";
    readonly input: "Unit";
    readonly inputWire: {
        readonly name: "Unit";
        readonly owner: "unit";
        readonly repeated: false;
    };
    readonly manifestName: "unload_project_bundle";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: true;
    readonly output: "Unit";
    readonly outputWire: {
        readonly name: "Unit";
        readonly owner: "unit";
        readonly repeated: false;
    };
    readonly surface: "stable";
}, {
    readonly capability: "replay_evidence";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "loadReplayFixture";
    readonly input: "protocol_replay::ReplayFixture";
    readonly inputWire: {
        readonly name: "ReplayFixture";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly manifestName: "load_replay_fixture";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: false;
    readonly output: "ReplaySessionHandle";
    readonly outputWire: {
        readonly name: "ReplaySessionHandle";
        readonly owner: "handle";
        readonly repeated: false;
    };
    readonly surface: "quarantined";
}, {
    readonly capability: "replay_evidence";
    readonly errors: "RuntimeBridgeError";
    readonly facadeMethod: "runReplayStep";
    readonly input: "ReplaySessionHandle";
    readonly inputWire: {
        readonly name: "ReplaySessionHandle";
        readonly owner: "handle";
        readonly repeated: false;
    };
    readonly manifestName: "run_replay_step";
    readonly maxInputBytes: 8388608;
    readonly maxOutputBytes: 8388608;
    readonly nativeWired: false;
    readonly output: "protocol_replay::ReplayStepReport";
    readonly outputWire: {
        readonly name: "ReplayStepReport";
        readonly owner: "custom";
        readonly repeated: false;
    };
    readonly surface: "quarantined";
}];
export type BridgeOperationDescriptor = (typeof BRIDGE_OPERATION_DESCRIPTORS)[number];
export declare const MANIFEST_OPERATIONS: readonly BridgeOperation[];
export declare const NATIVE_WIRED_OPERATIONS: ReadonlySet<string>;
export {};
//# sourceMappingURL=operations.d.ts.map