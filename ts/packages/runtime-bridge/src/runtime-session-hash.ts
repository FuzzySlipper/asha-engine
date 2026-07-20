import type {
  ParticleEmitterDescriptor,
  ParticleEmitterPatch,
  RenderFrameDiff,
  RuntimeProjectionFrame,
} from '@asha/contracts';
import type { EncounterDirectorState } from '@asha/runtime-session';
import type {
  RuntimeSessionHashRecord,
  RuntimeSessionHashValue,
  RuntimeSessionIdentity,
  RuntimeSessionLifecycleHealthReadout,
  RuntimeSessionLifecycleState,
  RuntimeSessionNonClaim,
} from '@asha/runtime-session';

// These hashes are deterministic TypeScript readout/projection fingerprints.
// Live Rust-backed authority hashes must come from bridge snapshots/results.

export function referenceRuntimeSessionNonClaims(): readonly RuntimeSessionNonClaim[] {
  return [
    'not_native_runtime',
    'not_raw_state_store',
    'not_arbitrary_json_bridge',
    'not_product_authority',
    'not_gameplay_loop',
    'not_renderer',
  ];
}
export function identityHashRecord(identity: RuntimeSessionIdentity): RuntimeSessionHashRecord {
  return {
    sessionId: identity.sessionId,
    mode: identity.mode,
    seed: identity.seed,
    project: {
      gameId: identity.project.gameId,
      workspaceId: identity.project.workspaceId,
    },
    nonClaims: identity.nonClaims,
  };
}

export function encounterStateHashRecord(state: EncounterDirectorState): RuntimeSessionHashRecord {
  return {
    presetId: state.presetId,
    status: state.status,
    spawnedEnemyIds: state.spawnedEnemyIds,
    defeatedEnemyIds: state.defeatedEnemyIds,
    revision: state.revision,
    lastTransition: state.lastTransition,
  };
}

export function lifecycleStateHashRecord(state: RuntimeSessionLifecycleState): RuntimeSessionHashRecord {
  return {
    player: lifecycleHealthHashRecord(state.player),
    enemy: lifecycleHealthHashRecord(state.enemy),
    terminalEventHash: state.terminalEvent?.eventHash ?? null,
    revision: state.revision,
  };
}

function lifecycleHealthHashRecord(health: RuntimeSessionLifecycleHealthReadout): RuntimeSessionHashRecord {
  return {
    entity: health.entity,
    current: health.current,
    max: health.max,
    dead: health.dead,
  };
}

export function renderFrameHashRecord(frame: RenderFrameDiff): RuntimeSessionHashRecord {
  return {
    opCount: frame.ops.length,
    opKinds: frame.ops.map((op) => op.op),
  };
}

export function runtimeProjectionFrameHashRecord(
  frame: RuntimeProjectionFrame,
): RuntimeSessionHashRecord {
  return {
    schemaVersion: frame.schemaVersion,
    authorityTick: frame.authorityTick,
    scene: renderFrameHashRecord(frame.scene),
    replayScope: frame.presentation.replayScope,
    presentationOps: frame.presentation.ops.map((operation) => {
      const common = {
        domain: operation.domain,
        sequence: operation.meta.sequence,
        originKind: operation.meta.origin?.kind ?? null,
        originId: operation.meta.origin?.id ?? null,
        causationId: operation.meta.origin?.causationId ?? null,
        correlationId: operation.meta.origin?.correlationId ?? null,
        op: operation.op.op,
      };
      if (operation.domain === 'audio') {
        return {
          ...common,
          signalOrHandle:
            operation.op.op === 'emit'
              ? operation.op.signalId
              : (operation.op.handle as number),
          clip:
            operation.op.op === 'emit' || operation.op.op === 'create'
              ? operation.op.descriptor.clip.asset
              : null,
          contentHash:
            operation.op.op === 'emit' || operation.op.op === 'create'
              ? operation.op.descriptor.clip.contentHash
              : null,
        };
      }
      if (operation.domain === 'billboard') {
        return {
          ...common,
          handle: operation.op.handle as number,
          contentKind:
            operation.op.op === 'create'
              ? operation.op.descriptor.content.kind
              : operation.op.op === 'update'
                ? (operation.op.patch.content?.kind ?? null)
                : null,
        };
      }
      if (operation.domain === 'particle') {
        return {
          ...common,
          signalOrHandle:
            operation.op.op === 'emit'
              ? operation.op.signalId
              : (operation.op.handle as number),
          descriptor:
            operation.op.op === 'emit' || operation.op.op === 'create'
              ? particleDescriptorHashRecord(operation.op.descriptor)
              : null,
          patch:
            operation.op.op === 'update'
              ? particlePatchHashRecord(operation.op.patch)
              : null,
        };
      }
      if (operation.domain === 'animation') {
        return {
          ...common,
          handle: operation.op.handle as number,
          target:
            operation.op.op === 'create' ? (operation.op.descriptor.target as number) : null,
          asset: operation.op.op === 'create' ? operation.op.descriptor.asset : null,
          tickDurationMillis:
            operation.op.op === 'create' ? operation.op.descriptor.tickDurationMillis : null,
          controller:
            operation.op.op === 'create'
              ? animationControllerHashRecord(operation.op.descriptor.controller)
              : operation.op.op === 'update'
                ? animationControllerHashRecord(operation.op.controller)
                : null,
        };
      }
      return {
        ...common,
        handle: operation.op.handle as number,
        title:
          operation.op.op === 'create'
            ? operation.op.descriptor.title
            : operation.op.op === 'update'
              ? (operation.op.patch.title ?? null)
              : null,
        visible:
          operation.op.op === 'create'
            ? operation.op.descriptor.visible
            : operation.op.op === 'update'
              ? (operation.op.patch.visible ?? null)
              : null,
      };
    }),
  };
}

