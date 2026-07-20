import { sceneId, sceneNodeId } from '@asha/contracts';
import type {
  ActiveRuntimeProjectContentReadout,
  EntityDefinition,
  EntityDefinitionCapability,
  FlatSceneDocument,
  GameRuleHookDeclaration,
  GameRuleModuleManifest,
  GameRuleModuleRef,
  SceneNodeRecord,
  SceneTransform,
} from '@asha/contracts';
import { lifecycleHealth } from './runtime-session-lifecycle.js';
import { projectBundleHashRecord, stableHash } from './runtime-session-hash.js';
import type {
  RuntimeSessionEcrpCapabilityKind,
  RuntimeSessionEcrpCapabilityState,
  RuntimeSessionEcrpEntityDefinition,
  RuntimeSessionEcrpEntityEventReadout,
  RuntimeSessionEcrpEntityReadout,
  RuntimeSessionEcrpEntityState,
  RuntimeSessionEcrpProjectCapabilityDefinition,
  RuntimeSessionEcrpProjectDiagnostic,
  RuntimeSessionEcrpProjectLoadInput,
  RuntimeSessionEcrpProjectState,
  RuntimeSessionEcrpRenderTargetIdentity,
  RuntimeSessionEcrpTransformState,
  RuntimeSessionEcrpReadout,
  RuntimeSessionIdentity,
  RuntimeSessionInitializeInput,
  RuntimeSessionLifecycleHealthReadout,
  RuntimeSessionLifecycleState,
} from '@asha/runtime-session';

export function defaultRuntimeSessionEcrpProjectLoadInput(
  input: RuntimeSessionInitializeInput & {
    readonly projectBundle: NonNullable<RuntimeSessionInitializeInput['projectBundle']>;
  },
): RuntimeSessionEcrpProjectLoadInput {
  return {
    kind: 'runtime_session.load_ecrp_project.v0',
    projectBundle: {
      kind: 'ProjectBundle',
      project: input.project,
      runtimeRequest: input.projectBundle,
    },
    bootstrapResolutionRegistry: {
      schemaVersion: 1,
      entityDefinitionIds: [
        'actor/demo-player',
        'actor/generated-tunnel-enemy',
      ],
      prefabIds: [],
      generatorPresets: [],
      catalogIds: [],
    },
    entityDefinitions: [
      {
        kind: 'EntityDefinition',
        stableId: 'actor/demo-player',
        displayName: 'Demo Player',
        source: {
          projectBundle: input.project.gameId,
          relativePath: 'catalogs/actors/demo-player.entity.json',
        },
        capabilities: [
          {
            kind: 'transform',
            initial: {
              position: [0, 1.62, 0],
              yawDegrees: 0,
              pitchDegrees: 0,
            },
          },
          {
            kind: 'collisionBody',
            halfExtents: [0.5, 1.4, 0.5],
          },
          {
            kind: 'controller',
            controller: 'player_input',
          },
          {
            kind: 'health',
            current: 100,
            max: 100,
          },
          {
            kind: 'weaponMount',
            weaponId: 'weapon.demo.primary',
          },
          {
            kind: 'renderProjection',
            projection: 'first_person_camera',
          },
          {
            kind: 'faction',
            factionId: 'player',
          },
        ],
      },
      {
        kind: 'EntityDefinition',
        stableId: 'actor/generated-tunnel-enemy',
        displayName: 'Generated Tunnel Enemy',
        source: {
          projectBundle: input.project.gameId,
          relativePath: 'catalogs/actors/generated-tunnel-enemy.entity.json',
        },
        capabilities: [
          {
            kind: 'transform',
            initial: {
              position: [0, 1.1, -3.5],
              yawDegrees: 180,
              pitchDegrees: 0,
            },
          },
          {
            kind: 'collisionBody',
            halfExtents: [0.7, 1.8, 0.7],
          },
          {
            kind: 'health',
            current: 40,
            max: 40,
          },
          {
            kind: 'renderProjection',
            projection: 'target_cube',
          },
          {
            kind: 'policyBinding',
            policyId: 'policy.enemy.generated_tunnel.v0',
          },
          {
            kind: 'spawnMarker',
            markerId: 'spawn.enemy.primary',
          },
          {
            kind: 'faction',
            factionId: 'hostile',
          },
        ],
      },
    ],
    sceneDocument: defaultRuntimeSessionSceneDocument(input.projectBundle.sceneId),
  };
}

function defaultRuntimeSessionSceneDocument(id: number): FlatSceneDocument {
  return {
    schemaVersion: 4,
    id: sceneId(id),
    metadata: {
      name: 'RuntimeSession default scene',
      authoringFormatVersion: 4,
    },
    dependencies: [],
    nodes: [
      runtimeEntitySceneNode({
        id: 10,
        label: 'Demo Player',
        instanceId: 'actor.demo-player.instance',
        definitionId: 'actor/demo-player',
        spawnMarkerId: 'spawn.player.start',
        translation: [0, 0, 0],
        rotation: [0, 0, 0, 1],
      }),
      runtimeEntitySceneNode({
        id: 20,
        label: 'Generated Tunnel Enemy',
        instanceId: 'actor.generated-tunnel-enemy.instance',
        definitionId: 'actor/generated-tunnel-enemy',
        spawnMarkerId: 'spawn.enemy.primary',
        translation: [0, 0, 0],
        rotation: [0, 0, 0, 1],
      }),
      runtimeMarkerSceneNode(30, 'spawn.player.start', [0, 1.62, 0], [0, 0, 0, 1]),
      runtimeMarkerSceneNode(40, 'spawn.enemy.primary', [0, 1.1, -3.5], [0, 1, 0, 0]),
    ],
  };
}

