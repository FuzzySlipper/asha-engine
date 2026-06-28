import type { CommandBatch, CommandResult, RenderFrameDiff } from '@asha/contracts';
export declare const ASHA_DEVTOOLS_PROTOCOL_VERSION = "devtools-protocol.v0";
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
export type DevtoolsCommandProposalResult = {
    readonly status: 'accepted';
    readonly sequenceId: string;
    readonly result: CommandResult;
    readonly authorityHashAfter: string;
} | {
    readonly status: 'rejected';
    readonly sequenceId: string;
    readonly result: CommandResult;
    readonly reason: 'authority_rejected' | 'compatibility_mismatch' | 'runtime_unavailable';
    readonly authorityHashAfter: string | null;
};
export type DevtoolsAttachClientMessage = {
    readonly type: 'handshake.request';
    readonly protocolVersion: typeof ASHA_DEVTOOLS_PROTOCOL_VERSION;
    readonly clientName: 'asha-studio' | 'headless-smoke';
    readonly requestedWorkspaceId: string;
} | {
    readonly type: 'projection.pull';
    readonly sinceTick: number | null;
} | {
    readonly type: 'render_diff.snapshot';
    readonly sinceHash: string | null;
} | {
    readonly type: 'telemetry.pull';
    readonly maxSamples: number;
} | {
    readonly type: 'command.propose';
    readonly sequenceId: string;
    readonly batch: CommandBatch;
} | {
    readonly type: 'replay.export';
    readonly replayId: string;
} | {
    readonly type: 'evidence.export';
    readonly sequenceId: string;
    readonly includeRenderDiff: boolean;
};
export type DevtoolsAttachServerMessage = {
    readonly type: 'handshake.response';
    readonly accepted: true;
    readonly protocolVersion: typeof ASHA_DEVTOOLS_PROTOCOL_VERSION;
    readonly compatibility: DevtoolsCompatibilityMetadata;
    readonly runtime: DevtoolsRuntimeIdentity;
} | {
    readonly type: 'handshake.response';
    readonly accepted: false;
    readonly protocolVersion: typeof ASHA_DEVTOOLS_PROTOCOL_VERSION;
    readonly reason: 'unsupported_protocol' | 'unknown_workspace' | 'runtime_not_ready';
} | {
    readonly type: 'projection.snapshot';
    readonly summary: DevtoolsProjectedStateSummary;
    readonly diagnostics: readonly string[];
} | {
    readonly type: 'render_diff.snapshot';
    readonly frame: RenderFrameDiff;
    readonly renderDiffHash: string;
} | {
    readonly type: 'telemetry.snapshot';
    readonly samples: readonly DevtoolsTelemetrySample[];
} | {
    readonly type: 'command.result';
    readonly proposal: DevtoolsCommandProposalResult;
} | {
    readonly type: 'replay.exported';
    readonly artifact: DevtoolsEvidenceArtifact;
} | {
    readonly type: 'evidence.exported';
    readonly artifacts: readonly DevtoolsEvidenceArtifact[];
};
export interface DevtoolsProtocolGoldenFixtures {
    readonly handshakeRequest: Extract<DevtoolsAttachClientMessage, {
        readonly type: 'handshake.request';
    }>;
    readonly handshakeResponse: Extract<DevtoolsAttachServerMessage, {
        readonly type: 'handshake.response';
        readonly accepted: true;
    }>;
    readonly projectionPull: Extract<DevtoolsAttachClientMessage, {
        readonly type: 'projection.pull';
    }>;
    readonly projectionSnapshot: Extract<DevtoolsAttachServerMessage, {
        readonly type: 'projection.snapshot';
    }>;
    readonly commandProposal: Extract<DevtoolsAttachClientMessage, {
        readonly type: 'command.propose';
    }>;
    readonly commandAccepted: Extract<DevtoolsAttachServerMessage, {
        readonly type: 'command.result';
    }>;
    readonly commandRejected: Extract<DevtoolsAttachServerMessage, {
        readonly type: 'command.result';
    }>;
    readonly evidenceExport: Extract<DevtoolsAttachClientMessage, {
        readonly type: 'evidence.export';
    }>;
    readonly evidenceExported: Extract<DevtoolsAttachServerMessage, {
        readonly type: 'evidence.exported';
    }>;
}
export type DevtoolsConformanceFailureCode = 'handshake_failed' | 'version_mismatch' | 'projection_unavailable' | 'telemetry_unavailable' | 'command_proposal_unavailable' | 'evidence_export_unavailable' | 'unexpected_response';
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
export declare function buildDevtoolsProtocolGoldenFixtures(): DevtoolsProtocolGoldenFixtures;
export declare function createDevtoolsFixtureEndpoint(options?: DevtoolsFixtureEndpointOptions): DevtoolsProtocolEndpoint;
export declare function runDevtoolsProtocolConformance(endpoint: DevtoolsProtocolEndpoint): Promise<DevtoolsConformanceReport>;
//# sourceMappingURL=attach-protocol.d.ts.map