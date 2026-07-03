import type { CameraHandle } from '@asha/contracts';

export type RuntimeActionIntentKind = 'primary_fire' | 'use';
export type RuntimeActionIntentPhase = 'pressed' | 'released';
export type RuntimeActionIntentSource = 'browser_fps_pointer' | 'programmatic';

export interface RuntimeActionIntentEnvelope {
  readonly kind: 'runtime_action_intent.v0';
  readonly action: RuntimeActionIntentKind;
  readonly phase: RuntimeActionIntentPhase;
  readonly camera: CameraHandle;
  readonly tick: number;
  readonly source: RuntimeActionIntentSource;
  readonly pressed: boolean;
}

export type RuntimeActionIntentRejectionReason =
  | 'combat_runtime_not_wired'
  | 'invalid_runtime_action_intent';

export interface RuntimeActionIntentRejection {
  readonly reason: RuntimeActionIntentRejectionReason;
  readonly detail: string;
}

export type RuntimeActionIntentStatus = 'accepted' | 'unsupported';
