import type {
  WorkspaceAuthoringProjectBundleRef,
  WorkspaceAuthoringProjectIdentity,
  WorkspaceAuthoringStateSummary,
} from '@asha/contracts';

import { RuntimeBridgeError } from './bridge.js';

/**
 * Package-private transport key for canonical WorkspaceAuthoring.openProject.
 *
 * It is deliberately absent from the package barrel, generated RuntimeBridge,
 * bridge manifest, and ordinary string-key enumeration. Browser host mirrors
 * this key on its capability-bound private adapter route.
 */
export const WORKSPACE_AUTHORING_OPEN_ADAPTER = Symbol.for(
  'asha.runtime_bridge.private.workspace_authoring_open.v1',
);

export interface WorkspaceAuthoringOpenAdapterRequest {
  readonly authoringId: string;
  readonly seed: number;
  readonly project: WorkspaceAuthoringProjectIdentity;
  readonly projectBundle: WorkspaceAuthoringProjectBundleRef;
}

export interface WorkspaceAuthoringOpenAdapterTransport {
  [WORKSPACE_AUTHORING_OPEN_ADAPTER](
    input: WorkspaceAuthoringOpenAdapterRequest,
  ): WorkspaceAuthoringStateSummary;
}

export function openWorkspaceAuthoringAdapter(
  bridge: object,
  input: WorkspaceAuthoringOpenAdapterRequest,
): WorkspaceAuthoringStateSummary {
  const transport = bridge as Partial<WorkspaceAuthoringOpenAdapterTransport>;
  const open = transport[WORKSPACE_AUTHORING_OPEN_ADAPTER];
  if (typeof open !== 'function') {
    throw new RuntimeBridgeError(
      'operation_unimplemented',
      'RuntimeBridge provider does not expose the private canonical workspace-authoring adapter.',
      { operation: 'WorkspaceAuthoring.openProject', provenance: 'transport_loader' },
    );
  }
  return Reflect.apply(open, bridge, [input]) as WorkspaceAuthoringStateSummary;
}