function runtimeMarkerSceneNode(
  id: number,
  markerId: string,
  translation: readonly [number, number, number],
  rotation: readonly [number, number, number, number],
): SceneNodeRecord {
  return {
    id: sceneNodeId(id),
    parent: null,
    childOrder: id,
    label: markerId,
    tags: [],
    transform: { translation, rotation, scale: [1, 1, 1] },
    kind: { kind: 'marker', markerId },
  };
}

function runtimeEntitySceneNode(input: {
  readonly id: number;
  readonly label: string;
  readonly instanceId: string;
  readonly definitionId: string;
  readonly spawnMarkerId: string | null;
  readonly translation: readonly [number, number, number];
  readonly rotation: readonly [number, number, number, number];
}): SceneNodeRecord {
  return {
    id: sceneNodeId(input.id),
    parent: null,
    childOrder: input.id,
    label: input.label,
    tags: [],
    transform: {
      translation: input.translation,
      rotation: input.rotation,
      scale: [1, 1, 1],
    },
    kind: {
      kind: 'entityInstance',
      instance: {
        instanceId: input.instanceId,
        reference: {
          kind: 'entityDefinition',
          stableId: input.definitionId,
        },
        spawnMarkerId: input.spawnMarkerId,
      },
    },
  };
}
export function validateEcrpProjectLoadInput(
  input: RuntimeSessionEcrpProjectLoadInput,
): readonly RuntimeSessionEcrpProjectDiagnostic[] {
  const diagnostics: RuntimeSessionEcrpProjectDiagnostic[] = [];
  if (input === null || typeof input !== 'object' || input.kind !== 'runtime_session.load_ecrp_project.v0') {
    return [
      {
        code: 'missingProjectBundle',
        path: 'input.kind',
        detail: 'ECRP project load input kind must be runtime_session.load_ecrp_project.v0',
      },
    ];
  }
  if (input.projectBundle?.kind !== 'ProjectBundle') {
    diagnostics.push({
      code: 'missingProjectBundle',
      path: 'projectBundle.kind',
      detail: 'projectBundle.kind must be ProjectBundle',
    });
  }
  validateBootstrapResolutionRegistry(input.bootstrapResolutionRegistry, diagnostics);
  if (!Array.isArray(input.entityDefinitions) || input.entityDefinitions.length === 0) {
    diagnostics.push({
      code: 'emptyEntityDefinitionList',
      path: 'entityDefinitions',
      detail: 'at least one EntityDefinition is required',
    });
  }
  validateGameRuleModuleManifests(input.gameRuleModules, diagnostics);
  const definitions = new Map<string, RuntimeSessionEcrpEntityDefinition>();
  input.entityDefinitions?.forEach((definition, index) => {
    if (definition.kind !== 'EntityDefinition' || definition.stableId.trim().length === 0) {
      diagnostics.push({
        code: 'missingEntityDefinition',
        path: `entityDefinitions.${index}.stableId`,
        detail: 'EntityDefinition stableId is required',
      });
      return;
    }
    if (definitions.has(definition.stableId)) {
      diagnostics.push({
        code: 'duplicateEntityDefinition',
        path: `entityDefinitions.${index}.stableId`,
        detail: `duplicate EntityDefinition ${definition.stableId}`,
      });
    }
    definitions.set(definition.stableId, definition);
    validateEcrpCapabilities(definition, `entityDefinitions.${index}.capabilities`, diagnostics);
  });
  if (!isPlainObject(input.sceneDocument) || input.sceneDocument.schemaVersion !== 3 || !isTypedArray(input.sceneDocument.nodes)) {
    diagnostics.push({
      code: 'missingPlacement',
      path: 'sceneDocument.nodes',
      detail: 'a canonical schema-3 FlatSceneDocument is required',
    });
    return diagnostics;
  }
  const placedDefinitions = new Set<string>();
  const instanceIds = new Set<string>();
  const runtimeIds = new Set<number>();
  const placements = input.sceneDocument.nodes.filter((node) => node.kind.kind === 'entityInstance');
  placements.forEach((placement, index) => {
    if (placement.kind.kind !== 'entityInstance') {
      return;
    }
    const instance = placement.kind.instance;
    if (instance.reference.kind === 'prefab') {
      diagnostics.push({
        code: 'unknownEntityDefinition',
        path: `sceneDocument.nodes.${index}.kind.instance.reference`,
        detail: `FPS RuntimeSession does not yet materialize prefab ${instance.reference.prefabId}`,
      });
    } else if (!definitions.has(instance.reference.stableId)) {
      diagnostics.push({
        code: 'unknownEntityDefinition',
        path: `sceneDocument.nodes.${index}.kind.instance.reference.stableId`,
        detail: `scene instance references unknown EntityDefinition ${instance.reference.stableId}`,
      });
    } else {
      placedDefinitions.add(instance.reference.stableId);
    }
    if (instanceIds.has(instance.instanceId)) {
      diagnostics.push({
        code: 'duplicatePlacement',
        path: `sceneDocument.nodes.${index}.kind.instance.instanceId`,
        detail: `duplicate scene instance id ${instance.instanceId}`,
      });
    }
    instanceIds.add(instance.instanceId);
    if (!Number.isSafeInteger(placement.id) || placement.id <= 0 || runtimeIds.has(placement.id)) {
      diagnostics.push({
        code: 'duplicatePlacement',
        path: `sceneDocument.nodes.${index}.id`,
        detail: `entity instance node id ${placement.id} must be a unique positive runtime entity id`,
      });
    }
    runtimeIds.add(placement.id);
  });
  for (const definition of definitions.values()) {
    if (!placedDefinitions.has(definition.stableId)) {
      diagnostics.push({
        code: 'missingPlacement',
        path: `sceneDocument.nodes.${definition.stableId}`,
        detail: `missing canonical scene instance for ${definition.stableId}`,
      });
    }
  }
  return diagnostics;
}