function animationControllerHashRecord(
  controller: import('@asha/contracts').AnimationControllerProjectionState,
): RuntimeSessionHashRecord {
  return {
    graphId: controller.graphId,
    graphVersion: controller.graphVersion,
    graphHash: controller.graphHash,
    stateId: controller.stateId,
    revision: controller.revision,
    stateHash: controller.stateHash,
    motion: animationMotionHashRecord(controller.motion),
    transition: controller.transition === null ? null : {
      transitionId: controller.transition.transitionId,
      fromStateId: controller.transition.fromStateId,
      toStateId: controller.transition.toStateId,
      elapsedTicks: controller.transition.elapsedTicks,
      durationTicks: controller.transition.durationTicks,
      targetMotion: animationMotionHashRecord(controller.transition.targetMotion),
    },
    timingFact: controller.timingFact === null ? null : {
      factId: controller.timingFact.factId,
      sourceFactId: controller.timingFact.sourceFactId,
      authorityTick: controller.timingFact.authorityTick,
      controllerInputSequence: controller.timingFact.controllerInputSequence,
      controllerTick: controller.timingFact.controllerTick,
      causationId: controller.timingFact.causationId,
      correlationId: controller.timingFact.correlationId,
      transitionId: controller.timingFact.transitionId,
      fromStateId: controller.timingFact.fromStateId,
      toStateId: controller.timingFact.toStateId,
      moment: controller.timingFact.moment,
      durationTicks: controller.timingFact.durationTicks,
      factHash: controller.timingFact.factHash,
    },
  };
}

function animationMotionHashRecord(
  motion: import('@asha/contracts').AnimationResolvedMotion,
): RuntimeSessionHashRecord {
  return {
    clipA: motion.clipA,
    clipB: motion.clipB,
    blendWeightMilli: motion.blendWeightMilli,
    speedMilli: motion.speedMilli,
  };
}

function particleDescriptorHashRecord(
  descriptor: ParticleEmitterDescriptor,
): RuntimeSessionHashRecord {
  return {
    anchor: particleAnchorHashRecord(descriptor.anchor),
    sprite: {
      asset: descriptor.sprite.asset,
      contentHash: descriptor.sprite.contentHash,
      frameCount: descriptor.sprite.frameCount,
    },
    ratePerSecond: descriptor.ratePerSecond,
    burstCount: descriptor.burstCount,
    lifetimeSeconds: descriptor.lifetimeSeconds,
    velocityMin: descriptor.velocityMin,
    velocityMax: descriptor.velocityMax,
    acceleration: descriptor.acceleration,
    sizeCurve: descriptor.sizeCurve.map((key) => ({ age: key.age, value: key.value })),
    colorCurve: descriptor.colorCurve.map((key) => ({ age: key.age, color: key.color })),
    flipbookFramesPerSecond: descriptor.flipbookFramesPerSecond,
    seed: descriptor.seed,
    maxParticles: descriptor.maxParticles,
    visible: descriptor.visible,
  };
}

function particlePatchHashRecord(patch: ParticleEmitterPatch): RuntimeSessionHashRecord {
  return {
    anchor: patch.anchor === null ? null : particleAnchorHashRecord(patch.anchor),
    sprite: patch.sprite === null ? null : {
      asset: patch.sprite.asset,
      contentHash: patch.sprite.contentHash,
      frameCount: patch.sprite.frameCount,
    },
    ratePerSecond: patch.ratePerSecond,
    burstCount: patch.burstCount,
    lifetimeSeconds: patch.lifetimeSeconds,
    velocityMin: patch.velocityMin,
    velocityMax: patch.velocityMax,
    acceleration: patch.acceleration,
    sizeCurve: patch.sizeCurve?.map((key) => ({ age: key.age, value: key.value })) ?? null,
    colorCurve: patch.colorCurve?.map((key) => ({ age: key.age, color: key.color })) ?? null,
    flipbookFramesPerSecond: patch.flipbookFramesPerSecond,
    maxParticles: patch.maxParticles,
    visible: patch.visible,
  };
}

function particleAnchorHashRecord(
  anchor: ParticleEmitterDescriptor['anchor'],
): RuntimeSessionHashRecord {
  return anchor.kind === 'world'
    ? { kind: anchor.kind, position: anchor.position }
    : { kind: anchor.kind, entity: anchor.entity, offset: anchor.offset };
}

export function stableHash(value: RuntimeSessionHashValue | undefined): string {
  return `fnv1a64:${fnv1a64(stableStringify(value))}`;
}

function stableStringify(value: RuntimeSessionHashValue | undefined): string {
  if (value === undefined) {
    return 'undefined';
  }
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    const entries = value as readonly RuntimeSessionHashValue[];
    return `[${entries.map((entry) => stableStringify(entry)).join(',')}]`;
  }
  const record = value as RuntimeSessionHashRecord;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(',')}}`;
}

function fnv1a64(text: string): string {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= BigInt(text.charCodeAt(index));
    hash = (hash * prime) & mask;
  }
  return hash.toString(16).padStart(16, '0');
}
