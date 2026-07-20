import type { RuntimeSessionMode, RuntimeSessionProjectIdentity } from './facade-core.js';
import type { RuntimeSessionLifecycleEventKind } from './facade-lifecycle.js';
import type { RuntimeSessionEcrpRenderTargetIdentity } from './ecrp-render-target.js';

export type RuntimeSessionEcrpCapabilityKind =
  | 'transform'
  | 'collisionBody'
  | 'controller'
  | 'health'
  | 'weaponMount'
  | 'renderProjection'
  | 'policyBinding'
  | 'spawnMarker'
  | 'faction';

export type RuntimeSessionEcrpCapabilityState =
  | {
      readonly kind: 'transform';
      readonly position: readonly [number, number, number];
      readonly yawDegrees: number;
      readonly pitchDegrees: number;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'collisionBody';
      readonly staticCollider: boolean;
      readonly bounds: readonly [number, number, number];
      readonly stateHash: string;
    }
  | {
      readonly kind: 'controller';
      readonly controller: 'player_input' | 'enemy_policy';
      readonly stateHash: string;
    }
  | {
      readonly kind: 'health';
      readonly current: number;
      readonly max: number;
      readonly dead: boolean;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'weaponMount';
      readonly weaponId: string;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'renderProjection';
      readonly visible: boolean;
      readonly projection: 'first_person_camera' | 'target_cube' | 'spawn_marker';
      readonly target: RuntimeSessionEcrpRenderTargetIdentity;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'policyBinding';
      readonly policyId: string;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'spawnMarker';
      readonly markerId: string;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'faction';
      readonly factionId: string;
      readonly stateHash: string;
    };

export interface RuntimeSessionEcrpEntityEventReadout {
  readonly kind: RuntimeSessionLifecycleEventKind | 'runtime_session.bootstrap_entity.v0';
  readonly entity: number;
  readonly tick: number;
  readonly eventHash: string;
}

export type { RuntimeSessionEcrpRenderTargetIdentity } from './ecrp-render-target.js';

export interface RuntimeSessionEcrpEntityReadout {
  readonly entity: number;
  readonly lifecycle: 'active' | 'tombstoned';
  readonly definitionStableId: string;
  readonly displayName: string;
  readonly source: {
    readonly projectBundle: string;
    readonly relativePath: string;
  };
  readonly capabilityKinds: readonly RuntimeSessionEcrpCapabilityKind[];
  readonly capabilities: readonly RuntimeSessionEcrpCapabilityState[];
  readonly recentEvents: readonly RuntimeSessionEcrpEntityEventReadout[];
  readonly entityHash: string;
}

export interface RuntimeSessionEcrpReadout {
  readonly kind: 'runtime_session.ecrp_readout.v0';
  readonly sequenceId: number;
  readonly tick: number;
  readonly sessionHash: string;
  readonly authority: {
    readonly mode: RuntimeSessionMode;
    readonly source: 'reference_fixture' | 'rust_bridge';
    readonly surface: string;
    readonly readSets: readonly {
      readonly viewKind: string;
      readonly owner: string;
      readonly readSet: readonly string[];
    }[];
  };
  readonly project: RuntimeSessionProjectIdentity;
  readonly entities: readonly RuntimeSessionEcrpEntityReadout[];
  readonly entityCount: number;
  readonly hashes: {
    readonly entityReadoutHash: string;
    readonly capabilityStateHash: string;
    readonly eventReadoutHash: string;
  };
  readonly nonClaims: readonly [
    'not_raw_state_store',
    'not_authoring_mode',
    'not_demo_local_authority',
  ];
}
