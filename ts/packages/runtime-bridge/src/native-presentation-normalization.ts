import type {
  AudioSourcePatch,
  BillboardContent,
  BillboardPatch,
  ParticleEmitterPatch,
  PresentationOp,
  PresentationOpMeta,
  PresentationOriginRef,
  RuntimeProjectionFrame,
  TelemetryOverlayPatch,
} from '@asha/contracts';

import { RuntimeBridgeError } from './bridge.js';
import { decodeRenderFrameDiff } from './render-decode.js';

type AudioPresentationOp = Extract<PresentationOp, { readonly domain: 'audio' }>;
type BillboardPresentationOp = Extract<PresentationOp, { readonly domain: 'billboard' }>;
type ParticlePresentationOp = Extract<PresentationOp, { readonly domain: 'particle' }>;
type TelemetryOverlayPresentationOp = Extract<
  PresentationOp,
  { readonly domain: 'telemetryOverlay' }
>;
type AnimationPresentationOp = Extract<PresentationOp, { readonly domain: 'animation' }>;

interface NativePresentationOpDto {
  readonly domain: string;
  readonly meta: NativePresentationOpMetaDto;
  readonly audioOp?: AudioPresentationOp['op'];
  readonly billboardOp?: BillboardPresentationOp['op'];
  readonly particleOp?: ParticlePresentationOp['op'];
  readonly telemetryOverlayOp?: TelemetryOverlayPresentationOp['op'];
  readonly animationOp?: AnimationPresentationOp['op'];
}

type NativePresentationOriginDto = Omit<PresentationOriginRef, 'causationId' | 'correlationId'> & {
  readonly causationId?: string;
  readonly correlationId?: string;
};

type NativePresentationOpMetaDto = Omit<PresentationOpMeta, 'origin'> & {
  readonly origin?: NativePresentationOriginDto;
};

export interface NativeRuntimeProjectionFrameDto {
  readonly schemaVersion: number;
  readonly authorityTick: number;
  readonly scene: {
    readonly frameJson: string;
  };
  readonly presentation: {
    readonly replayScope: RuntimeProjectionFrame['presentation']['replayScope'];
    readonly ops: readonly NativePresentationOpDto[];
  };
}

export function projectionFrameFromNative(
  native: NativeRuntimeProjectionFrameDto,
): RuntimeProjectionFrame {
  if (native.schemaVersion !== 1 || !Number.isSafeInteger(native.authorityTick)) {
    throw new RuntimeBridgeError('internal', 'native projection frame header is invalid');
  }
  if (native.presentation?.replayScope !== 'excludedFromReplayTruth') {
    throw new RuntimeBridgeError('internal', 'native projection replay scope is invalid');
  }
  if (!Array.isArray(native.presentation.ops)) {
    throw new RuntimeBridgeError('internal', 'native projection operations must be an array');
  }
  let scene: RuntimeProjectionFrame['scene'];
  try {
    scene = decodeRenderFrameDiff(JSON.parse(native.scene?.frameJson), '$.scene');
  } catch {
    throw new RuntimeBridgeError('internal', 'native scene projection frame is not valid JSON');
  }
  const nativeOperations = native.presentation.ops as unknown as readonly NativePresentationOpDto[];
  const ops = nativeOperations.map((operation, index): PresentationOp => {
    if (operation.meta?.sequence !== index) {
      throw new RuntimeBridgeError('internal', 'native presentation sequence is not contiguous');
    }
    const meta = presentationOpMetaFromNative(operation.meta);
    if (
      operation.domain === 'audio'
      && operation.audioOp !== undefined
      && operation.billboardOp === undefined
      && operation.particleOp === undefined
      && operation.telemetryOverlayOp === undefined
      && operation.animationOp === undefined
    ) {
      return { domain: 'audio', meta, op: audioProjectionOperationFromNative(operation.audioOp) };
    }
    if (
      operation.domain === 'billboard'
      && operation.billboardOp !== undefined
      && operation.audioOp === undefined
      && operation.particleOp === undefined
      && operation.telemetryOverlayOp === undefined
      && operation.animationOp === undefined
    ) {
      return { domain: 'billboard', meta, op: billboardProjectionOperationFromNative(operation.billboardOp) };
    }
    if (
      operation.domain === 'particle'
      && operation.particleOp !== undefined
      && operation.audioOp === undefined
      && operation.billboardOp === undefined
      && operation.telemetryOverlayOp === undefined
      && operation.animationOp === undefined
    ) {
      return { domain: 'particle', meta, op: particleProjectionOperationFromNative(operation.particleOp) };
    }
    if (
      operation.domain === 'telemetryOverlay'
      && operation.telemetryOverlayOp !== undefined
      && operation.audioOp === undefined
      && operation.billboardOp === undefined
      && operation.particleOp === undefined
      && operation.animationOp === undefined
    ) {
      return {
        domain: 'telemetryOverlay',
        meta,
        op: telemetryOverlayProjectionOperationFromNative(operation.telemetryOverlayOp),
      };
    }
    if (
      operation.domain === 'animation'
      && operation.animationOp !== undefined
      && operation.audioOp === undefined
      && operation.billboardOp === undefined
      && operation.particleOp === undefined
      && operation.telemetryOverlayOp === undefined
    ) {
      return {
        domain: 'animation',
        meta,
        op: animationProjectionOperationFromNative(operation.animationOp),
      };
    }
    throw new RuntimeBridgeError(
      'internal',
      `native presentation operation ${index} has an invalid closed-domain payload`,
    );
  });
  return {
    schemaVersion: native.schemaVersion,
    authorityTick: native.authorityTick,
    scene,
    presentation: {
      replayScope: native.presentation.replayScope,
      ops,
    },
  };
}