function validateBootstrapResolutionRegistry(
  value: RuntimeSessionEcrpProjectLoadInput['bootstrapResolutionRegistry'] | null | undefined,
  diagnostics: RuntimeSessionEcrpProjectDiagnostic[],
): void {
  if (typeof value !== 'object' || value === null || value.schemaVersion !== 1) {
    diagnostics.push({
      code: 'invalidBootstrapResolutionRegistry',
      path: 'bootstrapResolutionRegistry.schemaVersion',
      detail: 'a schema-1 bootstrap resolution registry is required',
    });
    return;
  }

  const identifierLists = [
    ['entityDefinitionIds', value.entityDefinitionIds],
    ['catalogIds', value.catalogIds],
  ] as const;
  for (const [field, identifiers] of identifierLists) {
    if (!Array.isArray(identifiers)
      || identifiers.some((identifier) => typeof identifier !== 'string' || identifier.trim() !== identifier || identifier === '')
      || new Set(identifiers).size !== identifiers.length) {
      diagnostics.push({
        code: 'invalidBootstrapResolutionRegistry',
        path: `bootstrapResolutionRegistry.${field}`,
        detail: `${field} must contain unique non-empty canonical identifiers`,
      });
    }
  }
  if (!Array.isArray(value.prefabIds)
    || value.prefabIds.some((prefabId) => !Number.isSafeInteger(prefabId) || prefabId <= 0)
    || new Set(value.prefabIds).size !== value.prefabIds.length) {
    diagnostics.push({
      code: 'invalidBootstrapResolutionRegistry',
      path: 'bootstrapResolutionRegistry.prefabIds',
      detail: 'prefabIds must contain unique positive safe integers',
    });
  }
  if (!isTypedArray(value.generatorPresets)
    || value.generatorPresets.some((preset) => typeof preset !== 'object'
      || preset === null
      || typeof preset.providerId !== 'string'
      || preset.providerId.trim() !== preset.providerId
      || preset.providerId === ''
      || typeof preset.presetId !== 'string'
      || preset.presetId.trim() !== preset.presetId
      || preset.presetId === '')
    || new Set(value.generatorPresets.map((preset) => `${preset.providerId}\u0000${preset.presetId}`)).size
      !== value.generatorPresets.length) {
    diagnostics.push({
      code: 'invalidBootstrapResolutionRegistry',
      path: 'bootstrapResolutionRegistry.generatorPresets',
      detail: 'generatorPresets must contain unique provider/preset identities',
    });
  }
}

function validateGameRuleModuleManifests(
  value: readonly GameRuleModuleManifest[] | null | undefined,
  diagnostics: RuntimeSessionEcrpProjectDiagnostic[],
): void {
  if (value === undefined) {
    return;
  }
  if (!isTypedArray(value)) {
    diagnostics.push({
      code: 'invalidGameRuleModuleManifest',
      path: 'gameRuleModules',
      detail: 'gameRuleModules must be an array of generated GameRuleModuleManifest declarations',
    });
    return;
  }
  value.forEach((manifest, index) => validateGameRuleModuleManifest(manifest, `gameRuleModules.${index}`, diagnostics));
}

function validateGameRuleModuleManifest(
  manifest: GameRuleModuleManifest | null | undefined,
  path: string,
  diagnostics: RuntimeSessionEcrpProjectDiagnostic[],
): void {
  if (!isPlainObject(manifest)) {
    diagnostics.push({
      code: 'invalidGameRuleModuleManifest',
      path,
      detail: 'GameRuleModuleManifest must be an object',
    });
    return;
  }
  validateGameRuleModuleRef(manifest['moduleRef'], `${path}.moduleRef`, diagnostics);
  validateGameRuleHookDeclarations(manifest['declaredHooks'], `${path}.declaredHooks`, diagnostics);
  validateStringArray(manifest['deterministicRequirements'], `${path}.deterministicRequirements`, diagnostics);
  validateNonEmptyString(manifest['sourceHash'], `${path}.sourceHash`, 'sourceHash is required', diagnostics);
}

function validateGameRuleModuleRef(
  value: GameRuleModuleRef | null | undefined,
  path: string,
  diagnostics: RuntimeSessionEcrpProjectDiagnostic[],
): void {
  if (!isPlainObject(value)) {
    diagnostics.push({
      code: 'invalidGameRuleModuleManifest',
      path,
      detail: 'moduleRef must be an object',
    });
    return;
  }
  validateNonEmptyString(value['moduleId'], `${path}.moduleId`, 'moduleRef.moduleId is required', diagnostics);
  validateNonEmptyString(value['version'], `${path}.version`, 'moduleRef.version is required', diagnostics);
  validateNonEmptyString(value['contractHash'], `${path}.contractHash`, 'moduleRef.contractHash is required', diagnostics);
}

function validateGameRuleHookDeclarations(
  value: readonly GameRuleHookDeclaration[] | null | undefined,
  path: string,
  diagnostics: RuntimeSessionEcrpProjectDiagnostic[],
): void {
  if (!isTypedArray(value) || value.length === 0) {
    diagnostics.push({
      code: 'invalidGameRuleModuleManifest',
      path,
      detail: 'declaredHooks must include at least one generated hook declaration',
    });
    return;
  }
  value.forEach((hook, index) => validateGameRuleHookDeclaration(hook, `${path}.${index}`, diagnostics));
}

