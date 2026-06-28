export const ASHA_DEVTOOLS_PROTOCOL_VERSION = 'devtools-protocol.v0';
export function buildDevtoolsProtocolGoldenFixtures() {
    const compatibility = {
        protocolVersion: ASHA_DEVTOOLS_PROTOCOL_VERSION,
        contractsCompatibility: 'contracts.v0',
        runtimeBridgeCompatibility: 'runtime-bridge.v0',
        publishArtifactFormat: 'publish-artifact.v0',
    };
    const commandBatch = {
        commands: [
            {
                op: 'setVoxel',
                grid: 0,
                coord: { x: 0, y: 0, z: 0 },
                value: { kind: 'solid', material: 1 },
            },
        ],
    };
    const acceptedResult = { accepted: 1, rejected: 0, rejections: [] };
    const rejectedResult = {
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
export function createDevtoolsFixtureEndpoint(options = {}) {
    const fixtures = buildDevtoolsProtocolGoldenFixtures();
    const protocolVersion = options.forceProtocolVersion ?? ASHA_DEVTOOLS_PROTOCOL_VERSION;
    const commandProposalSupported = options.commandProposalSupported ?? true;
    return {
        exchange(message) {
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
export async function runDevtoolsProtocolConformance(endpoint) {
    const fixtures = buildDevtoolsProtocolGoldenFixtures();
    const checks = [];
    const failures = [];
    const handshake = await endpoint.exchange(fixtures.handshakeRequest);
    if (handshake.type !== 'handshake.response') {
        failures.push(confFailure('unexpected_response', 'game-runtime', 'handshake returned a non-handshake response'));
    }
    else if (!handshake.accepted) {
        failures.push(confFailure(handshake.reason === 'unsupported_protocol' ? 'version_mismatch' : 'handshake_failed', 'game-runtime', `handshake rejected: ${handshake.reason}`));
    }
    else {
        checks.push('handshake');
        if (handshake.compatibility.protocolVersion !== ASHA_DEVTOOLS_PROTOCOL_VERSION) {
            failures.push(confFailure('version_mismatch', 'asha', 'handshake compatibility protocol version does not match ASHA devtools protocol'));
        }
    }
    const projection = await endpoint.exchange(fixtures.projectionPull);
    if (projection.type === 'projection.snapshot' && projection.summary.worldHash.length > 0) {
        checks.push('projection');
    }
    else {
        failures.push(confFailure('projection_unavailable', 'game-runtime', 'projection pull did not return a valid snapshot'));
    }
    const telemetry = await endpoint.exchange({ type: 'telemetry.pull', maxSamples: 8 });
    if (telemetry.type === 'telemetry.snapshot' && telemetry.samples.length > 0) {
        checks.push('telemetry');
    }
    else {
        failures.push(confFailure('telemetry_unavailable', 'game-runtime', 'telemetry pull did not return samples'));
    }
    const command = await endpoint.exchange(fixtures.commandProposal);
    if (command.type === 'command.result' && command.proposal.status === 'accepted') {
        checks.push('command_proposal');
    }
    else {
        failures.push(confFailure('command_proposal_unavailable', 'game-runtime', 'command proposal did not return an accepted authority result'));
    }
    const evidence = await endpoint.exchange(fixtures.evidenceExport);
    if (evidence.type === 'evidence.exported' && evidence.artifacts.length > 0) {
        checks.push('evidence_export');
    }
    else {
        failures.push(confFailure('evidence_export_unavailable', 'game-runtime', 'evidence export did not return artifacts'));
    }
    return {
        ok: failures.length === 0,
        protocolVersion: ASHA_DEVTOOLS_PROTOCOL_VERSION,
        checks,
        failures,
    };
}
function confFailure(code, lane, message) {
    return { code, lane, message };
}
//# sourceMappingURL=attach-protocol.js.map