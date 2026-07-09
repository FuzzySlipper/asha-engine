import * as THREE from 'three';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import * as SkeletonUtils from 'three/examples/jsm/utils/SkeletonUtils.js';
import type {
  AnimatedMeshAsset,
  AnimatedMeshInstanceDescriptor,
  AnimatedMeshPlaybackCommand,
  RenderHandle,
} from '@asha/contracts';

export class AnimatedMeshApplyError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'AnimatedMeshApplyError';
  }
}

export interface AnimatedMeshResource {
  readonly asset: string;
  readonly scene: THREE.Object3D;
  readonly clips: readonly THREE.AnimationClip[];
}

export interface AnimatedMeshAssetSource {
  getAnimatedMeshResource(asset: AnimatedMeshAsset): AnimatedMeshResource | undefined;
}

export interface AnimatedMeshPlaybackReadout {
  readonly handle: RenderHandle;
  readonly asset: string;
  readonly status: 'not_started' | 'playing' | 'paused' | 'stopped';
  readonly currentClip: string | null;
  readonly mixerTimeSeconds: number;
  readonly actionTimeSeconds: number | null;
  readonly running: boolean;
  readonly paused: boolean;
  readonly loop: 'once' | 'repeat' | 'pingPong' | null;
  readonly speed: number | null;
  readonly weight: number | null;
  readonly commandSelected: boolean;
  readonly poseSample: AnimatedMeshPoseSample;
  readonly diagnostics: readonly string[];
}

export interface AnimatedMeshPoseSample {
  readonly rootTranslation: readonly [number, number, number];
  readonly rootRotation: readonly [number, number, number, number];
  readonly rootScale: readonly [number, number, number];
}

interface AnimatedMeshAssetRecord {
  readonly asset: AnimatedMeshAsset;
  readonly resource: AnimatedMeshResource;
  refCount: number;
}

interface AnimatedMeshInstanceRecord {
  readonly handle: RenderHandle;
  readonly asset: string;
  readonly object: THREE.Object3D;
  readonly mixer: THREE.AnimationMixer;
  readonly actions: ReadonlyMap<string, THREE.AnimationAction>;
  currentClip: string | null;
  commandSelected: boolean;
  status: AnimatedMeshPlaybackReadout['status'];
  loop: AnimatedMeshPlaybackReadout['loop'];
  speed: number | null;
  weight: number | null;
}

export class MapAnimatedMeshAssetSource implements AnimatedMeshAssetSource {
  readonly #resources = new Map<string, AnimatedMeshResource>();

  constructor(resources: readonly AnimatedMeshResource[]) {
    for (const resource of resources) {
      this.#resources.set(resource.asset, resource);
    }
  }

  getAnimatedMeshResource(asset: AnimatedMeshAsset): AnimatedMeshResource | undefined {
    return this.#resources.get(asset.asset);
  }
}

export async function loadAnimatedMeshGlbResource(
  asset: string,
  data: ArrayBuffer,
): Promise<AnimatedMeshResource> {
  const loader = new GLTFLoader();
  const gltf = await new Promise<GLTF>((resolve, reject) => {
    loader.parse(data, '', resolve, reject);
  });
  return { asset, scene: gltf.scene, clips: gltf.animations };
}

export class AnimatedMeshRegistry {
  readonly #assetSource: AnimatedMeshAssetSource | undefined;
  readonly #assets = new Map<string, AnimatedMeshAssetRecord>();
  readonly #instances = new Map<RenderHandle, AnimatedMeshInstanceRecord>();

  constructor(assetSource: AnimatedMeshAssetSource | undefined) {
    this.#assetSource = assetSource;
  }

  define(asset: AnimatedMeshAsset): void {
    const existing = this.#assets.get(asset.asset);
    if (existing && existing.refCount > 0) {
      throw new AnimatedMeshApplyError(
        `defineAnimatedMesh: asset ${asset.asset} is in use by ${existing.refCount} instance(s)`,
      );
    }
    if (asset.runtimeFormat !== 'glb') {
      throw new AnimatedMeshApplyError(`defineAnimatedMesh: unsupported runtime format ${asset.runtimeFormat}`);
    }
    const resource = this.#assetSource?.getAnimatedMeshResource(asset);
    if (!resource) {
      throw new AnimatedMeshApplyError(`defineAnimatedMesh: missing animated mesh resource ${asset.asset}`);
    }
    assertClipDescriptors(asset, resource);
    this.#assets.set(asset.asset, { asset, resource, refCount: 0 });
  }

