import type {
  GameExtensionHookReceipt,
  GameExtensionReplayEvidence,
  GameRuleCatalog,
  GameRuleDiagnostic,
  GameRuleEvidenceRef,
  GameRuleModifierState,
  GameRuleModuleManifest,
  GameRuleResolutionRequest,
  GameRuleTraceEntry,
  GameplayContractRef,
  GameplayCompositionDiagnostic,
  GameplayCompositionLoadMode,
  WeaponEffectHookRequest,
} from '@asha/contracts';

export type EngineHandle = number & { readonly __brand: 'EngineHandle' };
export type FrameCursor = number & { readonly __brand: 'FrameCursor' };

export interface StepResult {
  readonly tick: number;
  readonly diffCount: number;
}

export type BridgeVec3 = readonly [number, number, number];
export type EnemyDirectNavAuthoritySource = 'seeded_from_request' | 'rust_entity_store';
export type EnemyDirectNavAuthorityTransport = 'native_rust' | 'reference_bridge';

export interface EnemyDirectNavMovementRequest {
  readonly entity: number;
  readonly seedPosition: BridgeVec3;
  readonly target: BridgeVec3;
  readonly maxStepUnits: number;
}

export interface EnemyDirectNavMovementResult {
  readonly entity: number;
  readonly authoritySource: EnemyDirectNavAuthoritySource;
  readonly authorityTransport: EnemyDirectNavAuthorityTransport;
  readonly from: BridgeVec3;
  readonly target: BridgeVec3;
  readonly nextWaypoint: BridgeVec3;
  readonly distanceUnits: number;
  readonly reached: boolean;
  readonly pathHash: string;
  readonly transformHash: string;
  readonly projectionChanged: boolean;
}

export type FpsRuntimeRole = 'player' | 'enemy' | 'neutral';
export type FpsRuntimeAuthorityTransport = 'native_rust' | 'reference_bridge';

export interface FpsTransformCapability {
  readonly translation: BridgeVec3;
  readonly rotation: readonly [number, number, number, number];
  readonly scale: BridgeVec3;
}

export interface FpsBoundsCapability {
  readonly min: BridgeVec3;
  readonly max: BridgeVec3;
}

export interface FpsHealth {
  readonly current: number;
  readonly max: number;
}

export interface FpsWeaponMount {
  readonly weaponId: string;
  readonly damage: number;
  readonly rangeUnits: number;
  readonly ammo: number;
  readonly cooldownTicksAfterFire: number;
}

export interface FpsPolicyBinding {
  readonly bindingId: string;
  readonly policyId: string;
  readonly viewKind: string;
  readonly viewVersion: string;
  readonly allowedIntents: readonly string[];
  readonly runtimeMoment: string;
}

export interface FpsStoredEntityDefinition {
  readonly entity: number;
  readonly stableId: string;
  readonly displayName: string;
  readonly sourcePath: string;
  readonly tags: readonly string[];
  readonly role: FpsRuntimeRole;
  readonly transform: FpsTransformCapability | null;
  readonly bounds: FpsBoundsCapability | null;
  readonly renderVisible: boolean | null;
  readonly staticCollider: boolean | null;
  readonly health: FpsHealth | null;
  readonly weapon: FpsWeaponMount | null;
  readonly policyBinding: FpsPolicyBinding | null;
}

export interface FpsRuntimeSessionLoadRequest {
  readonly projectBundle: string;
  readonly definitions: readonly FpsStoredEntityDefinition[];
  readonly gameRuleModules: readonly GameRuleModuleManifest[];
}

export interface FpsRuntimeSessionRestartRequest {
  readonly expectedEpoch: number;
}

export interface FpsPrimaryFireRequest {
  readonly tick: number;
  readonly origin: BridgeVec3;
  readonly direction: BridgeVec3;
  readonly shooterRole?: FpsRuntimeRole;
  readonly targetRole?: FpsRuntimeRole;
}

export type FpsLifecycleStatus =
  | { readonly state: 'active' }
  | { readonly state: 'enemy_defeated'; readonly entity: number; readonly tick: number };

export interface FpsReadSetEvidence {
  readonly viewKind: string;
  readonly owner: string;
  readonly readSet: readonly string[];
}

