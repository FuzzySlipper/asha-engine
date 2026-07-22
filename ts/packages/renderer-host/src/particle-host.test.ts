import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  particleEmitterHandle,
  type ParticleEmitterDescriptor,
  type PresentationOp,
  type RuntimeProjectionFrame,
} from '@asha/contracts';
import { applyAshaRuntimeProjectionFrame } from './audio-host.js';
import {
  AshaParticleHost,
  type AshaParticleBillboard,
  type AshaParticleBillboardSink,
} from './particle-host.js';

const SPRITE_HASH = '9f64a747e1b97f131fabb6b447296c9b6f0201e79fb3c5356e6c77e89b6a806a';
const SPRITE_FNV_HASH = 'be7a5e775165785d';

class FakeParticleSink implements AshaParticleBillboardSink {
  readonly active = new Map<number, AshaParticleBillboard>();
  readonly created: AshaParticleBillboard[] = [];
  readonly updated: AshaParticleBillboard[] = [];
  readonly destroyed: number[] = [];

  create(particle: AshaParticleBillboard): void {
    this.active.set(particle.id, particle);
    this.created.push(particle);
  }

  update(particle: AshaParticleBillboard): void {
    this.active.set(particle.id, particle);
    this.updated.push(particle);
  }

  destroy(id: number): void {
    this.active.delete(id);
    this.destroyed.push(id);
  }
}

function descriptor(
  overrides: Partial<ParticleEmitterDescriptor> = {},
): ParticleEmitterDescriptor {
  return {
    anchor: { kind: 'world', position: [1, 2, 3] },
    sprite: {
      asset: 'sprite-sheet/fixture-sparks',
      contentHash: SPRITE_HASH,
      frameCount: 4,
    },
    ratePerSecond: 8,
    burstCount: 3,
    lifetimeSeconds: [0.2, 0.4],
    velocityMin: [-1, 1, -1],
    velocityMax: [1, 2, 1],
    acceleration: [0, -3, 0],
    sizeCurve: [
      { age: 0, value: 0.4 },
      { age: 1, value: 0 },
    ],
    colorCurve: [
      { age: 0, color: [1, 0.8, 0.2, 1] },
      { age: 1, color: [1, 0.2, 0, 0] },
    ],
    flipbookFramesPerSecond: 12,
    seed: 44,
    maxParticles: 16,
    visible: true,
    ...overrides,
  };
}

function operation(
  sequence: number,
  op: Extract<PresentationOp, { readonly domain: 'particle' }>['op'],
): PresentationOp {
  return {
    domain: 'particle',
    meta: {
      sequence,
      origin: {
        kind: 'gameplayEvent',
        id: 'combat.primary-fire.hit:44',
        authorityTick: 9,
        causationId: 'command:fire:9',
        correlationId: 'encounter:fixture',
      },
    },
    op,
  };
}

function frame(ops: readonly PresentationOp[]): RuntimeProjectionFrame {
  return {
    schemaVersion: 1,
    authorityTick: 9,
    scene: { ops: [] },
    presentation: { replayScope: 'excludedFromReplayTruth', ops },
  };
}

function host(sink: FakeParticleSink, maxParticles = 64): AshaParticleHost {
  return new AshaParticleHost({
    maxParticles,
    resolveEntityPosition: (entity) => entity === 42 ? [10, 11, 12] : null,
    resolveResource: async () => ({
      bytes: new Uint8Array([1, 2, 3, 4]).buffer,
      url: '/sprites/fixture-sparks.png',
    }),
    sink,
  });
}

void test('particle host realizes deterministic bursts and expires disposable billboards', async () => {
  const sink = new FakeParticleSink();
  const particles = host(sink);
  const presentation = frame([
    operation(0, {
      op: 'emit',
      signalId: 'impact:44',
      descriptor: descriptor(),
    }),
  ]).presentation;

  const receipt = await particles.applyPresentation(presentation);
  assert.equal(receipt.applied, 1);
  assert.equal(receipt.readout.emittedBursts, 1);
  assert.equal(receipt.readout.activeParticles, 3);
  assert.equal(receipt.readout.loadedSprites, 1);
  assert.deepEqual(sink.created.map((particle) => particle.position), [
    [1, 2, 3],
    [1, 2, 3],
    [1, 2, 3],
  ]);

  const repeated = await particles.applyPresentation(presentation);
  assert.equal(repeated.applied, 1);
  assert.equal(repeated.readout.emittedBursts, 1);
  assert.equal(sink.created.length, 3, 'stable signal ids prevent duplicate realization');

  particles.advance(0.1);
  assert.equal(sink.updated.length, 3);
  assert.notDeepEqual(sink.updated[0]?.position, [1, 2, 3]);
  particles.advance(0.4);
  assert.equal(particles.readout().activeParticles, 0);
  assert.equal(sink.destroyed.length, 3);
});

