import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  audioHandle,
  type AudioSourceDescriptor,
  type PresentationOp,
  type RuntimeProjectionFrame,
} from '@asha/contracts';
import {
  AshaAudioHost,
  applyAshaRuntimeProjectionFrame,
  type AshaAudioContext,
} from './audio-host.js';

const FIXTURE_AUDIO_HASH = '9f64a747e1b97f131fabb6b447296c9b6f0201e79fb3c5356e6c77e89b6a806a';

class FakeParam {
  value = 0;
  readonly writes: number[] = [];

  setValueAtTime(value: number): void {
    this.value = value;
    this.writes.push(value);
  }
}

class FakeNode {
  readonly connections: FakeNode[] = [];
  disconnected = false;

  connect(destination: FakeNode): FakeNode {
    this.connections.push(destination);
    return destination;
  }

  disconnect(): void {
    this.disconnected = true;
  }
}

class FakeGain extends FakeNode {
  readonly gain = new FakeParam();
}

class FakeStereoPanner extends FakeNode {
  readonly pan = new FakeParam();
}

class FakePanner extends FakeNode {
  distanceModel: DistanceModelType = 'inverse';
  maxDistance = 0;
  panningModel: PanningModelType = 'HRTF';
  refDistance = 0;
  rolloffFactor = 0;
  readonly positionX = new FakeParam();
  readonly positionY = new FakeParam();
  readonly positionZ = new FakeParam();
}

class FakeListener {
  readonly forwardX = new FakeParam();
  readonly forwardY = new FakeParam();
  readonly forwardZ = new FakeParam();
  readonly positionX = new FakeParam();
  readonly positionY = new FakeParam();
  readonly positionZ = new FakeParam();
  readonly upX = new FakeParam();
  readonly upY = new FakeParam();
  readonly upZ = new FakeParam();
}

class FakeSource extends FakeNode {
  buffer: unknown = null;
  loop = false;
  onended: (() => void) | null = null;
  readonly playbackRate = new FakeParam();
  started = false;
  stopped = false;

  start(): void {
    this.started = true;
  }

  stop(): void {
    this.stopped = true;
  }
}

class FakeContext {
  readonly currentTime = 2;
  readonly destination = new FakeNode();
  readonly listener = new FakeListener();
  state: AudioContextState = 'suspended';
  readonly gains: FakeGain[] = [];
  readonly panners: FakePanner[] = [];
  readonly stereoPanners: FakeStereoPanner[] = [];
  readonly sources: FakeSource[] = [];
  decodeCount = 0;
  closed = false;
  blockResume = false;

  async close(): Promise<void> {
    this.closed = true;
    this.state = 'closed';
  }

  createBufferSource(): FakeSource {
    const source = new FakeSource();
    this.sources.push(source);
    return source;
  }

  createGain(): FakeGain {
    const gain = new FakeGain();
    this.gains.push(gain);
    return gain;
  }

  createPanner(): FakePanner {
    const panner = new FakePanner();
    this.panners.push(panner);
    return panner;
  }

  createStereoPanner(): FakeStereoPanner {
    const panner = new FakeStereoPanner();
    this.stereoPanners.push(panner);
    return panner;
  }

  async decodeAudioData(): Promise<unknown> {
    this.decodeCount += 1;
    return { decoded: true };
  }

  async resume(): Promise<void> {
    if (!this.blockResume) {
      this.state = 'running';
    }
  }
}

function descriptor(
  emitter: AudioSourceDescriptor['emitter'] = {
    kind: 'world3d',
    position: [1, 2, 3],
  },
): AudioSourceDescriptor {
  return {
    clip: { asset: 'audio/asha-primary-fire-pulse', contentHash: FIXTURE_AUDIO_HASH },
    bus: 'sfx',
    volume: 0.8,
    pitch: 1,
    looping: false,
    spatialBlend: 1,
    attenuation: 24,
    pan: 0.2,
    emitter,
  };
}

