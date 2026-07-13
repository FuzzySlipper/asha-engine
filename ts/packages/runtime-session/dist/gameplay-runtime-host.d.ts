import type { GameplayCausationRef, GameplayContractRef, GameplayEmitterRef, GameplayEventEnvelope, GameplayHeaderSelector, GameplayModuleBindingRegistry, GameplayOwnerRef, GameplayProposalEnvelope, GameplayTriggerDefinition, PrefabTransform } from '@asha/contracts';
export interface GameplayRuntimePrefabBootstrap {
    readonly registryJson: string;
    readonly catalog: {
        readonly assetIds: readonly string[];
        readonly entityDefinitionIds: readonly string[];
    };
    readonly placements: readonly GameplayRuntimePrefabPlacement[];
}
export interface GameplayRuntimePrefabPlacement {
    readonly commandId: string;
    readonly origin: 'authored' | 'player';
    readonly instance: number;
    readonly prefab: number;
    readonly seed: number;
    readonly transform: PrefabTransform;
    readonly overrides: readonly ({
        readonly targetRole: string;
    } & ({
        readonly field: 'transform';
        readonly transform: PrefabTransform;
    } | {
        readonly field: 'entityDefinition';
        readonly stableId: string;
    } | {
        readonly field: 'asset';
        readonly asset: string;
    } | {
        readonly field: 'material';
        readonly asset: string;
    } | {
        readonly field: 'activation';
        readonly active: boolean;
    }))[];
}
export interface GameplayRuntimeHostLoadInput {
    readonly kind: 'gameplay_runtime_host.load.v1';
    readonly projectId: string;
    readonly compositionHash: string;
    readonly declaredReadPlanHash: string;
    readonly bindings: GameplayModuleBindingRegistry;
    readonly triggers: readonly GameplayTriggerDefinition[];
    readonly scheduler: GameplayRuntimeSchedulerDefinition;
    readonly prefabs?: GameplayRuntimePrefabBootstrap;
}
export interface GameplayRuntimeSchedulerDefinition {
    readonly owner: GameplayOwnerRef;
    readonly declaredEvents: readonly GameplayContractRef[];
    readonly declaredProposals: readonly GameplayContractRef[];
}
export type GameplayRuntimeSchedulerCommand = {
    readonly kind: 'scheduleTick';
    readonly action: GameplayRuntimeTickScheduledActionDraft;
} | {
    readonly kind: 'scheduleEventConditioned';
    readonly action: GameplayRuntimeEventConditionedActionDraft;
} | {
    readonly kind: 'executeTick';
    readonly actionId: string;
    readonly tick: number;
    readonly targetsPresent: boolean;
    readonly causationCurrent: boolean;
} | {
    readonly kind: 'triggerEvent';
    readonly actionId: string;
    readonly event: GameplayEventEnvelope;
    readonly targetsPresent: boolean;
    readonly causationCurrent: boolean;
} | {
    readonly kind: 'timeout';
    readonly actionId: string;
    readonly tick: number;
} | {
    readonly kind: 'cancel';
    readonly actionId: string;
    readonly reason: string;
};
export interface GameplayRuntimeTickScheduledActionDraft {
    readonly id: string;
    readonly executeAt: number;
    readonly priority: number;
    readonly proposal: GameplayProposalEnvelope;
    readonly source: GameplayEmitterRef;
    readonly causation: GameplayCausationRef;
}
export interface GameplayRuntimeEventConditionedActionDraft {
    readonly id: string;
    readonly condition: {
        readonly event: GameplayContractRef;
        readonly selector: GameplayHeaderSelector;
    };
    readonly priority: number;
    readonly proposal: GameplayProposalEnvelope;
    readonly timeoutAt: number | null;
    readonly source: GameplayEmitterRef;
    readonly causation: GameplayCausationRef;
}
export type GameplayRuntimeHostMoment = {
    readonly kind: 'tick';
    readonly tick: number;
} | {
    readonly kind: 'actorMovement';
    readonly tick: number;
    readonly actor: number;
    readonly delta: readonly [number, number, number];
} | {
    readonly kind: 'ownerEvent';
    readonly event: GameplayEventEnvelope;
} | {
    readonly kind: 'prefabInteraction';
    readonly tick: number;
    readonly instance: number;
    readonly role: string;
} | {
    readonly kind: 'schedulerCommand';
    readonly command: GameplayRuntimeSchedulerCommand;
} | {
    readonly kind: 'schedulerRoute';
    readonly actionId: string;
};
export interface GameplayRuntimeRoutingReadout {
    readonly proposalId: string;
    readonly proposalKind: string;
    readonly ownerId: string;
    readonly accepted: boolean;
    readonly proposalHash: string;
    readonly routingHash: string;
    readonly diagnosticCodes: readonly string[];
}
export interface GameplayRuntimeReactionFrameReadout {
    readonly frameHash: string;
    readonly registryDigest: string;
    readonly deliveredEvents: readonly GameplayEventEnvelope[];
    readonly frozenViewHashes: readonly string[];
    readonly invocationOutputHashes: readonly string[];
    readonly routing: readonly GameplayRuntimeRoutingReadout[];
    readonly acceptedModuleFactHashes: readonly string[];
    readonly stateHashBefore: string;
    readonly stateHashAfter: string;
    readonly finalSessionHash: string;
    readonly diagnosticCodes: readonly string[];
}
export interface GameplayRuntimeHostReadout {
    readonly kind: 'gameplay_runtime_host.readout.v1';
    readonly gameplayRegistryDigest: string;
    readonly bindingRegistryHash: string;
    readonly activationHash: string;
    readonly moduleStateHash: string;
    readonly authorityStateHash: string;
    readonly triggerRevision: number;
    readonly triggerSnapshotHash: string;
    readonly activeOverlapCount: number;
    readonly reactionFrameCount: number;
    readonly lastReactionFrameHash: string | null;
    readonly recentFrames: readonly GameplayRuntimeReactionFrameReadout[];
    readonly scheduler: GameplayRuntimeSchedulerReadout;
    readonly runtimeHostHash: string;
    readonly prefabs?: GameplayRuntimePrefabReadout;
    readonly moduleStates?: readonly GameplayRuntimeModuleStateReadout[];
}
export interface GameplayRuntimeSchedulerReadout {
    readonly ownerId: string;
    readonly stateHash: string;
    readonly pendingActionCount: number;
    readonly outstandingDispatchCount: number;
    readonly factCount: number;
    readonly pendingActions: readonly GameplayRuntimeScheduledAction[];
    readonly outstandingDispatches: readonly GameplayRuntimeScheduledDispatch[];
    readonly truncated: boolean;
}
export type GameplayRuntimeScheduledAction = ({
    readonly kind: 'tick';
    readonly id: string;
    readonly executeAt: number;
} & GameplayRuntimeScheduledActionCommon) | ({
    readonly kind: 'eventConditioned';
    readonly id: string;
    readonly condition: {
        readonly event: GameplayContractRef;
        readonly selector: GameplayHeaderSelector;
    };
    readonly timeoutAt: number | null;
} & GameplayRuntimeScheduledActionCommon);
interface GameplayRuntimeScheduledActionCommon {
    readonly priority: number;
    readonly insertionSequence: number;
    readonly proposal: GameplayProposalEnvelope;
    readonly source: GameplayEmitterRef;
    readonly causation: GameplayCausationRef;
}
export interface GameplayRuntimeScheduledDispatch {
    readonly actionId: string;
    readonly proposal: GameplayProposalEnvelope;
    readonly proposalHash: string;
    readonly priority: number;
    readonly insertionSequence: number;
}
export interface GameplayRuntimePrefabReadout {
    readonly stateHash: string;
    readonly acceptedCommands: readonly {
        readonly commandId: string;
        readonly instance: number;
        readonly prefab: number;
        readonly origin: 'authored' | 'player';
    }[];
    readonly instances: readonly GameplayRuntimePrefabInstanceReadout[];
}
export interface GameplayRuntimePrefabInstanceReadout {
    readonly instance: number;
    readonly prefab: number;
    readonly origin: 'authored' | 'player';
    readonly provenanceHash: string;
    readonly overrideCount: number;
    readonly parts: readonly {
        readonly part: number;
        readonly namespace: string;
        readonly entity: number;
        readonly parentEntity: number | null;
        readonly translation: readonly [number, number, number];
        readonly sourceKind: 'scene' | 'entityDefinition' | 'voxelObject';
        readonly active: boolean;
        readonly roles: readonly string[];
    }[];
    readonly roles: readonly {
        readonly role: string;
        readonly entity: number;
    }[];
}
export interface GameplayRuntimeModuleStateReadout {
    readonly moduleId: string;
    readonly stateContract: string;
    readonly scope: {
        readonly kind: 'session';
    } | {
        readonly kind: 'entity';
        readonly entity: number;
    } | {
        readonly kind: 'prefabInstance';
        readonly instance: number;
    };
    readonly revision: number;
    readonly stateHash: string;
    readonly initializedFrom: string;
}
export interface GameplayRuntimeHostLoadReceipt {
    readonly kind: 'gameplay_runtime_host.load_receipt.v1';
    readonly accepted: boolean;
    readonly diagnostics: readonly string[];
    readonly readout: GameplayRuntimeHostReadout | null;
}
export interface GameplayRuntimeHostAdvanceReceipt {
    readonly kind: 'gameplay_runtime_host.advance_receipt.v1';
    readonly accepted: boolean;
    readonly diagnostics: readonly string[];
    readonly moment: GameplayRuntimeHostMoment;
    readonly frames: readonly GameplayRuntimeReactionFrameReadout[];
    readonly readout: GameplayRuntimeHostReadout;
}
export interface GameplayRuntimeHostSnapshot {
    readonly kind: 'gameplay_runtime_host.snapshot.v1';
    readonly canonicalText: string;
    readonly snapshotHash: string;
}
/**
 * Consumer-owned native host port. A downstream provider statically links its
 * Rust modules and implements this closed transport; TypeScript never supplies
 * callbacks or an authority mutation function.
 */
export interface GameplayRuntimeHostTransport {
    load(input: GameplayRuntimeHostLoadInput): GameplayRuntimeHostLoadReceipt;
    advance(moment: GameplayRuntimeHostMoment): GameplayRuntimeHostAdvanceReceipt;
    read(): GameplayRuntimeHostReadout;
    save(): GameplayRuntimeHostSnapshot;
    restore(input: GameplayRuntimeHostLoadInput, snapshot: GameplayRuntimeHostSnapshot): GameplayRuntimeHostLoadReceipt;
}
export {};
//# sourceMappingURL=gameplay-runtime-host.d.ts.map