void test('a missing entity anchor diagnoses without consuming the burst signal', async () => {
  const sink = new FakeParticleSink();
  let entityPosition: readonly [number, number, number] | null = null;
  const particles = new AshaParticleHost({
    resolveEntityPosition: () => entityPosition,
    resolveResource: async () => ({
      bytes: new Uint8Array([1, 2, 3, 4]).buffer,
      url: '/sprites/fixture-sparks.png',
    }),
    sink,
  });
  const presentation = frame([
    operation(0, {
      op: 'emit',
      signalId: 'late-anchor:44',
      descriptor: descriptor({
        anchor: { kind: 'entityAttached', entity: 404, offset: [0, 1, 0] },
      }),
    }),
  ]).presentation;

  const missing = await particles.applyPresentation(presentation);
  assert.equal(missing.applied, 0);
  assert.equal(missing.diagnostics[0]?.code, 'anchorMissing');
  assert.equal(missing.readout.emittedBursts, 0);
  assert.equal(missing.readout.activeParticles, 0);

  entityPosition = [4, 5, 6];
  const retried = await particles.applyPresentation(presentation);
  assert.equal(retried.applied, 1);
  assert.equal(retried.readout.emittedBursts, 1);
  assert.equal(retried.readout.activeParticles, 3);
  assert.deepEqual(sink.created[0]?.position, [4, 6, 6]);

  const replayed = await particles.applyPresentation(presentation);
  assert.equal(replayed.readout.emittedBursts, 1);
  assert.equal(replayed.readout.activeParticles, 3);
});

void test('missing particle resources fail locally without consuming the burst', async () => {
  const sink = new FakeParticleSink();
  const particles = new AshaParticleHost({
    resolveEntityPosition: () => [0, 0, 0],
    resolveResource: async () => null,
    sink,
  });
  const presentation = frame([
    operation(0, {
      op: 'emit',
      signalId: 'missing-particle:44',
      descriptor: descriptor(),
    }),
  ]).presentation;

  const receipt = await particles.applyPresentation(presentation);
  assert.equal(receipt.applied, 0);
  assert.equal(receipt.diagnostics[0]?.code, 'spriteLoadFailed');
  assert.equal(receipt.diagnostics[0]?.origin?.id, 'combat.primary-fire.hit:44');
  assert.equal(receipt.readout.emittedBursts, 0);
  assert.equal(receipt.readout.activeParticles, 0);
  assert.equal(sink.created.length, 0);
});

void test('particle host accepts a manifest-native FNV content hash', async () => {
  const sink = new FakeParticleSink();
  const particles = host(sink);
  const receipt = await particles.applyPresentation(frame([
    operation(0, {
      op: 'emit',
      signalId: 'fnv-particle',
      descriptor: descriptor({
        sprite: {
          asset: 'sprite/asha-primary-fire-spark',
          contentHash: SPRITE_FNV_HASH,
          frameCount: 1,
        },
      }),
    }),
  ]).presentation);

  assert.equal(receipt.applied, 1);
  assert.deepEqual(receipt.diagnostics, []);
  assert.equal(sink.created.length, 3);
});

void test('retained emitter create update destroy owns continuous simulation and cleanup', async () => {
  const sink = new FakeParticleSink();
  const particles = host(sink);
  const handle = particleEmitterHandle(7);
  const created = await particles.applyPresentation(frame([
    operation(0, {
      op: 'create',
      handle,
      descriptor: descriptor({
        anchor: { kind: 'entityAttached', entity: 42, offset: [0, 1, 0] },
        burstCount: 0,
        ratePerSecond: 4,
        lifetimeSeconds: [1, 1],
      }),
    }),
  ]).presentation);
  assert.equal(created.readout.activeEmitters, 1);
  particles.advance(0.5);
  assert.equal(sink.created.length, 2);
  assert.deepEqual(sink.created[0]?.position, [10, 12, 12]);

  const updated = await particles.applyPresentation(frame([
    operation(0, {
      op: 'update',
      handle,
      patch: {
        anchor: null,
        sprite: null,
        ratePerSecond: 8,
        burstCount: null,
        lifetimeSeconds: null,
        velocityMin: null,
        velocityMax: null,
        acceleration: null,
        sizeCurve: null,
        colorCurve: null,
        flipbookFramesPerSecond: null,
        maxParticles: null,
        visible: false,
      },
    }),
  ]).presentation);
  assert.equal(updated.applied, 1);
  particles.advance(0.5);
  assert.equal(sink.created.length, 2, 'invisible retained emitter pauses new realization');

  const destroyed = await particles.applyPresentation(frame([
    operation(0, { op: 'destroy', handle }),
  ]).presentation);
  assert.equal(destroyed.readout.activeEmitters, 0);
  assert.equal(destroyed.readout.activeParticles, 0);
  assert.equal(sink.destroyed.length, 2);
});

void test('missing anchor budgets and unavailable host degrade independently after scene', async () => {
  const sink = new FakeParticleSink();
  const particles = host(sink, 2);
  const missing = await particles.applyPresentation(frame([
    operation(0, {
      op: 'emit',
      signalId: 'missing-anchor',
      descriptor: descriptor({
        anchor: { kind: 'entityAttached', entity: 99, offset: [0, 0, 0] },
      }),
    }),
  ]).presentation);
  assert.equal(missing.diagnostics[0]?.code, 'anchorMissing');

  const budgeted = await particles.applyPresentation(frame([
    operation(0, {
      op: 'emit',
      signalId: 'large-burst',
      descriptor: descriptor({ burstCount: 4 }),
    }),
  ]).presentation);
  assert.equal(budgeted.diagnostics[0]?.code, 'budgetExceeded');
  assert.equal(budgeted.readout.activeParticles, 2);
  assert.equal(budgeted.readout.droppedParticles, 2);

  let sceneApplied = false;
  const unavailable = await applyAshaRuntimeProjectionFrame(frame([
    operation(0, {
      op: 'emit',
      signalId: 'unavailable',
      descriptor: descriptor(),
    }),
  ]), {
    applyScene: () => { sceneApplied = true; },
  });
  assert.equal(sceneApplied, true);
  assert.equal(unavailable.particle.diagnostics[0]?.code, 'unavailableHost');
  assert.equal(unavailable.particle.diagnostics[0]?.origin?.id, 'combat.primary-fire.hit:44');
});