function validateGameRuleHookDeclaration(
  value: GameRuleHookDeclaration | null | undefined,
  path: string,
  diagnostics: RuntimeSessionEcrpProjectDiagnostic[],
): void {
  if (!isPlainObject(value)) {
    diagnostics.push({
      code: 'invalidGameRuleModuleManifest',
      path,
      detail: 'GameRuleHookDeclaration must be an object',
    });
    return;
  }
  validateNonEmptyString(value['hookId'], `${path}.hookId`, 'hookId is required', diagnostics);
  const hookKind = value['kind'];
  if (hookKind !== 'weaponEffect' && hookKind !== 'interactionEffect' && hookKind !== 'spawnCondition') {
    diagnostics.push({
      code: 'invalidGameRuleModuleManifest',
      path: `${path}.kind`,
      detail: 'hook kind must be weaponEffect, interactionEffect, or spawnCondition',
    });
  }
  validateNonEmptyString(value['inputContract'], `${path}.inputContract`, 'inputContract is required', diagnostics);
  validateNonEmptyString(value['outputContract'], `${path}.outputContract`, 'outputContract is required', diagnostics);
  validateStringArray(value['requiredCapabilities'], `${path}.requiredCapabilities`, diagnostics);
}

function validateNonEmptyString(
  value: string | null | undefined,
  path: string,
  detail: string,
  diagnostics: RuntimeSessionEcrpProjectDiagnostic[],
): void {
  if (typeof value !== 'string' || value.trim().length === 0) {
    diagnostics.push({
      code: 'invalidGameRuleModuleManifest',
      path,
      detail,
    });
  }
}

function validateStringArray(
  value: readonly string[] | null | undefined,
  path: string,
  diagnostics: RuntimeSessionEcrpProjectDiagnostic[],
): void {
  if (!Array.isArray(value) || value.some((entry) => typeof entry !== 'string' || entry.trim().length === 0)) {
    diagnostics.push({
      code: 'invalidGameRuleModuleManifest',
      path,
      detail: `${path} must be an array of non-empty strings`,
    });
  }
}

