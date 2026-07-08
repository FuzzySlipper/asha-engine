import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  ASHA_DEVTOOLS_PROTOCOL_VERSION,
  buildDevtoolsProtocolGoldenFixtures,
  createDevtoolsFixtureEndpoint,
  runDevtoolsProtocolConformance,
  type DevtoolsAttachClientMessage,
  type DevtoolsAttachServerMessage,
} from './attach-protocol.js';

function clientType(message: DevtoolsAttachClientMessage): string {
  return message.type;
}

function serverType(message: DevtoolsAttachServerMessage): string {
  return message.type;
}

void test('golden fixtures cover handshake and projection pull protocol messages', () => {
  const fixtures = buildDevtoolsProtocolGoldenFixtures();
  assert.equal(fixtures.handshakeRequest.protocolVersion, ASHA_DEVTOOLS_PROTOCOL_VERSION);
  assert.equal(fixtures.handshakeResponse.accepted, true);
  assert.equal(fixtures.handshakeResponse.compatibility.protocolVersion, ASHA_DEVTOOLS_PROTOCOL_VERSION);
  assert.equal(clientType(fixtures.projectionPull), 'projection.pull');
  assert.equal(serverType(fixtures.projectionSnapshot), 'projection.snapshot');
  assert.equal(fixtures.projectionSnapshot.summary.runtimeSessionSummaryHash, 'runtime-session:demo:1');
});

void test('command proposal fixtures model accepted and rejected authority results', () => {
  const fixtures = buildDevtoolsProtocolGoldenFixtures();
  assert.equal(clientType(fixtures.commandProposal), 'command.propose');
  assert.equal(fixtures.commandProposal.batch.commands[0]!.op, 'setVoxel');
  assert.equal(fixtures.commandAccepted.proposal.status, 'accepted');
  assert.equal(fixtures.commandAccepted.proposal.result.accepted, 1);
  assert.equal(fixtures.commandRejected.proposal.status, 'rejected');
  assert.equal(fixtures.commandRejected.proposal.reason, 'authority_rejected');
  assert.equal(fixtures.commandRejected.proposal.result.rejections[0]!.reason, 'unknownMaterial');
});

void test('evidence export is typed and carries artifact metadata', () => {
  const fixtures = buildDevtoolsProtocolGoldenFixtures();
  assert.equal(clientType(fixtures.evidenceExport), 'evidence.export');
  assert.equal(serverType(fixtures.evidenceExported), 'evidence.exported');
  assert.equal(fixtures.evidenceExported.artifacts[0]!.kind, 'evidence_export');
  assert.equal(fixtures.evidenceExported.artifacts[0]!.sha256.startsWith('sha256-'), true);
});

void test('stable protocol fixtures expose no methodName or anyJson catchall', () => {
  const fixtures = buildDevtoolsProtocolGoldenFixtures();
  const serialized = JSON.stringify(fixtures);
  assert.equal(serialized.includes('methodName'), false);
  assert.equal(serialized.includes('anyJson'), false);
  assert.equal(serialized.includes('payload'), false);
});

void test('conformance harness passes against the in-process fixture endpoint', async () => {
  const report = await runDevtoolsProtocolConformance(createDevtoolsFixtureEndpoint());
  assert.equal(report.ok, true);
  assert.deepEqual(report.failures, []);
  assert.deepEqual(report.checks, [
    'handshake',
    'projection',
    'telemetry',
    'command_proposal',
    'evidence_export',
  ]);
});

void test('conformance harness fails closed on protocol version mismatch', async () => {
  const report = await runDevtoolsProtocolConformance(createDevtoolsFixtureEndpoint({ forceProtocolVersion: 'devtools-protocol.v999' }));
  assert.equal(report.ok, false);
  assert.equal(report.failures.some((failure) => failure.code === 'version_mismatch'), true);
});

void test('conformance harness fails closed when command proposal support is missing', async () => {
  const report = await runDevtoolsProtocolConformance(createDevtoolsFixtureEndpoint({ commandProposalSupported: false }));
  assert.equal(report.ok, false);
  assert.equal(report.failures.some((failure) => failure.code === 'command_proposal_unavailable'), true);
});
