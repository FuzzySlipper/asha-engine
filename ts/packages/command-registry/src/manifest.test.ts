import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  COMMAND_CATALOG,
  COMMAND_IDS,
  COMMAND_MANIFEST,
  requireCatalogCommand,
  requireKnownCommand,
  validateCommandDefinition,
  validateCommandManifest,
  validateExampleAgainstSchema,
  type DraftStudioCommandDefinition,
} from './index.js';

const REQUIRED_IDS = [
  'session.list_scenarios',
  'session.start',
  'session.load_scenario',
  'workspace.open_game_manifest',
  'workspace.validate_game_manifest',
  'inspection.session_status',
  'inspection.world_summary',
  'inspection.editor_state',
  'inspection.material',
  'inspection.model',
  'preview.model_material',
  'scene.load_asset',
  'scene.read_object_snapshot',
  'scene.apply_object_command',
  'selection.voxel_from_screen_point',
  'selection.set_active_entity',
  'entity.set_name',
  'transform.translate_entity',
  'inspection.voxel',
  'preview.voxel_brush',
  'authority.voxel.apply_brush',
  'inspection.last_command_result',
  'render.capture_before_after',
  'export.agent_readout',
] as const;

test('manifest contains the V1 stable command ids in reviewable order', () => {
  assert.deepEqual(COMMAND_IDS, REQUIRED_IDS);
  assert.equal(new Set(COMMAND_IDS).size, COMMAND_IDS.length);
});

test('manifest entries include all required metadata and validate cleanly', () => {
  assert.deepEqual(validateCommandManifest(COMMAND_MANIFEST), []);
  for (const command of COMMAND_MANIFEST) {
    assert.equal(command.version, 1);
    assert.ok(command.label.length > 0);
    assert.ok(command.summary.length > 0);
    assert.ok(command.menuPath.length > 0);
    assert.ok(command.commandPalette.keywords.length > 0);
    assert.ok(command.artifacts.length > 0);
    assert.equal(command.owningLane, 'ts-command-registry');
    assert.equal(command.owningPackage, '@asha/command-registry');
    assert.equal(command.compatibility.commandRegistry, 'command-registry.v0');
  }
});

test('non-hidden agent exposure requires GUI mirror metadata', () => {
  for (const command of COMMAND_MANIFEST) {
    if (command.agentExposure.kind !== 'hidden') {
      assert.equal(command.guiMirror.required, true, command.id);
      assert.deepEqual(command.guiMirror.menuPath, command.menuPath, command.id);
      assert.ok(command.guiMirror.menuPath.length > 0, command.id);
      assert.ok(command.guiMirror.commandPaletteVisible || command.guiMirror.panel !== undefined, command.id);
      assert.ok(command.guiMirror.argumentSummary.length > 0, command.id);
      assert.ok(command.guiMirror.resultSummary.length > 0, command.id);
      assert.ok(command.guiMirror.artifactSummary.length > 0, command.id);
      assert.ok(command.label.length > 0, command.id);
      assert.ok(command.summary.length > 0, command.id);
      assert.ok(command.operationClass.length > 0, command.id);
      assert.ok(command.owningLane.length > 0, command.id);
      assert.ok(command.owningPackage.length > 0, command.id);
    }
  }
});

test('command schemas are fail-closed and contain no freeform object payloads', () => {
  for (const command of COMMAND_MANIFEST) {
    const issues = validateCommandDefinition(command).filter((issue) => issue.message.includes('allowExtraFields'));
    assert.deepEqual(issues, [], command.id);
  }
});

test('typed examples match declared input and output schemas', () => {
  for (const command of COMMAND_MANIFEST) {
    assert.deepEqual(
      validateExampleAgainstSchema(command.id, 'typedInputExample', command.typedInputExample, command.inputSchema.shape),
      [],
      `${command.id} input example`,
    );
    assert.deepEqual(
      validateExampleAgainstSchema(command.id, 'typedOutputExample', command.typedOutputExample, command.outputSchema.shape),
      [],
      `${command.id} output example`,
    );
  }
});

