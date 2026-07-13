import type {
  AudioBus,
  AudioClipRef,
  AudioEmitter,
  AudioHandle,
  AudioProjectionDiagnostic,
  AudioProjectionOp,
  AudioProjectionReadout,
  AudioSourceDescriptor,
  AudioSourcePatch,
  BillboardProjectionDiagnostic,
  ParticleProjectionDiagnostic,
  PresentationFrameDiff,
  PresentationOp,
  RuntimeProjectionFrame,
  TelemetryOverlayDiagnostic,
} from '@asha/contracts';
import type { AshaBillboardFrameReceipt, AshaBillboardHost } from './billboard-host.js';
import type { AshaParticleFrameReceipt, AshaParticleHost } from './particle-host.js';
import type {
  AshaTelemetryOverlayFrameReceipt,
  AshaTelemetryOverlayHost,
} from './telemetry-host.js';
import type { AshaAnimationFrameReceipt, AshaAnimationHost } from './animation-host.js';

export interface AshaAudioResource {
  readonly bytes: ArrayBuffer;
  readonly contentHash: string;
}

export type AshaAudioResourceResolver = (clip: AudioClipRef) => Promise<AshaAudioResource>;
export type AshaAudioEntityPositionResolver = (
  entity: number,
) => readonly [number, number, number] | null;

interface AshaAudioParam {
  setValueAtTime(value: number, time: number): void;
}

interface AshaAudioNode {
  connect(destination: AshaAudioNode): unknown;
  disconnect(): void;
}

interface AshaGainNode extends AshaAudioNode {
  readonly gain: AshaAudioParam;
}

interface AshaStereoPannerNode extends AshaAudioNode {
  readonly pan: AshaAudioParam;
}

interface AshaPannerNode extends AshaAudioNode {
  distanceModel: DistanceModelType;
  maxDistance: number;
  panningModel: PanningModelType;
  refDistance: number;
  rolloffFactor: number;
  readonly positionX: AshaAudioParam;
  readonly positionY: AshaAudioParam;
  readonly positionZ: AshaAudioParam;
}

interface AshaAudioListener {
  readonly forwardX: AshaAudioParam;
  readonly forwardY: AshaAudioParam;
  readonly forwardZ: AshaAudioParam;
  readonly positionX: AshaAudioParam;
  readonly positionY: AshaAudioParam;
  readonly positionZ: AshaAudioParam;
  readonly upX: AshaAudioParam;
  readonly upY: AshaAudioParam;
  readonly upZ: AshaAudioParam;
}

interface AshaBufferSourceNode extends AshaAudioNode {
  buffer: unknown;
  loop: boolean;
  onended: (() => void) | null;
  readonly playbackRate: AshaAudioParam;
  start(): void;
  stop(): void;
}

export interface AshaAudioContext {
  readonly currentTime: number;
  readonly destination: AshaAudioNode;
  readonly listener: AshaAudioListener;
  readonly state: AudioContextState;
  close(): Promise<void>;
  createBufferSource(): AshaBufferSourceNode;
  createGain(): AshaGainNode;
  createPanner(): AshaPannerNode;
  createStereoPanner(): AshaStereoPannerNode;
  decodeAudioData(bytes: ArrayBuffer): Promise<unknown>;
  resume(): Promise<void>;
}

export interface AshaAudioHostOptions {
  readonly createContext?: () => AshaAudioContext;
  readonly resolveEntityPosition?: AshaAudioEntityPositionResolver;
  readonly resolveResource: AshaAudioResourceResolver;
}

export interface AshaAudioListenerPose {
  readonly position: readonly [number, number, number];
  readonly forward: readonly [number, number, number];
  readonly up: readonly [number, number, number];
}

export interface AshaAudioFrameReceipt {
  readonly applied: number;
  readonly diagnostics: readonly AudioProjectionDiagnostic[];
  readonly readout: AudioProjectionReadout;
}

