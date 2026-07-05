import type { CameraProjectionSnapshot } from '@asha/contracts';

import type {
  CombatReadoutScenario,
  CombatRuntimeReadout,
} from './combat-readout.js';
import type {
  RuntimeActionIntentEnvelope,
  RuntimeActionIntentRejection,
  RuntimeActionIntentStatus,
} from './runtime-action.js';

export type CombatFeedbackProjectionKind = 'combat_feedback_projection.v0';

export type CombatFeedbackScenario = CombatReadoutScenario | 'runtime_action_unsupported';

export type CombatFeedbackTraceResult = 'hit' | 'miss' | 'not_fired';

export type CombatFeedbackMarkerTone = 'hit' | 'blocked' | 'inactive';

export type CombatFeedbackHudTone = 'info' | 'warning' | 'danger';

export type CombatFeedbackFixturePath =
  | 'harness/fixtures/combat-feedback/generated-tunnel-hit-feedback.snapshot.txt'
  | 'harness/fixtures/combat-feedback/generated-tunnel-miss-feedback.snapshot.txt'
  | null;

export interface CombatFeedbackIntentInput {
  readonly envelope: RuntimeActionIntentEnvelope;
  readonly accepted: boolean;
  readonly status: RuntimeActionIntentStatus;
  readonly rejection: RuntimeActionIntentRejection | null;
}

export interface CombatFeedbackActionReceiptInput extends CombatFeedbackIntentInput {
  readonly sequenceId: number;
  readonly combatReadout: CombatRuntimeReadout | null;
}

export interface CombatFeedbackProjectionInput extends CombatFeedbackIntentInput {
  readonly sequenceId: number;
  readonly combatReadout: CombatRuntimeReadout | null;
  readonly camera?: CameraProjectionSnapshot | null;
}

export interface CombatFeedbackIntentProjection {
  readonly kind: 'combat_feedback.intent.v0';
  readonly action: RuntimeActionIntentEnvelope['action'];
  readonly phase: RuntimeActionIntentEnvelope['phase'];
  readonly tick: number;
  readonly source: RuntimeActionIntentEnvelope['source'];
  readonly accepted: boolean;
  readonly status: RuntimeActionIntentStatus;
  readonly rejectionReason: RuntimeActionIntentRejection['reason'] | null;
}

export interface CombatFeedbackTraceProjection {
  readonly kind: 'combat_feedback.trace.v0';
  readonly result: CombatFeedbackTraceResult;
  readonly shooter: number | null;
  readonly target: number | null;
  readonly reason: 'geometryBlocked' | 'noTarget' | 'intent_not_accepted' | null;
  readonly distance: number | null;
  readonly origin: readonly [number, number, number] | null;
  readonly direction: readonly [number, number, number] | null;
  readonly endpoint: readonly [number, number, number] | null;
  readonly cameraProjectionHash: string | null;
}

export interface CombatFeedbackMarkerProjection {
  readonly kind: 'combat_feedback.marker.v0';
  readonly visible: boolean;
  readonly tone: CombatFeedbackMarkerTone;
  readonly label: 'Hit' | 'Blocked' | 'Unavailable';
  readonly durationMs: 0 | 120 | 160;
  readonly screenAnchor: {
    readonly space: 'normalized';
    readonly x: 0.5;
    readonly y: 0.5;
  };
}

export interface CombatFeedbackNotificationProjection {
  readonly id: string;
  readonly tone: CombatFeedbackHudTone;
  readonly text: string;
  readonly entity: number | null;
  readonly eventKind: string;
}

export interface CombatFeedbackHudStatusDescriptor {
  readonly id: string;
  readonly tone: CombatFeedbackHudTone;
  readonly text: string;
}

export interface CombatFeedbackHudProjection {
  readonly reticle: {
    readonly tone: CombatFeedbackMarkerTone;
    readonly pulseMs: 0 | 120 | 160;
    readonly label: CombatFeedbackMarkerProjection['label'];
  };
  readonly status: readonly CombatFeedbackHudStatusDescriptor[];
  readonly ammo: number | null;
  readonly cooldownTicksRemaining: number | null;
}

