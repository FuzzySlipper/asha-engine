const CAMERA_HANDLE = 1;
const field = (name, shape, summary, required = true) => ({ name, required, shape, summary });
const scalar = (scalar) => ({ kind: 'scalar', scalar });
const stringShape = scalar('string');
const booleanShape = scalar('boolean');
const integerShape = scalar('integer');
const hashShape = scalar('state_hash');
const nullable = (inner) => ({ kind: 'nullable', inner });
const objectShape = (fields) => ({ kind: 'object', allowExtraFields: false, fields });
const objectSchema = (name, fields) => ({ name, version: 1, shape: objectShape(fields) });
const arrayOf = (items, minItems) => ({ kind: 'array', items, ...(minItems === undefined ? {} : { minItems }) });
const literal = (values) => ({ kind: 'literal', values });
const contract = (exportName) => ({ kind: 'contract', ref: { package: '@asha/contracts', exportName } });
const EMPTY_INPUT = { name: 'EmptyInput', version: 1, shape: { kind: 'empty' } };
const EMPTY_OUTPUT = objectSchema('EmptyOutput', [field('kind', literal(['ok']), 'Acknowledgement literal.')]);
const SESSION_ID_FIELD = field('sessionId', stringShape, 'Stable studio session identifier.');
const SCENARIO_ID_FIELD = field('scenarioId', stringShape, 'Named public studio scenario identifier.');
const VOXEL_COORD_SCHEMA = contract('VoxelCoord');
const VOXEL_COMMAND_SCHEMA = contract('VoxelCommand');
const CATALOG_ENTRY_SCHEMA = contract('CatalogEntry');
const MATERIAL_PROJECTION_SCHEMA = contract('MaterialProjection');
const STATIC_MESH_ASSET_SCHEMA = contract('StaticMeshAsset');
const RENDER_FRAME_DIFF_SCHEMA = contract('RenderFrameDiff');
const VOXEL_CONVERSION_EVIDENCE_SCHEMA = contract('VoxelConversionEvidenceRef');
const COMPAT = {
    contracts: 'contracts.v0',
    runtimeBridge: 'runtime-bridge.v0',
    commandRegistry: 'command-registry.v0',
};
const REGISTRY_COMPAT = { contracts: 'contracts.v0', commandRegistry: 'command-registry.v0' };
const OWNER = '@asha/command-registry';
const noStateImpact = { authority: 'none', editor: 'none', render: 'none', workspace: 'none' };
const readAuthority = { authority: 'read', editor: 'none', render: 'none', workspace: 'none' };
const readEditor = { authority: 'none', editor: 'read', render: 'none', workspace: 'none' };
const mutateEditor = { authority: 'none', editor: 'mutate', render: 'none', workspace: 'none' };
const mutateAuthority = { authority: 'mutate', editor: 'read', render: 'none', workspace: 'none' };
const captureRender = { authority: 'read', editor: 'read', render: 'capture', workspace: 'none' };
const writeWorkspace = { authority: 'read', editor: 'read', render: 'read', workspace: 'write' };
const sessionWorkspace = { authority: 'read', editor: 'mutate', render: 'none', workspace: 'write' };
function artifact(type, summary, required = true) {
    return { type, required, producedWhen: required ? 'always' : 'when_available', summary };
}
function runtime(operation) {
    return { kind: 'runtime_bridge_operation', operation };
}
function runtimeSession(method) {
    return { kind: 'runtime_session_facade_method', method };
}
function summarizeShape(shape) {
    switch (shape.kind) {
        case 'empty':
            return 'No arguments.';
        case 'contract':
            return `Uses ${shape.ref.exportName} from ${shape.ref.package}.`;
        case 'scalar':
            return `${shape.scalar} value.`;
        case 'literal':
            return `One of: ${shape.values.join(', ')}.`;
        case 'nullable':
            return `Nullable ${summarizeShape(shape.inner).replace(/\.$/, '')}.`;
        case 'array':
            return `Array of ${summarizeShape(shape.items).replace(/\.$/, '')}.`;
        case 'object':
            if (shape.fields.length === 0) {
                return 'Object with no fields.';
            }
            return shape.fields.map((fieldDef) => `${fieldDef.name}: ${fieldDef.summary}`).join(' ');
    }
}
function summarizeSchema(schema) {
    return `${schema.name}: ${summarizeShape(schema.shape)}`;
}
function summarizeArtifacts(artifacts) {
    return artifacts.map((decl) => `${decl.type}: ${decl.summary}`).join(' ');
}
function def(definition) {
    return definition;
}
function base(args) {
    const agentExposure = args.agentExposure ?? { kind: 'read_only' };
    return def({
        id: args.id,
        version: 1,
        label: args.label,
        summary: args.summary,
        category: args.category,
        menuPath: args.menuPath,
        commandPalette: { visible: true, keywords: args.keywords },
        inputSchema: args.inputSchema,
        outputSchema: args.outputSchema,
        inputContractRefs: args.inputContractRefs ?? [],
        outputContractRefs: args.outputContractRefs ?? [],
        operationClass: args.operationClass,
        agentExposure,
        guiMirror: {
            required: agentExposure.kind !== 'hidden',
            menuPath: args.menuPath,
            commandPaletteVisible: true,
            argumentSummary: summarizeSchema(args.inputSchema),
            resultSummary: summarizeSchema(args.outputSchema),
            artifactSummary: summarizeArtifacts(args.artifacts),
            ...(args.panel === undefined ? {} : { panel: args.panel }),
            ...(args.dialog === undefined ? {} : { dialog: args.dialog }),
        },
        undo: args.undo ?? { kind: 'not_undoable', reason: 'Read-only or diagnostic command has no mutation to reverse.' },
        retry: args.retry ?? 'safe_to_retry',
        idempotency: args.idempotency ?? { kind: 'idempotent', keyFields: ['sessionId'] },
        artifacts: args.artifacts,
        stateImpact: args.stateImpact,
        owningLane: 'ts-command-registry',
        owningPackage: OWNER,
        runtimeRequirements: args.runtimeRequirements,
        compatibility: args.compatibility ?? COMPAT,
        ...(args.knownLimitations === undefined ? {} : { knownLimitations: args.knownLimitations }),
        typedInputExample: args.typedInputExample,
        typedOutputExample: args.typedOutputExample,
    });
}
const scenarioListOutput = objectSchema('ScenarioListOutput', [
    field('scenarios', arrayOf({ kind: 'object', allowExtraFields: false, fields: [field('id', stringShape, 'Scenario id.'), field('label', stringShape, 'Human-readable scenario label.')] }), 'Bounded public scenario list.'),
]);
const gameWorkspaceManifestInput = objectSchema('GameWorkspaceManifestInput', [
    field('workspaceRoot', stringShape, 'Absolute or workspace-resolved game workspace root selected by the caller.'),
    field('manifestPath', stringShape, 'Manifest path inside the workspace root.'),
]);
const gameWorkspaceOpenOutput = objectSchema('GameWorkspaceOpenOutput', [
    field('workspaceVersion', stringShape, 'Studio game workspace read model version.'),
    field('workspaceRoot', stringShape, 'Opened game workspace root.'),
    field('manifestPath', stringShape, 'Manifest path inside the workspace root.'),
    field('gameId', stringShape, 'Stable game identity read from the workspace adapter metadata.'),
    field('engineVersion', stringShape, 'Required ASHA engine compatibility version from the manifest.'),
    field('contractsVersion', stringShape, 'Required ASHA contracts compatibility version from the manifest.'),
    field('runtimeBridgeVersion', stringShape, 'Required runtime bridge compatibility version from the manifest.'),
    field('devtoolsProtocolVersion', stringShape, 'Required devtools protocol compatibility version from the manifest.'),
    field('sceneRoots', arrayOf(stringShape), 'Scene root directories declared by the manifest.'),
    field('assetRoots', arrayOf(stringShape), 'Asset root directories declared by the manifest.'),
    field('catalogPackages', arrayOf(stringShape), 'Catalog package directories declared by the manifest.'),
    field('policyPackages', arrayOf(stringShape), 'Policy package directories declared by the manifest.'),
    field('attachEndpoint', stringShape, 'Local devtools websocket endpoint declared by the manifest.'),
    field('devCommand', stringShape, 'Development runtime command declared by the manifest.'),
    field('publishCommand', stringShape, 'Publish command declared by the manifest.'),
    field('workspaceHash', hashShape, 'Deterministic readout hash for the parsed workspace manifest.'),
]);
const gameWorkspaceDiagnosticShape = objectShape([
    field('code', stringShape, 'Classified workspace manifest diagnostic code.'),
    field('message', stringShape, 'Human-readable diagnostic message.'),
    field('source', nullable(stringShape), 'Manifest field or source path when available.'),
]);
const gameWorkspaceValidateOutput = objectSchema('GameWorkspaceValidateOutput', [
    field('valid', booleanShape, 'Whether the manifest validates for Studio workspace use.'),
    field('workspaceHash', nullable(hashShape), 'Deterministic workspace readout hash when validation succeeds.'),
    field('diagnostics', arrayOf(gameWorkspaceDiagnosticShape), 'Classified manifest diagnostics.'),
]);
const scenarioIdInput = objectSchema('ScenarioIdInput', [SCENARIO_ID_FIELD]);
const sessionIdInput = objectSchema('SessionIdInput', [SESSION_ID_FIELD]);
const sessionStatusOutput = objectSchema('SessionStatusOutput', [SESSION_ID_FIELD, field('status', literal(['not_started', 'ready', 'degraded', 'unavailable']), 'Session/runtime status.')]);
const worldSummaryOutput = objectSchema('WorldSummaryOutput', [
    field('authorityHash', nullable(hashShape), 'Authority hash when the public runtime can provide it.'),
    field('voxelVolumeCount', integerShape, 'Number of public voxel volumes.'),
    field('sceneNodeCount', integerShape, 'Number of public scene nodes.'),
]);
const editorStateOutput = objectSchema('EditorStateOutput', [
    field('editorVersion', stringShape, 'Editor state/schema version.'),
    field('selectedVoxel', nullable(VOXEL_COORD_SCHEMA), 'Currently selected voxel, if any.'),
]);
const materialInspectionInput = objectSchema('MaterialInspectionInput', [SESSION_ID_FIELD, field('materialId', stringShape, 'Public catalog material asset id.')]);
const materialInspectionOutput = objectSchema('MaterialInspectionOutput', [
    field('materialId', stringShape, 'Public catalog material asset id.'),
    field('catalogEntry', CATALOG_ENTRY_SCHEMA, 'Generated public catalog entry for the material.'),
    field('material', MATERIAL_PROJECTION_SCHEMA, 'Generated public material projection split into render/collision views.'),
]);
const modelInspectionInput = objectSchema('ModelInspectionInput', [SESSION_ID_FIELD, field('assetId', stringShape, 'Public static mesh asset id.')]);
const modelInspectionOutput = objectSchema('ModelInspectionOutput', [
    field('assetId', stringShape, 'Public static mesh asset id.'),
    field('meshAsset', STATIC_MESH_ASSET_SCHEMA, 'Generated public static mesh asset descriptor.'),
    field('materialSlots', arrayOf(stringShape), 'Material asset ids referenced by the model slots.'),
]);
const modelMaterialPreviewInput = objectSchema('ModelMaterialPreviewInput', [
    SESSION_ID_FIELD,
    field('modelAsset', STATIC_MESH_ASSET_SCHEMA, 'Generated public static mesh asset descriptor to preview.'),
    field('materialId', stringShape, 'Catalog material id to highlight in the preview.'),
]);
const modelMaterialPreviewOutput = objectSchema('ModelMaterialPreviewOutput', [
    field('previewDiff', RENDER_FRAME_DIFF_SCHEMA, 'Generated public retained-mode render diff preview evidence.'),
    field('rendererClassification', literal(['reference_preview', 'runtime_readback']), 'Whether evidence is reference preview or runtime readback.'),
    field('diagnostics', arrayOf(stringShape), 'Typed diagnostic strings for unavailable/degraded support.'),
]);
const voxelConversionPlanInput = objectSchema('VoxelConversionPlanCommandInput', [
    SESSION_ID_FIELD,
    field('request', contract('VoxelConversionPlanRequest'), 'Generated public voxel-conversion plan request.'),
]);
const voxelConversionPlanOutput = objectSchema('VoxelConversionPlanCommandOutput', [
    field('plan', contract('VoxelConversionPlan'), 'Generated public voxel-conversion plan with diagnostics and evidence refs.'),
]);
const voxelConversionPreviewInput = objectSchema('VoxelConversionPreviewCommandInput', [
    SESSION_ID_FIELD,
    field('request', contract('VoxelConversionPreviewRequest'), 'Generated public voxel-conversion preview request.'),
]);
const voxelConversionPreviewOutput = objectSchema('VoxelConversionPreviewCommandOutput', [
    field('preview', contract('VoxelConversionPreview'), 'Generated public voxel-conversion preview with sampled output and evidence refs.'),
]);
const voxelConversionApplyInput = objectSchema('VoxelConversionApplyCommandInput', [
    SESSION_ID_FIELD,
    field('request', contract('VoxelConversionApplyRequest'), 'Generated public voxel-conversion apply request.'),
]);
const voxelConversionApplyOutput = objectSchema('VoxelConversionApplyCommandOutput', [
    field('receipt', contract('VoxelConversionReceipt'), 'Generated public voxel-conversion apply receipt with diagnostics and evidence refs.'),
]);
const voxelConversionEvidenceExportInput = objectSchema('VoxelConversionEvidenceExportInput', [
    SESSION_ID_FIELD,
    field('evidence', arrayOf(VOXEL_CONVERSION_EVIDENCE_SCHEMA), 'Generated public voxel-conversion evidence refs selected for export.'),
]);
const voxelConversionEvidenceExportOutput = objectSchema('VoxelConversionEvidenceExportOutput', [
    field('evidence', arrayOf(VOXEL_CONVERSION_EVIDENCE_SCHEMA), 'Generated public voxel-conversion evidence refs accepted for export.'),
]);
const loadSceneAssetInput = objectSchema('LoadSceneAssetInput', [
    SESSION_ID_FIELD,
    field('assetId', stringShape, 'Public catalog static-mesh asset id to load into the scene.'),
    field('materialId', stringShape, 'Catalog material id to bind to the placed instance.'),
    field('placement', objectShape([
        field('translation', arrayOf(scalar('number'), 3), 'World translation x, y, z.'),
        field('rotation', arrayOf(scalar('number'), 4), 'Rotation quaternion x, y, z, w.'),
        field('scale', arrayOf(scalar('number'), 3), 'Scale x, y, z.'),
    ]), 'Scene placement transform for the loaded asset.'),
]);
const loadSceneAssetOutput = objectSchema('LoadSceneAssetOutput', [
    field('assetId', stringShape, 'Loaded catalog asset id.'),
    field('renderableIds', arrayOf(stringShape), 'Named renderable ids created for the placed asset.'),
    field('loadDiff', RENDER_FRAME_DIFF_SCHEMA, 'Generated public retained-mode render diff that defines and places the asset.'),
    field('rendererClassification', literal(['reference_placement', 'runtime_readback']), 'Whether evidence is reference placement or runtime readback.'),
    field('diagnostics', arrayOf(stringShape), 'Typed diagnostic strings for unavailable/degraded support.'),
]);
const sceneObjectSnapshotOutput = objectSchema('ReadSceneObjectSnapshotOutput', [
    field('snapshot', contract('SceneObjectSnapshot'), 'Generated public scene-object hierarchy snapshot.'),
]);
const sceneObjectCommandInput = objectSchema('ApplySceneObjectCommandInput', [
    SESSION_ID_FIELD,
    field('request', contract('SceneObjectCommandRequest'), 'Generated public scene-object command request envelope.'),
]);
const sceneObjectCommandOutput = objectSchema('ApplySceneObjectCommandOutput', [
    field('result', contract('SceneObjectCommandResult'), 'Generated public scene-object command result envelope.'),
]);
const screenPointInput = objectSchema('ScreenPointInput', [
    SESSION_ID_FIELD,
    field('request', contract('ScreenPointToPickRayRequest'), 'Generated public screen-point/camera selection request.'),
]);
const voxelSelectionOutput = objectSchema('VoxelSelectionOutput', [field('selection', contract('VoxelSelectionSnapshot'), 'Generated public selection evidence snapshot.')]);
const setActiveEntityInput = objectSchema('SetActiveEntityInput', [SESSION_ID_FIELD, field('entityId', stringShape, 'Entity/renderable id selected from the hierarchy/entity browser.')]);
const setActiveEntityOutput = objectSchema('SetActiveEntityOutput', [
    field('entityId', stringShape, 'Selected entity id echoed back.'),
    field('renderableId', stringShape, 'Viewport renderable id the selection is synced to.'),
    field('selectionHash', hashShape, 'Editor selection evidence hash.'),
    field('selected', booleanShape, 'Whether the entity is now the active editor selection.'),
]);
const setEntityNameInput = objectSchema('SetEntityNameInput', [
    SESSION_ID_FIELD,
    field('entityId', stringShape, 'Entity/renderable id of the currently selected entity to rename.'),
    field('name', stringShape, 'New non-empty display name for the selected entity.'),
]);
const setEntityNameOutput = objectSchema('SetEntityNameOutput', [
    field('entityId', stringShape, 'Selected entity id echoed back.'),
    field('renderableId', stringShape, 'Viewport renderable id the renamed entity is synced to.'),
    field('name', stringShape, 'Applied display name.'),
    field('nameHash', hashShape, 'Editor display-name evidence hash.'),
    field('applied', booleanShape, 'Whether the editor-local name edit was applied to the selected entity.'),
]);
const translateEntityInput = objectSchema('TranslateEntityInput', [
    SESSION_ID_FIELD,
    field('entityId', stringShape, 'Entity/renderable id of the currently selected entity to translate.'),
    field('axis', literal(['x', 'y', 'z']), 'Single world axis the gizmo translates along.'),
    field('delta', scalar('number'), 'Signed translation delta applied along the chosen axis.'),
    field('mode', literal(['preview', 'apply']), 'Whether this is an editor-local preview drag or the committed apply.'),
]);
const translateEntityOutput = objectSchema('TranslateEntityOutput', [
    field('entityId', stringShape, 'Selected entity id echoed back.'),
    field('renderableId', stringShape, 'Viewport renderable id the translated entity is synced to.'),
    field('axis', literal(['x', 'y', 'z']), 'Axis the translate was applied along.'),
    field('delta', scalar('number'), 'Signed translation delta applied along the axis.'),
    field('mode', literal(['preview', 'apply']), 'Preview drag or committed apply.'),
    field('translationBefore', arrayOf(scalar('number'), 3), 'Entity world translation x, y, z before the edit.'),
    field('translationAfter', arrayOf(scalar('number'), 3), 'Entity world translation x, y, z after the edit.'),
    field('transformHash', hashShape, 'Editor transform evidence hash.'),
    field('applied', booleanShape, 'Whether the editor-local translate was committed (apply) versus preview.'),
]);
const voxelInspectionInput = objectSchema('VoxelInspectionInput', [SESSION_ID_FIELD, field('voxel', VOXEL_COORD_SCHEMA, 'Voxel coordinate to inspect.')]);
const voxelInspectionOutput = objectSchema('VoxelInspectionOutput', [
    field('voxel', VOXEL_COORD_SCHEMA, 'Inspected voxel coordinate.'),
    field('materialId', nullable(integerShape), 'Material id when occupied.'),
    field('occupied', booleanShape, 'Whether the voxel is occupied.'),
]);
const previewInput = objectSchema('VoxelBrushPreviewInput', [
    SESSION_ID_FIELD,
    field('anchor', VOXEL_COORD_SCHEMA, 'Preview anchor coordinate.'),
    field('commands', arrayOf(VOXEL_COMMAND_SCHEMA, 1), 'Typed voxel command preview set.'),
]);
const previewOutput = objectSchema('VoxelBrushPreviewOutput', [
    field('targetVoxels', arrayOf(VOXEL_COORD_SCHEMA), 'Voxels affected by the preview.'),
    field('previewVersion', stringShape, 'Editor preview version.'),
]);
const applyInput = objectSchema('ApplyVoxelBrushInput', [
    SESSION_ID_FIELD,
    field('commands', arrayOf(VOXEL_COMMAND_SCHEMA, 1), 'Typed authority voxel commands.'),
    field('expectedStateHash', nullable(hashShape), 'Expected authority state hash or null when unavailable.'),
]);
const applyOutput = objectSchema('ApplyVoxelBrushOutput', [
    field('accepted', booleanShape, 'Whether authority accepted the command batch.'),
    field('authorityBeforeHash', nullable(hashShape), 'Before hash when available.'),
    field('authorityAfterHash', nullable(hashShape), 'After hash when available.'),
]);
const lastCommandResultOutput = objectSchema('LastCommandResultOutput', [
    field('sequenceId', nullable(stringShape), 'Last command sequence id, if any.'),
    field('status', nullable(literal(['ok', 'rejected', 'partial', 'failed', 'unavailable'])), 'Last command status, if any.'),
]);
const captureInput = objectSchema('CaptureBeforeAfterInput', [
    SESSION_ID_FIELD,
    field('beforeArtifactId', scalar('artifact_ref'), 'Before visual evidence artifact id.'),
    field('afterArtifactId', scalar('artifact_ref'), 'After visual evidence artifact id.'),
]);
const captureOutput = objectSchema('CaptureBeforeAfterOutput', [
    field('artifactId', scalar('artifact_ref'), 'Combined before/after artifact id.'),
    field('renderBeforeHash', nullable(hashShape), 'Before render evidence hash when available.'),
    field('renderAfterHash', nullable(hashShape), 'After render evidence hash when available.'),
]);
const exportInput = objectSchema('ExportAgentReadoutInput', [SESSION_ID_FIELD, field('includeVisualEvidence', booleanShape, 'Whether exported readout references visual artifacts.')]);
const exportOutput = objectSchema('ExportAgentReadoutOutput', [field('artifactId', scalar('artifact_ref'), 'Exported readout artifact id.'), field('commandCount', integerShape, 'Number of commands included in the readout.')]);
const materialProjectionExample = {
    render: { color: { r: 0.8, g: 0.4, b: 0.2, a: 1 }, texture: null, roughness: 0.6, emissive: 0, uvStrategy: 'flat' },
    collision: { solid: true, collidable: true, occludes: true, structuralClass: 'solid' },
};
const materialEntryExample = {
    id: 'material.copper',
    kind: 'material',
    version: 1,
    hash: 'sha256-material-copper',
    sourcePath: null,
    label: 'Copper',
    dependencies: [],
    material: materialProjectionExample,
};
const meshAssetExample = {
    asset: 'mesh.preview-cube',
    payload: {
        layout: {
            vertexCount: 8,
            indexCount: 36,
            indexWidth: 'u32',
            attributes: [
                { name: 'position', components: 3, kind: 'f32' },
                { name: 'normal', components: 3, kind: 'f32' },
            ],
        },
        groups: [{ materialSlot: 0, start: 0, count: 36 }],
        bounds: { min: [-0.5, -0.5, -0.5], max: [0.5, 0.5, 0.5] },
        source: { kind: 'inline', positions: [], normals: [], indices: [] },
        provenance: 'staticAsset',
    },
    materialSlots: [{ slot: 0, material: 'material.copper' }],
    collision: { kind: 'aabbFallback' },
};
const modelMaterialPreviewDiffExample = {
    ops: [
        { op: 'defineMaterial', material: { id: 'material.copper', color: [0.8, 0.4, 0.2, 1], texture: null, roughness: 0.6, emissive: 0, uvStrategy: 'flat' } },
        { op: 'defineStaticMesh', asset: meshAssetExample },
        { op: 'createStaticMeshInstance', handle: 7001, parent: null, instance: { asset: 'mesh.preview-cube', transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] }, materialOverrides: [], metadata: { source: null, tags: [], label: 'Model/material preview' } } },
    ],
};
const loadSceneAssetDiffExample = {
    ops: [
        { op: 'defineMaterial', material: { id: 'material.copper', color: [0.8, 0.4, 0.2, 1], texture: null, roughness: 0.6, emissive: 0, uvStrategy: 'flat' } },
        { op: 'defineStaticMesh', asset: meshAssetExample },
        { op: 'createStaticMeshInstance', handle: 7101, parent: null, instance: { asset: 'mesh.preview-cube', transform: { translation: [1, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] }, materialOverrides: [], metadata: { source: null, tags: [], label: 'Loaded demo asset instance' } } },
    ],
};
const sceneObjectDocumentExample = {
    schemaVersion: 1,
    id: 1,
    metadata: { name: 'Scene object example', authoringFormatVersion: 1 },
    dependencies: [],
    nodes: [
        {
            id: 1,
            parent: null,
            childOrder: 0,
            label: 'Root',
            tags: [],
            transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
            kind: { kind: 'emptyGroup' },
        },
    ],
};
const sceneObjectSnapshotExample = {
    documentHash: 1001,
    objects: [
        {
            id: 1,
            parent: null,
            childOrder: 0,
            label: 'Root',
            kind: 'emptyGroup',
            hasRenderableAsset: false,
        },
    ],
};
const renamedSceneObjectRecordExample = {
    id: 1,
    parent: null,
    childOrder: 0,
    label: 'Renamed root',
    kind: 'emptyGroup',
    hasRenderableAsset: false,
};
const sceneObjectCommandRequestExample = {
    expectedDocumentHash: sceneObjectSnapshotExample.documentHash,
    command: {
        kind: 'rename',
        id: 1,
        label: 'Renamed root',
    },
};
const sceneObjectCommandResultExample = {
    accepted: true,
    outcome: {
        document: sceneObjectDocumentExample,
        snapshot: {
            documentHash: 1002,
            objects: [renamedSceneObjectRecordExample],
        },
        selected: 1,
    },
    rejection: null,
};
const selectionExample = {
    pickRay: {
        camera: CAMERA_HANDLE,
        tick: 0,
        grid: 0,
        screenPoint: { x: 0.5, y: 0.5, space: 'normalized_0_1' },
        origin: [0, 0, 0],
        direction: [1, 0, 0],
        maxDistance: 128,
        cameraProjectionHash: 'projection-hash',
        rayHash: 'ray-hash',
    },
    outcome: 'miss',
    selectedVoxel: null,
    selectedFace: null,
    editAnchor: null,
    selectionHash: 'selection-hash',
};
const voxelConversionPlanRequestExample = {
    source: {
        assetId: 'mesh.preview-cube',
        assetKind: 'mesh',
        assetVersion: 1,
        sourceHash: 'sha256-source-mesh',
        meshPrimitive: null,
    },
    target: {
        grid: 0,
        volumeAssetId: 'voxel.generated.preview-cube',
        origin: { x: 0, y: 0, z: 0 },
    },
    settings: {
        mode: 'surface',
        fitPolicy: 'contain',
        originPolicy: 'target_min',
        resolution: [8, 8, 8],
        voxelSize: 0.25,
        maxOutputVoxels: 512,
        transform: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
        materialMap: {
            entries: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material.copper', voxelMaterial: 3 }],
            defaultVoxelMaterial: 3,
        },
    },
};
const voxelConversionEvidenceExample = {
    kind: 'plan',
    uri: 'asha://voxel-conversion/plan/plan-preview-cube',
    contentHash: 'sha256-plan',
};
const voxelConversionPlanExample = {
    planId: 'plan-preview-cube',
    source: voxelConversionPlanRequestExample.source,
    target: voxelConversionPlanRequestExample.target,
    settings: voxelConversionPlanRequestExample.settings,
    authorityVersion: 'svc-voxel-conversion.v0',
    expectedSourceHash: 'sha256-source-mesh',
    settingsHash: 'sha256-settings',
    planHash: 'sha256-plan',
    estimatedOutputVoxels: 24,
    estimatedBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 7, y: 7, z: 7 } },
    diagnostics: [
        {
            code: 'operation_unimplemented',
            severity: 'info',
            reference: 'runtime-session.voxel-conversion',
            message: 'Native voxel conversion backend may fail closed until wired.',
        },
    ],
    evidence: [voxelConversionEvidenceExample],
};
const voxelConversionPreviewRequestExample = {
    planId: voxelConversionPlanExample.planId,
    expectedPlanHash: voxelConversionPlanExample.planHash,
};
const voxelConversionPreviewExample = {
    planId: voxelConversionPlanExample.planId,
    outputHash: 'sha256-preview-output',
    outputVoxelCount: 2,
    outputBounds: voxelConversionPlanExample.estimatedBounds,
    sampleVoxels: [
        { coord: { x: 0, y: 0, z: 0 }, material: 3 },
        { coord: { x: 1, y: 0, z: 0 }, material: 3 },
    ],
    diagnostics: [],
    evidence: [{ kind: 'preview', uri: 'asha://voxel-conversion/preview/plan-preview-cube', contentHash: 'sha256-preview' }],
};
const voxelConversionReceiptExample = {
    planId: voxelConversionPlanExample.planId,
    applied: true,
    outputHash: 'sha256-applied-output',
    outputVoxelCount: 24,
    outputBounds: voxelConversionPlanExample.estimatedBounds,
    diagnostics: [],
    evidence: [{ kind: 'apply_receipt', uri: 'asha://voxel-conversion/apply/plan-preview-cube', contentHash: 'sha256-apply' }],
};
export const COMMAND_MANIFEST = [
    base({
        id: 'session.list_scenarios', label: 'List Studio Scenarios', summary: 'List named public scenarios available to a studio session.', category: 'session', menuPath: ['Session', 'List Scenarios'], keywords: ['scenario', 'list'],
        inputSchema: EMPTY_INPUT, outputSchema: scenarioListOutput, operationClass: 'read_only', stateImpact: noStateImpact, compatibility: REGISTRY_COMPAT, runtimeRequirements: [{ kind: 'none' }], artifacts: [artifact('scenario_manifest', 'Bounded scenario list for session loading.')], typedInputExample: { kind: 'empty' }, typedOutputExample: { scenarios: [{ id: 'voxel-basic', label: 'Basic Voxel Scenario' }] }, panel: 'inspector', dialog: 'readout_only', idempotency: { kind: 'idempotent', keyFields: [] },
    }),
    base({
        id: 'session.start', label: 'Start Studio Session', summary: 'Create/reset a studio session around a named scenario.', category: 'session', menuPath: ['Session', 'Start'], keywords: ['session', 'start'],
        inputSchema: scenarioIdInput, outputSchema: EMPTY_OUTPUT, operationClass: 'workspace_io', agentExposure: { kind: 'workspace_io', batchable: false }, stateImpact: sessionWorkspace, runtimeRequirements: [runtime('initialize_engine'), runtime('load_world_bundle')], artifacts: [artifact('session_status', 'Initial session status and compatibility readback.')], typedInputExample: { scenarioId: 'voxel-basic' }, typedOutputExample: { kind: 'ok' }, panel: 'timeline', dialog: 'simple_form', retry: 'retry_after_status_readback', idempotency: { kind: 'conditional', condition: 'Idempotent when scenarioId and session reset token match.' },
    }),
    base({
        id: 'session.load_scenario', label: 'Load Scenario', summary: 'Load a named scenario into the active studio session.', category: 'session', menuPath: ['Session', 'Load Scenario'], keywords: ['load', 'scenario'],
        inputSchema: scenarioIdInput, outputSchema: EMPTY_OUTPUT, operationClass: 'workspace_io', agentExposure: { kind: 'workspace_io', batchable: false }, stateImpact: sessionWorkspace, runtimeRequirements: [runtime('load_world_bundle')], artifacts: [artifact('session_status', 'Scenario load status and diagnostics.')], typedInputExample: { scenarioId: 'voxel-basic' }, typedOutputExample: { kind: 'ok' }, panel: 'timeline', dialog: 'simple_form', retry: 'retry_after_status_readback', idempotency: { kind: 'conditional', condition: 'Safe when current session already targets the same scenario id.' },
    }),
    base({
        id: 'workspace.open_game_manifest', label: 'Open Game Workspace', summary: 'Open a game workspace manifest into Studio as a typed readout before live runtime attach.', category: 'workspace', menuPath: ['File', 'Open Game Workspace'], keywords: ['workspace', 'game', 'manifest', 'open'],
        inputSchema: gameWorkspaceManifestInput, outputSchema: gameWorkspaceOpenOutput, operationClass: 'workspace_io', agentExposure: { kind: 'workspace_io', batchable: false }, stateImpact: { authority: 'none', editor: 'mutate', render: 'none', workspace: 'read' }, compatibility: REGISTRY_COMPAT, runtimeRequirements: [{ kind: 'editor_store' }], artifacts: [artifact('game_workspace', 'Parsed game workspace readout with compatibility, roots, commands, and attach endpoint.')], typedInputExample: { workspaceRoot: '/workspace/asha-demo', manifestPath: 'asha.game.toml' }, typedOutputExample: { workspaceVersion: 'studio-game-workspace.v0', workspaceRoot: '/workspace/asha-demo', manifestPath: 'asha.game.toml', gameId: 'asha-demo', engineVersion: '0.1.0', contractsVersion: '0.1.0', runtimeBridgeVersion: '0.1.0', devtoolsProtocolVersion: 'devtools-protocol.v0', sceneRoots: ['scenes'], assetRoots: ['assets'], catalogPackages: ['packages/game-catalogs'], policyPackages: ['packages/game-policy'], attachEndpoint: 'ws://127.0.0.1:7391', devCommand: 'npm run dev', publishCommand: 'npm run conformance', workspaceHash: 'studio-game-workspace-hash' }, panel: 'inspector', dialog: 'simple_form', retry: 'retry_after_status_readback', idempotency: { kind: 'conditional', condition: 'Safe when manifestPath, workspaceRoot, and manifest file contents are unchanged.' },
    }),
    base({
        id: 'workspace.validate_game_manifest', label: 'Validate Game Manifest', summary: 'Validate a game workspace manifest and return classified diagnostics without opening or attaching runtime state.', category: 'workspace', menuPath: ['File', 'Validate Game Manifest'], keywords: ['workspace', 'game', 'manifest', 'validate'],
        inputSchema: gameWorkspaceManifestInput, outputSchema: gameWorkspaceValidateOutput, operationClass: 'workspace_io', agentExposure: { kind: 'workspace_io', batchable: false }, stateImpact: { authority: 'none', editor: 'none', render: 'none', workspace: 'read' }, compatibility: REGISTRY_COMPAT, runtimeRequirements: [{ kind: 'none' }], artifacts: [artifact('game_workspace', 'Validation result and classified manifest diagnostics.')], typedInputExample: { workspaceRoot: '/workspace/asha-demo', manifestPath: 'asha.game.toml' }, typedOutputExample: { valid: true, workspaceHash: 'studio-game-workspace-hash', diagnostics: [] }, panel: 'diagnostics', dialog: 'readout_only', retry: 'safe_to_retry', idempotency: { kind: 'idempotent', keyFields: ['workspaceRoot', 'manifestPath'] },
    }),
    base({
        id: 'inspection.session_status', label: 'Inspect Session Status', summary: 'Read studio/runtime readiness, compatibility, and degradation status.', category: 'inspection', menuPath: ['Inspect', 'Session Status'], keywords: ['status', 'compatibility'],
        inputSchema: sessionIdInput, outputSchema: sessionStatusOutput, operationClass: 'read_only', stateImpact: noStateImpact, runtimeRequirements: [runtime('get_composition_status')], artifacts: [artifact('session_status', 'Status readback for the active session.')], typedInputExample: { sessionId: 'session-1' }, typedOutputExample: { sessionId: 'session-1', status: 'ready' }, panel: 'diagnostics', dialog: 'readout_only',
    }),
    base({
        id: 'inspection.world_summary', label: 'Inspect World Summary', summary: 'Read compact public world and authority evidence.', category: 'inspection', menuPath: ['Inspect', 'World Summary'], keywords: ['world', 'hash'],
        inputSchema: sessionIdInput, outputSchema: worldSummaryOutput, operationClass: 'read_only', stateImpact: readAuthority, runtimeRequirements: [runtime('read_voxel_mesh_evidence')], artifacts: [artifact('world_summary', 'Compact authority/render-neutral world summary.')], typedInputExample: { sessionId: 'session-1' }, typedOutputExample: { authorityHash: null, voxelVolumeCount: 1, sceneNodeCount: 1 }, panel: 'inspector', dialog: 'readout_only',
    }),
    base({
        id: 'inspection.editor_state', label: 'Inspect Editor State', summary: 'Read command-registry/editor-local selection and preview state.', category: 'inspection', menuPath: ['Inspect', 'Editor State'], keywords: ['editor', 'selection'],
        inputSchema: sessionIdInput, outputSchema: editorStateOutput, operationClass: 'read_only', stateImpact: readEditor, runtimeRequirements: [{ kind: 'editor_store' }], artifacts: [artifact('editor_state', 'Editor-local state snapshot.')], typedInputExample: { sessionId: 'session-1' }, typedOutputExample: { editorVersion: 'editor.v0', selectedVoxel: null }, panel: 'inspector', dialog: 'readout_only', compatibility: REGISTRY_COMPAT,
    }),
    base({
        id: 'inspection.material', label: 'Inspect Material', summary: 'Read a public catalog material projection for Studio preview and diagnostics.', category: 'inspection', menuPath: ['Inspect', 'Material'], keywords: ['material', 'catalog', 'inspect'],
        inputSchema: materialInspectionInput, outputSchema: materialInspectionOutput, operationClass: 'read_only', stateImpact: readAuthority, runtimeRequirements: [runtime('read_model_material_preview')], artifacts: [artifact('material_metadata', 'Material catalog entry and render/collision projection.')], typedInputExample: { sessionId: 'session-1', materialId: 'material.copper' }, typedOutputExample: { materialId: 'material.copper', catalogEntry: materialEntryExample, material: materialProjectionExample }, panel: 'inspector', dialog: 'readout_only', inputContractRefs: [], outputContractRefs: [{ package: '@asha/contracts', exportName: 'CatalogEntry' }, { package: '@asha/contracts', exportName: 'MaterialProjection' }], knownLimitations: ['Native model/material runtime readback may fail closed until the native bridge wires read_model_material_preview.'],
    }),
    base({
        id: 'inspection.model', label: 'Inspect Model', summary: 'Read a public static mesh asset descriptor and its material slots.', category: 'inspection', menuPath: ['Inspect', 'Model'], keywords: ['model', 'mesh', 'inspect'],
        inputSchema: modelInspectionInput, outputSchema: modelInspectionOutput, operationClass: 'read_only', stateImpact: readAuthority, runtimeRequirements: [runtime('read_model_material_preview')], artifacts: [artifact('model_metadata', 'Static mesh descriptor and material slot metadata.')], typedInputExample: { sessionId: 'session-1', assetId: 'mesh.preview-cube' }, typedOutputExample: { assetId: 'mesh.preview-cube', meshAsset: meshAssetExample, materialSlots: ['material.copper'] }, panel: 'inspector', dialog: 'readout_only', outputContractRefs: [{ package: '@asha/contracts', exportName: 'StaticMeshAsset' }], knownLimitations: ['Native model runtime readback may fail closed until the native bridge wires read_model_material_preview.'],
    }),
    base({
        id: 'preview.model_material', label: 'Preview Model / Material', summary: 'Preview a static mesh with catalog material projection as retained-mode render-diff evidence without authority mutation.', category: 'preview', menuPath: ['Preview', 'Model / Material'], keywords: ['preview', 'model', 'material', 'render diff'],
        inputSchema: modelMaterialPreviewInput, outputSchema: modelMaterialPreviewOutput, operationClass: 'editor_local', stateImpact: { authority: 'read', editor: 'mutate', render: 'capture', workspace: 'none' }, runtimeRequirements: [runtime('read_model_material_preview'), { kind: 'editor_store' }, { kind: 'render_surface' }], artifacts: [artifact('model_metadata', 'Static mesh preview metadata.'), artifact('material_metadata', 'Material projection metadata.'), artifact('render_diff_preview', 'Retained-mode render diff preview evidence.')], typedInputExample: { sessionId: 'session-1', modelAsset: meshAssetExample, materialId: 'material.copper' }, typedOutputExample: { previewDiff: modelMaterialPreviewDiffExample, rendererClassification: 'reference_preview', diagnostics: [] }, panel: 'viewport', dialog: 'simple_form', inputContractRefs: [{ package: '@asha/contracts', exportName: 'StaticMeshAsset' }], outputContractRefs: [{ package: '@asha/contracts', exportName: 'RenderFrameDiff' }], agentExposure: { kind: 'editor_local' }, undo: { kind: 'editor_local', inverseData: ['previous model/material preview snapshot'] }, idempotency: { kind: 'conditional', condition: 'Same model asset, material id, and catalog versions produce the same preview diff.' }, knownLimitations: ['Preview evidence is render-diff/readback metadata only; screenshots, hardware GPU, and performance evidence are out of scope.'],
    }),
    base({
        id: 'voxel_conversion.plan', label: 'Plan Voxel Conversion', summary: 'Request a deterministic Rust-owned voxel conversion plan for a supported static mesh source.', category: 'preview', menuPath: ['Voxel Conversion', 'Plan'], keywords: ['voxel', 'conversion', 'plan', 'mesh'],
        inputSchema: voxelConversionPlanInput, outputSchema: voxelConversionPlanOutput, operationClass: 'read_only', stateImpact: readAuthority, runtimeRequirements: [runtimeSession('planVoxelConversion')], artifacts: [artifact('voxel_conversion_plan', 'Plan id, hashes, output estimate, diagnostics, and evidence refs.')], typedInputExample: { sessionId: 'session-1', request: voxelConversionPlanRequestExample }, typedOutputExample: { plan: voxelConversionPlanExample }, panel: 'inspector', dialog: 'advanced_form', inputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelConversionPlanRequest' }], outputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelConversionPlan' }], retry: 'safe_to_retry_if_state_hash_unchanged', idempotency: { kind: 'conditional', condition: 'Safe when source hash, settings hash, and target authority snapshot are unchanged.' }, knownLimitations: ['RuntimeSessionFacade may fail closed with operation_unimplemented until voxel conversion backend wiring lands.'],
    }),
    base({
        id: 'voxel_conversion.preview', label: 'Preview Voxel Conversion', summary: 'Preview bounded conversion output from a previously planned voxel conversion without mutating authority.', category: 'preview', menuPath: ['Voxel Conversion', 'Preview'], keywords: ['voxel', 'conversion', 'preview', 'sample'],
        inputSchema: voxelConversionPreviewInput, outputSchema: voxelConversionPreviewOutput, operationClass: 'read_only', stateImpact: readAuthority, runtimeRequirements: [runtimeSession('previewVoxelConversion')], artifacts: [artifact('voxel_conversion_preview', 'Output hash, sampled voxels, bounds, diagnostics, and evidence refs.')], typedInputExample: { sessionId: 'session-1', request: voxelConversionPreviewRequestExample }, typedOutputExample: { preview: voxelConversionPreviewExample }, panel: 'viewport', dialog: 'simple_form', inputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelConversionPreviewRequest' }], outputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelConversionPreview' }], retry: 'safe_to_retry_if_state_hash_unchanged', idempotency: { kind: 'conditional', condition: 'Safe when plan id and expected plan hash still match authority state.' }, knownLimitations: ['Preview is Rust/runtime readback metadata; Studio must not voxelize meshes locally or use renderer buffers as authority.'],
    }),
    base({
        id: 'voxel_conversion.apply', label: 'Apply Voxel Conversion', summary: 'Apply a planned voxel conversion through RuntimeSession authority with receipt and replay-oriented hashes.', category: 'authority_edit', menuPath: ['Voxel Conversion', 'Apply'], keywords: ['voxel', 'conversion', 'apply', 'authority'],
        inputSchema: voxelConversionApplyInput, outputSchema: voxelConversionApplyOutput, operationClass: 'authority_mutating', stateImpact: mutateAuthority, runtimeRequirements: [runtimeSession('applyVoxelConversion')], artifacts: [artifact('voxel_conversion_receipt', 'Apply receipt with accepted output hash/count, diagnostics, and evidence refs.')], typedInputExample: { sessionId: 'session-1', request: { planId: voxelConversionPlanExample.planId, expectedPlanHash: voxelConversionPlanExample.planHash, expectedPreviewHash: voxelConversionPreviewExample.outputHash } }, typedOutputExample: { receipt: voxelConversionReceiptExample }, panel: 'timeline', dialog: 'advanced_form', inputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelConversionApplyRequest' }], outputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelConversionReceipt' }], agentExposure: { kind: 'authority_mutating', requiresPreview: true, batchable: false }, undo: { kind: 'not_undoable', reason: 'Conversion apply must be reversed by an explicit future authority command or snapshot restore, not registry-local metadata.' }, retry: 'safe_to_retry_if_state_hash_unchanged', idempotency: { kind: 'conditional', condition: 'Safe only when expected plan/preview hashes still match and the conversion has not already committed.' }, knownLimitations: ['RuntimeSessionFacade may fail closed with operation_unimplemented until voxel conversion backend wiring lands.'],
    }),
    base({
        id: 'voxel_conversion.export_evidence', label: 'Export Voxel Conversion Evidence', summary: 'Export selected voxel conversion evidence refs for Studio timeline and review packets.', category: 'export', menuPath: ['Voxel Conversion', 'Export Evidence'], keywords: ['voxel', 'conversion', 'evidence', 'export'],
        inputSchema: voxelConversionEvidenceExportInput, outputSchema: voxelConversionEvidenceExportOutput, operationClass: 'diagnostic_export', stateImpact: { authority: 'read', editor: 'read', render: 'none', workspace: 'write' }, runtimeRequirements: [runtimeSession('exportVoxelConversionEvidence'), { kind: 'artifact_writer' }], artifacts: [artifact('voxel_conversion_evidence', 'Exported conversion evidence refs and content hashes.')], typedInputExample: { sessionId: 'session-1', evidence: [voxelConversionEvidenceExample] }, typedOutputExample: { evidence: [voxelConversionEvidenceExample] }, panel: 'export', dialog: 'simple_form', inputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelConversionEvidenceRef' }], outputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelConversionEvidenceRef' }], agentExposure: { kind: 'diagnostic_export' }, retry: 'safe_to_retry', idempotency: { kind: 'idempotent', keyFields: ['sessionId', 'evidence'] }, knownLimitations: ['Evidence export only records authority-produced refs; it must not synthesize conversion success or inspect private renderer/native buffers.'],
    }),
    base({
        id: 'scene.load_asset', label: 'Load Asset Into Scene', summary: 'Load a catalog asset into the active scene as a placed renderable using public retained-mode render-diff evidence without authority mutation.', category: 'scene', menuPath: ['Scene', 'Load Asset'], keywords: ['scene', 'load', 'asset', 'place', 'render diff'],
        inputSchema: loadSceneAssetInput, outputSchema: loadSceneAssetOutput, operationClass: 'editor_local', stateImpact: { authority: 'read', editor: 'mutate', render: 'capture', workspace: 'none' }, runtimeRequirements: [runtime('read_model_material_preview'), { kind: 'editor_store' }, { kind: 'render_surface' }], artifacts: [artifact('model_metadata', 'Loaded static mesh asset metadata.'), artifact('material_metadata', 'Bound material projection metadata.'), artifact('render_diff_preview', 'Retained-mode render diff that defines and places the loaded asset.')], typedInputExample: { sessionId: 'session-1', assetId: 'mesh.preview-cube', materialId: 'material.copper', placement: { translation: [1, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] } }, typedOutputExample: { assetId: 'mesh.preview-cube', renderableIds: ['scene-asset:mesh.preview-cube:0'], loadDiff: loadSceneAssetDiffExample, rendererClassification: 'reference_placement', diagnostics: [] }, panel: 'viewport', dialog: 'simple_form', outputContractRefs: [{ package: '@asha/contracts', exportName: 'RenderFrameDiff' }], agentExposure: { kind: 'editor_local' }, undo: { kind: 'editor_local', inverseData: ['previous loaded scene asset snapshot'] }, idempotency: { kind: 'conditional', condition: 'Same asset id, material id, placement, and catalog versions produce the same load diff.' }, knownLimitations: ['Load evidence is render-diff/readback metadata only; screenshots, hardware GPU, and performance evidence are out of scope.', 'Asset placement is browser/reference projection; Rust/WASM authority placement remains deferred until the runtime bridge is approved.'],
    }),
    base({
        id: 'scene.read_object_snapshot', label: 'Read Scene Object Snapshot', summary: 'Read the canonical scene-object hierarchy snapshot from public Rust authority/protocol DTOs.', category: 'scene', menuPath: ['Scene', 'Read Object Snapshot'], keywords: ['scene', 'hierarchy', 'object', 'snapshot'],
        inputSchema: sessionIdInput, outputSchema: sceneObjectSnapshotOutput, operationClass: 'read_only', stateImpact: readAuthority, runtimeRequirements: [runtime('read_scene_object_snapshot')], artifacts: [artifact('editor_state', 'Canonical scene-object hierarchy snapshot.')], typedInputExample: { sessionId: 'session-1' }, typedOutputExample: { snapshot: sceneObjectSnapshotExample }, panel: 'inspector', dialog: 'readout_only', outputContractRefs: [{ package: '@asha/contracts', exportName: 'SceneObjectSnapshot' }], knownLimitations: ['Mock/runtime bridge snapshot is a deterministic public contract surface; native authority wiring may fail closed until read_scene_object_snapshot is wired.'],
    }),
    base({
        id: 'scene.apply_object_command', label: 'Apply Scene Object Command', summary: 'Apply one typed scene-object hierarchy command with expected snapshot hash validation and classified rejection output.', category: 'scene', menuPath: ['Scene', 'Apply Object Command'], keywords: ['scene', 'hierarchy', 'rename', 'reparent', 'authority'],
        inputSchema: sceneObjectCommandInput, outputSchema: sceneObjectCommandOutput, operationClass: 'authority_mutating', stateImpact: mutateAuthority, runtimeRequirements: [runtime('apply_scene_object_command'), runtime('read_scene_object_snapshot')], artifacts: [artifact('command_result', 'Accepted/rejected scene-object command result with hierarchy snapshot evidence.')], typedInputExample: { sessionId: 'session-1', request: sceneObjectCommandRequestExample }, typedOutputExample: { result: sceneObjectCommandResultExample }, panel: 'timeline', dialog: 'simple_form', inputContractRefs: [{ package: '@asha/contracts', exportName: 'SceneObjectCommandRequest' }], outputContractRefs: [{ package: '@asha/contracts', exportName: 'SceneObjectCommandResult' }], agentExposure: { kind: 'authority_mutating', requiresPreview: false, batchable: false }, undo: { kind: 'authority_reversing', inverseCommandRefs: ['scene.apply_object_command'], requiresSameStateHash: true }, retry: 'safe_to_retry_if_state_hash_unchanged', idempotency: { kind: 'conditional', condition: 'Safe when expectedDocumentHash still matches and the command has not already committed.' }, knownLimitations: ['Native runtime bridge may fail closed until apply_scene_object_command is wired; no private UI-only mutation path is declared.'],
    }),
    base({
        id: 'selection.voxel_from_screen_point', label: 'Select Voxel From Screen Point', summary: 'Project a screen point through public camera evidence into typed ASHA voxel selection evidence.', category: 'selection', menuPath: ['Select', 'Voxel From Screen Point'], keywords: ['screen point', 'pick', 'select', 'voxel'],
        inputSchema: screenPointInput, outputSchema: voxelSelectionOutput, operationClass: 'editor_local', stateImpact: mutateEditor, runtimeRequirements: [runtime('select_voxel'), { kind: 'editor_store' }], artifacts: [artifact('selection_snapshot', 'Selected voxel hit or no-hit result.')], typedInputExample: { sessionId: 'session-1', request: { camera: CAMERA_HANDLE, grid: 0, viewport: null, screenPoint: { x: 0.5, y: 0.5, space: 'normalized_0_1' }, maxDistance: 128 } }, typedOutputExample: { selection: selectionExample }, panel: 'viewport', dialog: 'none', inputContractRefs: [{ package: '@asha/contracts', exportName: 'ScreenPointToPickRayRequest' }], outputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelSelectionSnapshot' }], agentExposure: { kind: 'editor_local' }, undo: { kind: 'editor_local', inverseData: ['previous selection snapshot'] }, idempotency: { kind: 'conditional', condition: 'Same screen point, camera, viewport, and unchanged projection evidence selects the same voxel.' },
    }),
    base({
        id: 'selection.set_active_entity', label: 'Set Active Entity', summary: 'Select an entity/renderable by id from the hierarchy/entity browser and sync the editor selection to the viewport.', category: 'selection', menuPath: ['Select', 'Active Entity'], keywords: ['select', 'entity', 'hierarchy', 'browser'],
        inputSchema: setActiveEntityInput, outputSchema: setActiveEntityOutput, operationClass: 'editor_local', stateImpact: mutateEditor, compatibility: REGISTRY_COMPAT, runtimeRequirements: [{ kind: 'editor_store' }], artifacts: [artifact('selection_snapshot', 'Selected entity/renderable selection evidence.')], typedInputExample: { sessionId: 'session-1', entityId: 'selected-voxel:0,0,0' }, typedOutputExample: { entityId: 'selected-voxel:0,0,0', renderableId: 'selected-voxel:0,0,0', selectionHash: 'selection-hash', selected: true }, panel: 'inspector', dialog: 'none', agentExposure: { kind: 'editor_local' }, undo: { kind: 'editor_local', inverseData: ['previous active entity selection'] }, idempotency: { kind: 'conditional', condition: 'Selecting the same entity id with the same scene readback yields the same selection.' },
    }),
    base({
        id: 'entity.set_name', label: 'Set Entity Name', summary: 'Rename the selected entity from the inspector through an editor-local typed command and sync the new display name to the viewport readback.', category: 'entity', menuPath: ['Inspect', 'Rename Entity'], keywords: ['rename', 'entity', 'name', 'inspector', 'edit'],
        inputSchema: setEntityNameInput, outputSchema: setEntityNameOutput, operationClass: 'editor_local', stateImpact: mutateEditor, compatibility: REGISTRY_COMPAT, runtimeRequirements: [{ kind: 'editor_store' }], artifacts: [artifact('editor_state', 'Editor-local selected-entity name edit evidence.')], typedInputExample: { sessionId: 'session-1', entityId: 'selected-voxel:0,0,0', name: 'Primary voxel' }, typedOutputExample: { entityId: 'selected-voxel:0,0,0', renderableId: 'selected-voxel:0,0,0', name: 'Primary voxel', nameHash: 'name-hash', applied: true }, panel: 'inspector', dialog: 'simple_form', agentExposure: { kind: 'editor_local' }, undo: { kind: 'editor_local', inverseData: ['previous selected-entity display name'] }, idempotency: { kind: 'conditional', condition: 'Renaming the same entity id to the same name with the same scene readback yields the same edit.' }, knownLimitations: ['Name is an editor-local display field over public scene-view readback; it is not an authoritative ECS/runtime mutation until the runtime bridge is approved.'],
    }),
    base({
        id: 'transform.translate_entity', label: 'Translate Entity', summary: 'Translate the selected entity along a single world axis from the viewport transform gizmo through an editor-local typed command, recording preview and apply modes on the shared timeline.', category: 'entity', menuPath: ['Transform', 'Translate Along Axis'], keywords: ['transform', 'translate', 'gizmo', 'move', 'axis', 'entity'],
        inputSchema: translateEntityInput, outputSchema: translateEntityOutput, operationClass: 'editor_local', stateImpact: mutateEditor, compatibility: REGISTRY_COMPAT, runtimeRequirements: [{ kind: 'editor_store' }], artifacts: [artifact('editor_state', 'Editor-local selected-entity transform translate evidence.')], typedInputExample: { sessionId: 'session-1', entityId: 'selected-voxel:0,0,0', axis: 'x', delta: 2, mode: 'apply' }, typedOutputExample: { entityId: 'selected-voxel:0,0,0', renderableId: 'selected-voxel:0,0,0', axis: 'x', delta: 2, mode: 'apply', translationBefore: [0.5, 0.5, 0.5], translationAfter: [2.5, 0.5, 0.5], transformHash: 'transform-hash', applied: true }, panel: 'viewport', dialog: 'simple_form', agentExposure: { kind: 'editor_local' }, undo: { kind: 'editor_local', inverseData: ['previous selected-entity transform'] }, idempotency: { kind: 'conditional', condition: 'Translating the same entity id along the same axis by the same delta and mode with the same scene readback yields the same transform.' }, knownLimitations: ['Translate is an editor-local transform over public scene-view readback; it does not claim physics, native runtime, or performance evidence, and authoritative/runtime transform mutation remains deferred until the runtime bridge is approved.'],
    }),
    base({
        id: 'inspection.voxel', label: 'Inspect Voxel', summary: 'Read typed voxel/material state for one public coordinate.', category: 'inspection', menuPath: ['Inspect', 'Voxel'], keywords: ['voxel', 'inspect'],
        inputSchema: voxelInspectionInput, outputSchema: voxelInspectionOutput, operationClass: 'read_only', stateImpact: readAuthority, runtimeRequirements: [runtime('read_voxel_mesh_evidence')], artifacts: [artifact('voxel_inspection', 'Voxel occupancy/material readout.')], typedInputExample: { sessionId: 'session-1', voxel: { x: 0, y: 0, z: 0 } }, typedOutputExample: { voxel: { x: 0, y: 0, z: 0 }, materialId: null, occupied: false }, panel: 'inspector', dialog: 'readout_only', inputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelCoord' }], outputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelCoord' }],
    }),
    base({
        id: 'preview.voxel_brush', label: 'Preview Voxel Brush', summary: 'Preview a typed voxel edit without mutating authority.', category: 'preview', menuPath: ['Edit', 'Preview Voxel Brush'], keywords: ['preview', 'brush', 'voxel'],
        inputSchema: previewInput, outputSchema: previewOutput, operationClass: 'editor_local', stateImpact: mutateEditor, runtimeRequirements: [{ kind: 'editor_store' }], artifacts: [artifact('voxel_preview', 'Editor-local target voxel preview.')], typedInputExample: { sessionId: 'session-1', anchor: { x: 0, y: 0, z: 0 }, commands: [{ op: 'setVoxel', grid: 0, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } }] }, typedOutputExample: { targetVoxels: [{ x: 0, y: 0, z: 0 }], previewVersion: 'preview.v0' }, panel: 'viewport', dialog: 'simple_form', inputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelCoord' }, { package: '@asha/contracts', exportName: 'VoxelCommand' }], outputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelCoord' }], agentExposure: { kind: 'editor_local' }, undo: { kind: 'editor_local', inverseData: ['previous preview snapshot'] }, idempotency: { kind: 'idempotent', keyFields: ['sessionId', 'anchor', 'commands'] }, compatibility: REGISTRY_COMPAT,
    }),
    base({
        id: 'authority.voxel.apply_brush', label: 'Apply Voxel Brush', summary: 'Apply typed voxel commands through ASHA authority validation.', category: 'authority_edit', menuPath: ['Edit', 'Apply Voxel Brush'], keywords: ['apply', 'voxel', 'authority'],
        inputSchema: applyInput, outputSchema: applyOutput, operationClass: 'authority_mutating', stateImpact: mutateAuthority, runtimeRequirements: [runtime('submit_commands'), runtime('read_voxel_mesh_evidence')], artifacts: [artifact('command_result', 'Accepted/rejected authority command result with state hash evidence.')], typedInputExample: { sessionId: 'session-1', commands: [{ op: 'setVoxel', grid: 0, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } }], expectedStateHash: null }, typedOutputExample: { accepted: true, authorityBeforeHash: null, authorityAfterHash: null }, panel: 'timeline', dialog: 'advanced_form', inputContractRefs: [{ package: '@asha/contracts', exportName: 'VoxelCommand' }], agentExposure: { kind: 'authority_mutating', requiresPreview: true, batchable: true }, undo: { kind: 'authority_reversing', inverseCommandRefs: [], requiresSameStateHash: true }, retry: 'safe_to_retry_if_state_hash_unchanged', idempotency: { kind: 'conditional', condition: 'Safe when expectedStateHash still matches and command sequence id has not committed.' }, knownLimitations: ['V1 records reversal posture but does not declare a generic authority undo stack.'],
    }),
    base({
        id: 'inspection.last_command_result', label: 'Inspect Last Command Result', summary: 'Read the last timeline command result for human/agent correlation.', category: 'inspection', menuPath: ['Inspect', 'Last Command Result'], keywords: ['timeline', 'result'],
        inputSchema: sessionIdInput, outputSchema: lastCommandResultOutput, operationClass: 'read_only', stateImpact: readEditor, runtimeRequirements: [{ kind: 'editor_store' }], artifacts: [artifact('command_result', 'Last known command result reference.', false)], typedInputExample: { sessionId: 'session-1' }, typedOutputExample: { sequenceId: null, status: null }, panel: 'timeline', dialog: 'readout_only', compatibility: REGISTRY_COMPAT,
    }),
    base({
        id: 'render.capture_before_after', label: 'Capture Before/After Evidence', summary: 'Capture/render before-after evidence as non-authoritative artifacts.', category: 'render_evidence', menuPath: ['Evidence', 'Capture Before/After'], keywords: ['capture', 'evidence', 'render'],
        inputSchema: captureInput, outputSchema: captureOutput, operationClass: 'render_evidence', stateImpact: captureRender, runtimeRequirements: [runtime('read_render_diffs'), { kind: 'render_surface' }, { kind: 'artifact_writer' }], artifacts: [artifact('render_before_after', 'Before/after visual evidence artifact.')], typedInputExample: { sessionId: 'session-1', beforeArtifactId: 'artifact-before', afterArtifactId: 'artifact-after' }, typedOutputExample: { artifactId: 'artifact-before-after', renderBeforeHash: null, renderAfterHash: null }, panel: 'evidence', dialog: 'simple_form', agentExposure: { kind: 'render_evidence' }, retry: 'retry_after_status_readback', idempotency: { kind: 'conditional', condition: 'Safe after reading current render/artifact status.' }, knownLimitations: ['Render screenshots are evidence only and never authority.'],
    }),
    base({
        id: 'export.agent_readout', label: 'Export Agent Readout', summary: 'Export command timeline, compatibility, diagnostics, and artifact refs for review.', category: 'export', menuPath: ['Export', 'Agent Readout'], keywords: ['export', 'agent', 'review'],
        inputSchema: exportInput, outputSchema: exportOutput, operationClass: 'diagnostic_export', stateImpact: writeWorkspace, runtimeRequirements: [{ kind: 'artifact_writer' }], artifacts: [artifact('agent_readout', 'Human/agent review artifact index.')], typedInputExample: { sessionId: 'session-1', includeVisualEvidence: true }, typedOutputExample: { artifactId: 'agent-readout', commandCount: 0 }, panel: 'export', dialog: 'simple_form', agentExposure: { kind: 'diagnostic_export' }, retry: 'safe_to_retry', idempotency: { kind: 'idempotent', keyFields: ['sessionId', 'includeVisualEvidence'] }, compatibility: REGISTRY_COMPAT,
    }),
];
export const COMMAND_IDS = COMMAND_MANIFEST.map((command) => command.id);
//# sourceMappingURL=manifest.js.map