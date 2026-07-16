import type { NativeAddon } from '@asha/native-bridge';

type ComposedGameplayHandlers = Pick<
  NativeAddon,
  | 'readComposedRuntimeSession'
  | 'readGameplayModuleView'
  | 'applyGameplayPrefabPartInteraction'
>;

export function createNativeComposedGameplayHandlers(
  calls: string[],
  hashA: string,
  hashB: string,
  hashC: string,
): ComposedGameplayHandlers {
  return {
    readComposedRuntimeSession: (handle: number) => {
      void handle;
      calls.push('composedRead');
      return {
        schemaVersion: 1,
        entityAuthorityHash: hashA,
        gameplay: {
          gameplayRegistryDigest: hashA,
          semanticCompatibilityDigest: hashB,
          artifactProvenanceDigest: hashA,
          compositionLoadMode: 'compatible',
          compatibilityDiagnostics: [],
          bindingRegistryHash: hashA,
          activationHash: hashA,
          moduleStateHash: hashA,
          authorityStateHash: hashA,
          triggerRevision: 0,
          triggerSnapshotHash: hashA,
          activeOverlapCount: 0,
          reactionFrameCount: 1,
          lastReactionFrameHash: hashB,
          decisionReceiptCount: 1,
          pendingDecisionCount: 0,
          lastDecisionReceiptHash: hashC,
          schedulerStateHash: hashA,
          schedulerPendingActionCount: 0,
          schedulerOutstandingDispatchCount: 0,
          schedulerOutstandingEventDeliveryCount: 0,
          schedulerFactCount: 0,
          schedulerTruncated: false,
          runtimeHostHash: hashB,
        },
        fpsSessionEpoch: 1,
        fpsReplayHash: hashC,
        runtimeSessionHash: hashA,
      };
    },
    readGameplayModuleView: (
      handle,
      namespace,
      name,
      version,
      schemaHash,
      scopeKind,
      scopeValue,
      expectedRuntimeSessionHash,
    ) => {
      void handle;
      calls.push(`moduleView:${namespace}:${name}:${scopeKind}:${scopeValue ?? 'none'}`);
      return {
        view: { namespace, name, version, schemaHash },
        providerId: 'provider.fixture-pulse',
        scopeKind,
        scopeValue: scopeValue ?? null,
        revision: 1,
        canonicalPayload: Uint8Array.from([52]),
        viewHash: hashB,
        runtimeSessionHash: expectedRuntimeSessionHash,
      };
    },
    applyGameplayPrefabPartInteraction: (
      handle,
      actor,
      instance,
      role,
      expectedTarget,
      tick,
      expectedRuntimeSessionHash,
    ) => {
      void handle;
      void tick;
      calls.push(`prefabInteraction:${actor}:${instance}:${role}:${expectedTarget}`);
      return {
        actor,
        instance,
        role,
        target: expectedTarget,
        eventHash: hashB,
        reactionFrameHash: hashC,
        runtimeSessionHash: expectedRuntimeSessionHash,
      };
    },
  };
}
