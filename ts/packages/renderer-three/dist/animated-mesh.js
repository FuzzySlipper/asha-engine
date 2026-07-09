import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import * as SkeletonUtils from 'three/examples/jsm/utils/SkeletonUtils.js';
export class AnimatedMeshApplyError extends Error {
    constructor(message) {
        super(message);
        this.name = 'AnimatedMeshApplyError';
    }
}
export class MapAnimatedMeshAssetSource {
    #resources = new Map();
    constructor(resources) {
        for (const resource of resources) {
            this.#resources.set(resource.asset, resource);
        }
    }
    getAnimatedMeshResource(asset) {
        return this.#resources.get(asset.asset);
    }
}
export async function loadAnimatedMeshGlbResource(asset, data) {
    const loader = new GLTFLoader();
    const gltf = await new Promise((resolve, reject) => {
        loader.parse(data, '', resolve, reject);
    });
    return { asset, scene: gltf.scene, clips: gltf.animations };
}
export class AnimatedMeshRegistry {
    #assetSource;
    #assets = new Map();
    #instances = new Map();
    constructor(assetSource) {
        this.#assetSource = assetSource;
    }
    define(asset) {
        const existing = this.#assets.get(asset.asset);
        if (existing && existing.refCount > 0) {
            throw new AnimatedMeshApplyError(`defineAnimatedMesh: asset ${asset.asset} is in use by ${existing.refCount} instance(s)`);
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
    create(handle, instance) {
        const record = this.#assets.get(instance.asset);
        if (!record) {
            throw new AnimatedMeshApplyError(`createAnimatedMeshInstance: undefined animated mesh asset ${instance.asset}`);
        }
        if (instance.materialOverrides.length > 0) {
            throw new AnimatedMeshApplyError(`createAnimatedMeshInstance: material overrides are not implemented for animated mesh ${instance.asset}`);
        }
        const object = SkeletonUtils.clone(record.resource.scene);
        const mixer = new THREE.AnimationMixer(object);
        const actions = new Map();
        for (const clip of record.asset.clips) {
            actions.set(clip.id, mixer.clipAction(requireClip(record.resource, clip.id, clip.name)));
        }
        const instanceRecord = {
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
    setPlayback(handle, command) {
        const instance = this.#requireInstance(handle, 'setAnimatedMeshPlayback');
        applyPlaybackCommand(instance, command);
    }
    advance(deltaSeconds) {
        if (!Number.isFinite(deltaSeconds) || deltaSeconds < 0) {
            throw new AnimatedMeshApplyError(`advanceAnimation: deltaSeconds must be finite and non-negative`);
        }
        for (const instance of this.#instances.values()) {
            instance.mixer.update(deltaSeconds);
        }
    }
    playback(handle) {
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
    release(handle) {
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
    #requireInstance(handle, ctx) {
        const instance = this.#instances.get(handle);
        if (!instance) {
            throw new AnimatedMeshApplyError(`${ctx}: handle ${handle} is not an animated mesh`);
        }
        return instance;
    }
}
function assertClipDescriptors(asset, resource) {
    for (const clip of asset.clips) {
        requireClip(resource, clip.id, clip.name);
    }
}
function requireClip(resource, id, name) {
    const clip = resource.clips.find((candidate) => candidate.name === id || (name !== null && candidate.name === name));
    if (!clip) {
        throw new AnimatedMeshApplyError(`animated mesh ${resource.asset} does not contain clip ${id}`);
    }
    return clip;
}
function applyPlaybackCommand(instance, command) {
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
function playClip(instance, command) {
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
        }
        else {
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
function stopCurrent(instance, fadeSeconds) {
    const action = instance.currentClip === null ? null : instance.actions.get(instance.currentClip) ?? null;
    if (!action) {
        return;
    }
    if (fadeSeconds !== null && fadeSeconds > 0) {
        action.fadeOut(fadeSeconds);
    }
    else {
        action.stop();
    }
}
function currentAction(instance, ctx) {
    const action = instance.currentClip === null ? null : instance.actions.get(instance.currentClip) ?? null;
    if (!action) {
        throw new AnimatedMeshApplyError(`setAnimatedMeshPlayback.${ctx}: no current clip on ${instance.asset}`);
    }
    return action;
}
function toThreeLoop(loop) {
    switch (loop) {
        case 'once':
            return THREE.LoopOnce;
        case 'repeat':
            return THREE.LoopRepeat;
        case 'pingPong':
            return THREE.LoopPingPong;
    }
}
function poseSample(object) {
    return {
        rootTranslation: [object.position.x, object.position.y, object.position.z],
        rootRotation: [object.quaternion.x, object.quaternion.y, object.quaternion.z, object.quaternion.w],
        rootScale: [object.scale.x, object.scale.y, object.scale.z],
    };
}
function playbackDiagnostics(instance, action) {
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
//# sourceMappingURL=animated-mesh.js.map