function isPlainObject(value: object | null | undefined): value is object {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isTypedArray<T>(value: readonly T[] | null | undefined): value is readonly T[] {
  return Array.isArray(value);
}

function validateEcrpCapabilities(
  definition: RuntimeSessionEcrpEntityDefinition,
  path: string,
  diagnostics: RuntimeSessionEcrpProjectDiagnostic[],
): void {
  const capabilityKinds = new Set<RuntimeSessionEcrpCapabilityKind>();
  for (const capability of definition.capabilities ?? []) {
    capabilityKinds.add(capability.kind);
    if (capability.kind === 'transform' && !isVec3(capability.initial?.position)) {
      diagnostics.push({
        code: 'invalidCapability',
        path: `${path}.transform.initial.position`,
        detail: 'transform initial position must be a finite vec3',
      });
    }
    if (capability.kind === 'collisionBody' && !isVec3(capability.halfExtents)) {
      diagnostics.push({
        code: 'invalidCapability',
        path: `${path}.collisionBody.halfExtents`,
        detail: 'collisionBody halfExtents must be a finite vec3',
      });
    }
    if (capability.kind === 'health' && (!Number.isFinite(capability.current) || capability.current < 0 || !Number.isFinite(capability.max) || capability.max <= 0)) {
      diagnostics.push({
        code: 'invalidCapability',
        path: `${path}.health`,
        detail: 'health current/max must be finite and max must be positive',
      });
    }
  }
  for (const required of ['transform', 'health', 'renderProjection'] as const) {
    if (!capabilityKinds.has(required)) {
      diagnostics.push({
        code: 'missingCapability',
        path,
        detail: `${definition.stableId} missing required ${required} capability`,
      });
    }
  }
}

function isVec3(value: readonly number[] | undefined): value is readonly [number, number, number] {
  return Array.isArray(value) && value.length === 3 && value.every((component) => Number.isFinite(component));
}

export function buildEcrpProjectState(input: RuntimeSessionEcrpProjectLoadInput): RuntimeSessionEcrpProjectState {
  return buildEcrpProjectStateFromParts(input, input.entityDefinitions, input.sceneDocument);
}

/** Project the accepted Rust-owned canonical content into the existing ECRP
 * display/readout vocabulary. This projection is never used as bootstrap. */
export function buildEcrpProjectStateFromCanonical(
  readout: ActiveRuntimeProjectContentReadout,
): RuntimeSessionEcrpProjectState {
  const definitions = readout.content.documents.flatMap((document) =>
    document.kind === 'entityDefinition'
      ? [ecrpDefinitionFromStored(document.definition)]
      : [],
  );
  return buildEcrpProjectStateFromParts(null, definitions, readout.entryScene);
}

function buildEcrpProjectStateFromParts(
  input: RuntimeSessionEcrpProjectLoadInput | null,
  entityDefinitions: readonly RuntimeSessionEcrpEntityDefinition[],
  sceneDocument: FlatSceneDocument,
): RuntimeSessionEcrpProjectState {
  const definitions = new Map(entityDefinitions.map((definition) => [definition.stableId, definition]));
  const worldTransforms = sceneWorldTransforms(sceneDocument);
  const markerTransforms = new Map(
    sceneDocument.nodes.flatMap((node) => node.kind.kind === 'marker'
      ? [[node.kind.markerId, worldTransforms.get(node.id) ?? node.transform] as const]
      : []),
  );
  const entities = sceneDocument.nodes.flatMap((placement) => {
    if (placement.kind.kind !== 'entityInstance' || placement.kind.instance.reference.kind !== 'entityDefinition') {
      return [];
    }
    const definition = definitions.get(placement.kind.instance.reference.stableId);
    if (definition === undefined) {
      return [];
    }
    const authoredWorldTransform = worldTransforms.get(placement.id) ?? placement.transform;
    const spawnMarkerTransform = placement.kind.instance.spawnMarkerId === null
      ? undefined
      : markerTransforms.get(placement.kind.instance.spawnMarkerId);
    return {
      entity: placement.id,
      instanceId: placement.kind.instance.instanceId,
      spawnMarkerId: placement.kind.instance.spawnMarkerId,
      worldTransform: spawnMarkerTransform === undefined
        ? authoredWorldTransform
        : composeSceneTransform(spawnMarkerTransform, placement.transform),
      definition,
      role: inferRuntimeRole(definition),
    };
  });
  return {
    input,
    entities,
    bootstrapHash: stableHash({
      project: {
        gameId: input?.projectBundle.project.gameId ?? 'canonical-runtime-project',
        workspaceId: input?.projectBundle.project.workspaceId ?? 'rust-authority',
      },
      runtimeRequest: input === null ? null : projectBundleHashRecord(input.projectBundle.runtimeRequest),
      sceneDocumentHash: stableHash(sceneDocument as never),
      entityIds: entities.map((entity) => entity.entity),
      instanceIds: entities.map((entity) => entity.instanceId),
      definitionIds: entities.map((entity) => entity.definition.stableId),
      capabilityKinds: entities.map((entity) => entity.definition.capabilities.map((capability) => capability.kind)),
    }),
  };
}

function ecrpDefinitionFromStored(definition: EntityDefinition): RuntimeSessionEcrpEntityDefinition {
  const collision = definition.capabilities.find((capability) => capability.kind === 'collision');
  const staticCollider = collision?.kind === 'collision' ? collision.staticCollider : false;
  return {
    kind: 'EntityDefinition',
    stableId: definition.stableId,
    displayName: definition.displayName,
    source: definition.source,
    capabilities: definition.capabilities.flatMap((capability) =>
      ecrpCapabilityFromStored(capability, staticCollider),
    ),
  };
}

function ecrpCapabilityFromStored(
  capability: EntityDefinitionCapability,
  staticCollider: boolean,
): readonly RuntimeSessionEcrpProjectCapabilityDefinition[] {
  switch (capability.kind) {
    case 'transform':
      return [{
        kind: 'transform',
        initial: {
          position: capability.transform.translation,
          yawDegrees: 0,
          pitchDegrees: 0,
        },
      }];
    case 'collision':
    case 'render':
    case 'unknown':
      return [];
    case 'bounds':
      return [{
        kind: 'collisionBody',
        halfExtents: [
          (capability.max[0] - capability.min[0]) * 0.5,
          (capability.max[1] - capability.min[1]) * 0.5,
          (capability.max[2] - capability.min[2]) * 0.5,
        ],
        staticCollider,
      }];
    case 'controller':
      return capability.controllerId === 'player_input' || capability.controllerId === 'enemy_policy'
        ? [{ kind: 'controller', controller: capability.controllerId }]
        : [];
    case 'health':
      return [{ kind: 'health', current: capability.current, max: capability.max }];
    case 'weaponMount':
      return [{
        kind: 'weaponMount',
        weaponId: capability.weaponId,
        tuning: {
          damage: capability.damage,
          rangeUnits: capability.rangeUnits,
          ammo: capability.ammo,
          cooldownTicksAfterFire: capability.cooldownTicksAfterFire,
        },
      }];
    case 'renderProjection':
      return isEcrpProjection(capability.projectionId)
        ? [{
            kind: 'renderProjection',
            projection: capability.projectionId,
            visible: capability.visible,
          }]
        : [];
    case 'policyBinding':
      return [{
        kind: 'policyBinding',
        policyId: capability.policyId,
        policyLoopRef: capability.runtimeMoment,
      }];
    case 'spawnMarker':
      return [{ kind: 'spawnMarker', markerId: capability.markerId }];
    case 'faction':
      return [{ kind: 'faction', factionId: capability.factionId }];
  }
}

function isEcrpProjection(
  value: string,
): value is Extract<RuntimeSessionEcrpProjectCapabilityDefinition, { readonly kind: 'renderProjection' }>['projection'] {
  return value === 'first_person_camera' || value === 'target_cube' || value === 'spawn_marker';
}

function sceneWorldTransforms(document: FlatSceneDocument): ReadonlyMap<number, SceneTransform> {
  const nodes = new Map(document.nodes.map((node) => [node.id, node]));
  const resolved = new Map<number, SceneTransform>();
  const resolving = new Set<number>();
  const resolve = (node: SceneNodeRecord): SceneTransform => {
    const existing = resolved.get(node.id);
    if (existing !== undefined) {
      return existing;
    }
    if (resolving.has(node.id) || node.parent === null) {
      resolved.set(node.id, node.transform);
      return node.transform;
    }
    resolving.add(node.id);
    const parent = nodes.get(node.parent);
    const world = parent === undefined ? node.transform : composeSceneTransform(resolve(parent), node.transform);
    resolving.delete(node.id);
    resolved.set(node.id, world);
    return world;
  };
  for (const node of document.nodes) {
    resolve(node);
  }
  return resolved;
}

function composeSceneTransform(parent: SceneTransform, local: SceneTransform): SceneTransform {
  const scaled: readonly [number, number, number] = [
    local.translation[0] * parent.scale[0],
    local.translation[1] * parent.scale[1],
    local.translation[2] * parent.scale[2],
  ];
  const rotated = rotateSceneVector(parent.rotation, scaled);
  return {
    translation: [
      parent.translation[0] + rotated[0],
      parent.translation[1] + rotated[1],
      parent.translation[2] + rotated[2],
    ],
    rotation: multiplySceneQuaternion(parent.rotation, local.rotation),
    scale: [
      parent.scale[0] * local.scale[0],
      parent.scale[1] * local.scale[1],
      parent.scale[2] * local.scale[2],
    ],
  };
}

function multiplySceneQuaternion(
  left: readonly [number, number, number, number],
  right: readonly [number, number, number, number],
): readonly [number, number, number, number] {
  const [lx, ly, lz, lw] = left;
  const [rx, ry, rz, rw] = right;
  return [
    lw * rx + lx * rw + ly * rz - lz * ry,
    lw * ry - lx * rz + ly * rw + lz * rx,
    lw * rz + lx * ry - ly * rx + lz * rw,
    lw * rw - lx * rx - ly * ry - lz * rz,
  ];
}

function rotateSceneVector(
  rotation: readonly [number, number, number, number],
  vector: readonly [number, number, number],
): readonly [number, number, number] {
  const length = Math.hypot(...rotation);
  const normalized: readonly [number, number, number, number] = length === 0
    ? [0, 0, 0, 1]
    : [rotation[0] / length, rotation[1] / length, rotation[2] / length, rotation[3] / length];
  const vectorQuaternion: readonly [number, number, number, number] = [vector[0], vector[1], vector[2], 0];
  const conjugate: readonly [number, number, number, number] = [
    -normalized[0],
    -normalized[1],
    -normalized[2],
    normalized[3],
  ];
  const result = multiplySceneQuaternion(multiplySceneQuaternion(normalized, vectorQuaternion), conjugate);
  return [result[0], result[1], result[2]];
}

function inferRuntimeRole(definition: RuntimeSessionEcrpEntityDefinition): RuntimeSessionEcrpEntityState['role'] {
  const faction = definition.capabilities.find((capability) => capability.kind === 'faction');
  if (faction?.kind === 'faction') {
    if (faction.factionId === 'player') {
      return 'player';
    }
    if (faction.factionId === 'hostile') {
      return 'enemy';
    }
  }
  const controller = definition.capabilities.find((capability) => capability.kind === 'controller');
  if (controller?.kind === 'controller' && controller.controller === 'player_input') {
    return 'player';
  }
  if (definition.capabilities.some((capability) => capability.kind === 'policyBinding')) {
    return 'enemy';
  }
  return 'neutral';
}

export function lifecycleStateFromEcrpProject(state: RuntimeSessionEcrpProjectState): RuntimeSessionLifecycleState {
  const player = state.entities.find((entity) => entity.role === 'player');
  const enemy = state.entities.find((entity) => entity.role === 'enemy');
  return {
    player: lifecycleHealthFromEntity(player, 100),
    enemy: lifecycleHealthFromEntity(enemy, 40),
    terminalEvent: null,
    revision: 0,
  };
}

function lifecycleHealthFromEntity(
  entity: RuntimeSessionEcrpEntityState | undefined,
  fallbackMax: number,
): RuntimeSessionLifecycleHealthReadout {
  const health = entity?.definition.capabilities.find((capability) => capability.kind === 'health');
  if (entity !== undefined && health?.kind === 'health') {
    return lifecycleHealth(entity.entity, health.current, health.max, health.current <= 0);
  }
  return lifecycleHealth(entity?.entity ?? 0, fallbackMax, fallbackMax, false);
}

function ecrpCapabilitiesForEntity(
  entity: RuntimeSessionEcrpEntityState,
  lifecycleState: RuntimeSessionLifecycleState,
  runtimeTransforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>,
): readonly RuntimeSessionEcrpCapabilityState[] {
  return entity.definition.capabilities.map((capability) =>
    ecrpCapabilityForDefinition(entity, capability, lifecycleState, runtimeTransforms),
  );
}

function ecrpCapabilityForDefinition(
  entity: RuntimeSessionEcrpEntityState,
  capability: RuntimeSessionEcrpProjectCapabilityDefinition,
  lifecycleState: RuntimeSessionLifecycleState,
  runtimeTransforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>,
): RuntimeSessionEcrpCapabilityState {
  switch (capability.kind) {
    case 'transform':
      return ecrpRuntimeTransform(entity, capability, runtimeTransforms);
    case 'collisionBody':
      return ecrpCollisionBody(capability.staticCollider ?? false, capability.halfExtents);
    case 'controller':
      return ecrpController(capability.controller);
    case 'health':
      return ecrpHealth(runtimeHealthForEntity(entity, capability, lifecycleState));
    case 'weaponMount':
      return ecrpWeaponMount(capability.weaponId);
    case 'renderProjection':
      return ecrpRenderProjection(
        entity,
        capability,
        renderVisibleForEntity(entity, capability, lifecycleState),
        runtimeTransforms,
      );
    case 'policyBinding':
      return ecrpPolicyBinding(capability.policyId);
    case 'spawnMarker':
      return ecrpSpawnMarker(capability.markerId);
    case 'faction':
      return ecrpFaction(capability.factionId);
  }
}

function runtimeHealthForEntity(
  entity: RuntimeSessionEcrpEntityState,
  capability: Extract<RuntimeSessionEcrpProjectCapabilityDefinition, { readonly kind: 'health' }>,
  lifecycleState: RuntimeSessionLifecycleState,
): RuntimeSessionLifecycleHealthReadout {
  if (entity.role === 'player') {
    return lifecycleState.player;
  }
  if (entity.role === 'enemy') {
    return lifecycleState.enemy;
  }
  return lifecycleHealth(entity.entity, capability.current, capability.max, capability.current <= 0);
}

function renderVisibleForEntity(
  entity: RuntimeSessionEcrpEntityState,
  capability: Extract<RuntimeSessionEcrpProjectCapabilityDefinition, { readonly kind: 'renderProjection' }>,
  lifecycleState: RuntimeSessionLifecycleState,
): boolean {
  if (capability.visible !== undefined) {
    return capability.visible;
  }
  if (entity.role === 'enemy') {
    return !lifecycleState.enemy.dead;
  }
  if (entity.role === 'player') {
    return !lifecycleState.player.dead;
  }
  return true;
}

export function buildEcrpRuntimeReadout(input: {
  readonly identity: RuntimeSessionIdentity;
  readonly projectState: RuntimeSessionEcrpProjectState | null;
  readonly lifecycleState: RuntimeSessionLifecycleState;
  readonly runtimeTransforms?: ReadonlyMap<number, RuntimeSessionEcrpTransformState>;
  readonly sequenceId: number;
  readonly tick: number;
  readonly sessionHash: string;
  readonly authority?: RuntimeSessionEcrpReadout['authority'];
}): RuntimeSessionEcrpReadout {
  if (input.identity.projectBundle === null && input.projectState === null) {
    throw new Error(
      'ECRP readout requires an active canonical project or compatibility ProjectBundle',
    );
  }
  const projectState = input.projectState ?? buildEcrpProjectState(
    defaultRuntimeSessionEcrpProjectLoadInput({
      sessionId: input.identity.sessionId,
      seed: input.identity.seed,
      project: input.identity.project,
      projectBundle: requireCompatibilityProjectBundle(input.identity.projectBundle),
    }),
  );
  const entities = projectState.entities.map((entity) =>
    ecrpEntityReadout({
      entity: entity.entity,
      definition: entity.definition,
      capabilities: ecrpCapabilitiesForEntity(entity, input.lifecycleState, input.runtimeTransforms ?? new Map()),
      events: ecrpEventsForEntity(input.lifecycleState, entity.entity),
    }),
  );
  const capabilityStateHash = stableHash(
    entities.map((entity) => entity.capabilities.map((capability) => capability.stateHash)),
  );
  const eventReadoutHash = stableHash(
    entities.map((entity) => entity.recentEvents.map((event) => event.eventHash)),
  );
  const entityReadoutHash = stableHash({
    entities: entities.map((entity) => entity.entityHash),
    capabilityStateHash,
    eventReadoutHash,
  });

  return {
    kind: 'runtime_session.ecrp_readout.v0',
    sequenceId: input.sequenceId,
    tick: input.tick,
    sessionHash: input.sessionHash,
    authority: input.authority ?? {
      mode: 'reference',
      source: 'reference_fixture',
      surface: 'runtime_session.ecrp.reference_fixture.v0',
      readSets: [{
        viewKind: 'runtime_session.ecrp.reference_fixture_readout.v0',
        owner: 'reference-runtime-session',
        readSet: ['reference.entities', 'reference.lifecycle', 'reference.capability_projection'],
      }],
    },
    project: input.identity.project,
    projectBundle: input.identity.projectBundle,
    entities,
    entityCount: entities.length,
    hashes: {
      entityReadoutHash,
      capabilityStateHash,
      eventReadoutHash,
    },
    nonClaims: [
      'not_raw_state_store',
      'not_authoring_mode',
      'not_demo_local_authority',
    ],
  };
}

function requireCompatibilityProjectBundle(
  projectBundle: RuntimeSessionIdentity['projectBundle'],
): NonNullable<RuntimeSessionIdentity['projectBundle']> {
  if (projectBundle === null) {
    throw new Error('compatibility ProjectBundle is unavailable');
  }
  return projectBundle;
}

function ecrpEntityReadout(input: {
  readonly entity: number;
  readonly definition: RuntimeSessionEcrpEntityDefinition;
  readonly capabilities: readonly RuntimeSessionEcrpCapabilityState[];
  readonly events: readonly RuntimeSessionEcrpEntityEventReadout[];
}): RuntimeSessionEcrpEntityReadout {
  const capabilityKinds = input.capabilities.map((capability) => capability.kind);
  const entityHash = stableHash({
    entity: input.entity,
    definitionStableId: input.definition.stableId,
    displayName: input.definition.displayName,
    sourcePath: input.definition.source.relativePath,
    capabilityKinds,
    capabilityStateHashes: input.capabilities.map((capability) => capability.stateHash),
    eventHashes: input.events.map((event) => event.eventHash),
  });
  return {
    entity: input.entity,
    lifecycle: 'active',
    definitionStableId: input.definition.stableId,
    displayName: input.definition.displayName,
    source: {
      projectBundle: input.definition.source.projectBundle,
      relativePath: input.definition.source.relativePath,
    },
    capabilityKinds,
    capabilities: input.capabilities,
    recentEvents: input.events,
    entityHash,
  };
}

function ecrpEventsForEntity(
  state: RuntimeSessionLifecycleState,
  entity: number,
): readonly RuntimeSessionEcrpEntityEventReadout[] {
  const events: RuntimeSessionEcrpEntityEventReadout[] = [
    {
      kind: 'runtime_session.bootstrap_entity.v0',
      entity,
      tick: 0,
      eventHash: stableHash({
        kind: 'runtime_session.bootstrap_entity.v0',
        entity,
      }),
    },
  ];
  if (state.terminalEvent !== null && state.terminalEvent.entity === entity) {
    events.push({
      kind: state.terminalEvent.kind,
      entity,
      tick: state.terminalEvent.tick,
      eventHash: state.terminalEvent.eventHash,
    });
  }
  return events;
}

function ecrpTransform(
  position: readonly [number, number, number],
  yawDegrees: number,
  pitchDegrees: number,
): RuntimeSessionEcrpCapabilityState {
  const state = { kind: 'transform' as const, position, yawDegrees, pitchDegrees };
  return { ...state, stateHash: stableHash(state) };
}

function ecrpRuntimeTransform(
  entity: RuntimeSessionEcrpEntityState,
  _capability: Extract<RuntimeSessionEcrpProjectCapabilityDefinition, { readonly kind: 'transform' }>,
  runtimeTransforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>,
): RuntimeSessionEcrpCapabilityState {
  const runtimeTransform = runtimeTransforms.get(entity.entity);
  if (runtimeTransform === undefined) {
    const orientation = sceneQuaternionToYawPitch(entity.worldTransform.rotation);
    return ecrpTransform(entity.worldTransform.translation, orientation.yawDegrees, orientation.pitchDegrees);
  }
  return ecrpTransform(runtimeTransform.position, runtimeTransform.yawDegrees, runtimeTransform.pitchDegrees);
}

function sceneQuaternionToYawPitch(
  rotation: readonly [number, number, number, number],
): { readonly yawDegrees: number; readonly pitchDegrees: number } {
  const [x, y, z, w] = rotation;
  const pitchRadians = Math.asin(Math.max(-1, Math.min(1, 2 * (w * x - y * z))));
  const yawRadians = Math.atan2(2 * (w * y + x * z), 1 - 2 * (x * x + y * y));
  return {
    yawDegrees: yawRadians * 180 / Math.PI,
    pitchDegrees: pitchRadians * 180 / Math.PI,
  };
}

function ecrpCollisionBody(
  staticCollider: boolean,
  bounds: readonly [number, number, number],
): RuntimeSessionEcrpCapabilityState {
  const state = { kind: 'collisionBody' as const, staticCollider, bounds };
  return { ...state, stateHash: stableHash(state) };
}

function ecrpController(
  controller: Extract<RuntimeSessionEcrpCapabilityState, { readonly kind: 'controller' }>['controller'],
): RuntimeSessionEcrpCapabilityState {
  const state = { kind: 'controller' as const, controller };
  return { ...state, stateHash: stableHash(state) };
}

function ecrpHealth(health: RuntimeSessionLifecycleHealthReadout): RuntimeSessionEcrpCapabilityState {
  const state = {
    kind: 'health' as const,
    current: health.current,
    max: health.max,
    dead: health.dead,
  };
  return { ...state, stateHash: stableHash(state) };
}

function ecrpWeaponMount(
  weaponId: Extract<RuntimeSessionEcrpCapabilityState, { readonly kind: 'weaponMount' }>['weaponId'],
): RuntimeSessionEcrpCapabilityState {
  const state = { kind: 'weaponMount' as const, weaponId };
  return { ...state, stateHash: stableHash(state) };
}

function ecrpRenderProjection(
  entity: RuntimeSessionEcrpEntityState,
  capability: Extract<RuntimeSessionEcrpProjectCapabilityDefinition, { readonly kind: 'renderProjection' }>,
  visible: boolean,
  runtimeTransforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>,
): RuntimeSessionEcrpCapabilityState {
  const target = ecrpRenderTargetIdentity(entity, capability, visible, runtimeTransforms);
  const state = { kind: 'renderProjection' as const, visible, projection: capability.projection, target };
  return {
    ...state,
    stateHash: stableHash({
      kind: state.kind,
      visible: state.visible,
      projection: state.projection,
      targetHash: target.targetHash,
    }),
  };
}

function ecrpRenderTargetIdentity(
  entity: RuntimeSessionEcrpEntityState,
  capability: Extract<RuntimeSessionEcrpProjectCapabilityDefinition, { readonly kind: 'renderProjection' }>,
  visible: boolean,
  runtimeTransforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>,
): RuntimeSessionEcrpRenderTargetIdentity {
  const transform = readRuntimeTransformForEntity(entity, runtimeTransforms);
  const scale = renderTargetScaleForEntity(entity);
  const targetWithoutHash = {
    kind: 'runtime_session.ecrp_render_target.v0' as const,
    targetId: `ecrp:${entity.entity}:${entity.definition.stableId}`,
    entity: entity.entity,
    definitionStableId: entity.definition.stableId,
    displayName: entity.definition.displayName,
    source: {
      projectBundle: entity.definition.source.projectBundle,
      relativePath: entity.definition.source.relativePath,
    },
    role: entity.role,
    projection: capability.projection,
    renderLabel: entity.definition.stableId,
    renderHandle: null,
    visible,
    position: transform.position,
    yawDegrees: transform.yawDegrees,
    pitchDegrees: transform.pitchDegrees,
    scale,
  };
  return {
    ...targetWithoutHash,
    targetHash: stableHash(targetWithoutHash),
  };
}

function readRuntimeTransformForEntity(
  entity: RuntimeSessionEcrpEntityState,
  runtimeTransforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>,
): RuntimeSessionEcrpTransformState {
  const runtimeTransform = runtimeTransforms.get(entity.entity);
  if (runtimeTransform !== undefined) {
    return runtimeTransform;
  }
  const transform = entity.definition.capabilities.find((capability) => capability.kind === 'transform');
  if (transform?.kind === 'transform') {
    return transform.initial;
  }
  return { position: [0, 0, 0], yawDegrees: 0, pitchDegrees: 0 };
}

function renderTargetScaleForEntity(
  entity: RuntimeSessionEcrpEntityState,
): readonly [number, number, number] | null {
  const collisionBody = entity.definition.capabilities.find((capability) => capability.kind === 'collisionBody');
  if (collisionBody?.kind !== 'collisionBody') {
    return null;
  }
  return [
    collisionBody.halfExtents[0] * 2,
    collisionBody.halfExtents[1] * 2,
    collisionBody.halfExtents[2] * 2,
  ];
}

function ecrpPolicyBinding(
  policyId: Extract<RuntimeSessionEcrpCapabilityState, { readonly kind: 'policyBinding' }>['policyId'],
): RuntimeSessionEcrpCapabilityState {
  const state = { kind: 'policyBinding' as const, policyId };
  return { ...state, stateHash: stableHash(state) };
}

function ecrpSpawnMarker(
  markerId: Extract<RuntimeSessionEcrpCapabilityState, { readonly kind: 'spawnMarker' }>['markerId'],
): RuntimeSessionEcrpCapabilityState {
  const state = { kind: 'spawnMarker' as const, markerId };
  return { ...state, stateHash: stableHash(state) };
}

function ecrpFaction(
  factionId: Extract<RuntimeSessionEcrpCapabilityState, { readonly kind: 'faction' }>['factionId'],
): RuntimeSessionEcrpCapabilityState {
  const state = { kind: 'faction' as const, factionId };
  return { ...state, stateHash: stableHash(state) };
}