interface AshaAudioSourceGraph {
  descriptor: AudioSourceDescriptor;
  sequence: number;
  origin: AudioProjectionDiagnostic['origin'];
  readonly source: AshaBufferSourceNode;
  readonly dryGain: AshaGainNode;
  readonly wetGain: AshaGainNode;
  readonly stereoPanner: AshaStereoPannerNode;
  readonly panner: AshaPannerNode | null;
  disposed: boolean;
}

export class AshaAudioHost {
  readonly #context: AshaAudioContext;
  readonly #resolveEntityPosition: AshaAudioEntityPositionResolver;
  readonly #resolveResource: AshaAudioResourceResolver;
  readonly #buses: Readonly<Record<AudioBus, AshaGainNode>>;
  readonly #cache = new Map<string, Promise<unknown>>();
  readonly #retained = new Map<number, AshaAudioSourceGraph>();
  readonly #oneShots = new Set<AshaAudioSourceGraph>();
  readonly #seenSignals = new Set<string>();
  readonly #diagnostics: AudioProjectionDiagnostic[] = [];
  #emittedSignals = 0;
  #disposed = false;

  constructor(options: AshaAudioHostOptions) {
    this.#context = options.createContext?.() ?? createBrowserAudioContext();
    this.#resolveResource = options.resolveResource;
    this.#resolveEntityPosition = options.resolveEntityPosition ?? (() => null);
    const sfx = this.#context.createGain();
    const ambient = this.#context.createGain();
    const ui = this.#context.createGain();
    sfx.connect(this.#context.destination);
    ambient.connect(this.#context.destination);
    ui.connect(this.#context.destination);
    this.#buses = { sfx, ambient, ui };
  }

  async resume(): Promise<readonly AudioProjectionDiagnostic[]> {
    try {
      await this.#context.resume();
      if (this.#context.state === 'running') {
        return [];
      }
      return this.#recordHostDiagnostic(
        'audioContextBlocked',
        'audio context remained ' + this.#context.state,
      );
    } catch (error) {
      return this.#recordHostDiagnostic(
        'audioContextBlocked',
        errorMessage(error, 'audio context resume failed'),
      );
    }
  }

  updateListener(pose: AshaAudioListenerPose): readonly AudioProjectionDiagnostic[] {
    if (![...pose.position, ...pose.forward, ...pose.up].every(Number.isFinite)) {
      return this.#recordHostDiagnostic('invalidDescriptor', 'audio listener pose must be finite');
    }
    const time = this.#context.currentTime;
    setVector(this.#context.listener, 'position', pose.position, time);
    setVector(this.#context.listener, 'forward', pose.forward, time);
    setVector(this.#context.listener, 'up', pose.up, time);
    return [];
  }

  async applyPresentation(presentation: PresentationFrameDiff): Promise<AshaAudioFrameReceipt> {
    if (this.#disposed) {
      return this.#receipt(
        0,
        this.#recordHostDiagnostic('hostFailure', 'audio host is disposed'),
      );
    }
    const diagnostics: AudioProjectionDiagnostic[] = [];
    let applied = 0;
    for (const operation of presentation.ops) {
      if (operation.domain !== 'audio') {
        continue;
      }
      const diagnostic = await this.#applyOperation(operation);
      if (diagnostic === null) {
        applied += 1;
      } else {
        diagnostics.push(diagnostic);
        this.#diagnostics.push(diagnostic);
      }
    }
    return this.#receipt(applied, diagnostics);
  }

  readout(): AudioProjectionReadout {
    return {
      activeSources: this.#retained.size,
      cachedClips: this.#cache.size,
      emittedSignals: this.#emittedSignals,
      diagnostics: [...this.#diagnostics],
    };
  }

  refreshLayout(): readonly AudioProjectionDiagnostic[] {
    if (this.#disposed) {
      return this.#recordHostDiagnostic('hostFailure', 'audio host is disposed');
    }
    const diagnostics: AudioProjectionDiagnostic[] = [];
    for (const [handle, graph] of this.#retained) {
      if (graph.descriptor.emitter.kind !== 'entityAttached' || graph.panner === null) {
        continue;
      }
      const position = emitterPosition(graph.descriptor.emitter, this.#resolveEntityPosition);
      if (position === null || !position.every(Number.isFinite)) {
        diagnostics.push({
          code: 'hostFailure',
          sequence: graph.sequence,
          handle: handle as AudioHandle,
          message: 'entity-attached audio source has no finite projected position',
          origin: graph.origin,
        });
        continue;
      }
      setPannerPosition(graph.panner, position, this.#context.currentTime);
    }
    this.#diagnostics.push(...diagnostics);
    return diagnostics;
  }

  async dispose(): Promise<void> {
    if (this.#disposed) {
      return;
    }
    this.#disposed = true;
    for (const graph of [...this.#retained.values(), ...this.#oneShots]) {
      disposeGraph(graph);
    }
    this.#retained.clear();
    this.#oneShots.clear();
    this.#seenSignals.clear();
    for (const bus of Object.values(this.#buses)) {
      bus.disconnect();
    }
    await this.#context.close();
  }

  async #applyOperation(
    operation: Extract<PresentationOp, { readonly domain: 'audio' }>,
  ): Promise<AudioProjectionDiagnostic | null> {
    const { meta, op } = operation;
    try {
      if (op.op === 'emit') {
        if (this.#seenSignals.has(op.signalId)) {
          return null;
        }
        const graph = await this.#createGraph(op.descriptor, meta.sequence, meta.origin);
        this.#seenSignals.add(op.signalId);
        this.#oneShots.add(graph);
        graph.source.onended = () => {
          this.#oneShots.delete(graph);
          disposeGraph(graph);
        };
        graph.source.start();
        this.#emittedSignals += 1;
        return null;
      }
      if (op.op === 'create') {
        if (this.#retained.has(op.handle as number)) {
          return operationDiagnostic('duplicateHandle', meta, op.handle, 'audio handle is active');
        }
        const graph = await this.#createGraph(op.descriptor, meta.sequence, meta.origin);
        this.#retained.set(op.handle as number, graph);
        graph.source.start();
        return null;
      }
      if (op.op === 'destroy') {
        const graph = this.#retained.get(op.handle as number);
        if (graph === undefined) {
          return operationDiagnostic('unknownHandle', meta, op.handle, 'audio handle is unknown');
        }
        this.#retained.delete(op.handle as number);
        disposeGraph(graph);
        return null;
      }
      return await this.#updateGraph(meta, op.handle, op.patch);
    } catch (error) {
      return operationDiagnostic(
        classifyHostError(error),
        meta,
        operationHandle(op),
        errorMessage(error, 'audio host operation failed'),
      );
    }
  }

  async #updateGraph(
    meta: Extract<PresentationOp, { readonly domain: 'audio' }>['meta'],
    handle: AudioHandle,
    patch: AudioSourcePatch,
  ): Promise<AudioProjectionDiagnostic | null> {
    const graph = this.#retained.get(handle as number);
    if (graph === undefined) {
      return operationDiagnostic('unknownHandle', meta, handle, 'audio handle is unknown');
    }
    const next = patchedDescriptor(graph.descriptor, patch);
    if (patch.emitter !== null) {
      const replacement = await this.#createGraph(next, meta.sequence, meta.origin);
      disposeGraph(graph);
      this.#retained.set(handle as number, replacement);
      replacement.source.start();
      return null;
    }
    graph.descriptor = next;
    graph.sequence = meta.sequence;
    graph.origin = meta.origin;
    applyGraphParameters(this.#context, graph, next, this.#resolveEntityPosition);
    return null;
  }

  async #createGraph(
    descriptor: AudioSourceDescriptor,
    sequence: number,
    origin: AudioProjectionDiagnostic['origin'],
  ): Promise<AshaAudioSourceGraph> {
    const source = this.#context.createBufferSource();
    source.buffer = await this.#decodeClip(descriptor.clip);
    const graph: AshaAudioSourceGraph = {
      descriptor,
      sequence,
      origin,
      source,
      dryGain: this.#context.createGain(),
      wetGain: this.#context.createGain(),
      stereoPanner: this.#context.createStereoPanner(),
      panner: descriptor.emitter.kind === 'global2d' ? null : this.#context.createPanner(),
      disposed: false,
    };
    source.connect(graph.stereoPanner);
    graph.stereoPanner.connect(graph.dryGain);
    graph.dryGain.connect(this.#buses[descriptor.bus]);
    if (graph.panner !== null) {
      source.connect(graph.panner);
      graph.panner.connect(graph.wetGain);
      graph.wetGain.connect(this.#buses[descriptor.bus]);
    }
    applyGraphParameters(this.#context, graph, descriptor, this.#resolveEntityPosition);
    return graph;
  }

  async #decodeClip(clip: AudioClipRef): Promise<unknown> {
    const existing = this.#cache.get(clip.contentHash);
    if (existing !== undefined) {
      return existing;
    }
    const decoded = this.#resolveResource(clip).then(async (resource) => {
      if (resource.contentHash !== clip.contentHash) {
        throw new AshaAudioResourceError(
          'contentHashMismatch',
          'resolved audio content hash does not match the catalog projection',
        );
      }
      const actualHash = await sha256Hex(resource.bytes);
      if (actualHash !== clip.contentHash) {
        throw new AshaAudioResourceError(
          'contentHashMismatch',
          `audio bytes hash ${actualHash} does not match ${clip.contentHash}`,
        );
      }
      try {
        return await this.#context.decodeAudioData(resource.bytes.slice(0));
      } catch (error) {
        throw new AshaAudioResourceError(
          'decodeFailed',
          errorMessage(error, 'audio clip decoding failed'),
        );
      }
    });
    this.#cache.set(clip.contentHash, decoded);
    try {
      return await decoded;
    } catch (error) {
      this.#cache.delete(clip.contentHash);
      throw error;
    }
  }

  #recordHostDiagnostic(
    code: AudioProjectionDiagnostic['code'],
    message: string,
  ): readonly AudioProjectionDiagnostic[] {
    const diagnostic = hostDiagnostic(code, message);
    this.#diagnostics.push(diagnostic);
    return [diagnostic];
  }

  #receipt(
    applied: number,
    diagnostics: readonly AudioProjectionDiagnostic[],
  ): AshaAudioFrameReceipt {
    return { applied, diagnostics, readout: this.readout() };
  }
}

async function sha256Hex(data: ArrayBuffer): Promise<string> {
  if (globalThis.crypto?.subtle === undefined) {
    throw new AshaAudioResourceError(
      'contentHashMismatch',
      'Web Crypto SHA-256 is unavailable for audio content validation',
    );
  }
  const digest = await globalThis.crypto.subtle.digest('SHA-256', data);
  return [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('');
}

export interface AshaRuntimeProjectionApplicationPorts {
  readonly applyScene: (frame: RuntimeProjectionFrame['scene']) => void;
  readonly audioHost?: AshaAudioHost;
  readonly billboardHost?: AshaBillboardHost;
  readonly particleHost?: AshaParticleHost;
  readonly telemetryOverlayHost?: AshaTelemetryOverlayHost;
  readonly animationHost?: AshaAnimationHost;
}

export interface AshaRuntimeProjectionApplicationReceipt {
  readonly authorityTick: number;
  readonly sceneApplied: boolean;
  readonly audio: AshaAudioFrameReceipt;
  readonly billboard: AshaBillboardFrameReceipt;
  readonly particle: AshaParticleFrameReceipt;
  readonly telemetryOverlay: AshaTelemetryOverlayFrameReceipt;
  readonly animation: AshaAnimationFrameReceipt;
}

export async function applyAshaRuntimeProjectionFrame(
  frame: RuntimeProjectionFrame,
  ports: AshaRuntimeProjectionApplicationPorts,
): Promise<AshaRuntimeProjectionApplicationReceipt> {
  validateRuntimeProjectionFrame(frame);
  ports.applyScene(frame.scene);
  let audio: AshaAudioFrameReceipt = emptyAudioReceipt();
  let billboard: AshaBillboardFrameReceipt = emptyBillboardReceipt();
  let particle: AshaParticleFrameReceipt = emptyParticleReceipt();
  let telemetryOverlay: AshaTelemetryOverlayFrameReceipt = emptyTelemetryOverlayReceipt();
  let animation: AshaAnimationFrameReceipt = emptyAnimationReceipt();
  for (const operation of frame.presentation.ops) {
    const singleOperationFrame: PresentationFrameDiff = {
      replayScope: frame.presentation.replayScope,
      ops: [operation],
    };
    if (operation.domain === 'audio') {
      const next = ports.audioHost === undefined
        ? unavailableAudioReceipt(singleOperationFrame)
        : await ports.audioHost.applyPresentation(singleOperationFrame);
      audio = mergeAudioReceipts(audio, next);
    } else if (operation.domain === 'billboard') {
      const next = ports.billboardHost === undefined
        ? unavailableBillboardReceipt(singleOperationFrame)
        : await ports.billboardHost.applyPresentation(singleOperationFrame);
      billboard = mergeBillboardReceipts(billboard, next);
    } else if (operation.domain === 'particle') {
      const next = ports.particleHost === undefined
        ? unavailableParticleReceipt(singleOperationFrame)
        : await ports.particleHost.applyPresentation(singleOperationFrame);
      particle = mergeParticleReceipts(particle, next);
    } else if (operation.domain === 'animation') {
      const next = ports.animationHost === undefined
        ? unavailableAnimationReceipt(singleOperationFrame)
        : ports.animationHost.applyPresentation(singleOperationFrame);
      animation = mergeAnimationReceipts(animation, next);
    } else {
      const next = ports.telemetryOverlayHost === undefined
        ? unavailableTelemetryOverlayReceipt(singleOperationFrame)
        : ports.telemetryOverlayHost.applyPresentation(singleOperationFrame);
      telemetryOverlay = mergeTelemetryOverlayReceipts(telemetryOverlay, next);
    }
  }
  if (ports.audioHost !== undefined) {
    const refreshDiagnostics = ports.audioHost.refreshLayout();
    audio = {
      applied: audio.applied,
      diagnostics: [...audio.diagnostics, ...refreshDiagnostics],
      readout: ports.audioHost.readout(),
    };
  }
  return {
    authorityTick: frame.authorityTick,
    sceneApplied: true,
    audio,
    billboard,
    particle,
    telemetryOverlay,
    animation,
  };
}

function emptyAudioReceipt(): AshaAudioFrameReceipt {
  return {
    applied: 0,
    diagnostics: [],
    readout: { activeSources: 0, cachedClips: 0, emittedSignals: 0, diagnostics: [] },
  };
}

function emptyBillboardReceipt(): AshaBillboardFrameReceipt {
  return {
    applied: 0,
    diagnostics: [],
    readout: {
      activeBillboards: 0,
      loadedFonts: 0,
      loadedIcons: 0,
      culledBillboards: 0,
      diagnostics: [],
    },
  };
}

function emptyParticleReceipt(): AshaParticleFrameReceipt {
  return {
    applied: 0,
    diagnostics: [],
    readout: {
      activeEmitters: 0,
      activeParticles: 0,
      loadedSprites: 0,
      emittedBursts: 0,
      droppedParticles: 0,
      diagnostics: [],
    },
  };
}

function emptyTelemetryOverlayReceipt(): AshaTelemetryOverlayFrameReceipt {
  return {
    applied: 0,
    diagnostics: [],
    readout: { activeOverlays: 0, renderedSnapshots: 0, diagnostics: [] },
  };
}

function emptyAnimationReceipt(): AshaAnimationFrameReceipt {
  return {
    applied: 0,
    diagnostics: [],
    cues: [],
    readout: {
      activeControllers: 0,
      sampledFrames: 0,
      compatibilityFallbacks: 0,
      diagnostics: [],
    },
  };
}

function mergeAudioReceipts(
  prior: AshaAudioFrameReceipt,
  next: AshaAudioFrameReceipt,
): AshaAudioFrameReceipt {
  return {
    applied: prior.applied + next.applied,
    diagnostics: [...prior.diagnostics, ...next.diagnostics],
    readout: next.readout,
  };
}

function mergeBillboardReceipts(
  prior: AshaBillboardFrameReceipt,
  next: AshaBillboardFrameReceipt,
): AshaBillboardFrameReceipt {
  return {
    applied: prior.applied + next.applied,
    diagnostics: [...prior.diagnostics, ...next.diagnostics],
    readout: next.readout,
  };
}

function mergeParticleReceipts(
  prior: AshaParticleFrameReceipt,
  next: AshaParticleFrameReceipt,
): AshaParticleFrameReceipt {
  return {
    applied: prior.applied + next.applied,
    diagnostics: [...prior.diagnostics, ...next.diagnostics],
    readout: next.readout,
  };
}

function mergeTelemetryOverlayReceipts(
  prior: AshaTelemetryOverlayFrameReceipt,
  next: AshaTelemetryOverlayFrameReceipt,
): AshaTelemetryOverlayFrameReceipt {
  return {
    applied: prior.applied + next.applied,
    diagnostics: [...prior.diagnostics, ...next.diagnostics],
    readout: next.readout,
  };
}

function mergeAnimationReceipts(
  prior: AshaAnimationFrameReceipt,
  next: AshaAnimationFrameReceipt,
): AshaAnimationFrameReceipt {
  return {
    applied: prior.applied + next.applied,
    diagnostics: [...prior.diagnostics, ...next.diagnostics],
    cues: [...prior.cues, ...next.cues],
    readout: next.readout,
  };
}

function unavailableAudioReceipt(frame: PresentationFrameDiff): AshaAudioFrameReceipt {
  const diagnostics = frame.ops
    .filter(
      (value): value is Extract<PresentationOp, { readonly domain: 'audio' }> =>
        value.domain === 'audio',
    )
    .map((value) =>
      operationDiagnostic(
        'unavailableHost',
        value.meta,
        operationHandle(value.op),
        'audio host capability is unavailable',
      ),
    );
  return {
    applied: 0,
    diagnostics,
    readout: {
      activeSources: 0,
      cachedClips: 0,
      emittedSignals: 0,
      diagnostics,
    },
  };
}

function unavailableBillboardReceipt(frame: PresentationFrameDiff): AshaBillboardFrameReceipt {
  const diagnostics: BillboardProjectionDiagnostic[] = frame.ops
    .filter(
      (value): value is Extract<PresentationOp, { readonly domain: 'billboard' }> =>
        value.domain === 'billboard',
    )
    .map((value) => ({
      code: 'unavailableHost',
      sequence: value.meta.sequence,
      handle: value.op.handle,
      message: 'billboard host capability is unavailable',
      origin: value.meta.origin,
    }));
  return {
    applied: 0,
    diagnostics,
    readout: {
      activeBillboards: 0,
      loadedFonts: 0,
      loadedIcons: 0,
      culledBillboards: 0,
      diagnostics,
    },
  };
}

function unavailableParticleReceipt(frame: PresentationFrameDiff): AshaParticleFrameReceipt {
  const diagnostics: ParticleProjectionDiagnostic[] = frame.ops
    .filter(
      (value): value is Extract<PresentationOp, { readonly domain: 'particle' }> =>
        value.domain === 'particle',
    )
    .map((value) => ({
      code: 'unavailableHost',
      sequence: value.meta.sequence,
      handle: value.op.op === 'emit' ? null : value.op.handle,
      message: 'particle host capability is unavailable',
      origin: value.meta.origin,
    }));
  return {
    applied: 0,
    diagnostics,
    readout: {
      activeEmitters: 0,
      activeParticles: 0,
      loadedSprites: 0,
      emittedBursts: 0,
      droppedParticles: 0,
      diagnostics,
    },
  };
}

function unavailableTelemetryOverlayReceipt(
  frame: PresentationFrameDiff,
): AshaTelemetryOverlayFrameReceipt {
  const diagnostics: TelemetryOverlayDiagnostic[] = frame.ops
    .filter(
      (value): value is Extract<PresentationOp, { readonly domain: 'telemetryOverlay' }> =>
        value.domain === 'telemetryOverlay',
    )
    .map((value) => ({
      code: 'unavailableHost',
      sequence: value.meta.sequence,
      handle: value.op.handle,
      message: 'telemetry overlay host capability is unavailable',
      origin: value.meta.origin,
    }));
  return {
    applied: 0,
    diagnostics,
    readout: { activeOverlays: 0, renderedSnapshots: 0, diagnostics },
  };
}

function unavailableAnimationReceipt(frame: PresentationFrameDiff): AshaAnimationFrameReceipt {
  const diagnostics = frame.ops
    .filter(
      (value): value is Extract<PresentationOp, { readonly domain: 'animation' }> =>
        value.domain === 'animation',
    )
    .map((value) => ({
      code: 'unavailableHost' as const,
      sequence: value.meta.sequence,
      handle: value.op.handle,
      target: value.op.op === 'create' ? value.op.descriptor.target : null,
      message: 'animation host capability is unavailable',
      origin: value.meta.origin,
    }));
  return {
    applied: 0,
    diagnostics,
    cues: [],
    readout: {
      activeControllers: 0,
      sampledFrames: 0,
      compatibilityFallbacks: 0,
      diagnostics,
    },
  };
}

export function validateRuntimeProjectionFrame(frame: RuntimeProjectionFrame): void {
  if (frame.schemaVersion !== 1) {
    throw new Error('unsupported RuntimeProjectionFrame schema ' + frame.schemaVersion);
  }
  if (frame.presentation.replayScope !== 'excludedFromReplayTruth') {
    throw new Error('presentation replay scope must be excludedFromReplayTruth');
  }
  frame.presentation.ops.forEach((operation, index) => {
    if (operation.meta.sequence !== index) {
      throw new Error(
        'presentation sequence must be contiguous: expected ' +
          index +
          ', got ' +
          operation.meta.sequence,
      );
    }
  });
}

class AshaAudioResourceError extends Error {
  constructor(
    readonly code: 'contentHashMismatch' | 'decodeFailed',
    message: string,
  ) {
    super(message);
  }
}

function createBrowserAudioContext(): AshaAudioContext {
  const Context = globalThis.AudioContext;
  if (Context === undefined) {
    throw new Error('Web Audio AudioContext is unavailable');
  }
  return new Context() as unknown as AshaAudioContext;
}

function applyGraphParameters(
  context: AshaAudioContext,
  graph: AshaAudioSourceGraph,
  descriptor: AudioSourceDescriptor,
  resolveEntityPosition: AshaAudioEntityPositionResolver,
): void {
  const time = context.currentTime;
  graph.source.loop = descriptor.looping;
  graph.source.playbackRate.setValueAtTime(descriptor.pitch, time);
  graph.stereoPanner.pan.setValueAtTime(descriptor.pan, time);
  const blend = descriptor.emitter.kind === 'global2d' ? 0 : descriptor.spatialBlend;
  graph.dryGain.gain.setValueAtTime(descriptor.volume * (1 - blend), time);
  graph.wetGain.gain.setValueAtTime(descriptor.volume * blend, time);
  if (graph.panner === null) {
    return;
  }
  const position = emitterPosition(descriptor.emitter, resolveEntityPosition);
  if (position === null) {
    throw new Error('entity-attached audio source has no projected position');
  }
  graph.panner.panningModel = 'equalpower';
  graph.panner.distanceModel = 'inverse';
  graph.panner.refDistance = 1;
  graph.panner.maxDistance = descriptor.attenuation;
  graph.panner.rolloffFactor = 1;
  setPannerPosition(graph.panner, position, time);
}

function setPannerPosition(
  panner: AshaPannerNode,
  position: readonly [number, number, number],
  time: number,
): void {
  panner.positionX.setValueAtTime(position[0], time);
  panner.positionY.setValueAtTime(position[1], time);
  panner.positionZ.setValueAtTime(position[2], time);
}

function setVector(
  listener: AshaAudioListener,
  prefix: 'position' | 'forward' | 'up',
  value: readonly [number, number, number],
  time: number,
): void {
  listener[`${prefix}X`].setValueAtTime(value[0], time);
  listener[`${prefix}Y`].setValueAtTime(value[1], time);
  listener[`${prefix}Z`].setValueAtTime(value[2], time);
}

function emitterPosition(
  emitter: AudioEmitter,
  resolveEntityPosition: AshaAudioEntityPositionResolver,
): readonly [number, number, number] | null {
  if (emitter.kind === 'global2d') {
    return [0, 0, 0];
  }
  if (emitter.kind === 'world3d') {
    return emitter.position;
  }
  const base = resolveEntityPosition(emitter.entity as number);
  return base === null
    ? null
    : [
        base[0] + emitter.offset[0],
        base[1] + emitter.offset[1],
        base[2] + emitter.offset[2],
      ];
}

function patchedDescriptor(
  descriptor: AudioSourceDescriptor,
  patch: AudioSourcePatch,
): AudioSourceDescriptor {
  return {
    ...descriptor,
    volume: patch.volume ?? descriptor.volume,
    pitch: patch.pitch ?? descriptor.pitch,
    looping: patch.looping ?? descriptor.looping,
    spatialBlend: patch.spatialBlend ?? descriptor.spatialBlend,
    attenuation: patch.attenuation ?? descriptor.attenuation,
    pan: patch.pan ?? descriptor.pan,
    emitter: patch.emitter ?? descriptor.emitter,
  };
}

function disposeGraph(graph: AshaAudioSourceGraph): void {
  if (graph.disposed) {
    return;
  }
  graph.disposed = true;
  graph.source.onended = null;
  try {
    graph.source.stop();
  } catch {
    // A naturally-ended one-shot is already stopped.
  }
  graph.source.disconnect();
  graph.stereoPanner.disconnect();
  graph.dryGain.disconnect();
  graph.panner?.disconnect();
  graph.wetGain.disconnect();
}

function operationHandle(op: AudioProjectionOp): AudioHandle | null {
  return op.op === 'emit' ? null : op.handle;
}

function operationDiagnostic(
  code: AudioProjectionDiagnostic['code'],
  meta: Extract<PresentationOp, { readonly domain: 'audio' }>['meta'],
  handle: AudioHandle | null,
  message: string,
): AudioProjectionDiagnostic {
  return { code, sequence: meta.sequence, handle, message, origin: meta.origin };
}

function hostDiagnostic(
  code: AudioProjectionDiagnostic['code'],
  message: string,
): AudioProjectionDiagnostic {
  return { code, sequence: 0, handle: null, message, origin: null };
}

function classifyHostError(error: unknown): AudioProjectionDiagnostic['code'] {
  return error instanceof AshaAudioResourceError ? error.code : 'hostFailure';
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}