  create(handle: RenderHandle, instance: AnimatedMeshInstanceDescriptor): AnimatedMeshInstanceRecord {
    const record = this.#assets.get(instance.asset);
    if (!record) {
      throw new AnimatedMeshApplyError(`createAnimatedMeshInstance: undefined animated mesh asset ${instance.asset}`);
    }
    if (instance.materialOverrides.length > 0) {
      throw new AnimatedMeshApplyError(
        `createAnimatedMeshInstance: material overrides are not implemented for animated mesh ${instance.asset}`,
      );
    }
    const object = SkeletonUtils.clone(record.resource.scene);
    const mixer = new THREE.AnimationMixer(object);
    const actions = new Map<string, THREE.AnimationAction>();
    for (const clip of record.asset.clips) {
      actions.set(clip.id, mixer.clipAction(requireClip(record.resource, clip.id, clip.name)));
    }
    const instanceRecord: AnimatedMeshInstanceRecord = {
      handle,
      asset: instance.asset,
      object,
      mixer,
      actions,
      currentClip: null,
      commandSelected: false,
      status: 'not_started',
      loop: null,
      speed: null,
      weight: null,
    };
    this.#instances.set(handle, instanceRecord);
    record.refCount += 1;
    if (instance.playback) {
      this.setPlayback(handle, instance.playback);
    }
    return instanceRecord;
  }

  setPlayback(handle: RenderHandle, command: AnimatedMeshPlaybackCommand): void {
    const instance = this.#requireInstance(handle, 'setAnimatedMeshPlayback');
    applyPlaybackCommand(instance, command);
  }

  advance(deltaSeconds: number): void {
    if (!Number.isFinite(deltaSeconds) || deltaSeconds < 0) {
      throw new AnimatedMeshApplyError(`advanceAnimation: deltaSeconds must be finite and non-negative`);
    }
    for (const instance of this.#instances.values()) {
      instance.mixer.update(deltaSeconds);
    }
  }

  playback(handle: RenderHandle): AnimatedMeshPlaybackReadout | undefined {
    const instance = this.#instances.get(handle);
    if (!instance) {
      return undefined;
    }
    const action = instance.currentClip === null ? null : instance.actions.get(instance.currentClip) ?? null;
    return {
      handle,
      asset: instance.asset,
      status: instance.status,
      currentClip: instance.currentClip,
      mixerTimeSeconds: instance.mixer.time,
      actionTimeSeconds: action?.time ?? null,
      running: action?.isRunning() ?? false,
      paused: action?.paused ?? false,
      loop: instance.loop,
      speed: instance.speed,
      weight: instance.weight,
      commandSelected: instance.commandSelected,
      poseSample: poseSample(instance.object),
      diagnostics: playbackDiagnostics(instance, action),
    };
  }

  release(handle: RenderHandle): void {
    const instance = this.#instances.get(handle);
    if (!instance) {
      return;
    }
    instance.mixer.stopAllAction();
    this.#instances.delete(handle);
    const asset = this.#assets.get(instance.asset);
    if (asset) {
      asset.refCount -= 1;
    }
  }

  #requireInstance(handle: RenderHandle, ctx: string): AnimatedMeshInstanceRecord {
    const instance = this.#instances.get(handle);
    if (!instance) {
      throw new AnimatedMeshApplyError(`${ctx}: handle ${handle} is not an animated mesh`);
    }
    return instance;
  }
}

function assertClipDescriptors(asset: AnimatedMeshAsset, resource: AnimatedMeshResource): void {
  for (const clip of asset.clips) {
    requireClip(resource, clip.id, clip.name);
  }
}

function requireClip(
  resource: AnimatedMeshResource,
  id: string,
  name: string | null,
): THREE.AnimationClip {
  const clip = resource.clips.find((candidate) => candidate.name === id || (name !== null && candidate.name === name));
  if (!clip) {
    throw new AnimatedMeshApplyError(`animated mesh ${resource.asset} does not contain clip ${id}`);
  }
  return clip;
}

