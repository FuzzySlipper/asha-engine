import type {
  CommandBatch,
  CommandResult,
  RenderFrameDiff,
} from '@asha/contracts';

export const ASHA_DEVTOOLS_PROTOCOL_VERSION = 'devtools-protocol.v0';

export interface DevtoolsCompatibilityMetadata {
  readonly protocolVersion: typeof ASHA_DEVTOOLS_PROTOCOL_VERSION;
  readonly contractsCompatibility: string;
  readonly runtimeBridgeCompatibility: string;
  readonly publishArtifactFormat: string;
}

export type DevtoolsRuntimeMode = 'native' | 'reference' | 'mock' | 'degraded';

export interface DevtoolsRuntimeIdentity {
  readonly engineVersion: string;
  readonly gameId: string;
  readonly workspaceId: string;
  readonly runtimeMode: DevtoolsRuntimeMode;
  readonly startedAtIso: string;
}

export interface DevtoolsProjectedStateSummary {
  readonly tick: number;
  readonly worldHash: string;
  readonly entityCount: number;
  readonly sceneCount: number;
  readonly selectedEntityId: string | null;
  readonly renderDiffHash: string | null;
}

export interface DevtoolsTelemetrySample {
  readonly metric: 'frame_ms' | 'simulation_ms' | 'command_queue_depth' | 'render_op_count';
  readonly value: number;
  readonly unit: 'ms' | 'count';
}

export interface DevtoolsEvidenceArtifact {
  readonly artifactId: string;
  readonly kind: 'attach_handshake' | 'projection_snapshot' | 'command_result' | 'replay_export' | 'evidence_export';
  readonly path: string;
  readonly sha256: string;
}

export type DevtoolsCommandProposalResult =
  | {
      readonly status: 'accepted';
      readonly sequenceId: string;
      readonly result: CommandResult;
      readonly authorityHashAfter: string;
    }
  | {
      readonly status: 'rejected';
      readonly sequenceId: string;
      readonly result: CommandResult;
      readonly reason: 'authority_rejected' | 'compatibility_mismatch' | 'runtime_unavailable';
      readonly authorityHashAfter: string | null;
    };

export type DevtoolsAttachClientMessage =
  | {
      readonly type: 'handshake.request';
      readonly protocolVersion: typeof ASHA_DEVTOOLS_PROTOCOL_VERSION;
      readonly clientName: 'asha-studio' | 'headless-smoke';
      readonly requestedWorkspaceId: string;
    }
  | {
      readonly type: 'projection.pull';
      readonly sinceTick: number | null;
    }
  | {
      readonly type: 'render_diff.snapshot';
      readonly sinceHash: string | null;
    }
  | {
      readonly type: 'telemetry.pull';
      readonly maxSamples: number;
    }
  | {
      readonly type: 'command.propose';
      readonly sequenceId: string;
      readonly batch: CommandBatch;
    }
  | {
      readonly type: 'replay.export';
      readonly replayId: string;
    }
  | {
      readonly type: 'evidence.export';
      readonly sequenceId: string;
      readonly includeRenderDiff: boolean;
    };

export type DevtoolsAttachServerMessage =
  | {
      readonly type: 'handshake.response';
      readonly accepted: true;
      readonly protocolVersion: typeof ASHA_DEVTOOLS_PROTOCOL_VERSION;
      readonly compatibility: DevtoolsCompatibilityMetadata;
      readonly runtime: DevtoolsRuntimeIdentity;
    }
  | {
      readonly type: 'handshake.response';
      readonly accepted: false;
      readonly protocolVersion: typeof ASHA_DEVTOOLS_PROTOCOL_VERSION;
      readonly reason: 'unsupported_protocol' | 'unknown_workspace' | 'runtime_not_ready';
    }
  | {
      readonly type: 'projection.snapshot';
      readonly summary: DevtoolsProjectedStateSummary;
      readonly diagnostics: readonly string[];
    }
  | {
      readonly type: 'render_diff.snapshot';
      readonly frame: RenderFrameDiff;
      readonly renderDiffHash: string;
    }
  | {
      readonly type: 'telemetry.snapshot';
      readonly samples: readonly DevtoolsTelemetrySample[];
    }
  | {
      readonly type: 'command.result';
      readonly proposal: DevtoolsCommandProposalResult;
    }
  | {
      readonly type: 'replay.exported';
      readonly artifact: DevtoolsEvidenceArtifact;
    }
  | {
      readonly type: 'evidence.exported';
      readonly artifacts: readonly DevtoolsEvidenceArtifact[];
    };