export interface CombatFeedbackProjection {
  readonly kind: CombatFeedbackProjectionKind;
  readonly sequenceId: number;
  readonly scenario: CombatFeedbackScenario;
  readonly intent: CombatFeedbackIntentProjection;
  readonly trace: CombatFeedbackTraceProjection;
  readonly marker: CombatFeedbackMarkerProjection;
  readonly notifications: readonly CombatFeedbackNotificationProjection[];
  readonly hud: CombatFeedbackHudProjection;
  readonly health: CombatRuntimeReadout['health'];
  readonly debug: {
    readonly fixturePath: CombatFeedbackFixturePath;
    readonly combatReplayHash: string | null;
    readonly healthHash: string | null;
    readonly cameraProjectionHash: string | null;
    readonly viewport: {
      readonly width: number;
      readonly height: number;
    } | null;
  };
  readonly hashes: {
    readonly traceHash: string;
    readonly markerHash: string;
    readonly notificationHash: string;
    readonly projectionHash: string;
  };
  readonly nonClaims: readonly [
    'not_combat_authority',
    'not_renderer_state',
    'not_ui_state',
    'not_animation_or_audio',
  ];
}

type CombatFeedbackHashPrimitive = string | number | boolean | null;
type CombatFeedbackHashValue =
  | CombatFeedbackHashPrimitive
  | readonly CombatFeedbackHashValue[]
  | object;

export function buildCombatFeedbackProjectionFromReceipt(
  receipt: CombatFeedbackActionReceiptInput,
  camera: CameraProjectionSnapshot | null = null,
): CombatFeedbackProjection {
  return buildCombatFeedbackProjection({
    sequenceId: receipt.sequenceId,
    envelope: receipt.envelope,
    accepted: receipt.accepted,
    status: receipt.status,
    rejection: receipt.rejection,
    combatReadout: receipt.combatReadout,
    camera,
  });
}

export function buildCombatFeedbackProjection(
  input: CombatFeedbackProjectionInput,
): CombatFeedbackProjection {
  const intent = projectIntent(input);
  const scenario = input.combatReadout?.scenario ?? 'runtime_action_unsupported';
  const trace = projectTrace(input.combatReadout, input.camera);
  const marker = projectMarker(trace);
  const notifications = projectNotifications(input.combatReadout, intent);
  const hud = projectHud(input.combatReadout, marker, notifications);
  const traceHash = stableHash(trace);
  const markerHash = stableHash(marker);
  const notificationHash = stableHash(notifications);
  const debug = {
    fixturePath: feedbackFixturePath(input.combatReadout),
    combatReplayHash: input.combatReadout?.replayHash ?? null,
    healthHash: input.combatReadout?.healthHash ?? null,
    cameraProjectionHash: input.camera?.projectionHash ?? null,
    viewport:
      input.camera === null || input.camera === undefined
        ? null
        : {
            width: input.camera.viewport.width,
            height: input.camera.viewport.height,
          },
  };
  const projectionHash = stableHash({
    kind: 'combat_feedback_projection.v0',
    sequenceId: input.sequenceId,
    scenario,
    intent,
    traceHash,
    markerHash,
    notificationHash,
    healthHash: debug.healthHash,
    combatReplayHash: debug.combatReplayHash,
    cameraProjectionHash: debug.cameraProjectionHash,
  });

  return {
    kind: 'combat_feedback_projection.v0',
    sequenceId: input.sequenceId,
    scenario,
    intent,
    trace,
    marker,
    notifications,
    hud,
    health: input.combatReadout?.health ?? [],
    debug,
    hashes: {
      traceHash,
      markerHash,
      notificationHash,
      projectionHash,
    },
    nonClaims: [
      'not_combat_authority',
      'not_renderer_state',
      'not_ui_state',
      'not_animation_or_audio',
    ],
  };
}