function operation(
  sequence: number,
  op: Extract<PresentationOp, { readonly domain: 'audio' }>['op'],
): PresentationOp {
  return {
    domain: 'audio',
    meta: {
      sequence,
      origin: {
        kind: 'ownerFact',
        id: 'combat.primary-fire.accepted:44',
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
    presentation: {
      replayScope: 'excludedFromReplayTruth',
      ops,
    },
  };
}

function host(context: FakeContext): AshaAudioHost {
  return new AshaAudioHost({
    createContext: () => context as unknown as AshaAudioContext,
    resolveEntityPosition: () => [10, 11, 12],
    resolveResource: async (clip) => ({
      bytes: new Uint8Array([1, 2, 3, 4]).buffer,
      contentHash: clip.contentHash,
    }),
  });
}

void test('Web Audio host emits catalog-hash-bound 3D cues and caches decoded clips', async () => {
  const context = new FakeContext();
  const audio = host(context);
  assert.deepEqual(await audio.resume(), []);
  assert.deepEqual(audio.updateListener({
    position: [4, 5, 6],
    forward: [0, 0, -1],
    up: [0, 1, 0],
  }), []);

  const receipt = await audio.applyPresentation(
    frame([
      operation(0, {
        op: 'emit',
        signalId: 'shot:44',
        descriptor: descriptor(),
      }),
      operation(1, {
        op: 'emit',
        signalId: 'impact:44',
        descriptor: descriptor(),
      }),
    ]).presentation,
  );

  assert.equal(receipt.applied, 2);
  assert.equal(receipt.readout.emittedSignals, 2);
  assert.equal(receipt.readout.cachedClips, 1);
  assert.equal(context.decodeCount, 1);
  assert.equal(context.sources.every((source) => source.started), true);
  assert.equal(context.panners.length, 2);
  assert.deepEqual(context.panners[0]?.positionX.writes, [1]);
  assert.deepEqual(context.listener.positionX.writes, [4]);
  assert.deepEqual(context.listener.forwardZ.writes, [-1]);
  assert.equal(context.panners[0]?.maxDistance, 24);
  assert.equal(context.panners[0]?.panningModel, 'equalpower');
  assert.deepEqual(receipt.diagnostics, []);

  const repeated = await audio.applyPresentation(
    frame([
      operation(0, {
        op: 'emit',
        signalId: 'shot:44',
        descriptor: descriptor(),
      }),
    ]).presentation,
  );
  assert.equal(repeated.applied, 1);
  assert.equal(repeated.readout.emittedSignals, 2);
  assert.equal(context.sources.length, 2, 're-reading a frame does not replay a one-shot signal');
});

void test('retained 2D/3D sources create update destroy and clean up independently', async () => {
  const context = new FakeContext();
  const audio = host(context);
  const handle = audioHandle(7);
  const receipt = await audio.applyPresentation(
    frame([
      operation(0, {
        op: 'create',
        handle,
        descriptor: { ...descriptor({ kind: 'global2d' }), looping: true, bus: 'ambient' },
      }),
      operation(1, {
        op: 'update',
        handle,
        patch: {
          volume: 0.25,
          pitch: 1.5,
          looping: true,
          spatialBlend: null,
          attenuation: null,
          pan: -0.5,
          emitter: { kind: 'entityAttached', entity: 5 as never, offset: [1, 0, -1] },
        },
      }),
      operation(2, { op: 'destroy', handle }),
    ]).presentation,
  );

  assert.equal(receipt.applied, 3);
  assert.equal(receipt.readout.activeSources, 0);
  assert.equal(context.sources.length, 2, 'emitter-mode update rebuilds the node graph');
  assert.equal(context.sources.every((source) => source.stopped), true);
  assert.deepEqual(context.panners[0]?.positionX.writes, [11]);
});

void test('retained entity-attached audio follows scene movement without descriptor updates', async () => {
  const context = new FakeContext();
  let entityPosition: readonly [number, number, number] | null = [10, 11, 12];
  const audio = new AshaAudioHost({
    createContext: () => context as unknown as AshaAudioContext,
    resolveEntityPosition: () => entityPosition,
    resolveResource: async (clip) => ({
      bytes: new Uint8Array([1, 2, 3, 4]).buffer,
      contentHash: clip.contentHash,
    }),
  });
  const handle = audioHandle(9);
  await audio.applyPresentation(frame([
    operation(0, {
      op: 'create',
      handle,
      descriptor: {
        ...descriptor({ kind: 'entityAttached', entity: 5 as never, offset: [1, 0, -1] }),
        looping: true,
      },
    }),
  ]).presentation);
  assert.deepEqual(context.panners[0]?.positionX.writes, [11]);

  const receipt = await applyAshaRuntimeProjectionFrame(frame([]), {
    applyScene: () => {
      entityPosition = [20, 21, 22];
    },
    audioHost: audio,
  });

  assert.deepEqual(context.panners[0]?.positionX.writes, [11, 21]);
  assert.deepEqual(context.panners[0]?.positionY.writes, [11, 21]);
  assert.deepEqual(context.panners[0]?.positionZ.writes, [11, 21]);
  assert.equal(receipt.audio.readout.activeSources, 1);
  assert.deepEqual(receipt.audio.diagnostics, []);

  entityPosition = null;
  const missing = await applyAshaRuntimeProjectionFrame(frame([]), {
    applyScene: () => {},
    audioHost: audio,
  });
  assert.equal(missing.audio.diagnostics[0]?.code, 'hostFailure');
  assert.equal(missing.audio.diagnostics[0]?.handle, handle);
  assert.equal(missing.audio.readout.activeSources, 1);
});

void test('missing audio host degrades after scene application with origin diagnostics', async () => {
  let sceneApplied = false;
  const receipt = await applyAshaRuntimeProjectionFrame(
    frame([
      operation(0, {
        op: 'emit',
        signalId: 'shot:44',
        descriptor: descriptor(),
      }),
    ]),
    { applyScene: () => { sceneApplied = true; } },
  );

  assert.equal(sceneApplied, true);
  assert.equal(receipt.audio.applied, 0);
  assert.equal(receipt.audio.diagnostics[0]?.code, 'unavailableHost');
  assert.equal(
    receipt.audio.diagnostics[0]?.origin?.id,
    'combat.primary-fire.accepted:44',
  );
});

void test('audio host hashes resolved bytes before decode and reports catalog drift', async () => {
  const context = new FakeContext();
  const audio = host(context);
  const badDescriptor = {
    ...descriptor(),
    clip: {
      asset: 'audio/asha-primary-fire-pulse',
      contentHash: '0'.repeat(64),
    },
  };
  const receipt = await audio.applyPresentation(
    frame([
      operation(0, {
        op: 'emit',
        signalId: 'bad-hash',
        descriptor: badDescriptor,
      }),
    ]).presentation,
  );

  assert.equal(receipt.applied, 0);
  assert.equal(receipt.diagnostics[0]?.code, 'contentHashMismatch');
  assert.equal(receipt.readout.cachedClips, 0);
  assert.equal(receipt.readout.emittedSignals, 0);
  assert.equal(context.decodeCount, 0);
});

void test('missing audio resources fail locally with origin-preserving diagnostics', async () => {
  const context = new FakeContext();
  const audio = new AshaAudioHost({
    createContext: () => context as unknown as AshaAudioContext,
    resolveResource: async () => {
      throw new Error('fixture audio resource unavailable');
    },
  });
  const receipt = await audio.applyPresentation(frame([
    operation(0, {
      op: 'emit',
      signalId: 'missing-audio:44',
      descriptor: descriptor(),
    }),
  ]).presentation);

  assert.equal(receipt.applied, 0);
  assert.equal(receipt.diagnostics[0]?.code, 'hostFailure');
  assert.equal(receipt.diagnostics[0]?.origin?.id, 'combat.primary-fire.accepted:44');
  assert.equal(receipt.readout.emittedSignals, 0);
  assert.equal(receipt.readout.activeSources, 0);
});

void test('blocked AudioContext and malformed frame return explicit failures', async () => {
  const context = new FakeContext();
  context.blockResume = true;
  const audio = host(context);
  const diagnostics = await audio.resume();
  assert.equal(diagnostics[0]?.code, 'audioContextBlocked');

  let sceneApplied = false;
  await assert.rejects(
    applyAshaRuntimeProjectionFrame(
      frame([
        operation(1, {
          op: 'emit',
          signalId: 'bad-sequence',
          descriptor: descriptor(),
        }),
      ]),
      { applyScene: () => { sceneApplied = true; }, audioHost: audio },
    ),
    /sequence must be contiguous/,
  );
  assert.equal(sceneApplied, false, 'malformed outer framing rejects before scene');
});