export interface DevtoolsProtocolGoldenFixtures {
  readonly handshakeRequest: Extract<DevtoolsAttachClientMessage, { readonly type: 'handshake.request' }>;
  readonly handshakeResponse: Extract<DevtoolsAttachServerMessage, { readonly type: 'handshake.response'; readonly accepted: true }>;
  readonly projectionPull: Extract<DevtoolsAttachClientMessage, { readonly type: 'projection.pull' }>;
  readonly projectionSnapshot: Extract<DevtoolsAttachServerMessage, { readonly type: 'projection.snapshot' }>;
  readonly commandProposal: Extract<DevtoolsAttachClientMessage, { readonly type: 'command.propose' }>;
  readonly commandAccepted: Extract<DevtoolsAttachServerMessage, { readonly type: 'command.result' }>;
  readonly commandRejected: Extract<DevtoolsAttachServerMessage, { readonly type: 'command.result' }>;
  readonly evidenceExport: Extract<DevtoolsAttachClientMessage, { readonly type: 'evidence.export' }>;
  readonly evidenceExported: Extract<DevtoolsAttachServerMessage, { readonly type: 'evidence.exported' }>;
}

export type DevtoolsConformanceFailureCode =
  | 'handshake_failed'
  | 'version_mismatch'
  | 'projection_unavailable'
  | 'telemetry_unavailable'
  | 'command_proposal_unavailable'
  | 'evidence_export_unavailable'
  | 'unexpected_response';

export type DevtoolsConformanceLane = 'asha' | 'asha-studio' | 'game-runtime';

export interface DevtoolsConformanceFailure {
  readonly code: DevtoolsConformanceFailureCode;
  readonly lane: DevtoolsConformanceLane;
  readonly message: string;
}

export interface DevtoolsConformanceReport {
  readonly ok: boolean;
  readonly protocolVersion: typeof ASHA_DEVTOOLS_PROTOCOL_VERSION;
  readonly checks: readonly string[];
  readonly failures: readonly DevtoolsConformanceFailure[];
}

export interface DevtoolsProtocolEndpoint {
  readonly exchange: (message: DevtoolsAttachClientMessage) => DevtoolsAttachServerMessage | Promise<DevtoolsAttachServerMessage>;
}

export interface DevtoolsFixtureEndpointOptions {
  readonly forceProtocolVersion?: string;
  readonly commandProposalSupported?: boolean;
}

export function buildDevtoolsProtocolGoldenFixtures(): DevtoolsProtocolGoldenFixtures {
  const compatibility: DevtoolsCompatibilityMetadata = {
    protocolVersion: ASHA_DEVTOOLS_PROTOCOL_VERSION,
    contractsCompatibility: 'contracts.v0',
    runtimeBridgeCompatibility: 'runtime-bridge.v0',
    publishArtifactFormat: 'publish-artifact.v0',
  };
  const commandBatch: CommandBatch = {
    commands: [
      {
        op: 'setVoxel',
        grid: 0,
        coord: { x: 0, y: 0, z: 0 },
        value: { kind: 'solid', material: 1 },
      },
    ],
  };
  const acceptedResult: CommandResult = { accepted: 1, rejected: 0, rejections: [] };
  const rejectedResult: CommandResult = {
    accepted: 0,
    rejected: 1,
    rejections: [{ reason: 'unknownMaterial', material: 999 }],
  };

  return {
    handshakeRequest: {
      type: 'handshake.request',
      protocolVersion: ASHA_DEVTOOLS_PROTOCOL_VERSION,
      clientName: 'asha-studio',
      requestedWorkspaceId: 'asha-demo',
    },
    handshakeResponse: {
      type: 'handshake.response',
      accepted: true,
      protocolVersion: ASHA_DEVTOOLS_PROTOCOL_VERSION,
      compatibility,
      runtime: {
        engineVersion: '0.1.0',
        gameId: 'asha-demo',
        workspaceId: 'asha-demo',
        runtimeMode: 'reference',
        startedAtIso: '2026-06-28T00:00:00.000Z',
      },
    },
    projectionPull: { type: 'projection.pull', sinceTick: null },
    projectionSnapshot: {
      type: 'projection.snapshot',
      summary: {
        tick: 1,
        worldHash: 'world:demo:1',
        entityCount: 1,
        sceneCount: 1,
        selectedEntityId: null,
        renderDiffHash: 'render:demo:1',
      },
      diagnostics: [],
    },
    commandProposal: {
      type: 'command.propose',
      sequenceId: 'seq-1',
      batch: commandBatch,
    },
    commandAccepted: {
      type: 'command.result',
      proposal: {
        status: 'accepted',
        sequenceId: 'seq-1',
        result: acceptedResult,
        authorityHashAfter: 'authority:after:accepted',
      },
    },
    commandRejected: {
      type: 'command.result',
      proposal: {
        status: 'rejected',
        sequenceId: 'seq-2',
        result: rejectedResult,
        reason: 'authority_rejected',
        authorityHashAfter: 'authority:after:rejected',
      },
    },
    evidenceExport: {
      type: 'evidence.export',
      sequenceId: 'seq-1',
      includeRenderDiff: true,
    },
    evidenceExported: {
      type: 'evidence.exported',
      artifacts: [
        {
          artifactId: 'attach-seq-1',
          kind: 'evidence_export',
          path: 'harness/out/attach/latest/index.json',
          sha256: 'sha256-demo-evidence',
        },
      ],
    },
  };
}

