import type {
  ActiveRuntimeProjectContentReadout,
  EntityDefinition,
  EntityDefinitionCapability,
  FlatSceneDocument,
  SceneNodeRecord,
  SceneTransform,
} from '@asha/contracts';
import { lifecycleHealth } from './runtime-session-lifecycle.js';
import { stableHash } from './runtime-session-hash.js';
import type {
  RuntimeSessionEcrpCapabilityState,
  RuntimeSessionEcrpEntityEventReadout,
  RuntimeSessionEcrpEntityReadout,
  RuntimeSessionEcrpRenderTargetIdentity,
  RuntimeSessionEcrpReadout,
  RuntimeSessionIdentity,
  RuntimeSessionLifecycleHealthReadout,
  RuntimeSessionLifecycleRole,
  RuntimeSessionLifecycleState,
} from '@asha/runtime-session';

export type RuntimeSessionEcrpProjectCapabilityDefinition =
  | { readonly kind: 'transform'; readonly initial: { readonly position: readonly [number, number, number]; readonly yawDegrees: number; readonly pitchDegrees: number } }
  | { readonly kind: 'collisionBody'; readonly halfExtents: readonly [number, number, number]; readonly staticCollider?: boolean; readonly policy?: object }
  | { readonly kind: 'controller'; readonly controller: 'player_input' | 'enemy_policy'; readonly tuning?: object }
  | { readonly kind: 'health'; readonly current: number; readonly max: number }
  | { readonly kind: 'weaponMount'; readonly weaponId: string; readonly tuning?: object }
  | { readonly kind: 'renderProjection'; readonly projection: 'first_person_camera' | 'target_cube' | 'spawn_marker'; readonly visible?: boolean }
  | { readonly kind: 'policyBinding'; readonly policyId: string; readonly policyLoopRef?: string }
  | { readonly kind: 'spawnMarker'; readonly markerId: string }
  | { readonly kind: 'faction'; readonly factionId: string };

export interface RuntimeSessionEcrpEntityDefinition {
  readonly kind: 'EntityDefinition';
  readonly stableId: string;
  readonly displayName: string;
  readonly source: { readonly projectBundle: string; readonly relativePath: string };
  readonly capabilities: readonly RuntimeSessionEcrpProjectCapabilityDefinition[];
}

export interface RuntimeSessionEcrpEntityState {
  readonly entity: number;
  readonly instanceId: string;
  readonly spawnMarkerId: string | null;
  readonly worldTransform: SceneTransform;
  readonly definition: RuntimeSessionEcrpEntityDefinition;
  readonly role: RuntimeSessionLifecycleRole | 'neutral';
}

export interface RuntimeSessionEcrpTransformState {
  readonly position: readonly [number, number, number];
  readonly yawDegrees: number;
  readonly pitchDegrees: number;
}

export interface RuntimeSessionEcrpProjectState {
  readonly entities: readonly RuntimeSessionEcrpEntityState[];
  readonly contentHash: string;
}

/* Caller-built ECRP bootstrap was removed. This module only projects admitted Rust content. */

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
  const roleByEntity = new Map(
    readout.activeDomains.flatMap((domain) =>
      domain.entityRoles.map((entry) => [entry.entity, entry.role] as const),
    ),
  );
  return buildEcrpProjectStateFromParts(
    definitions,
    readout.entryScene,
    roleByEntity,
    readout.contentSetHash,
  );
}

function buildEcrpProjectStateFromParts(
  entityDefinitions: readonly RuntimeSessionEcrpEntityDefinition[],
  sceneDocument: FlatSceneDocument,
  canonicalRoleByEntity: ReadonlyMap<number, RuntimeSessionEcrpEntityState['role']>,
  contentHash: string,
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
      role: canonicalRoleByEntity.get(placement.id) ?? 'neutral',
    };
  });
  return {
    entities,
    contentHash,
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
  readonly projectState: RuntimeSessionEcrpProjectState;
  readonly lifecycleState: RuntimeSessionLifecycleState;
  readonly runtimeTransforms?: ReadonlyMap<number, RuntimeSessionEcrpTransformState>;
  readonly sequenceId: number;
  readonly tick: number;
  readonly sessionHash: string;
  readonly authority?: RuntimeSessionEcrpReadout['authority'];
}): RuntimeSessionEcrpReadout {
  const entities = input.projectState.entities.map((entity) =>
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