function presentationOpMetaFromNative(meta: NativePresentationOpMetaDto): PresentationOpMeta {
  const origin = meta.origin;
  return {
    sequence: meta.sequence,
    origin: origin === undefined
      ? null
      : {
          ...origin,
          causationId: origin.causationId ?? null,
          correlationId: origin.correlationId ?? null,
        },
  };
}

function audioProjectionOperationFromNative(
  operation: AudioPresentationOp['op'],
): AudioPresentationOp['op'] {
  if (operation.op !== 'update') return operation;
  const patch = operation.patch as Partial<AudioSourcePatch>;
  return {
    ...operation,
    patch: {
      volume: patch.volume ?? null,
      pitch: patch.pitch ?? null,
      looping: patch.looping ?? null,
      spatialBlend: patch.spatialBlend ?? null,
      attenuation: patch.attenuation ?? null,
      pan: patch.pan ?? null,
      emitter: patch.emitter ?? null,
    },
  };
}

function billboardProjectionOperationFromNative(
  operation: BillboardPresentationOp['op'],
): BillboardPresentationOp['op'] {
  if (operation.op === 'create') {
    return {
      ...operation,
      descriptor: {
        ...operation.descriptor,
        content: billboardContentFromNative(operation.descriptor.content),
      },
    };
  }
  if (operation.op !== 'update') return operation;
  const patch = operation.patch as Partial<BillboardPatch>;
  return {
    ...operation,
    patch: {
      anchor: patch.anchor ?? null,
      content: patch.content == null ? null : billboardContentFromNative(patch.content),
      font: patch.font ?? null,
      heightPixels: patch.heightPixels ?? null,
      color: patch.color ?? null,
      background: patch.background ?? null,
      maxDistance: patch.maxDistance ?? null,
      layer: patch.layer ?? null,
      visible: patch.visible ?? null,
    },
  };
}

function billboardContentFromNative(content: BillboardContent): BillboardContent {
  if (content.kind !== 'value') return content;
  return {
    ...content,
    unitKey: content.unitKey ?? null,
    fallbackUnit: content.fallbackUnit ?? null,
  };
}

function particleProjectionOperationFromNative(
  operation: ParticlePresentationOp['op'],
): ParticlePresentationOp['op'] {
  if (operation.op !== 'update') return operation;
  const patch = operation.patch as Partial<ParticleEmitterPatch>;
  return {
    ...operation,
    patch: {
      anchor: patch.anchor ?? null,
      sprite: patch.sprite ?? null,
      ratePerSecond: patch.ratePerSecond ?? null,
      burstCount: patch.burstCount ?? null,
      lifetimeSeconds: patch.lifetimeSeconds ?? null,
      velocityMin: patch.velocityMin ?? null,
      velocityMax: patch.velocityMax ?? null,
      acceleration: patch.acceleration ?? null,
      sizeCurve: patch.sizeCurve ?? null,
      colorCurve: patch.colorCurve ?? null,
      flipbookFramesPerSecond: patch.flipbookFramesPerSecond ?? null,
      maxParticles: patch.maxParticles ?? null,
      visible: patch.visible ?? null,
    },
  };
}

function telemetryOverlayProjectionOperationFromNative(
  operation: TelemetryOverlayPresentationOp['op'],
): TelemetryOverlayPresentationOp['op'] {
  if (operation.op !== 'update') return operation;
  const patch = operation.patch as Partial<TelemetryOverlayPatch>;
  return {
    ...operation,
    patch: {
      title: patch.title ?? null,
      corner: patch.corner ?? null,
      refreshIntervalMs: patch.refreshIntervalMs ?? null,
      maxFrameTimeSamples: patch.maxFrameTimeSamples ?? null,
      visible: patch.visible ?? null,
    },
  };
}

function animationProjectionOperationFromNative(
  operation: AnimationPresentationOp['op'],
): AnimationPresentationOp['op'] {
  if (operation.op === 'destroy') return operation;
  if (operation.op === 'create') {
    if (operation.descriptor === undefined) {
      throw new RuntimeBridgeError('internal', 'native animation create descriptor is missing');
    }
    return {
      ...operation,
      descriptor: {
        ...operation.descriptor,
        controller: animationControllerFromNative(operation.descriptor.controller),
      },
    };
  }
  if (operation.controller === undefined) {
    throw new RuntimeBridgeError('internal', 'native animation update controller is missing');
  }
  return {
    ...operation,
    controller: animationControllerFromNative(operation.controller),
  };
}

function animationControllerFromNative(
  controller: import('@asha/contracts').AnimationControllerProjectionState,
): import('@asha/contracts').AnimationControllerProjectionState {
  const native = controller as unknown as {
    transition?: import('@asha/contracts').AnimationTransitionProjection;
    timingFact?: import('@asha/contracts').AnimationTransitionFactRef;
  } & Omit<import('@asha/contracts').AnimationControllerProjectionState, 'transition' | 'timingFact'>;
  return {
    ...native,
    motion: { ...native.motion, clipB: native.motion.clipB ?? null },
    transition: native.transition === undefined
      ? null
      : {
          ...native.transition,
          targetMotion: {
            ...native.transition.targetMotion,
            clipB: native.transition.targetMotion.clipB ?? null,
          },
        },
    timingFact: native.timingFact ?? null,
  };
}