export function createDevtoolsFixtureEndpoint(options: DevtoolsFixtureEndpointOptions = {}): DevtoolsProtocolEndpoint {
  const fixtures = buildDevtoolsProtocolGoldenFixtures();
  const protocolVersion = options.forceProtocolVersion ?? ASHA_DEVTOOLS_PROTOCOL_VERSION;
  const commandProposalSupported = options.commandProposalSupported ?? true;

  return {
    exchange(message: DevtoolsAttachClientMessage): DevtoolsAttachServerMessage {
      switch (message.type) {
        case 'handshake.request':
          if (message.protocolVersion !== ASHA_DEVTOOLS_PROTOCOL_VERSION || protocolVersion !== ASHA_DEVTOOLS_PROTOCOL_VERSION) {
            return {
              type: 'handshake.response',
              accepted: false,
              protocolVersion: ASHA_DEVTOOLS_PROTOCOL_VERSION,
              reason: 'unsupported_protocol',
            };
          }
          return fixtures.handshakeResponse;
        case 'projection.pull':
          return fixtures.projectionSnapshot;
        case 'render_diff.snapshot':
          return { type: 'render_diff.snapshot', frame: { ops: [] }, renderDiffHash: 'render:demo:1' };
        case 'telemetry.pull':
          return {
            type: 'telemetry.snapshot',
            samples: [
              { metric: 'frame_ms', value: 16.6, unit: 'ms' },
              { metric: 'command_queue_depth', value: 0, unit: 'count' },
            ],
          };
        case 'command.propose':
          if (!commandProposalSupported) {
            return {
              type: 'command.result',
              proposal: {
                status: 'rejected',
                sequenceId: message.sequenceId,
                result: { accepted: 0, rejected: message.batch.commands.length, rejections: [] },
                reason: 'runtime_unavailable',
                authorityHashAfter: null,
              },
            };
          }
          return fixtures.commandAccepted;
        case 'replay.export':
          return {
            type: 'replay.exported',
            artifact: {
              artifactId: message.replayId,
              kind: 'replay_export',
              path: `harness/out/replay/${message.replayId}.json`,
              sha256: 'sha256-demo-replay',
            },
          };
        case 'evidence.export':
          return fixtures.evidenceExported;
      }
    },
  };
}

export async function runDevtoolsProtocolConformance(endpoint: DevtoolsProtocolEndpoint): Promise<DevtoolsConformanceReport> {
  const fixtures = buildDevtoolsProtocolGoldenFixtures();
  const checks: string[] = [];
  const failures: DevtoolsConformanceFailure[] = [];

  const handshake = await endpoint.exchange(fixtures.handshakeRequest);
  if (handshake.type !== 'handshake.response') {
    failures.push(confFailure('unexpected_response', 'game-runtime', 'handshake returned a non-handshake response'));
  } else if (!handshake.accepted) {
    failures.push(confFailure(handshake.reason === 'unsupported_protocol' ? 'version_mismatch' : 'handshake_failed', 'game-runtime', `handshake rejected: ${handshake.reason}`));
  } else {
    checks.push('handshake');
    if (handshake.compatibility.protocolVersion !== ASHA_DEVTOOLS_PROTOCOL_VERSION) {
      failures.push(confFailure('version_mismatch', 'asha', 'handshake compatibility protocol version does not match ASHA devtools protocol'));
    }
  }

  const projection = await endpoint.exchange(fixtures.projectionPull);
  if (projection.type === 'projection.snapshot' && projection.summary.worldHash.length > 0) {
    checks.push('projection');
  } else {
    failures.push(confFailure('projection_unavailable', 'game-runtime', 'projection pull did not return a valid snapshot'));
  }

  const telemetry = await endpoint.exchange({ type: 'telemetry.pull', maxSamples: 8 });
  if (telemetry.type === 'telemetry.snapshot' && telemetry.samples.length > 0) {
    checks.push('telemetry');
  } else {
    failures.push(confFailure('telemetry_unavailable', 'game-runtime', 'telemetry pull did not return samples'));
  }

  const command = await endpoint.exchange(fixtures.commandProposal);
  if (command.type === 'command.result' && command.proposal.status === 'accepted') {
    checks.push('command_proposal');
  } else {
    failures.push(confFailure('command_proposal_unavailable', 'game-runtime', 'command proposal did not return an accepted authority result'));
  }

  const evidence = await endpoint.exchange(fixtures.evidenceExport);
  if (evidence.type === 'evidence.exported' && evidence.artifacts.length > 0) {
    checks.push('evidence_export');
  } else {
    failures.push(confFailure('evidence_export_unavailable', 'game-runtime', 'evidence export did not return artifacts'));
  }

  return {
    ok: failures.length === 0,
    protocolVersion: ASHA_DEVTOOLS_PROTOCOL_VERSION,
    checks,
    failures,
  };
}

function confFailure(
  code: DevtoolsConformanceFailureCode,
  lane: DevtoolsConformanceLane,
  message: string,
): DevtoolsConformanceFailure {
  return { code, lane, message };
}