export interface FpsReplayEvidence {
  readonly replayUnit: string;
  readonly entityHash: string;
  readonly healthHash: string;
  readonly recordHash: string;
}

export interface FpsEntityHealthReadout {
  readonly entity: number;
  readonly current: number;
  readonly max: number;
}

export interface FpsPolicyBindingReadout extends FpsPolicyBinding {
  readonly entity: number;
}

export interface FpsRuntimeSessionSnapshot {
  readonly backend: FpsRuntimeAuthorityTransport;
  readonly authoritySurface: string;
  readonly projectBundle: string;
  readonly sessionEpoch: number;
  readonly lifecycleStatus: FpsLifecycleStatus;
  readonly playerEntity: number;
  readonly enemyEntity: number;
  readonly health: readonly FpsEntityHealthReadout[];
  readonly policyBindings: readonly FpsPolicyBindingReadout[];
  readonly replayRecords: readonly FpsReplayEvidence[];
  readonly readSets: readonly FpsReadSetEvidence[];
  readonly entityHash: string;
  readonly healthHash: string;
  readonly replayHash: string;
}

export interface FpsPrimaryFireResult {
  readonly backend: FpsRuntimeAuthorityTransport;
  readonly authoritySurface: string;
  readonly mutationOwner: string;
  readonly workspaceTrace: readonly string[];
  readonly shooter: number;
  readonly target: number | null;
  readonly targetHealthBefore: FpsHealth | null;
  readonly targetHealthAfter: FpsHealth | null;
  readonly lifecycleStatus: FpsLifecycleStatus;
  readonly targetRenderVisible: boolean | null;
  readonly entityHash: string;
  readonly healthHash: string;
  readonly replayHash: string;
}

export type GameplayModuleViewScope =
  | { readonly kind: 'session' }
  | { readonly kind: 'entity'; readonly entity: number }
  | { readonly kind: 'prefabInstance'; readonly instance: number };

export interface GameplayModuleViewRequest {
  readonly view: GameplayContractRef;
  readonly scope: GameplayModuleViewScope;
  readonly expectedRuntimeSessionHash: string;
}

export interface GameplayModuleViewSnapshot {
  readonly view: GameplayContractRef;
  readonly providerId: string;
  readonly scope: GameplayModuleViewScope;
  readonly revision: number;
  readonly canonicalPayload: readonly number[];
  readonly viewHash: string;
  readonly runtimeSessionHash: string;
}

export interface ComposedGameplayReadout {
  readonly gameplayRegistryDigest: string;
  readonly semanticCompatibilityDigest: string;
  readonly artifactProvenanceDigest: string;
  readonly compositionLoadMode: GameplayCompositionLoadMode;
  readonly compatibilityDiagnostics: readonly GameplayCompositionDiagnostic[];
  readonly bindingRegistryHash: string;
  readonly activationHash: string;
  readonly moduleStateHash: string;
  readonly authorityStateHash: string;
  readonly triggerRevision: number;
  readonly triggerSnapshotHash: string;
  readonly activeOverlapCount: number;
  readonly reactionFrameCount: number;
  readonly lastReactionFrameHash: string | null;
  readonly decisionReceiptCount: number;
  readonly pendingDecisionCount: number;
  readonly lastDecisionReceiptHash: string | null;
  readonly schedulerStateHash: string;
  readonly schedulerPendingActionCount: number;
  readonly schedulerOutstandingDispatchCount: number;
  readonly schedulerOutstandingEventDeliveryCount: number;
  readonly schedulerFactCount: number;
  readonly schedulerTruncated: boolean;
  readonly runtimeHostHash: string;
}

export interface ComposedRuntimeSessionReadout {
  readonly schemaVersion: number;
  readonly entityAuthorityHash: string;
  readonly gameplay: ComposedGameplayReadout;
  readonly fpsSessionEpoch: number;
  readonly fpsReplayHash: string | null;
  readonly runtimeSessionHash: string;
}

export interface GameplayPrefabPartInteractionRequest {
  readonly actor: number;
  readonly instance: number;
  readonly role: string;
  readonly expectedTarget: number;
  readonly tick: number;
  readonly expectedRuntimeSessionHash: string;
}