function applyPlaybackCommand(
  instance: AnimatedMeshInstanceRecord,
  command: AnimatedMeshPlaybackCommand,
): void {
  switch (command.action) {
    case 'play':
      playClip(instance, command);
      return;
    case 'stop':
      stopCurrent(instance, command.fadeSeconds);
      instance.currentClip = null;
      instance.commandSelected = true;
      instance.status = 'stopped';
      instance.loop = null;
      instance.speed = null;
      instance.weight = null;
      return;
    case 'pause':
      currentAction(instance, 'pause').paused = true;
      instance.commandSelected = true;
      instance.status = 'paused';
      return;
    case 'resume': {
      const action = currentAction(instance, 'resume');
      action.paused = false;
      action.play();
      instance.commandSelected = true;
      instance.status = 'playing';
      return;
    }
  }
}

function playClip(
  instance: AnimatedMeshInstanceRecord,
  command: Extract<AnimatedMeshPlaybackCommand, { readonly action: 'play' }>,
): void {
  const action = instance.actions.get(command.clip);
  if (!action) {
    throw new AnimatedMeshApplyError(`setAnimatedMeshPlayback: missing clip ${command.clip} on ${instance.asset}`);
  }
  const prior = instance.currentClip === null ? null : instance.actions.get(instance.currentClip) ?? null;
  if (command.restart) {
    action.reset();
  }
  action.enabled = true;
  action.paused = false;
  action.clampWhenFinished = command.loop === 'once';
  action.setLoop(toThreeLoop(command.loop), command.loop === 'once' ? 1 : Infinity);
  action.setEffectiveTimeScale(command.speed);
  action.setEffectiveWeight(command.weight);
  if (prior && prior !== action) {
    if (command.fadeSeconds !== null && command.fadeSeconds > 0) {
      action.crossFadeFrom(prior, command.fadeSeconds, false);
    } else {
      prior.stop();
    }
  }
  action.play();
  instance.currentClip = command.clip;
  instance.commandSelected = true;
  instance.status = 'playing';
  instance.loop = command.loop;
  instance.speed = command.speed;
  instance.weight = command.weight;
}

function stopCurrent(instance: AnimatedMeshInstanceRecord, fadeSeconds: number | null): void {
  const action = instance.currentClip === null ? null : instance.actions.get(instance.currentClip) ?? null;
  if (!action) {
    return;
  }
  if (fadeSeconds !== null && fadeSeconds > 0) {
    action.fadeOut(fadeSeconds);
  } else {
    action.stop();
  }
}

function currentAction(instance: AnimatedMeshInstanceRecord, ctx: string): THREE.AnimationAction {
  const action = instance.currentClip === null ? null : instance.actions.get(instance.currentClip) ?? null;
  if (!action) {
    throw new AnimatedMeshApplyError(`setAnimatedMeshPlayback.${ctx}: no current clip on ${instance.asset}`);
  }
  return action;
}

function toThreeLoop(loop: 'once' | 'repeat' | 'pingPong'): THREE.AnimationActionLoopStyles {
  switch (loop) {
    case 'once':
      return THREE.LoopOnce;
    case 'repeat':
      return THREE.LoopRepeat;
    case 'pingPong':
      return THREE.LoopPingPong;
  }
}

function poseSample(object: THREE.Object3D): AnimatedMeshPoseSample {
  return {
    rootTranslation: [object.position.x, object.position.y, object.position.z],
    rootRotation: [object.quaternion.x, object.quaternion.y, object.quaternion.z, object.quaternion.w],
    rootScale: [object.scale.x, object.scale.y, object.scale.z],
  };
}

function playbackDiagnostics(
  instance: AnimatedMeshInstanceRecord,
  action: THREE.AnimationAction | null,
): readonly string[] {
  if (!instance.commandSelected) {
    return ['animation_not_started'];
  }
  if (instance.status === 'stopped') {
    return ['animation_stopped'];
  }
  if (action?.paused || instance.status === 'paused') {
    return ['animation_paused'];
  }
  return [];
}