test('example validation rejects opaque contract payloads and malformed empty inputs', () => {
  const scenarios = requireKnownCommand('session.list_scenarios', COMMAND_MANIFEST);
  assert.deepEqual(
    validateExampleAgainstSchema(scenarios.id, 'typedInputExample', { kind: 'anything' }, scenarios.inputSchema.shape),
    [{ commandId: 'session.list_scenarios', field: 'typedInputExample', message: 'typedInputExample does not match its declared schema' }],
  );

  const select = requireKnownCommand('selection.voxel_from_screen_point', COMMAND_MANIFEST);
  assert.deepEqual(
    validateExampleAgainstSchema(select.id, 'typedInputExample', { sessionId: 'session-1', request: { ray: { origin: [0, 0, 0] } } }, select.inputSchema.shape),
    [{ commandId: 'selection.voxel_from_screen_point', field: 'typedInputExample', message: 'typedInputExample does not match its declared schema' }],
  );

  const apply = requireKnownCommand('authority.voxel.apply_brush', COMMAND_MANIFEST);
  assert.deepEqual(
    validateExampleAgainstSchema(apply.id, 'typedInputExample', { sessionId: 'session-1', commands: [{ op: 'setVoxel', grid: 0, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid' } }], expectedStateHash: null }, apply.inputSchema.shape),
    [{ commandId: 'authority.voxel.apply_brush', field: 'typedInputExample', message: 'typedInputExample does not match its declared schema' }],
  );
});

test('contract-backed selection schemas reject extra freeform fields', () => {
  const select = requireKnownCommand('selection.voxel_from_screen_point', COMMAND_MANIFEST);
  assert.deepEqual(
    validateExampleAgainstSchema(
      select.id,
      'typedInputExample',
      {
        sessionId: 'session-1',
        request: {
          camera: 1,
          grid: 0,
          viewport: null,
          screenPoint: { x: 0.5, y: 0.5, space: 'normalized_0_1', arbitrary: true },
          maxDistance: 128,
        },
      },
      select.inputSchema.shape,
    ),
    [{ commandId: 'selection.voxel_from_screen_point', field: 'typedInputExample', message: 'typedInputExample does not match its declared schema' }],
  );
});

test('mutating, writing, and capture commands are not advertised as read-only to agents', () => {
  const nonReadOnlyByImpact = COMMAND_MANIFEST.filter(
    (command) => command.operationClass !== 'read_only' || command.stateImpact.authority === 'mutate' || command.stateImpact.editor === 'mutate' || command.stateImpact.render === 'capture' || command.stateImpact.workspace === 'write',
  );
  assert.ok(nonReadOnlyByImpact.length > 0);
  for (const command of nonReadOnlyByImpact) {
    assert.notEqual(command.agentExposure.kind, 'read_only', command.id);
  }
  assert.equal(requireKnownCommand('session.start', COMMAND_MANIFEST).agentExposure.kind, 'workspace_io');
  assert.equal(requireKnownCommand('session.load_scenario', COMMAND_MANIFEST).agentExposure.kind, 'workspace_io');
});

test('model/material commands use public contract DTOs and runtime readback classification', () => {
  const material = requireKnownCommand('inspection.material', COMMAND_MANIFEST);
  assert.deepEqual(material.outputContractRefs.map((ref) => ref.exportName), ['CatalogEntry', 'MaterialProjection']);
  assert.deepEqual(material.runtimeRequirements, [{ kind: 'runtime_bridge_operation', operation: 'read_model_material_preview' }]);

  const model = requireKnownCommand('inspection.model', COMMAND_MANIFEST);
  assert.deepEqual(model.outputContractRefs.map((ref) => ref.exportName), ['StaticMeshAsset']);
  assert.deepEqual(model.runtimeRequirements, [{ kind: 'runtime_bridge_operation', operation: 'read_model_material_preview' }]);

  const preview = requireKnownCommand('preview.model_material', COMMAND_MANIFEST);
  assert.equal(preview.operationClass, 'editor_local');
  assert.equal(preview.agentExposure.kind, 'editor_local');
  assert.deepEqual(preview.inputContractRefs.map((ref) => ref.exportName), ['StaticMeshAsset']);
  assert.deepEqual(preview.outputContractRefs.map((ref) => ref.exportName), ['RenderFrameDiff']);
  assert.ok(preview.artifacts.some((artifact) => artifact.type === 'render_diff_preview'));
  assert.ok(preview.runtimeRequirements.some((requirement) => requirement.kind === 'runtime_bridge_operation' && requirement.operation === 'read_model_material_preview'));
});

test('scene load command places a catalog asset through editor-local render-diff evidence', () => {
  const load = requireKnownCommand('scene.load_asset', COMMAND_MANIFEST);
  assert.equal(load.category, 'scene');
  assert.equal(load.operationClass, 'editor_local');
  assert.equal(load.agentExposure.kind, 'editor_local');
  assert.deepEqual(load.menuPath, ['Scene', 'Load Asset']);
  assert.deepEqual(load.outputContractRefs.map((ref) => ref.exportName), ['RenderFrameDiff']);
  assert.ok(load.runtimeRequirements.some((requirement) => requirement.kind === 'runtime_bridge_operation' && requirement.operation === 'read_model_material_preview'));
  assert.ok(load.artifacts.some((artifact) => artifact.type === 'render_diff_preview'));
  assert.equal(load.stateImpact.authority, 'read');
  assert.equal(load.idempotency.kind, 'conditional');
});

test('scene-object hierarchy commands use generated contracts and bridge operations', () => {
  const read = requireKnownCommand('scene.read_object_snapshot', COMMAND_MANIFEST);
  assert.equal(read.category, 'scene');
  assert.equal(read.operationClass, 'read_only');
  assert.deepEqual(read.runtimeRequirements, [{ kind: 'runtime_bridge_operation', operation: 'read_scene_object_snapshot' }]);
  assert.deepEqual(read.outputContractRefs.map((ref) => ref.exportName), ['SceneObjectSnapshot']);
  assert.equal(read.stateImpact.authority, 'read');

  const apply = requireKnownCommand('scene.apply_object_command', COMMAND_MANIFEST);
  assert.equal(apply.category, 'scene');
  assert.equal(apply.operationClass, 'authority_mutating');
  assert.equal(apply.agentExposure.kind, 'authority_mutating');
  assert.deepEqual(apply.inputContractRefs.map((ref) => ref.exportName), ['SceneObjectCommandRequest']);
  assert.deepEqual(apply.outputContractRefs.map((ref) => ref.exportName), ['SceneObjectCommandResult']);
  assert.ok(apply.runtimeRequirements.some((requirement) => requirement.kind === 'runtime_bridge_operation' && requirement.operation === 'apply_scene_object_command'));
  assert.equal(apply.retry, 'safe_to_retry_if_state_hash_unchanged');
  assert.deepEqual(
    validateExampleAgainstSchema(apply.id, 'typedInputExample', { sessionId: 's', request: { expectedDocumentHash: 1, command: { kind: 'rename', id: 1 } } }, apply.inputSchema.shape),
    [{ commandId: 'scene.apply_object_command', field: 'typedInputExample', message: 'typedInputExample does not match its declared schema' }],
  );
});

test('set-active-entity selection command is editor-local and hierarchy-driven', () => {
  const select = requireKnownCommand('selection.set_active_entity', COMMAND_MANIFEST);
  assert.equal(select.category, 'selection');
  assert.equal(select.operationClass, 'editor_local');
  assert.equal(select.agentExposure.kind, 'editor_local');
  assert.deepEqual(select.menuPath, ['Select', 'Active Entity']);
  assert.deepEqual(select.runtimeRequirements, [{ kind: 'editor_store' }]);
  assert.ok(select.artifacts.some((artifact) => artifact.type === 'selection_snapshot'));
  assert.equal(select.stateImpact.editor, 'mutate');
  assert.equal(select.idempotency.kind, 'conditional');
});

test('set-entity-name inspector edit is an editor-local typed command, not a freeform JSON field write', () => {
  const rename = requireKnownCommand('entity.set_name', COMMAND_MANIFEST);
  assert.equal(rename.category, 'entity');
  assert.equal(rename.operationClass, 'editor_local');
  assert.equal(rename.agentExposure.kind, 'editor_local');
  assert.deepEqual(rename.menuPath, ['Inspect', 'Rename Entity']);
  assert.deepEqual(rename.runtimeRequirements, [{ kind: 'editor_store' }]);
  assert.ok(rename.artifacts.some((artifact) => artifact.type === 'editor_state'));
  assert.equal(rename.stateImpact.editor, 'mutate');
  assert.equal(rename.stateImpact.authority, 'none');
  assert.equal(rename.idempotency.kind, 'conditional');
  assert.equal(rename.guiMirror.panel, 'inspector');
  // Input/output are typed object schemas (no contract/opaque or freeform-object payload).
  assert.equal(rename.inputSchema.shape.kind, 'object');
  assert.equal(rename.outputSchema.shape.kind, 'object');
  assert.deepEqual(
    validateExampleAgainstSchema(rename.id, 'typedOutputExample', { entityId: 'e', renderableId: 'e', name: 'n', nameHash: 'h', applied: true }, rename.outputSchema.shape),
    [],
  );
  // A name edit that omits the required new name fails closed against the schema.
  assert.deepEqual(
    validateExampleAgainstSchema(rename.id, 'typedInputExample', { sessionId: 's', entityId: 'e' }, rename.inputSchema.shape),
    [{ commandId: 'entity.set_name', field: 'typedInputExample', message: 'typedInputExample does not match its declared schema' }],
  );
});

test('translate-entity gizmo edit is an editor-local typed transform command with preview/apply modes', () => {
  const translate = requireKnownCommand('transform.translate_entity', COMMAND_MANIFEST);
  assert.equal(translate.category, 'entity');
  assert.equal(translate.operationClass, 'editor_local');
  assert.equal(translate.agentExposure.kind, 'editor_local');
  assert.deepEqual(translate.menuPath, ['Transform', 'Translate Along Axis']);
  assert.deepEqual(translate.runtimeRequirements, [{ kind: 'editor_store' }]);
  assert.ok(translate.artifacts.some((artifact) => artifact.type === 'editor_state'));
  assert.equal(translate.stateImpact.editor, 'mutate');
  assert.equal(translate.stateImpact.authority, 'none');
  assert.equal(translate.idempotency.kind, 'conditional');
  assert.equal(translate.guiMirror.panel, 'viewport');
  // Input/output are typed object schemas (no contract/opaque or freeform-object payload).
  assert.equal(translate.inputSchema.shape.kind, 'object');
  assert.equal(translate.outputSchema.shape.kind, 'object');
  // A committed apply with a typed before/after translation validates against the output schema.
  assert.deepEqual(
    validateExampleAgainstSchema(translate.id, 'typedOutputExample', { entityId: 'e', renderableId: 'e', axis: 'x', delta: 2, mode: 'apply', translationBefore: [0, 0, 0], translationAfter: [2, 0, 0], transformHash: 'h', applied: true }, translate.outputSchema.shape),
    [],
  );
  // An unknown axis is rejected — the gizmo cannot translate along a freeform axis string.
  assert.deepEqual(
    validateExampleAgainstSchema(translate.id, 'typedInputExample', { sessionId: 's', entityId: 'e', axis: 'w', delta: 2, mode: 'apply' }, translate.inputSchema.shape),
    [{ commandId: 'transform.translate_entity', field: 'typedInputExample', message: 'typedInputExample does not match its declared schema' }],
  );
  // A transform edit that omits the preview/apply mode fails closed against the schema.
  assert.deepEqual(
    validateExampleAgainstSchema(translate.id, 'typedInputExample', { sessionId: 's', entityId: 'e', axis: 'x', delta: 2 }, translate.inputSchema.shape),
    [{ commandId: 'transform.translate_entity', field: 'typedInputExample', message: 'typedInputExample does not match its declared schema' }],
  );
});

test('game workspace manifest commands expose UI and agent-equivalent workspace actions', () => {
  const open = requireKnownCommand('workspace.open_game_manifest', COMMAND_MANIFEST);
  assert.equal(open.category, 'workspace');
  assert.equal(open.operationClass, 'workspace_io');
  assert.equal(open.agentExposure.kind, 'workspace_io');
  assert.deepEqual(open.menuPath, ['File', 'Open Game Workspace']);
  assert.deepEqual(open.runtimeRequirements, [{ kind: 'editor_store' }]);
  assert.ok(open.artifacts.some((artifact) => artifact.type === 'game_workspace'));
  assert.equal(open.stateImpact.workspace, 'read');
  assert.equal(open.stateImpact.editor, 'mutate');
  assert.equal(open.guiMirror.dialog, 'simple_form');
  assert.equal(open.outputSchema.shape.kind, 'object');
  assert.deepEqual(
    validateExampleAgainstSchema(open.id, 'typedInputExample', { workspaceRoot: '/workspace/asha-demo' }, open.inputSchema.shape),
    [{ commandId: 'workspace.open_game_manifest', field: 'typedInputExample', message: 'typedInputExample does not match its declared schema' }],
  );

  const validate = requireKnownCommand('workspace.validate_game_manifest', COMMAND_MANIFEST);
  assert.equal(validate.category, 'workspace');
  assert.equal(validate.operationClass, 'workspace_io');
  assert.equal(validate.agentExposure.kind, 'workspace_io');
  assert.deepEqual(validate.menuPath, ['File', 'Validate Game Manifest']);
  assert.deepEqual(validate.runtimeRequirements, [{ kind: 'none' }]);
  assert.equal(validate.stateImpact.workspace, 'read');
  assert.equal(validate.stateImpact.editor, 'none');
  assert.equal(validate.guiMirror.dialog, 'readout_only');
  assert.deepEqual(
    validateExampleAgainstSchema(validate.id, 'typedOutputExample', { valid: false, workspaceHash: null, diagnostics: [{ code: 'manifest_invalid', message: 'bad', source: null }] }, validate.outputSchema.shape),
    [],
  );
});

test('selection command uses screen-point camera request, not a caller-supplied pick ray', () => {
  const select = requireKnownCommand('selection.voxel_from_screen_point', COMMAND_MANIFEST);
  assert.deepEqual(select.inputContractRefs, [{ package: '@asha/contracts', exportName: 'ScreenPointToPickRayRequest' }]);
  assert.deepEqual(select.outputContractRefs, [{ package: '@asha/contracts', exportName: 'VoxelSelectionSnapshot' }]);
  const inputSchema = JSON.stringify(select.inputSchema);
  assert.ok(inputSchema.includes('ScreenPointToPickRayRequest'));
  assert.equal(inputSchema.includes('"exportName":"PickRay"'), false);
  assert.deepEqual(select.runtimeRequirements, [{ kind: 'runtime_bridge_operation', operation: 'select_voxel' }, { kind: 'editor_store' }]);
});

test('validation rejects read-only exposure for non-read-only or mutating impacts', () => {
  const start = requireKnownCommand('session.start', COMMAND_MANIFEST);
  const broken: DraftStudioCommandDefinition = { ...start, agentExposure: { kind: 'read_only' } };
  const issues = validateCommandDefinition(broken);
  assert.ok(issues.some((issue) => issue.field === 'agentExposure' && issue.message.includes('read_only exposure')));
});

test('validation rejects incomplete GUI mirror parity metadata', () => {
  const inspect = requireKnownCommand('inspection.world_summary', COMMAND_MANIFEST);
  const broken: DraftStudioCommandDefinition = {
    ...inspect,
    label: ' ',
    guiMirror: {
      ...inspect.guiMirror,
      menuPath: ['Wrong'],
      argumentSummary: '',
      resultSummary: '',
      artifactSummary: '',
    },
  };
  const fields = validateCommandDefinition(broken).map((issue) => issue.field);
  assert.ok(fields.includes('label'));
  assert.ok(fields.includes('guiMirror.menuPath'));
  assert.ok(fields.includes('guiMirror.argumentSummary'));
  assert.ok(fields.includes('guiMirror.resultSummary'));
  assert.ok(fields.includes('guiMirror.artifactSummary'));
});

test('validation rejects output schemas that do not describe typed outputs', () => {
  const world = requireKnownCommand('inspection.world_summary', COMMAND_MANIFEST);
  const broken = validateExampleAgainstSchema(
    world.id,
    'typedOutputExample',
    world.typedOutputExample,
    { kind: 'object', allowExtraFields: false, fields: [{ name: 'artifactId', required: true, shape: { kind: 'scalar', scalar: 'artifact_ref' }, summary: 'Wrong artifact-only output.' }] },
  );
  assert.deepEqual(broken, [{ commandId: 'inspection.world_summary', field: 'typedOutputExample', message: 'typedOutputExample does not match its declared schema' }]);
});

test('validation rejects missing metadata and open object schemas', () => {
  const broken: DraftStudioCommandDefinition = {
    id: 'inspection.world_summary',
    version: 1,
    inputSchema: {
      name: 'BrokenInput',
      version: 1,
      shape: {
        kind: 'object',
        allowExtraFields: true as false,
        fields: [],
      },
    },
  };
  const issues = validateCommandDefinition(broken);
  assert.ok(issues.some((issue) => issue.field === 'label'));
  assert.ok(issues.some((issue) => issue.field === 'operationClass'));
  assert.ok(issues.some((issue) => issue.field === 'inputSchema.shape'));
});

test('unknown command ids are rejected rather than treated as dynamic method names', () => {
  assert.throws(() => requireKnownCommand('authority.voxel.delete_everything', COMMAND_MANIFEST), /Unknown ASHA studio command id/);
});

test('authority command uses typed voxel contracts and guarded retry/idempotency posture', () => {
  const apply = requireKnownCommand('authority.voxel.apply_brush', COMMAND_MANIFEST);
  assert.equal(apply.operationClass, 'authority_mutating');
  assert.deepEqual(apply.inputContractRefs, [{ package: '@asha/contracts', exportName: 'VoxelCommand' }]);
  assert.equal(apply.agentExposure.kind, 'authority_mutating');
  assert.equal(apply.retry, 'safe_to_retry_if_state_hash_unchanged');
  assert.equal(apply.idempotency.kind, 'conditional');
});

test('command catalog projects every visible command back to a registry identity', () => {
  assert.equal(COMMAND_CATALOG.schemaVersion, 1);
  assert.equal(COMMAND_CATALOG.generatedFrom, 'COMMAND_MANIFEST');
  assert.deepEqual(COMMAND_CATALOG.commands.map((command) => command.id), COMMAND_IDS);
  for (const command of COMMAND_CATALOG.commands) {
    const definition = requireKnownCommand(command.id, COMMAND_MANIFEST);
    assert.equal(command.label, definition.label);
    assert.equal(command.operationClass, definition.operationClass);
    assert.deepEqual(command.menuPath, definition.menuPath);
    assert.deepEqual(command.guiMirror.menuPath, definition.guiMirror.menuPath);
    assert.ok(command.guiMirror.argumentSummary.length > 0, command.id);
    assert.ok(command.guiMirror.resultSummary.length > 0, command.id);
    assert.ok(command.guiMirror.artifactSummary.length > 0, command.id);
  }
  assert.equal(requireCatalogCommand('render.capture_before_after', COMMAND_CATALOG).agentExposureKind, 'render_evidence');
  assert.throws(() => requireCatalogCommand('inspection.missing', COMMAND_CATALOG), /Unknown ASHA studio command id/);
});

test('command catalog golden stays stable and readable', () => {
  const goldenPath = join(process.cwd(), 'src', 'command-catalog.golden.json');
  const expected = readFileSync(goldenPath, 'utf8');
  const actual = `${JSON.stringify(COMMAND_CATALOG, null, 2)}\n`;
  assert.equal(actual, expected);
});

test('manifest golden stays stable and reviewable', () => {
  const goldenPath = join(process.cwd(), 'src', 'manifest.golden.json');
  const expected = readFileSync(goldenPath, 'utf8');
  const actual = `${JSON.stringify(COMMAND_MANIFEST, null, 2)}\n`;
  assert.equal(actual, expected);
});