export function defaultCombatFeedbackIntent(input: {
  readonly camera: RuntimeActionIntentEnvelope['camera'];
  readonly tick: number;
}): CombatFeedbackIntentInput {
  return {
    envelope: {
      kind: 'runtime_action_intent.v0',
      action: 'primary_fire',
      phase: 'pressed',
      camera: input.camera,
      tick: input.tick,
      source: 'programmatic',
      pressed: true,
    },
    accepted: true,
    status: 'accepted',
    rejection: null,
  };
}

function projectIntent(input: CombatFeedbackProjectionInput): CombatFeedbackIntentProjection {
  return {
    kind: 'combat_feedback.intent.v0',
    action: input.envelope.action,
    phase: input.envelope.phase,
    tick: input.envelope.tick,
    source: input.envelope.source,
    accepted: input.accepted,
    status: input.status,
    rejectionReason: input.rejection?.reason ?? null,
  };
}

function projectTrace(
  readout: CombatRuntimeReadout | null,
  camera: CameraProjectionSnapshot | null | undefined,
): CombatFeedbackTraceProjection {
  const origin = camera?.pose.position ?? null;
  const direction = camera === null || camera === undefined ? null : cameraForward(camera);
  const distance = readout?.outcome.kind === 'hit' ? readout.outcome.distance : null;
  const endpoint =
    origin !== null && direction !== null && distance !== null
      ? [
          round3(origin[0] + direction[0] * distance),
          round3(origin[1] + direction[1] * distance),
          round3(origin[2] + direction[2] * distance),
        ] as const
      : null;

  if (readout === null) {
    return {
      kind: 'combat_feedback.trace.v0',
      result: 'not_fired',
      shooter: null,
      target: null,
      reason: 'intent_not_accepted',
      distance: null,
      origin,
      direction,
      endpoint: null,
      cameraProjectionHash: camera?.projectionHash ?? null,
    };
  }

  if (readout.outcome.kind === 'hit') {
    const hitEvent = readout.events.find((event) => event.kind === 'fire_hit');
    return {
      kind: 'combat_feedback.trace.v0',
      result: 'hit',
      shooter: hitEvent?.kind === 'fire_hit' ? hitEvent.shooter : null,
      target: readout.outcome.target,
      reason: null,
      distance,
      origin,
      direction,
      endpoint,
      cameraProjectionHash: camera?.projectionHash ?? null,
    };
  }

  const missedEvent = readout.events.find((event) => event.kind === 'fire_missed');
  return {
    kind: 'combat_feedback.trace.v0',
    result: 'miss',
    shooter: missedEvent?.kind === 'fire_missed' ? missedEvent.shooter : 10,
    target: null,
    reason: readout.outcome.reason,
    distance: null,
    origin,
    direction,
    endpoint: null,
    cameraProjectionHash: camera?.projectionHash ?? null,
  };
}

function projectMarker(trace: CombatFeedbackTraceProjection): CombatFeedbackMarkerProjection {
  const common = {
    kind: 'combat_feedback.marker.v0',
    screenAnchor: {
      space: 'normalized',
      x: 0.5,
      y: 0.5,
    },
  } as const;

  if (trace.result === 'hit') {
    return {
      ...common,
      visible: true,
      tone: 'hit',
      label: 'Hit',
      durationMs: 160,
    };
  }

  if (trace.result === 'miss') {
    return {
      ...common,
      visible: true,
      tone: 'blocked',
      label: 'Blocked',
      durationMs: 120,
    };
  }

  return {
    ...common,
    visible: false,
    tone: 'inactive',
    label: 'Unavailable',
    durationMs: 0,
  };
}

