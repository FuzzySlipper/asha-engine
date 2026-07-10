// Native facade parity / fail-closed conformance (task #2423).
//
// Proves the seam closed in this task: a *loaded* native facade either executes a
// real native implementation or throws a classified `operation_unimplemented`
// error for every manifest operation. It must NEVER silently inherit mock /
// reference behaviour for an unwired op (the prior `extends MockRuntimeBridge`
// hazard). We inject a fake addon so the test runs without a built `.node` binary.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { entityId } from '@asha/contracts';
import { MANIFEST_OPERATIONS, NATIVE_WIRED_OPERATIONS, NativeRuntimeBridge, RuntimeBridgeError, frameCursor, } from './index.js';
import { fpsLoadRequest } from './native-fps-fixtures.test-fixture.js';
const MODEL_MATERIAL_PREVIEW_REQUEST = {
    catalogEntry: {
        id: 'material.copper',
        kind: 'material',
        version: 1,
        hash: 'sha256-material-copper',
        sourcePath: null,
        label: 'Copper',
        dependencies: [],
        material: {
            render: { color: { r: 0.8, g: 0.4, b: 0.2, a: 1 }, texture: null, roughness: 0.6, emissive: 0, uvStrategy: 'flat' },
            collision: { solid: true, collidable: true, occludes: true, structuralClass: 'solid' },
        },
    },
    meshAsset: {
        asset: 'mesh.preview-cube',
        payload: {
            layout: { vertexCount: 8, indexCount: 36, indexWidth: 'u32', attributes: [{ name: 'position', components: 3, kind: 'f32' }] },
            groups: [{ materialSlot: 0, start: 0, count: 36 }],
            bounds: { min: [-0.5, -0.5, -0.5], max: [0.5, 0.5, 0.5] },
            source: { kind: 'inline', positions: [], normals: [], indices: [] },
            provenance: 'staticAsset',
        },
        materialSlots: [{ slot: 0, material: 'material.copper' }],
        collision: { kind: 'aabbFallback' },
    },
    instanceHandle: 7001,
};
const CAMERA_CREATE_REQUEST = {
    initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
    viewport: { width: 1280, height: 720 },
};
const CAMERA_INPUT = {
    camera: 1,
    tick: 1,
    input: {
        moveForward: 1,
        moveRight: 0,
        moveUp: 0,
        yawDeltaDegrees: 15,
        pitchDeltaDegrees: -5,
        dtSeconds: 1 / 60,
        moveSpeedUnitsPerSecond: 3,
    },
};
const COLLISION_CAMERA_INPUT = {
    ...CAMERA_INPUT,
    grid: 1,
    shape: { halfExtents: [0.2, 0.2, 0.2] },
    policy: { mode: 'axis_separable_slide', maxIterations: 3 },
};
const REQUIRED_NATIVE_CONFORMANCE_OPS = [
    'initialize_engine',
    'load_project_bundle',
    'submit_commands',
    'step_simulation',
    'create_camera',
    'apply_collision_constrained_camera_input',
    'apply_enemy_direct_nav_movement',
    'load_fps_runtime_session',
    'read_fps_runtime_session',
    'apply_fps_primary_fire',
    'invoke_game_extension_weapon_effect',
    'validate_game_rule_catalog',
    'submit_game_rule_effect_intent',
    'read_game_rule_runtime_readout',
    'restart_fps_runtime_session',
    'read_fps_encounter_director',
    'apply_fps_encounter_transition',
    'plan_voxel_conversion',
    'register_voxel_conversion_source',
    'register_voxel_conversion_mesh_asset',
    'read_voxel_conversion_source_metadata',
    'preview_voxel_conversion',
    'apply_voxel_conversion',
    'export_voxel_conversion_evidence',
    'read_voxel_model_info',
    'read_voxel_model_window',
    'export_voxel_volume_asset',
    'save_voxel_volume_asset',
    'load_voxel_volume_asset',
    'validate_voxel_annotation_layer',
    'load_voxel_annotation_layer',
    'read_voxel_annotation_query',
    'apply_voxel_annotation_edit',
    'export_voxel_annotation_layer',
    'read_voxel_edit_history',
    'preview_voxel_edit_revert',
    'apply_voxel_edit_revert',
    'undo_voxel_edit',
    'redo_voxel_edit',
    'read_render_diffs',
    'save_project_bundle',
    'get_project_bundle_composition_status',
];
const HASH_A = 'fnv1a64:00000000000000aa';
const HASH_B = 'fnv1a64:00000000000000bb';
const HASH_C = 'fnv1a64:00000000000000cc';
const VOXEL_PLAN_HASH = 'fnv1a64:0000000000000102';
const VOXEL_PREVIEW_HASH = 'fnv1a64:0000000000000103';
const GAME_RULE_CATALOG = {
    catalog: { catalogId: 'catalog.game-rules.native', version: '0.1.0', contentHash: HASH_A },
    valueChannels: [{ channelId: 'value.health', displayName: 'Health' }],
    bundles: [{
            bundleId: 'bundle.poisoned-impact',
            effectOps: [
                { kind: 'applyDelta', opId: 'op.impact-damage', channelId: 'value.health', amount: -3, tags: ['tag.impact'] },
                {
                    kind: 'schedulePeriodicEffect',
                    opId: 'op.schedule-poison',
                    modifierId: 'modifier.poison',
                    cadence: { periodTicks: 2 },
                    duration: { kind: 'ticks', ticks: 6 },
                    tags: ['tag.poison'],
                },
            ],
            modifiers: [{
                    modifierId: 'modifier.poison',
                    stackPolicy: { kind: 'refresh' },
                    duration: { kind: 'ticks', ticks: 6 },
                    tickCadence: { periodTicks: 2 },
                    tags: ['tag.poison'],
                    effectOpIds: ['op.poison-tick'],
                    sourceHash: HASH_B,
                }],
            tags: ['tag.poison'],
            sourceHash: HASH_C,
        }],
};
const GAME_RULE_REQUEST = {
    catalog: GAME_RULE_CATALOG.catalog,
    bundleId: 'bundle.poisoned-impact',
    source: entityId(101),
    target: entityId(777),
    values: [{ channelId: 'value.health', min: 0, current: 75, max: 75 }],
    tick: 9,
};
const VOXEL_CONVERSION_PLAN_REQUEST = {
    source: {
        assetId: 'mesh/quad',
        assetKind: 'mesh',
        assetVersion: 1,
        sourceHash: 'sha256:quad',
        meshPrimitive: null,
    },
    target: {
        grid: 1,
        volumeAssetId: 'voxel/generated',
        origin: { x: 0, y: 0, z: 0 },
    },
    settings: {
        mode: 'surface',
        fitPolicy: 'contain',
        originPolicy: 'target_min',
        resolution: [4, 4, 1],
        voxelSize: 1,
        maxOutputVoxels: 16,
        transform: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
        materialMap: {
            entries: [{ sourceMaterialSlot: 0, sourceMaterialId: 'mat/a', voxelMaterial: 3 }],
            textureAssets: [],
            textureBindings: [],
            defaultVoxelMaterial: 3,
        },
    },
};
const VOXEL_CONVERSION_SOURCE_REGISTRATION_REQUEST = {
    source: {
        assetId: 'mesh/native-registered-triangle',
        assetKind: 'mesh',
        assetVersion: 2,
        sourceHash: 'sha256:native-registered-triangle',
        meshPrimitive: 'default',
    },
    positions: [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
    triangles: [{ indices: [0, 1, 2], sourceMaterialSlot: 0 }],
    materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'mat/a' }],
};
const VOXEL_CONVERSION_MESH_ASSET_REGISTRATION_REQUEST = {
    source: VOXEL_CONVERSION_PLAN_REQUEST.source,
    meshAsset: {
        assetId: 'mesh/quad',
        sourcePath: 'assets/mesh/quad.mesh.json',
        positions: [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
        normals: [],
        indices: [0, 1, 2],
        groups: [{ materialSlot: 0, start: 0, count: 3 }],
        materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'mat/a' }],
    },
};
const VOXEL_CONVERSION_EVIDENCE = [
    {
        kind: 'plan',
        uri: 'asha://voxel-conversion/plan/fnv1a64:0000000000000101',
        contentHash: VOXEL_PLAN_HASH,
    },
];
const VOXEL_MODEL_INFO_REQUEST = {
    grid: 1,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
};
const VOXEL_MODEL_WINDOW_REQUEST = {
    grid: 1,
    volumeAssetId: 'voxel/generated',
    bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
    includeEmpty: false,
    materialFilter: [],
    maxSamples: 1,
};
const VOXEL_VOLUME_ASSET_EXPORT_REQUEST = {
    grid: 1,
    volumeAssetId: 'voxel/generated',
    targetAssetId: 'voxel-volume/native-export',
    label: 'Native export',
    createdBy: 'native-fail-closed-test',
    sourceTool: '@asha/runtime-bridge',
    maxSparseRuns: 16,
    expectedSessionHash: 'fnv1a64:0000000000000105',
};
const VOXEL_VOLUME_ASSET_LOAD_REQUEST = {
    asset: {
        assetId: 'voxel-volume/native-export',
        schemaVersion: 1,
        mediaType: 'application/vnd.asha.voxel-volume+json;version=1',
        grid: {
            origin: [0, 0, 0],
            cellSize: 1,
            coordinateSystem: 'y_up_right_handed',
        },
        bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
        representation: {
            kind: 'sparse_runs',
            sparseRuns: [{ start: { x: 0, y: 0, z: 0 }, length: 1, material: 3 }],
        },
        materialPalette: [{
                voxelMaterial: 3,
                paletteEntryId: 'voxel-material/surface-a',
                displayName: 'Surface A',
                materialAssetId: 'material/surface-a',
                materialCatalogBindingId: 'catalog-binding/surface-a',
            }],
        provenance: [{
                kind: 'runtime_export',
                uri: 'asha://runtime-session/voxel-volume-export/voxel-volume/native-export',
                contentHash: 'fnv1a64:0000000000000107',
            }],
        authoring: {
            label: 'Native export',
            createdBy: 'native-fail-closed-test',
            sourceTool: '@asha/runtime-bridge',
        },
        validationDiagnostics: [],
        contentHashes: {
            canonicalJson: 'fnv1a64:0000000000000108',
            voxelData: 'fnv1a64:0000000000000109',
        },
    },
    targetGrid: 1,
    targetVolumeAssetId: 'voxel/generated',
    replaceExisting: true,
    includeMaterialCounts: true,
};
const VOXEL_VOLUME_ASSET_SAVE_REQUEST = {
    exportRequest: VOXEL_VOLUME_ASSET_EXPORT_REQUEST,
    targetProjectBundle: 'asha-demo',
    targetAssetPath: 'assets/voxels/native-export.avxl.json',
    representationKind: 'sparse_runs',
    expectedExistingCanonicalJsonHash: null,
    expectedCanonicalJsonHash: 'fnv1a64:0000000000000108',
    expectedVoxelDataHash: 'fnv1a64:0000000000000109',
};
const VOXEL_ANNOTATION_LAYER = {
    layerId: 'voxel-annotation/native-fixture',
    schemaVersion: 1,
    mediaType: 'application/vnd.asha.voxel-annotation+json;version=1',
    targetVoxelVolumeAssetId: 'voxel/generated',
    targetVoxelDataHash: 'fnv1a64:0000000000000109',
    targetBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
    regions: [{
            regionId: 'region/native-room',
            label: 'Native room',
            kind: 'room',
            tags: ['fixture'],
            parentRegionId: null,
            bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
            selection: { sparseRuns: [{ start: { x: 0, y: 0, z: 0 }, length: 1 }] },
        }],
    provenance: [{
            kind: 'authored',
            uri: 'asha://runtime-bridge/native-fail-closed/annotation',
            contentHash: 'fnv1a64:0000000000000112',
        }],
    contentHashes: {
        canonicalJson: 'fnv1a64:0000000000000113',
        membershipData: 'fnv1a64:0000000000000114',
    },
    validationDiagnostics: [],
};
const VOXEL_ANNOTATION_VALIDATION_REQUEST = {
    layer: VOXEL_ANNOTATION_LAYER,
    expectedTargetVoxelVolumeAssetId: 'voxel/generated',
    expectedTargetVoxelDataHash: 'fnv1a64:0000000000000109',
    maxRegions: 16,
    maxSparseRunsPerRegion: 16,
    maxTotalAssignedCells: 16,
};
const VOXEL_ANNOTATION_LOAD_REQUEST = {
    layer: VOXEL_ANNOTATION_LAYER,
    targetGrid: 1,
    replaceExisting: true,
    expectedSessionHash: 'fnv1a64:0000000000000110',
};
const VOXEL_ANNOTATION_QUERY_REQUEST = {
    runtimeLayerId: 'runtime-annotation/voxel-annotation/native-fixture',
    layerId: VOXEL_ANNOTATION_LAYER.layerId,
    mode: 'cell',
    cell: { x: 0, y: 0, z: 0 },
    bounds: null,
    regionId: null,
    maxRegions: 4,
    expectedLayerHash: VOXEL_ANNOTATION_LAYER.contentHashes.canonicalJson,
};
const VOXEL_ANNOTATION_EDIT_REQUEST = {
    runtimeLayerId: 'runtime-annotation/voxel-annotation/native-fixture',
    layerId: VOXEL_ANNOTATION_LAYER.layerId,
    expectedLayerHash: VOXEL_ANNOTATION_LAYER.contentHashes.canonicalJson,
    operation: 'set_label',
    regionId: 'region/native-room',
    region: null,
    sparseRuns: [],
    tags: [],
    label: 'Native room edited',
    kind: null,
    parentRegionId: null,
};
const VOXEL_ANNOTATION_EXPORT_REQUEST = {
    runtimeLayerId: 'runtime-annotation/voxel-annotation/native-fixture',
    layerId: VOXEL_ANNOTATION_LAYER.layerId,
    expectedLayerHash: 'fnv1a64:0000000000000115',
    includeDiagnostics: true,
};
const VOXEL_EDIT_HISTORY_READ_REQUEST = {
    historyId: 'history/native-fixture',
    cursorId: null,
    maxEntries: 4,
    includeRedoTail: true,
    expectedHistoryHash: null,
};
const VOXEL_EDIT_HISTORY_REVERT_REQUEST = {
    historyId: 'history/native-fixture',
    mode: 'preview_revert',
    target: { transactionId: null, cursorId: 'cursor/0', cursorIndex: 0 },
    expectedHistoryHash: 'fnv1a64:history',
    expectedCursorHash: 'fnv1a64:cursor',
    maxReplaySteps: 16,
    maxDiffVoxels: 32,
    includeSampleWindow: false,
};
const VOXEL_EDIT_HISTORY_UNDO_REQUEST = {
    historyId: 'history/native-fixture',
    expectedHistoryHash: 'fnv1a64:history',
    expectedCursorHash: 'fnv1a64:cursor',
    maxReplaySteps: 16,
    maxDiffVoxels: 32,
};
const VOXEL_EDIT_HISTORY_REDO_REQUEST = {
    historyId: 'history/native-fixture',
    expectedHistoryHash: 'fnv1a64:history',
    expectedCursorHash: 'fnv1a64:cursor',
    maxReplaySteps: 16,
    maxDiffVoxels: 32,
};
function parseJsonFixture(payload) {
    return JSON.parse(payload);
}
function voxelHistoryRevertFixture(request, applied) {
    const cursor = {
        cursorId: 'cursor/native',
        cursorKind: 'applied',
        appliedTransactionId: null,
        parentCursorId: null,
        historyHash: applied ? 'fnv1a64:history-native-after' : 'fnv1a64:history-native',
        voxelStateHash: 'fnv1a64:voxel-native',
        materialCatalogHash: 'fnv1a64:materials-native',
        undoDepth: applied ? 0 : 1,
        redoDepth: applied ? 1 : 0,
        entryCount: 1,
        checkpointCount: 0,
    };
    return {
        request,
        applied,
        preview: request.mode === 'preview_revert',
        historyId: request.historyId,
        cursorBefore: cursor,
        cursorAfter: cursor,
        durableEntry: null,
        previewEvidence: null,
        diffSummary: null,
        replayHash: 'fnv1a64:replay-native',
        historyHashBefore: 'fnv1a64:history-native',
        historyHashAfter: cursor.historyHash,
        diagnostics: [],
    };
}
// A fake addon with sentinel return values distinct from MockRuntimeBridge, so a
// silent mock fallback would be observable in the wired-op assertions below.
function fakeAddon(calls = []) {
    return {
        initializeEngine: (seed) => {
            calls.push(`initialize:${seed}`);
            return seed + 100;
        },
        loadProjectBundle: (_handle, bundleSchemaVersion, protocolVersion, sceneId) => {
            calls.push(`load:${bundleSchemaVersion}:${protocolVersion}:${sceneId}`);
            return { loadedProjectBundle: sceneId + 1000, fatalCount: 0, totalCount: 0, blocksLoad: false };
        },
        submitCommands: (_handle, commandsJson) => {
            calls.push(`submit:${commandsJson}`);
            const commands = JSON.parse(commandsJson);
            return { accepted: Array.isArray(commands) ? commands.length : 0, rejected: 0, rejections: [] };
        },
        stepSimulation: (_handle, tick) => {
            calls.push(`step:${tick}`);
            return 9;
        },
        createCamera: (_handle, request) => {
            calls.push(`createCamera:${request.initialPose.position.join(',')}`);
            return {
                camera: 1,
                tick: 0,
                pose: request.initialPose,
                basis: {
                    forward: [0, 0, -1],
                    right: [1, 0, 0],
                    up: [0, 1, 0],
                },
                projection: request.projection,
                viewport: request.viewport,
            };
        },
        applyCollisionConstrainedCameraInput: (_handle, envelope) => {
            calls.push(`cameraCollision:${envelope.camera}:${envelope.grid}:${envelope.tick}`);
            const before = {
                camera: envelope.camera,
                tick: envelope.tick - 1,
                pose: CAMERA_CREATE_REQUEST.initialPose,
                basis: {
                    forward: [0, 0, -1],
                    right: [1, 0, 0],
                    up: [0, 1, 0],
                },
                projection: CAMERA_CREATE_REQUEST.projection,
                viewport: CAMERA_CREATE_REQUEST.viewport,
            };
            const attempted = {
                ...before,
                tick: envelope.tick,
                pose: { ...CAMERA_CREATE_REQUEST.initialPose, position: [0, 1.6, -0.05] },
            };
            const after = {
                ...before,
                tick: envelope.tick,
                pose: { ...CAMERA_CREATE_REQUEST.initialPose, position: [0, 1.6, -0.04] },
            };
            return {
                camera: envelope.camera,
                tick: envelope.tick,
                before,
                attempted,
                after,
                collision: {
                    grid: envelope.grid,
                    shape: envelope.shape,
                    policy: envelope.policy,
                    collided: true,
                    blockedAxes: ['z'],
                    correction: [0, 0, 0.01],
                    queriedAabb: { min: [-0.2, 1.4, -0.25], max: [0.2, 1.8, 0.15] },
                    collisionSourceHash: 'fnv1a64:sentinel-collision-source',
                    collisionProjectionHash: 'fnv1a64:sentinel-collision-projection',
                },
                movementHash: 'fnv1a64:sentinel-movement',
            };
        },
        applyEnemyDirectNavMovement: (_handle, entity, seedPosition, target, maxStepUnits) => {
            calls.push(`enemyMove:${entity}:${seedPosition.x},${seedPosition.y},${seedPosition.z}:${target.x},${target.y},${target.z}:${maxStepUnits}`);
            return {
                entity,
                authoritySource: 'rust_entity_store',
                from: seedPosition,
                target,
                nextWaypoint: { x: 2, y: 1, z: 7 },
                distanceUnits: 4.01,
                reached: false,
                pathHash: 'fnv1a64:sentinel-path',
                transformHash: 'fnv1a64:sentinel-transform',
                projectionChanged: true,
            };
        },
        loadFpsRuntimeSession: (_handle, projectBundle, definitions, gameRuleModulesJson) => {
            const gameRuleModules = parseJsonFixture(gameRuleModulesJson);
            calls.push(`fpsLoad:${projectBundle}:${definitions.length}:${gameRuleModules.length}`);
            const player = definitions[0];
            const enemy = definitions[1];
            const playerTransform = player['transform'];
            const playerWeapon = player['weapon'];
            const enemyPolicy = enemy['policyBinding'];
            assert.equal(player['stableId'], 'actor/custom-player');
            assert.equal(player['stable_id'], undefined);
            assert.equal(playerWeapon?.['weaponId'], 'weapon.custom.primary');
            assert.equal(playerWeapon?.['weapon_id'], undefined);
            assert.equal(enemyPolicy?.['policyId'], 'policy.enemy.custom.v0');
            assert.equal(enemy['policy_binding'], undefined);
            calls.push(`fpsNativeShape:${player['policyBinding'] === undefined}:${enemy['weapon'] === undefined}:${playerTransform?.translation?.x ?? 'missing'}`);
            return {
                backend: 'engine_bridge_rust',
                authoritySurface: 'runtime_session.fps.authority.v0',
                projectBundle,
                sessionEpoch: 1,
                lifecycleStatus: { state: 'active' },
                playerEntity: 101,
                enemyEntity: 777,
                health: [{ entity: 777, current: 75, max: 75 }],
                policyBindings: [{
                        entity: 777,
                        bindingId: 'binding.enemy.custom.v0',
                        policyId: 'policy.enemy.custom.v0',
                        viewKind: 'runtime_session.nav_policy_view.v0',
                        viewVersion: 'v0',
                        allowedIntents: ['runtime.intent.primary_fire.v0'],
                        runtimeMoment: 'runtime.tick.enemy_policy.v0',
                    }],
                replayRecords: [{ replayUnit: 'runtime_session.fps.bootstrap.v0', entityHash: HASH_A, healthHash: HASH_B, recordHash: HASH_C }],
                readSets: [{ viewKind: 'runtime_session.health.v0', owner: 'svc-combat', readSet: ['CombatState.health'] }],
                entityHash: HASH_A,
                healthHash: HASH_B,
                replayHash: HASH_C,
            };
        },
        readFpsRuntimeSession: (handle) => {
            void handle;
            calls.push('fpsRead');
            return {
                backend: 'engine_bridge_rust',
                authoritySurface: 'runtime_session.fps.authority.v0',
                projectBundle: 'custom-demo',
                sessionEpoch: 1,
                lifecycleStatus: { state: 'active' },
                playerEntity: 101,
                enemyEntity: 777,
                health: [{ entity: 777, current: 75, max: 75 }],
                policyBindings: [],
                replayRecords: [{ replayUnit: 'runtime_session.fps.bootstrap.v0', entityHash: HASH_A, healthHash: HASH_B, recordHash: HASH_C }],
                readSets: [],
                entityHash: HASH_A,
                healthHash: HASH_B,
                replayHash: HASH_C,
            };
        },
        applyFpsPrimaryFire: (_handle, tick, origin, direction) => {
            calls.push(`fpsFire:${tick}:${origin.x},${origin.y},${origin.z}:${direction.x},${direction.y},${direction.z}`);
            return {
                backend: 'engine_bridge_rust',
                authoritySurface: 'runtime_session.fps.primary_fire.v0',
                mutationOwner: 'rule-lifecycle + svc-combat',
                workspaceTrace: ['accepted'],
                shooter: 101,
                target: 777,
                targetHealthBefore: { current: 75, max: 75 },
                targetHealthAfter: { current: 0, max: 75 },
                lifecycleStatus: { state: 'enemy_defeated', entity: 777, tick },
                targetRenderVisible: false,
                entityHash: HASH_A,
                healthHash: HASH_B,
                replayHash: HASH_C,
            };
        },
        invokeGameExtensionWeaponEffect: (_handle, hookJson, tick, origin, direction) => {
            calls.push(`gameExtension:${tick}:${origin.x},${origin.y},${origin.z}:${direction.x},${direction.y},${direction.z}`);
            const hook = parseJsonFixture(hookJson);
            return {
                hookReceiptJson: JSON.stringify({
                    moduleRef: hook.moduleRef,
                    hookId: hook.hookId,
                    requestId: hook.requestId,
                    status: 'proposed',
                    inputHash: hook.inputHash,
                    proposal: hook.target === null
                        ? null
                        : {
                            kind: 'damageModifier',
                            proposalId: `${hook.requestId}.native`,
                            target: hook.target,
                            channelId: 'combat.primary_fire.damage',
                            amountDelta: 5,
                            tags: ['native-fixture'],
                            proposalHash: HASH_A,
                        },
                    diagnostics: [],
                    trace: [],
                    proposalHash: HASH_A,
                }),
                replayEvidenceJson: JSON.stringify({
                    moduleRef: hook.moduleRef,
                    hookId: hook.hookId,
                    requestId: hook.requestId,
                    inputHash: hook.inputHash,
                    proposalHash: HASH_A,
                    validationStatus: 'accepted',
                    eventHashes: [HASH_C],
                    rejectionHashes: [],
                    replayHash: HASH_B,
                }),
                primaryFire: {
                    backend: 'engine_bridge_rust',
                    authoritySurface: 'runtime_session.fps.primary_fire.v0',
                    mutationOwner: 'rule-lifecycle + svc-combat',
                    workspaceTrace: ['accepted extension'],
                    shooter: 101,
                    target: 777,
                    targetHealthBefore: { current: 75, max: 75 },
                    targetHealthAfter: { current: 0, max: 75 },
                    lifecycleStatus: { state: 'enemy_defeated', entity: 777, tick },
                    targetRenderVisible: false,
                    entityHash: HASH_A,
                    healthHash: HASH_B,
                    replayHash: HASH_C,
                },
            };
        },
        validateGameRuleCatalog: (_handle, catalogJson) => {
            const catalog = parseJsonFixture(catalogJson);
            calls.push(`gameRuleValidate:${catalog.catalog.catalogId}`);
            return JSON.stringify({
                accepted: true,
                catalogHash: HASH_A,
                diagnostics: [],
                trace: [{ step: 1, code: 'catalog.accepted', message: 'sentinel catalog accepted', refs: [] }],
                evidence: [{ kind: 'catalogValidation', uri: 'asha://game-rules/catalog-validation/native', contentHash: HASH_B }],
            });
        },
        submitGameRuleEffectIntent: (_handle, catalogJson, requestJson) => {
            const catalog = parseJsonFixture(catalogJson);
            const request = parseJsonFixture(requestJson);
            calls.push(`gameRuleSubmit:${catalog.catalog.catalogId}:${request.bundleId}`);
            return JSON.stringify({
                accepted: true,
                requestHash: HASH_A,
                pendingValueDeltas: [{ channelId: 'value.health', amount: -3 }],
                appliedModifiers: [{
                        modifierId: 'modifier.poison',
                        source: request.source,
                        target: request.target,
                        stacks: 1,
                        appliedTick: request.tick,
                        expiresTick: request.tick + 6,
                        nextTick: request.tick + 2,
                        sourceHash: HASH_B,
                    }],
                diagnostics: [],
                trace: [{ step: 1, code: 'resolution.accepted', message: 'sentinel effect accepted', refs: [] }],
                evidence: [{ kind: 'resolutionReceipt', uri: 'asha://game-rules/receipt/native', contentHash: HASH_C }],
                replayHash: HASH_C,
            });
        },
        readGameRuleRuntimeReadout: (_handle) => {
            void _handle;
            calls.push('gameRuleReadout');
            return JSON.stringify({
                backend: 'engine_bridge_rust',
                authoritySurface: 'runtime_session.game_rules.v0',
                activeModifiers: [{
                        modifierId: 'modifier.poison',
                        source: 101,
                        target: 777,
                        stacks: 1,
                        appliedTick: 9,
                        expiresTick: 15,
                        nextTick: 11,
                        sourceHash: HASH_B,
                    }],
                recentTrace: [{ step: 1, code: 'resolution.accepted', message: 'sentinel effect accepted', refs: [] }],
                recentReplayHashes: [HASH_C],
                latestReplayHash: HASH_C,
            });
        },
        restartFpsRuntimeSession: (_handle, expectedEpoch) => {
            calls.push(`fpsRestart:${expectedEpoch}`);
            return {
                backend: 'engine_bridge_rust',
                authoritySurface: 'runtime_session.fps.authority.v0',
                projectBundle: 'custom-demo',
                sessionEpoch: expectedEpoch + 1,
                lifecycleStatus: { state: 'active' },
                playerEntity: 101,
                enemyEntity: 777,
                health: [{ entity: 777, current: 75, max: 75 }],
                policyBindings: [],
                replayRecords: [{ replayUnit: 'runtime_session.fps.bootstrap.v0', entityHash: HASH_A, healthHash: HASH_B, recordHash: HASH_C }],
                readSets: [],
                entityHash: HASH_A,
                healthHash: HASH_B,
                replayHash: HASH_C,
            };
        },
        readFpsEncounterDirector: (_handle, lifecycle) => {
            calls.push('fpsEncounterRead');
            return {
                backend: 'engine_bridge_rust',
                authoritySurface: 'runtime_session.fps.encounter_director.v0',
                mutationOwner: 'rule-lifecycle',
                workspaceTrace: ['sentinel encounter read'],
                state: {
                    presetId: 'generated-tunnel-small-encounter',
                    status: 'pending',
                    spawnedEnemyIds: [],
                    defeatedEnemyIds: [],
                    revision: 0,
                    lastTransition: 'initialized',
                },
                lifecycle,
                readSets: [{ viewKind: 'runtime_session.encounter_director.v0', owner: 'rule-lifecycle', readSet: ['FpsRuntimeSessionState.encounter'] }],
                encounterHash: 'fnv1a64:00000000000000dd',
                replayHash: 'fnv1a64:00000000000000ee',
            };
        },
        applyFpsEncounterTransition: (_handle, request) => {
            calls.push('fpsEncounterTransition');
            return {
                backend: 'engine_bridge_rust',
                authoritySurface: 'runtime_session.fps.encounter_transition.v0',
                mutationOwner: 'rule-lifecycle',
                workspaceTrace: ['sentinel encounter transition'],
                accepted: true,
                rejectionReason: null,
                eventKind: 'runtime_encounter.activated.v0',
                state: {
                    presetId: 'generated-tunnel-small-encounter',
                    status: 'active',
                    spawnedEnemyIds: ['encounter.generated_tunnel_small.wave_1.enemy_001'],
                    defeatedEnemyIds: [],
                    revision: 1,
                    lastTransition: 'activated',
                },
                lifecycle: request.lifecycle,
                encounterHash: 'fnv1a64:00000000000000ef',
                replayHash: 'fnv1a64:00000000000000f0',
            };
        },
        readRenderDiffs: (_handle, cursor) => {
            calls.push(`render:${cursor}`);
            return { ops: [{ op: 'sentinel' }] };
        },
        saveProjectBundle: (handle) => {
            void handle;
            calls.push('save');
            return { artifactsWritten: 5, compactedEdits: 2, retainedEdits: 3 };
        },
        getProjectBundleCompositionStatus: (handle) => {
            void handle;
            calls.push('status');
            return { loadedProjectBundle: 2001, fatalCount: 0, totalCount: 0, blocksLoad: false };
        },
        planVoxelConversion: (_handle, requestJson) => {
            calls.push(`voxelPlan:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                planId: 'fnv1a64:0000000000000101',
                source: {
                    assetId: 'mesh/quad',
                    assetKind: 'mesh',
                    assetVersion: 1,
                    sourceHash: 'sha256:quad',
                    meshPrimitive: null,
                },
                target: {
                    grid: 1,
                    volumeAssetId: 'voxel/generated',
                    origin: { x: 0, y: 0, z: 0 },
                },
                settings: request.settings,
                authorityVersion: 'svc-voxel-conversion.v0',
                expectedSourceHash: 'sha256:quad',
                settingsHash: 'fnv1a64:0000000000000102',
                planHash: VOXEL_PLAN_HASH,
                estimatedOutputVoxels: 1,
                estimatedBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
                diagnostics: [],
                evidence: [{ kind: 'plan', uri: 'asha://voxel-conversion/plan/fnv1a64:0000000000000101', contentHash: 'fnv1a64:0000000000000102' }],
            });
        },
        registerVoxelConversionSource: (_handle, requestJson) => {
            calls.push(`voxelRegister:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                source: request.source,
                registered: true,
                materialSlots: request.materialSlots,
                diagnostics: [],
                evidence: [{
                        kind: 'source_snapshot',
                        uri: `asha://voxel-conversion/source/${request.source.assetId}`,
                        contentHash: request.source.sourceHash,
                    }],
            });
        },
        registerVoxelConversionMeshAsset: (_handle, requestJson) => {
            calls.push(`voxelMeshAssetRegister:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                source: request.source,
                registered: true,
                materialSlots: request.meshAsset.materialSlots,
                diagnostics: [],
                evidence: [{
                        kind: 'source_snapshot',
                        uri: `asha://voxel-conversion/source/${request.meshAsset.assetId}`,
                        contentHash: request.source.sourceHash,
                    }],
            });
        },
        readVoxelConversionSourceMetadata: (_handle, requestJson) => {
            calls.push(`voxelSourceMetadata:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                request,
                registered: true,
                source: request.source,
                sourcePath: 'assets/mesh/quad.mesh.json',
                sourceBounds: { min: [0, 0, 0], max: [1, 1, 0] },
                vertexCount: 3,
                triangleCount: 1,
                groups: [{
                        groupId: 'group:0:material-slot:0',
                        label: 'Group 0 / material slot 0',
                        materialSlot: 0,
                        start: 0,
                        count: 3,
                        bounds: { min: [0, 0, 0], max: [1, 1, 0] },
                    }],
                materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'mat/a' }],
                latestPlanId: null,
                latestPlanTransform: null,
                diagnostics: [],
                evidence: [{
                        kind: 'source_snapshot',
                        uri: `asha://voxel-conversion/source/${request.source.assetId}`,
                        contentHash: request.source.sourceHash,
                    }],
            });
        },
        previewVoxelConversion: (_handle, requestJson) => {
            calls.push(`voxelPreview:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                planId: request.planId,
                outputHash: 'fnv1a64:0000000000000103',
                outputVoxelCount: 1,
                outputBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
                sampleVoxels: [{ coord: { x: 0, y: 0, z: 0 }, material: 3 }],
                diagnostics: [],
                evidence: [{ kind: 'preview', uri: 'asha://voxel-conversion/preview/fnv1a64:0000000000000101', contentHash: 'fnv1a64:0000000000000103' }],
            });
        },
        applyVoxelConversion: (_handle, requestJson) => {
            calls.push(`voxelApply:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                planId: request.planId,
                applied: true,
                outputHash: 'fnv1a64:0000000000000103',
                outputVoxelCount: 1,
                outputBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
                diagnostics: [],
                evidence: [{ kind: 'apply_receipt', uri: 'asha://voxel-conversion/apply/fnv1a64:0000000000000101', contentHash: 'fnv1a64:0000000000000104' }],
            });
        },
        exportVoxelConversionEvidence: (_handle, evidenceJson) => {
            calls.push(`voxelEvidence:${evidenceJson}`);
            return evidenceJson;
        },
        readVoxelModelInfo: (_handle, requestJson) => {
            calls.push(`voxelModelInfo:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                request,
                resident: true,
                modelId: 'voxel-model:grid:1:volume:voxel/generated',
                volumeAssetId: 'voxel/generated',
                grid: 1,
                bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
                voxelCount: 1,
                materialCounts: [{ material: 3, voxelCount: 1 }],
                source: VOXEL_CONVERSION_PLAN_REQUEST.source,
                latestPlanId: 'fnv1a64:0000000000000101',
                latestOutputHash: VOXEL_PREVIEW_HASH,
                sessionHash: 'fnv1a64:0000000000000105',
                replayHash: 'fnv1a64:0000000000000106',
                evidence: VOXEL_CONVERSION_EVIDENCE,
                diagnostics: [],
            });
        },
        readVoxelModelWindow: (_handle, requestJson) => {
            calls.push(`voxelModelWindow:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                request,
                resident: true,
                modelId: 'voxel-model:grid:1:volume:voxel/generated',
                volumeAssetId: 'voxel/generated',
                grid: 1,
                requestedBounds: request.bounds,
                modelBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
                scannedVoxelCount: 1,
                returnedSampleCount: 1,
                samples: [{ coord: { x: 0, y: 0, z: 0 }, occupied: true, material: 3 }],
                sessionHash: 'fnv1a64:0000000000000107',
                replayHash: 'fnv1a64:0000000000000108',
                diagnostics: [],
            });
        },
        exportVoxelVolumeAsset: (_handle, requestJson) => {
            calls.push(`voxelVolumeAssetExport:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            const asset = {
                assetId: request.targetAssetId,
                schemaVersion: 1,
                mediaType: 'application/vnd.asha.voxel-volume+json;version=1',
                grid: {
                    origin: [0, 0, 0],
                    cellSize: 1,
                    coordinateSystem: 'y_up_right_handed',
                },
                bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
                representation: {
                    kind: 'sparse_runs',
                    sparseRuns: [{ start: { x: 0, y: 0, z: 0 }, length: 1, material: 3 }],
                },
                materialPalette: [{
                        voxelMaterial: 3,
                        paletteEntryId: 'voxel-material/a',
                        displayName: 'A',
                        materialAssetId: 'mat/a',
                        materialCatalogBindingId: 'catalog-binding/a',
                    }],
                provenance: [{
                        kind: 'runtime_export',
                        uri: `asha://runtime-session/voxel-volume-export/${request.targetAssetId}`,
                        contentHash: 'fnv1a64:0000000000000107',
                    }],
                authoring: {
                    label: request.label,
                    createdBy: request.createdBy,
                    sourceTool: request.sourceTool,
                },
                validationDiagnostics: [],
                contentHashes: {
                    canonicalJson: 'fnv1a64:0000000000000108',
                    voxelData: 'fnv1a64:0000000000000109',
                },
            };
            return JSON.stringify({
                request,
                exported: true,
                asset,
                canonicalJson: `${JSON.stringify(asset)}\n`,
                canonicalJsonHash: 'fnv1a64:0000000000000108',
                voxelDataHash: 'fnv1a64:0000000000000109',
                diagnostics: [],
            });
        },
        saveVoxelVolumeAsset: (_handle, requestJson) => {
            calls.push(`voxelVolumeAssetSave:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            const asset = {
                assetId: request.exportRequest.targetAssetId,
                schemaVersion: 1,
                mediaType: 'application/vnd.asha.voxel-volume+json;version=1',
                grid: {
                    origin: [0, 0, 0],
                    cellSize: 1,
                    coordinateSystem: 'y_up_right_handed',
                },
                bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
                representation: {
                    kind: 'sparse_runs',
                    sparseRuns: [{ start: { x: 0, y: 0, z: 0 }, length: 1, material: 3 }],
                },
                materialPalette: [{
                        voxelMaterial: 3,
                        paletteEntryId: 'voxel-material/a',
                        displayName: 'A',
                        materialAssetId: 'mat/a',
                        materialCatalogBindingId: 'catalog-binding/a',
                    }],
                provenance: [{
                        kind: 'runtime_export',
                        uri: `asha://runtime-session/voxel-volume-export/${request.exportRequest.targetAssetId}`,
                        contentHash: 'fnv1a64:0000000000000107',
                    }],
                authoring: {
                    label: request.exportRequest.label,
                    createdBy: request.exportRequest.createdBy,
                    sourceTool: request.exportRequest.sourceTool,
                },
                validationDiagnostics: [],
                contentHashes: {
                    canonicalJson: 'fnv1a64:0000000000000108',
                    voxelData: 'fnv1a64:0000000000000109',
                },
            };
            return JSON.stringify({
                request,
                saved: true,
                diff: {
                    projectBundle: request.targetProjectBundle,
                    assetId: asset.assetId,
                    assetPath: request.targetAssetPath,
                    operation: 'create',
                    previousCanonicalJsonHash: null,
                    nextCanonicalJsonHash: asset.contentHashes.canonicalJson,
                    nextVoxelDataHash: asset.contentHashes.voxelData,
                    representationKind: 'sparse_runs',
                    sparseRunCount: 1,
                    voxelCount: 1,
                    materialCount: 1,
                    provenanceCount: 1,
                    runtimeSessionHash: request.exportRequest.expectedSessionHash ?? 'fnv1a64:0000000000000105',
                },
                asset,
                canonicalJson: `${JSON.stringify(asset)}\n`,
                canonicalJsonHash: asset.contentHashes.canonicalJson,
                voxelDataHash: asset.contentHashes.voxelData,
                diagnostics: [],
            });
        },
        loadVoxelVolumeAsset: (_handle, requestJson) => {
            calls.push(`voxelVolumeAssetLoad:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                requestAssetId: request.asset.assetId,
                loaded: true,
                modelId: `voxel-model:grid:${request.targetGrid}:volume:${request.targetVolumeAssetId}`,
                volumeAssetId: request.targetVolumeAssetId,
                grid: request.targetGrid,
                bounds: request.asset.bounds,
                voxelCount: 1,
                materialCounts: [{ material: 3, voxelCount: 1 }],
                provenance: request.asset.provenance,
                canonicalJsonHash: request.asset.contentHashes.canonicalJson,
                voxelDataHash: request.asset.contentHashes.voxelData,
                sessionHash: 'fnv1a64:0000000000000110',
                replayHash: 'fnv1a64:0000000000000111',
                diagnostics: [],
            });
        },
        validateVoxelAnnotationLayer: (_handle, requestJson) => {
            calls.push(`voxelAnnotationValidate:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                layerId: request.layer.layerId,
                valid: true,
                canonicalJsonHash: request.layer.contentHashes.canonicalJson,
                membershipDataHash: request.layer.contentHashes.membershipData,
                regionCount: request.layer.regions.length,
                sparseRunCount: 1,
                assignedCellCount: 1,
                diagnostics: [],
            });
        },
        loadVoxelAnnotationLayer: (_handle, requestJson) => {
            calls.push(`voxelAnnotationLoad:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                requestLayerId: request.layer.layerId,
                loaded: true,
                runtimeLayerId: `runtime-annotation/${request.layer.layerId}`,
                targetVoxelVolumeAssetId: request.layer.targetVoxelVolumeAssetId,
                targetVoxelDataHash: request.layer.targetVoxelDataHash,
                regionCount: request.layer.regions.length,
                assignedCellCount: 1,
                layerHash: request.layer.contentHashes.canonicalJson,
                sessionHash: 'fnv1a64:0000000000000110',
                replayHash: 'fnv1a64:0000000000000116',
                diagnostics: [],
            });
        },
        readVoxelAnnotationQuery: (_handle, requestJson) => {
            calls.push(`voxelAnnotationQuery:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                request,
                matchedRegions: [{
                        regionId: 'region/native-room',
                        label: 'Native room',
                        kind: 'room',
                        tags: ['fixture'],
                        parentRegionId: null,
                        bounds: VOXEL_ANNOTATION_LAYER.targetBounds,
                        assignedCellCount: 1,
                    }],
                regionCount: 1,
                truncated: false,
                layerHash: request.expectedLayerHash,
                diagnostics: [],
            });
        },
        applyVoxelAnnotationEdit: (_handle, requestJson) => {
            calls.push(`voxelAnnotationEdit:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                request,
                edited: true,
                layerHashBefore: request.expectedLayerHash,
                layerHashAfter: 'fnv1a64:0000000000000115',
                regionCount: 1,
                assignedCellCount: 1,
                diagnostics: [],
                replayHash: 'fnv1a64:0000000000000117',
            });
        },
        exportVoxelAnnotationLayer: (_handle, requestJson) => {
            calls.push(`voxelAnnotationExport:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            const layer = {
                ...VOXEL_ANNOTATION_LAYER,
                contentHashes: {
                    canonicalJson: request.expectedLayerHash,
                    membershipData: VOXEL_ANNOTATION_LAYER.contentHashes.membershipData,
                },
            };
            return JSON.stringify({
                request,
                exported: true,
                layer,
                canonicalJson: `${JSON.stringify(layer)}\n`,
                canonicalJsonHash: layer.contentHashes.canonicalJson,
                membershipDataHash: layer.contentHashes.membershipData,
                diagnostics: [],
            });
        },
        readVoxelEditHistory: (_handle, requestJson) => {
            calls.push(`voxelHistoryRead:${requestJson}`);
            const request = parseJsonFixture(requestJson);
            return JSON.stringify({
                historyId: request.historyId,
                schemaVersion: 1,
                mediaType: 'application/vnd.asha.voxel-edit-history+json;version=1',
                targetGrid: 1,
                targetVoxelVolumeAssetId: 'voxel/generated',
                baseVoxelHash: 'fnv1a64:base-native',
                materialCatalogHash: 'fnv1a64:materials-native',
                cursor: voxelHistoryRevertFixture({ ...VOXEL_EDIT_HISTORY_REVERT_REQUEST, historyId: request.historyId }, false).cursorBefore,
                entries: [],
                retainedRedoTransactionIds: [],
                historyHash: 'fnv1a64:history-native',
                diagnostics: [],
            });
        },
        previewVoxelEditRevert: (_handle, requestJson) => {
            calls.push(`voxelHistoryPreview:${requestJson}`);
            return JSON.stringify(voxelHistoryRevertFixture(parseJsonFixture(requestJson), false));
        },
        applyVoxelEditRevert: (_handle, requestJson) => {
            calls.push(`voxelHistoryApply:${requestJson}`);
            return JSON.stringify(voxelHistoryRevertFixture(parseJsonFixture(requestJson), true));
        },
        undoVoxelEdit: (_handle, requestJson) => {
            calls.push(`voxelHistoryUndo:${requestJson}`);
            return JSON.stringify({
                request: parseJsonFixture(requestJson),
                receipt: voxelHistoryRevertFixture({ ...VOXEL_EDIT_HISTORY_REVERT_REQUEST, mode: 'undo' }, true),
            });
        },
        redoVoxelEdit: (_handle, requestJson) => {
            calls.push(`voxelHistoryRedo:${requestJson}`);
            return JSON.stringify({
                request: parseJsonFixture(requestJson),
                receipt: voxelHistoryRevertFixture({ ...VOXEL_EDIT_HISTORY_REVERT_REQUEST, mode: 'redo' }, true),
            });
        },
    };
}
// One invocation per facade method. The native bridge is fully initialized first
// so that wired ops exercise their happy path rather than `not_initialized`.
// Typed against the `RuntimeBridge` interface (which carries the operation
// payloads); a `NativeRuntimeBridge` instance is assignable to it.
const INVOKE = new Map([
    ['initializeEngine', (b) => b.initializeEngine({ seed: 7 })],
    ['stepSimulation', (b) => b.stepSimulation({ tick: 6 })],
    ['submitCommands', (b) => b.submitCommands({ commands: [] })],
    [
        'pickVoxel',
        (b) => b.pickVoxel({ grid: 1, origin: [0, 0, 0], direction: [1, 0, 0], maxDistance: 10 }),
    ],
    [
        'applyCollisionConstrainedCameraInput',
        (b) => b.applyCollisionConstrainedCameraInput(COLLISION_CAMERA_INPUT),
    ],
    [
        'selectVoxel',
        (b) => b.selectVoxel({
            camera: CAMERA_INPUT.camera,
            grid: 1,
            viewport: null,
            screenPoint: { x: 0.5, y: 0.5, space: 'normalized_0_1' },
            maxDistance: 10,
        }),
    ],
    ['readVoxelMeshEvidence', (b) => b.readVoxelMeshEvidence({ grid: 1, chunks: [] })],
    ['loadFpsRuntimeSession', (b) => b.loadFpsRuntimeSession(fpsLoadRequest())],
    ['readFpsRuntimeSession', (b) => b.readFpsRuntimeSession()],
    ['applyFpsPrimaryFire', (b) => b.applyFpsPrimaryFire({ tick: 9, origin: [2.5, 1.5, 1.5], direction: [0, 0, 1] })],
    ['invokeGameExtensionWeaponEffect', (b) => b.invokeGameExtensionWeaponEffect({
            hook: {
                moduleRef: {
                    moduleId: 'asha.reference.primary_fire_damage_modifier',
                    version: '0.1.0',
                    contractHash: 'sha256:asha-reference-primary-fire-damage-modifier-v0',
                },
                hookId: 'weapon.primary.damage_modifier',
                requestId: 'request.native-fixture',
                tick: 9,
                source: entityId(101),
                target: entityId(777),
                baseDamage: 75,
                rangeMillimeters: 16000,
                tags: ['primary-fire'],
                inputHash: HASH_A,
            },
            primaryFire: { tick: 9, origin: [2.5, 1.5, 1.5], direction: [0, 0, 1] },
        })],
    ['validateGameRuleCatalog', (b) => b.validateGameRuleCatalog(GAME_RULE_CATALOG)],
    ['submitGameRuleEffectIntent', (b) => b.submitGameRuleEffectIntent({
            catalog: GAME_RULE_CATALOG,
            request: GAME_RULE_REQUEST,
        })],
    ['readGameRuleRuntimeReadout', (b) => b.readGameRuleRuntimeReadout()],
    ['restartFpsRuntimeSession', (b) => b.restartFpsRuntimeSession({ expectedEpoch: 1 })],
    ['readFpsEncounterDirector', (b) => b.readFpsEncounterDirector({
            outcomeKind: 'in_progress',
            terminal: false,
            enemyDead: false,
            playerDead: false,
            lifecycleHash: HASH_A,
        })],
    ['applyFpsEncounterTransition', (b) => b.applyFpsEncounterTransition({
            presetId: 'generated-tunnel-small-encounter',
            action: 'activate',
            lifecycle: {
                outcomeKind: 'in_progress',
                terminal: false,
                enemyDead: false,
                playerDead: false,
                lifecycleHash: HASH_A,
            },
        })],
    ['planVoxelConversion', (b) => b.planVoxelConversion(VOXEL_CONVERSION_PLAN_REQUEST)],
    ['registerVoxelConversionSource', (b) => b.registerVoxelConversionSource(VOXEL_CONVERSION_SOURCE_REGISTRATION_REQUEST)],
    ['registerVoxelConversionMeshAsset', (b) => b.registerVoxelConversionMeshAsset(VOXEL_CONVERSION_MESH_ASSET_REGISTRATION_REQUEST)],
    ['readVoxelConversionSourceMetadata', (b) => b.readVoxelConversionSourceMetadata({
            source: VOXEL_CONVERSION_SOURCE_REGISTRATION_REQUEST.source,
        })],
    ['previewVoxelConversion', (b) => b.previewVoxelConversion({
            planId: 'fnv1a64:0000000000000101',
            expectedPlanHash: VOXEL_PLAN_HASH,
        })],
    ['applyVoxelConversion', (b) => b.applyVoxelConversion({
            planId: 'fnv1a64:0000000000000101',
            expectedPlanHash: VOXEL_PLAN_HASH,
            expectedPreviewHash: VOXEL_PREVIEW_HASH,
        })],
    ['exportVoxelConversionEvidence', (b) => b.exportVoxelConversionEvidence(VOXEL_CONVERSION_EVIDENCE)],
    ['readVoxelModelInfo', (b) => b.readVoxelModelInfo(VOXEL_MODEL_INFO_REQUEST)],
    ['readVoxelModelWindow', (b) => b.readVoxelModelWindow(VOXEL_MODEL_WINDOW_REQUEST)],
    ['exportVoxelVolumeAsset', (b) => b.exportVoxelVolumeAsset(VOXEL_VOLUME_ASSET_EXPORT_REQUEST)],
    ['saveVoxelVolumeAsset', (b) => b.saveVoxelVolumeAsset(VOXEL_VOLUME_ASSET_SAVE_REQUEST)],
    ['loadVoxelVolumeAsset', (b) => b.loadVoxelVolumeAsset(VOXEL_VOLUME_ASSET_LOAD_REQUEST)],
    ['validateVoxelAnnotationLayer', (b) => b.validateVoxelAnnotationLayer(VOXEL_ANNOTATION_VALIDATION_REQUEST)],
    ['loadVoxelAnnotationLayer', (b) => b.loadVoxelAnnotationLayer(VOXEL_ANNOTATION_LOAD_REQUEST)],
    ['readVoxelAnnotationQuery', (b) => b.readVoxelAnnotationQuery(VOXEL_ANNOTATION_QUERY_REQUEST)],
    ['applyVoxelAnnotationEdit', (b) => b.applyVoxelAnnotationEdit(VOXEL_ANNOTATION_EDIT_REQUEST)],
    ['exportVoxelAnnotationLayer', (b) => b.exportVoxelAnnotationLayer(VOXEL_ANNOTATION_EXPORT_REQUEST)],
    ['readVoxelEditHistory', (b) => b.readVoxelEditHistory(VOXEL_EDIT_HISTORY_READ_REQUEST)],
    ['previewVoxelEditRevert', (b) => b.previewVoxelEditRevert(VOXEL_EDIT_HISTORY_REVERT_REQUEST)],
    [
        'applyVoxelEditRevert',
        (b) => b.applyVoxelEditRevert({ ...VOXEL_EDIT_HISTORY_REVERT_REQUEST, mode: 'apply_revert' }),
    ],
    ['undoVoxelEdit', (b) => b.undoVoxelEdit(VOXEL_EDIT_HISTORY_UNDO_REQUEST)],
    ['redoVoxelEdit', (b) => b.redoVoxelEdit(VOXEL_EDIT_HISTORY_REDO_REQUEST)],
    ['readModelMaterialPreview', (b) => b.readModelMaterialPreview(MODEL_MATERIAL_PREVIEW_REQUEST)],
    ['readSceneObjectSnapshot', (b) => b.readSceneObjectSnapshot()],
    [
        'applySceneObjectCommand',
        (b) => b.applySceneObjectCommand({
            expectedDocumentHash: 1,
            command: { kind: 'select', id: null },
        }),
    ],
    ['readRenderDiffs', (b) => b.readRenderDiffs(frameCursor(0))],
    ['createCamera', (b) => b.createCamera(CAMERA_CREATE_REQUEST)],
    ['applyFirstPersonCameraInput', (b) => b.applyFirstPersonCameraInput(CAMERA_INPUT)],
    [
        'applyEnemyDirectNavMovement',
        (b) => b.applyEnemyDirectNavMovement({
            entity: 777,
            seedPosition: [0, 0.5, -2.6],
            target: [0, 1.62, 1.25],
            maxStepUnits: 0.35,
        }),
    ],
    ['readCameraProjection', (b) => b.readCameraProjection({ camera: CAMERA_INPUT.camera, viewport: null })],
    ['getBuffer', (b) => b.getBuffer(0)],
    ['releaseBuffer', (b) => b.releaseBuffer(0)],
    [
        'loadProjectBundle',
        (b) => b.loadProjectBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1 }),
    ],
    ['saveProjectBundle', (b) => b.saveProjectBundle()],
    ['getProjectBundleCompositionStatus', (b) => b.getProjectBundleCompositionStatus()],
    ['unloadProjectBundle', (b) => b.unloadProjectBundle()],
    ['loadReplayFixture', (b) => b.loadReplayFixture({ name: 'x', steps: 1 })],
    ['runReplayStep', (b) => b.runReplayStep(0)],
]);
void test('every manifest op has a native invocation in this test', () => {
    for (const op of MANIFEST_OPERATIONS) {
        assert.ok(INVOKE.has(op.facadeMethod), `missing invocation for ${op.facadeMethod}`);
    }
});
void test('unwired native ops fail closed with operation_unimplemented (no mock fallback)', () => {
    for (const op of MANIFEST_OPERATIONS) {
        if (NATIVE_WIRED_OPERATIONS.has(op.manifestName))
            continue;
        const invoke = INVOKE.get(op.facadeMethod);
        assert.ok(invoke, `missing invocation for ${op.facadeMethod}`);
        const bridge = new NativeRuntimeBridge(fakeAddon());
        // A fresh, initialized bridge: proves the throw is fail-closed classification,
        // not an incidental `not_initialized`.
        bridge.initializeEngine({ seed: 1 });
        assert.throws(() => invoke(bridge), (e) => e instanceof RuntimeBridgeError && e.kind === 'operation_unimplemented', `${op.manifestName} must fail closed, not inherit mock behaviour`);
    }
});
void test('required native conformance operations are declared wired', () => {
    for (const manifestName of REQUIRED_NATIVE_CONFORMANCE_OPS) {
        assert.ok(NATIVE_WIRED_OPERATIONS.has(manifestName), `${manifestName} must be wired for native authority conformance`);
    }
});
void test('native conformance sequence routes through the addon without mock fallback', () => {
    const calls = [];
    const bridge = new NativeRuntimeBridge(fakeAddon(calls));
    assert.equal(bridge.initializeEngine({ seed: 7 }), 107);
    assert.deepEqual(bridge.loadProjectBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1001 }), {
        loadedProjectBundle: 2001,
        fatalCount: 0,
        totalCount: 0,
        blocksLoad: false,
    });
    assert.deepEqual(bridge.submitCommands({
        commands: [
            { op: 'setVoxel', grid: 1, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } },
        ],
    }), { accepted: 1, rejected: 0, rejections: [] });
    assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 9 });
    assert.deepEqual(bridge.createCamera(CAMERA_CREATE_REQUEST), {
        camera: 1,
        tick: 0,
        pose: CAMERA_CREATE_REQUEST.initialPose,
        basis: {
            forward: [0, 0, -1],
            right: [1, 0, 0],
            up: [0, 1, 0],
        },
        projection: CAMERA_CREATE_REQUEST.projection,
        viewport: CAMERA_CREATE_REQUEST.viewport,
    });
    const constrainedCamera = bridge.applyCollisionConstrainedCameraInput(COLLISION_CAMERA_INPUT);
    assert.equal(constrainedCamera.camera, COLLISION_CAMERA_INPUT.camera);
    assert.equal(constrainedCamera.tick, COLLISION_CAMERA_INPUT.tick);
    assert.equal(constrainedCamera.collision.collided, true);
    assert.deepEqual(constrainedCamera.collision.blockedAxes, ['z']);
    assert.equal(constrainedCamera.movementHash, 'fnv1a64:sentinel-movement');
    assert.deepEqual(bridge.applyEnemyDirectNavMovement({
        entity: 777,
        seedPosition: [0, 0.5, -2.6],
        target: [0, 1.62, 1.25],
        maxStepUnits: 0.35,
    }), {
        entity: 777,
        authoritySource: 'rust_entity_store',
        authorityTransport: 'native_rust',
        from: [0, 0.5, -2.6],
        target: [0, 1.62, 1.25],
        nextWaypoint: [2, 1, 7],
        distanceUnits: 4.01,
        reached: false,
        pathHash: 'fnv1a64:sentinel-path',
        transformHash: 'fnv1a64:sentinel-transform',
        projectionChanged: true,
    });
    const loadedFps = bridge.loadFpsRuntimeSession(fpsLoadRequest());
    assert.equal(loadedFps.backend, 'native_rust');
    assert.equal(loadedFps.playerEntity, 101);
    assert.equal(loadedFps.enemyEntity, 777);
    assert.equal(loadedFps.replayHash, HASH_C);
    const fired = bridge.applyFpsPrimaryFire({ tick: 9, origin: [2.5, 1.5, 1.5], direction: [0, 0, 1] });
    assert.equal(fired.backend, 'native_rust');
    assert.deepEqual(fired.lifecycleStatus, { state: 'enemy_defeated', entity: 777, tick: 9 });
    assert.equal(fired.targetHealthAfter?.current, 0);
    const catalogValidation = bridge.validateGameRuleCatalog(GAME_RULE_CATALOG);
    assert.equal(catalogValidation.accepted, true);
    const gameRuleReceipt = bridge.submitGameRuleEffectIntent({
        catalog: GAME_RULE_CATALOG,
        request: GAME_RULE_REQUEST,
    });
    assert.equal(gameRuleReceipt.accepted, true);
    assert.equal(gameRuleReceipt.appliedModifiers[0]?.nextTick, 11);
    const gameRuleReadout = bridge.readGameRuleRuntimeReadout();
    assert.equal(gameRuleReadout.backend, 'native_rust');
    assert.equal(gameRuleReadout.activeModifiers[0]?.modifierId, 'modifier.poison');
    assert.equal(bridge.readFpsRuntimeSession().replayHash, HASH_C);
    assert.equal(bridge.restartFpsRuntimeSession({ expectedEpoch: 1 }).sessionEpoch, 2);
    const encounter = bridge.readFpsEncounterDirector({
        outcomeKind: 'in_progress',
        terminal: false,
        enemyDead: false,
        playerDead: false,
        lifecycleHash: HASH_A,
    });
    assert.equal(encounter.backend, 'native_rust');
    assert.equal(encounter.encounterHash, 'fnv1a64:00000000000000dd');
    const encounterTransition = bridge.applyFpsEncounterTransition({
        presetId: 'generated-tunnel-small-encounter',
        action: 'activate',
        lifecycle: {
            outcomeKind: 'in_progress',
            terminal: false,
            enemyDead: false,
            playerDead: false,
            lifecycleHash: HASH_A,
        },
    });
    assert.equal(encounterTransition.accepted, true);
    assert.equal(encounterTransition.replayHash, 'fnv1a64:00000000000000f0');
    const registration = bridge.registerVoxelConversionSource(VOXEL_CONVERSION_SOURCE_REGISTRATION_REQUEST);
    assert.equal(registration.registered, true);
    assert.equal(registration.source.assetId, 'mesh/native-registered-triangle');
    assert.equal(registration.materialSlots[0]?.sourceMaterialId, 'mat/a');
    const meshAssetRegistration = bridge.registerVoxelConversionMeshAsset(VOXEL_CONVERSION_MESH_ASSET_REGISTRATION_REQUEST);
    assert.equal(meshAssetRegistration.registered, true);
    assert.equal(meshAssetRegistration.source.assetId, 'mesh/quad');
    assert.equal(meshAssetRegistration.materialSlots[0]?.sourceMaterialId, 'mat/a');
    assert.deepEqual(bridge.readRenderDiffs(frameCursor(0)), { ops: [{ op: 'sentinel' }] });
    assert.deepEqual(bridge.saveProjectBundle(), { artifactsWritten: 5, compactedEdits: 2, retainedEdits: 3 });
    assert.deepEqual(bridge.getProjectBundleCompositionStatus(), {
        loadedProjectBundle: 2001,
        fatalCount: 0,
        totalCount: 0,
        blocksLoad: false,
    });
    assert.deepEqual(calls, [
        'initialize:7',
        'load:1:1:1001',
        'submit:[{"op":"setVoxel","grid":1,"coord":{"x":0,"y":0,"z":0},"value":{"kind":"solid","material":1}}]',
        'step:6',
        'createCamera:0,1.6,0',
        'cameraCollision:1:1:1',
        'enemyMove:777:0,0.5,-2.6:0,1.62,1.25:0.35',
        'fpsLoad:custom-demo:2:0',
        'fpsNativeShape:true:true:0',
        'fpsFire:9:2.5,1.5,1.5:0,0,1',
        'gameRuleValidate:catalog.game-rules.native',
        'gameRuleSubmit:catalog.game-rules.native:bundle.poisoned-impact',
        'gameRuleReadout',
        'fpsRead',
        'fpsRestart:1',
        'fpsEncounterRead',
        'fpsEncounterTransition',
        'voxelRegister:{"source":{"assetId":"mesh/native-registered-triangle","assetKind":"mesh","assetVersion":2,"sourceHash":"sha256:native-registered-triangle","meshPrimitive":"default"},"positions":[[0,0,0],[1,0,0],[0,1,0]],"triangles":[{"indices":[0,1,2],"sourceMaterialSlot":0}],"materialSlots":[{"sourceMaterialSlot":0,"sourceMaterialId":"mat/a"}]}',
        'voxelMeshAssetRegister:{"source":{"assetId":"mesh/quad","assetKind":"mesh","assetVersion":1,"sourceHash":"sha256:quad","meshPrimitive":null},"meshAsset":{"assetId":"mesh/quad","sourcePath":"assets/mesh/quad.mesh.json","positions":[[0,0,0],[1,0,0],[0,1,0]],"normals":[],"indices":[0,1,2],"groups":[{"materialSlot":0,"start":0,"count":3}],"materialSlots":[{"sourceMaterialSlot":0,"sourceMaterialId":"mat/a"}]}}',
        'render:0',
        'save',
        'status',
    ]);
});
void test('native facade validates numeric inputs before addon casts can wrap', () => {
    const calls = [];
    const bridge = new NativeRuntimeBridge(fakeAddon(calls));
    bridge.initializeEngine({ seed: 1 });
    assert.throws(() => bridge.loadProjectBundle({ bundleSchemaVersion: 1.5, protocolVersion: 1, sceneId: 1 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input');
    assert.throws(() => bridge.loadProjectBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: -1 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input');
    assert.throws(() => bridge.stepSimulation({ tick: -1 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input');
    assert.throws(() => bridge.readRenderDiffs(frameCursor(-1)), (e) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input');
    assert.deepEqual(calls, ['initialize:1']);
});
void test('native facade defaults omitted FPS game-rule modules before addon conversion', () => {
    const calls = [];
    const bridge = new NativeRuntimeBridge(fakeAddon(calls));
    bridge.initializeEngine({ seed: 1 });
    const request = fpsLoadRequest();
    const legacyRequest = {
        projectBundle: request.projectBundle,
        definitions: request.definitions,
    };
    const loaded = bridge.loadFpsRuntimeSession(legacyRequest);
    assert.equal(loaded.backend, 'native_rust');
    assert.equal(calls.includes('fpsLoad:custom-demo:2:0'), true);
});
void test('native addon semantic errors are reclassified into RuntimeBridgeError', () => {
    const addon = fakeAddon();
    addon.loadProjectBundle = () => {
        throw new Error('InvalidInput: unsupported bundle schema 99 / protocol 1');
    };
    const bridge = new NativeRuntimeBridge(addon);
    bridge.initializeEngine({ seed: 1 });
    assert.throws(() => bridge.loadProjectBundle({ bundleSchemaVersion: 99, protocolVersion: 1, sceneId: 1 }), (e) => e instanceof RuntimeBridgeError &&
        e.kind === 'invalid_input' &&
        e.message.includes('unsupported bundle schema 99 / protocol 1'));
});
void test('wired native ops route through the addon, not the mock', () => {
    const calls = [];
    const bridge = new NativeRuntimeBridge(fakeAddon(calls));
    // Mock would return the seed (7) and diffCount 2; the addon returns 107 / 9.
    assert.equal(bridge.initializeEngine({ seed: 7 }), 107);
    assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 9 });
    assert.deepEqual(calls, ['initialize:7', 'step:6']);
});
void test('native bridge does not extend MockRuntimeBridge (no inherited mock methods)', () => {
    // Guards against re-introducing the `extends MockRuntimeBridge` seam: every
    // own/inherited facade method must be declared on NativeRuntimeBridge itself.
    const proto = NativeRuntimeBridge.prototype;
    for (const op of MANIFEST_OPERATIONS) {
        assert.ok(Object.prototype.hasOwnProperty.call(Object.getPrototypeOf(new NativeRuntimeBridge(fakeAddon())), op.facadeMethod), `${op.facadeMethod} must be declared on NativeRuntimeBridge, not inherited`);
        assert.equal(typeof proto[op.facadeMethod], 'function');
    }
});
void test('native bridge step before init fails closed (not_initialized)', () => {
    const bridge = new NativeRuntimeBridge(fakeAddon());
    assert.throws(() => bridge.stepSimulation({ tick: 1 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized');
});
void test('wired set names are real manifest operations', () => {
    const manifestNames = new Set(MANIFEST_OPERATIONS.map((o) => o.manifestName));
    for (const name of NATIVE_WIRED_OPERATIONS) {
        assert.ok(manifestNames.has(name), `${name} in NATIVE_WIRED_OPERATIONS is not a manifest op`);
    }
});
//# sourceMappingURL=native-fail-closed.test.js.map