import type {
  ProjectSourceBody,
  RuntimeProjectDiagnostic,
  RuntimeProjectLifecycleVersion,
  RuntimeProjectLoadReceipt,
  RuntimeProjectSourceAdapterInput,
  RuntimeProjectSourceAdapterKind,
} from '@asha/contracts';
import { loadAshaProjectSource } from '@asha/game-workspace';
import type { RuntimeSessionProjectLoadInput } from '@asha/runtime-session';

import type { RuntimeBridge } from './bridge.js';

/**
 * The one ordinary host-side project loading path. It materializes only the
 * manifest-authorized byte closure, lends each body through bounded binary
 * transport, and asks Rust to compile/link/activate it atomically.
 */
export async function loadRuntimeSessionProject(
  bridge: RuntimeBridge,
  input: RuntimeSessionProjectLoadInput,
  expectedLifecycle: RuntimeProjectLifecycleVersion,
): Promise<RuntimeProjectLoadReceipt> {
  const provisionalSource = sourceAdapterInput(
    input.source.kind,
    input.source.identity,
    'unavailable',
  );
  let loaded;
  try {
    loaded = await loadAshaProjectSource(input.source);
  } catch (error) {
    return rejectedLoad(
      provisionalSource,
      expectedLifecycle,
      sourceDiagnostic(
        'sourceAdapterRejected',
        error instanceof Error ? error.message : String(error),
      ),
    );
  }

  const source = sourceAdapterInput(
    loaded.sourceKind,
    loaded.sourceIdentity,
    loaded.materializationHash,
  );
  try {
    const transaction = bridge.beginRuntimeProjectSourceResources({
      manifestJson: loaded.manifestJson,
    });
    const bodies: ProjectSourceBody[] = [];
    try {
      for (const file of loaded.files) {
        bodies.push({
          kind: 'resource',
          path: file.path,
          resource: bridge.stageRuntimeProjectSourceResource({
            generation: transaction.generation,
            path: file.path,
            bytes: file.bytes,
          }),
        });
      }
    } catch (error) {
      // Submit the incomplete generation so Rust deterministically aborts every
      // staged handle before this host-side failure is surfaced.
      bridge.admitRuntimeProjectSourceBatch({
        manifestJson: loaded.manifestJson,
        resourceGeneration: transaction.generation,
        bodies,
      });
      return rejectedLoad(
        source,
        expectedLifecycle,
        sourceDiagnostic(
          'sourceTransportRejected',
          error instanceof Error ? error.message : String(error),
          'sourceBatch',
        ),
      );
    }
    const admission = bridge.admitRuntimeProjectSourceBatch({
      manifestJson: loaded.manifestJson,
      resourceGeneration: transaction.generation,
      bodies,
    });
    if (!admission.accepted) {
      return {
        accepted: false,
        source,
        activeProject: null,
        lifecycle: expectedLifecycle,
        diagnostics: admission.diagnostics.map((diagnostic) => ({
          phase: 'sourceBatch',
          code: diagnostic.code,
          documentId: null,
          path: diagnostic.path,
          message: diagnostic.message,
        })),
      };
    }
    return bridge.loadRuntimeProject({ source, expectedLifecycle });
  } catch (error) {
    return rejectedLoad(
      source,
      expectedLifecycle,
      sourceDiagnostic(
        'sourceTransportRejected',
        error instanceof Error ? error.message : String(error),
        'sourceBatch',
      ),
    );
  }
}

function sourceAdapterInput(
  kind: RuntimeSessionProjectLoadInput['source']['kind'],
  identity: string,
  materializationHash: string,
): RuntimeProjectSourceAdapterInput {
  return {
    kind: generatedSourceKind(kind),
    identity,
    materializationHash,
  };
}

function generatedSourceKind(
  kind: RuntimeSessionProjectLoadInput['source']['kind'],
): RuntimeProjectSourceAdapterKind {
  switch (kind) {
    case 'development-directory': return 'developmentDirectory';
    case 'packaged-directory':
    case 'packaged-archive': return 'packagedProject';
    case 'memory': return 'inMemory';
  }
}

function sourceDiagnostic(
  code: string,
  message: string,
  phase: RuntimeProjectDiagnostic['phase'] = 'sourceAdapter',
): RuntimeProjectDiagnostic {
  return {
    phase,
    code,
    documentId: null,
    path: null,
    message,
  };
}

function rejectedLoad(
  source: RuntimeProjectSourceAdapterInput,
  lifecycle: RuntimeProjectLifecycleVersion,
  diagnostic: RuntimeProjectDiagnostic,
): RuntimeProjectLoadReceipt {
  return {
    accepted: false,
    source,
    activeProject: null,
    lifecycle,
    diagnostics: [diagnostic],
  };
}