function projectNotifications(
  readout: CombatRuntimeReadout | null,
  intent: CombatFeedbackIntentProjection,
): readonly CombatFeedbackNotificationProjection[] {
  if (readout === null) {
    return [
      {
        id: `combat-${intent.action}-unsupported`,
        tone: 'warning',
        text: `${intent.action} unavailable`,
        entity: null,
        eventKind: 'runtime_action_unsupported',
      },
    ];
  }

  return readout.events.map((event): CombatFeedbackNotificationProjection => {
    switch (event.kind) {
      case 'fire_hit':
        return {
          id: `combat-hit-${event.target}`,
          tone: 'info',
          text: `Hit entity ${event.target}`,
          entity: event.target,
          eventKind: event.kind,
        };
      case 'fire_missed':
        return {
          id: 'combat-miss-geometry',
          tone: 'warning',
          text: event.reason === 'geometryBlocked' ? 'Shot blocked' : 'No target hit',
          entity: null,
          eventKind: event.kind,
        };
      case 'damage_applied':
        return {
          id: `combat-damage-${event.target}`,
          tone: 'danger',
          text: `Entity ${event.target} -${event.amount}`,
          entity: event.target,
          eventKind: event.kind,
        };
      case 'entity_defeated':
        return {
          id: `combat-defeated-${event.target}`,
          tone: 'danger',
          text: `Entity ${event.target} defeated`,
          entity: event.target,
          eventKind: event.kind,
        };
    }
  });
}

function projectHud(
  readout: CombatRuntimeReadout | null,
  marker: CombatFeedbackMarkerProjection,
  notifications: readonly CombatFeedbackNotificationProjection[],
): CombatFeedbackHudProjection {
  const primaryNotification = notifications.at(-1);
  const fallbackText =
    marker.tone === 'hit' ? 'Hit confirmed' : marker.tone === 'blocked' ? 'Shot blocked' : 'Action unavailable';
  return {
    reticle: {
      tone: marker.tone,
      pulseMs: marker.durationMs,
      label: marker.label,
    },
    status: [
      {
        id: 'combat-feedback',
        tone: primaryNotification?.tone ?? (marker.tone === 'inactive' ? 'warning' : 'info'),
        text: primaryNotification?.text ?? fallbackText,
      },
    ],
    ammo: readout?.nextFireControl.ammo ?? null,
    cooldownTicksRemaining: readout?.nextFireControl.cooldownTicksRemaining ?? null,
  };
}

function feedbackFixturePath(readout: CombatRuntimeReadout | null): CombatFeedbackFixturePath {
  if (readout?.scenario === 'generated_tunnel_fire_hit') {
    return 'harness/fixtures/combat-feedback/generated-tunnel-hit-feedback.snapshot.txt';
  }

  if (readout?.scenario === 'generated_tunnel_geometry_blocked_miss') {
    return 'harness/fixtures/combat-feedback/generated-tunnel-miss-feedback.snapshot.txt';
  }

  return null;
}

function cameraForward(camera: CameraProjectionSnapshot): readonly [number, number, number] {
  const yawRadians = (camera.pose.yawDegrees * Math.PI) / 180;
  const pitchRadians = (camera.pose.pitchDegrees * Math.PI) / 180;
  const cosPitch = Math.cos(pitchRadians);
  return [
    round3(Math.sin(yawRadians) * cosPitch),
    round3(Math.sin(pitchRadians)),
    round3(-Math.cos(yawRadians) * cosPitch),
  ];
}

function round3(value: number): number {
  return Math.round(value * 1000) / 1000;
}

function stableHash(value: CombatFeedbackHashValue | undefined): string {
  const json = stableStringify(value);
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;

  for (let index = 0; index < json.length; index += 1) {
    hash ^= BigInt(json.charCodeAt(index));
    hash = (hash * prime) & mask;
  }

  return `fnv1a64:${hash.toString(16).padStart(16, '0')}`;
}

function stableStringify(value: CombatFeedbackHashValue | undefined): string {
  if (value === undefined) {
    return 'undefined';
  }

  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }

  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(',')}]`;
  }

  const record = value as Record<string, CombatFeedbackHashValue | undefined>;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(',')}}`;
}
