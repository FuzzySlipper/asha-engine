import type { FlatSceneDocument, GameRuleModuleManifest, SceneTransform } from '@asha/contracts';
import type { RuntimeSessionMode, RuntimeSessionProjectIdentity } from './facade-core.js';
import type {
  RuntimeSessionLifecycleEventKind,
  RuntimeSessionLifecycleRole,
} from './facade-lifecycle.js';
import type { RuntimeSessionEcrpRenderTargetIdentity } from './ecrp-render-target.js';
import type { ProjectBundleLoadRequest } from './transport-contracts.js';
import type { FpsBootstrapResolutionRegistry } from './transport-contracts.js';

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
  readonly projectBundle: ProjectBundleLoadRequest | null;
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

export type RuntimeSessionEcrpProjectDiagnosticCode =
  | 'duplicateEntityDefinition'
  | 'duplicatePlacement'
  | 'emptyEntityDefinitionList'
  | 'invalidGameRuleModuleManifest'
  | 'invalidBootstrapResolutionRegistry'
  | 'invalidCapability'
  | 'missingCapability'
  | 'missingEntityDefinition'
  | 'missingPlacement'
  | 'missingProjectBundle'
  | 'unknownEntityDefinition';

export interface RuntimeSessionEcrpProjectDiagnostic {
  readonly code: RuntimeSessionEcrpProjectDiagnosticCode;
  readonly path: string;
  readonly detail: string;
}

export type RuntimeSessionEcrpProjectCapabilityDefinition =
  | {
      readonly kind: 'transform';
      readonly initial: {
        readonly position: readonly [number, number, number];
        readonly yawDegrees: number;
        readonly pitchDegrees: number;
      };
    }
  | {
      readonly kind: 'collisionBody';
      readonly halfExtents: readonly [number, number, number];
      readonly staticCollider?: boolean;
      readonly policy?: object;
    }
  | {
      readonly kind: 'controller';
      readonly controller: 'player_input' | 'enemy_policy';
      readonly tuning?: object;
    }
  | {
      readonly kind: 'health';
      readonly current: number;
      readonly max: number;
    }
  | {
      readonly kind: 'weaponMount';
      readonly weaponId: string;
      readonly tuning?: object;
    }
  | {
      readonly kind: 'renderProjection';
      readonly projection: 'first_person_camera' | 'target_cube' | 'spawn_marker';
      readonly visible?: boolean;
    }
  | {
      readonly kind: 'policyBinding';
      readonly policyId: string;
      readonly policyLoopRef?: string;
    }
  | {
      readonly kind: 'spawnMarker';
      readonly markerId: string;
    }
  | {
      readonly kind: 'faction';
      readonly factionId: string;
    };

export interface RuntimeSessionEcrpEntityDefinition {
  readonly kind: 'EntityDefinition';
  readonly stableId: string;
  readonly displayName: string;
  readonly source: {
    readonly projectBundle: string;
    readonly relativePath: string;
  };
  readonly capabilities: readonly RuntimeSessionEcrpProjectCapabilityDefinition[];
}

/** @deprecated Compatibility-only compiled bootstrap input. Persist typed
 * ProjectBundle content and use `RuntimeSessionFacade.loadProject` instead. */
export interface RuntimeSessionEcrpProjectLoadInput {
  readonly kind: 'runtime_session.load_ecrp_project.v0';
  readonly projectBundle: {
    readonly kind: 'ProjectBundle';
    readonly project: RuntimeSessionProjectIdentity;
    readonly runtimeRequest: ProjectBundleLoadRequest;
  };
  readonly bootstrapResolutionRegistry: FpsBootstrapResolutionRegistry;
  readonly entityDefinitions: readonly RuntimeSessionEcrpEntityDefinition[];
  readonly sceneDocument: FlatSceneDocument;
  readonly gameRuleModules?: readonly GameRuleModuleManifest[];
}

export interface RuntimeSessionEcrpProjectLoadReceipt {
  readonly kind: 'runtime_session.ecrp_project_load_receipt.v0';
  readonly sequenceId: number;
  readonly accepted: boolean;
  readonly diagnostics: readonly RuntimeSessionEcrpProjectDiagnostic[];
  readonly entityCount: number;
  readonly bootstrapHash: string | null;
  readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
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
  readonly input: RuntimeSessionEcrpProjectLoadInput | null;
  readonly entities: readonly RuntimeSessionEcrpEntityState[];
  readonly bootstrapHash: string;
}