export interface GameplayPrefabPartInteractionReceipt {
  readonly actor: number;
  readonly instance: number;
  readonly role: string;
  readonly target: number;
  readonly eventHash: string;
  readonly reactionFrameHash: string;
  readonly runtimeSessionHash: string;
}

export interface GameExtensionWeaponEffectInvocationRequest {
  readonly hook: WeaponEffectHookRequest;
  readonly primaryFire: FpsPrimaryFireRequest;
}

export interface GameExtensionWeaponEffectInvocationResult {
  readonly hookReceipt: GameExtensionHookReceipt;
  readonly replayEvidence: GameExtensionReplayEvidence;
  readonly primaryFire: FpsPrimaryFireResult | null;
}

export interface GameRuleCatalogValidationReceipt {
  readonly accepted: boolean;
  readonly catalogHash: string;
  readonly diagnostics: readonly GameRuleDiagnostic[];
  readonly trace: readonly GameRuleTraceEntry[];
  readonly evidence: readonly GameRuleEvidenceRef[];
}

export interface GameRuleEffectIntentRequest {
  readonly catalog: GameRuleCatalog;
  readonly request: GameRuleResolutionRequest;
}

export interface GameRuleRuntimeReadout {
  readonly backend: FpsRuntimeAuthorityTransport;
  readonly authoritySurface: string;
  readonly activeModifiers: readonly GameRuleModifierState[];
  readonly recentTrace: readonly GameRuleTraceEntry[];
  readonly recentReplayHashes: readonly string[];
  readonly latestReplayHash: string | null;
}

export type FpsEncounterStatus = 'pending' | 'active' | 'cleared' | 'failed';
export type FpsEncounterLastTransition = 'initialized' | 'activated' | 'cleared' | 'failed' | 'reset';
export type FpsEncounterTransitionAction = 'activate' | 'sync_lifecycle' | 'reset';

export interface FpsEncounterLifecycleInput {
  readonly outcomeKind: 'in_progress' | 'won' | 'lost';
  readonly terminal: boolean;
  readonly enemyDead: boolean;
  readonly playerDead: boolean;
  readonly lifecycleHash: string;
}

export interface FpsEncounterTransitionRequest {
  readonly presetId: string;
  readonly action: FpsEncounterTransitionAction;
  readonly lifecycle: FpsEncounterLifecycleInput;
}

export interface FpsEncounterStateReadout {
  readonly presetId: string;
  readonly status: FpsEncounterStatus;
  readonly spawnedEnemyIds: readonly string[];
  readonly defeatedEnemyIds: readonly string[];
  readonly revision: number;
  readonly lastTransition: FpsEncounterLastTransition;
}

export interface FpsEncounterDirectorSnapshot {
  readonly backend: FpsRuntimeAuthorityTransport;
  readonly authoritySurface: string;
  readonly mutationOwner: string;
  readonly workspaceTrace: readonly string[];
  readonly state: FpsEncounterStateReadout;
  readonly lifecycle: FpsEncounterLifecycleInput;
  readonly readSets: readonly FpsReadSetEvidence[];
  readonly encounterHash: string;
  readonly replayHash: string;
}

export interface FpsEncounterTransitionResult {
  readonly backend: FpsRuntimeAuthorityTransport;
  readonly authoritySurface: string;
  readonly mutationOwner: string;
  readonly workspaceTrace: readonly string[];
  readonly accepted: boolean;
  readonly rejectionReason:
    | 'encounter_not_pending'
    | 'invalid_encounter_transition'
    | 'unknown_encounter_preset'
    | null;
  readonly eventKind:
    | 'runtime_encounter.activated.v0'
    | 'runtime_encounter.lifecycle_synced.v0'
    | 'runtime_encounter.reset.v0'
    | null;
  readonly state: FpsEncounterStateReadout;
  readonly lifecycle: FpsEncounterLifecycleInput;
  readonly encounterHash: string;
  readonly replayHash: string;
}

export interface ProjectBundleLoadRequest {
  readonly bundleSchemaVersion: number;
  readonly protocolVersion: number;
  readonly sceneId: number;
}

export interface CompositionStatus {
  readonly loadedProjectBundle: number | null;
  readonly fatalCount: number;
  readonly totalCount: number;
  readonly blocksLoad: boolean;
}
