export class AshaAnimationHost {
    #projection;
    #cues;
    #controllers = new Map();
    #diagnostics = [];
    #sampledFrames = 0;
    #compatibilityFallbacks = 0;
    constructor(projection, options = {}) {
        this.#projection = projection;
        this.#cues = validateCueDefinitions(options.cues ?? []);
    }
    applyPresentation(frame) {
        const diagnostics = [];
        let applied = 0;
        for (const operation of frame.ops) {
            if (operation.domain !== 'animation') {
                continue;
            }
            const diagnostic = this.#applyOperation(operation);
            if (diagnostic === null) {
                applied += 1;
            }
            else {
                diagnostics.push(diagnostic);
                this.#diagnostics.push(diagnostic);
            }
        }
        return { applied, diagnostics, cues: [], readout: this.readout() };
    }
    advance(deltaSeconds) {
        if (!Number.isFinite(deltaSeconds) || deltaSeconds < 0) {
            throw new Error('animation host deltaSeconds must be finite and non-negative');
        }
        const diagnostics = [];
        const cues = [];
        for (const realization of this.#controllers.values()) {
            const interpolation = realization.interpolation;
            if (interpolation !== null) {
                interpolation.elapsedSeconds = Math.min(interpolation.durationSeconds, interpolation.elapsedSeconds + deltaSeconds);
                const progress = interpolation.durationSeconds === 0
                    ? 1
                    : interpolation.elapsedSeconds / interpolation.durationSeconds;
                realization.presented = interpolateWeights(interpolation.from, interpolation.to, progress);
                try {
                    this.#projection.setAnimationControllerWeights(realization.target, realization.presented);
                }
                catch (cause) {
                    const diagnostic = animationDiagnostic('hostFailure', 0, realization.handle, realization.target, errorMessage(cause), realization.origin);
                    diagnostics.push(diagnostic);
                    this.#diagnostics.push(diagnostic);
                }
                if (progress === 1) {
                    realization.interpolation = null;
                }
            }
            cues.push(...sampleAnimationCues(realization, this.#cues, deltaSeconds));
        }
        this.#projection.advance(deltaSeconds);
        this.#sampledFrames += 1;
        return { applied: this.#controllers.size, diagnostics, cues, readout: this.readout() };
    }
    readout() {
        return {
            activeControllers: this.#controllers.size,
            sampledFrames: this.#sampledFrames,
            compatibilityFallbacks: this.#compatibilityFallbacks,
            diagnostics: [...this.#diagnostics],
        };
    }
    cleanup() {
        const diagnostics = [];
        let applied = 0;
        for (const realization of this.#controllers.values()) {
            try {
                this.#projection.clearAnimationControllerWeights(realization.target);
                applied += 1;
            }
            catch (cause) {
                const diagnostic = animationDiagnostic('hostFailure', 0, realization.handle, realization.target, errorMessage(cause), null);
                diagnostics.push(diagnostic);
                this.#diagnostics.push(diagnostic);
            }
        }
        this.#controllers.clear();
        return { applied, diagnostics, cues: [], readout: this.readout() };
    }
    #applyOperation(operation) {
        const { op, meta } = operation;
        if (op.op === 'create') {
            if (this.#controllers.has(op.handle)) {
                return animationDiagnostic('duplicateHandle', meta.sequence, op.handle, op.descriptor.target, 'animation handle already exists', meta.origin);
            }
            const validation = validateController(op.descriptor.controller);
            if (validation !== null || op.descriptor.tickDurationMillis === 0) {
                return animationDiagnostic('invalidDescriptor', meta.sequence, op.handle, op.descriptor.target, validation ?? 'tick duration must be non-zero', meta.origin);
            }
            if (!this.#projection.hasAnimationTarget(op.descriptor.target)) {
                return animationDiagnostic('unknownTarget', meta.sequence, op.handle, op.descriptor.target, 'animation target is unavailable', meta.origin);
            }
            const playback = this.#projection.playback(op.descriptor.target);
            if (playback.asset === null) {
                return animationDiagnostic('assetMissing', meta.sequence, op.handle, op.descriptor.target, 'animation target has no loaded asset', meta.origin);
            }
            if (playback.asset !== op.descriptor.asset) {
                return animationDiagnostic('incompatibleRig', meta.sequence, op.handle, op.descriptor.target, 'animation descriptor asset does not match the target rig', meta.origin);
            }
            const weights = controllerWeights(op.descriptor.controller);
            if (!this.#projection.hasAnimationClips(op.descriptor.target, weights.map((clip) => clip.clip))) {
                return animationDiagnostic('clipMissing', meta.sequence, op.handle, op.descriptor.target, 'controller references an unavailable clip', meta.origin);
            }
            try {
                this.#projection.setAnimationControllerWeights(op.descriptor.target, weights);
            }
            catch (cause) {
                return hostDiagnostic(cause, meta.sequence, op.handle, op.descriptor.target, meta.origin);
            }
            this.#controllers.set(op.handle, {
                handle: op.handle,
                target: op.descriptor.target,
                asset: op.descriptor.asset,
                tickDurationSeconds: op.descriptor.tickDurationMillis / 1_000,
                controller: op.descriptor.controller,
                presented: weights,
                interpolation: null,
                origin: meta.origin,
                clipSampleSeconds: new Map(),
                emittedCueKeys: new Set(),
            });
            return null;
        }
        const realization = this.#controllers.get(op.handle);
        if (realization === undefined) {
            return animationDiagnostic('unknownHandle', meta.sequence, op.handle, null, 'animation handle is unavailable', meta.origin);
        }
        if (op.op === 'destroy') {
            try {
                this.#projection.clearAnimationControllerWeights(realization.target);
            }
            catch (cause) {
                return hostDiagnostic(cause, meta.sequence, op.handle, realization.target, meta.origin);
            }
            this.#controllers.delete(op.handle);
            return null;
        }
        const validation = validateController(op.controller);
        if (validation !== null) {
            return animationDiagnostic('invalidDescriptor', meta.sequence, op.handle, realization.target, validation, meta.origin);
        }
        if (op.controller.revision < realization.controller.revision) {
            return animationDiagnostic('staleRevision', meta.sequence, op.handle, realization.target, 'controller revision moved backward', meta.origin);
        }
        if (op.controller.revision === realization.controller.revision
            && !isMonotonicSameRevisionUpdate(realization.controller, op.controller)) {
            return animationDiagnostic('staleRevision', meta.sequence, op.handle, realization.target, 'controller state or transition progress moved backward without an authority revision', meta.origin);
        }
        const target = controllerWeights(op.controller);
        if (!this.#projection.hasAnimationClips(realization.target, target.map((clip) => clip.clip))) {
            return animationDiagnostic('clipMissing', meta.sequence, op.handle, realization.target, 'controller references an unavailable clip', meta.origin);
        }
        realization.controller = op.controller;
        realization.origin = meta.origin ?? realization.origin;
        realization.interpolation = {
            from: realization.presented,
            to: target,
            durationSeconds: realization.tickDurationSeconds,
            elapsedSeconds: 0,
        };
        return null;
    }
}
function validateCueDefinitions(definitions) {
    const keys = new Set();
    return definitions.map((definition) => {
        if (definition.cueId.trim().length === 0
            || definition.asset.trim().length === 0
            || definition.clip.trim().length === 0
            || definition.signal.id.trim().length === 0
            || !Number.isFinite(definition.atSeconds)
            || definition.atSeconds < 0) {
            throw new Error('animation cue definitions require non-empty identifiers and a finite non-negative marker');
        }
        const key = animationCueKey(definition);
        if (keys.has(key)) {
            throw new Error(`duplicate animation cue definition ${key}`);
        }
        keys.add(key);
        return definition;
    });
}
function sampleAnimationCues(realization, definitions, deltaSeconds) {
    const activeClips = new Set(realization.presented.filter((clip) => clip.weight > 0).map((clip) => clip.clip));
    for (const clip of realization.clipSampleSeconds.keys()) {
        if (!activeClips.has(clip)) {
            realization.clipSampleSeconds.delete(clip);
            for (const definition of definitions) {
                if (definition.asset === realization.asset && definition.clip === clip) {
                    realization.emittedCueKeys.delete(animationCueKey(definition));
                }
            }
        }
    }
    const sampled = [];
    for (const clip of realization.presented) {
        if (clip.weight <= 0) {
            continue;
        }
        const prior = realization.clipSampleSeconds.get(clip.clip);
        const sampledAtSeconds = (prior ?? 0) + deltaSeconds * clip.speed;
        realization.clipSampleSeconds.set(clip.clip, sampledAtSeconds);
        for (const definition of definitions) {
            if (definition.asset !== realization.asset || definition.clip !== clip.clip) {
                continue;
            }
            const key = animationCueKey(definition);
            const crossedMarker = prior === undefined
                ? definition.atSeconds <= sampledAtSeconds
                : prior < definition.atSeconds && definition.atSeconds <= sampledAtSeconds;
            if (!crossedMarker || realization.emittedCueKeys.has(key)) {
                continue;
            }
            realization.emittedCueKeys.add(key);
            sampled.push({
                kind: 'asha.animation.sampled_cue.v1',
                cueId: definition.cueId,
                handle: realization.handle,
                target: realization.target,
                asset: realization.asset,
                clip: definition.clip,
                markerSeconds: definition.atSeconds,
                sampledAtSeconds,
                signal: definition.signal,
                origin: realization.origin,
                replayScope: 'excludedFromReplayTruth',
                authorityMutation: 'forbidden',
            });
        }
    }
    return sampled;
}
function animationCueKey(definition) {
    return `${definition.asset}:${definition.clip}:${definition.cueId}`;
}
function isMonotonicSameRevisionUpdate(previous, next) {
    if (previous.graphId !== next.graphId
        || previous.graphVersion !== next.graphVersion
        || previous.graphHash !== next.graphHash
        || previous.stateId !== next.stateId) {
        return false;
    }
    if (previous.transition === null) {
        return true;
    }
    if (next.transition === null) {
        return false;
    }
    return previous.transition.transitionId === next.transition.transitionId
        && previous.transition.fromStateId === next.transition.fromStateId
        && previous.transition.toStateId === next.transition.toStateId
        && previous.transition.durationTicks === next.transition.durationTicks
        && next.transition.elapsedTicks >= previous.transition.elapsedTicks;
}
function validateController(controller) {
    const motions = [controller.motion, controller.transition?.targetMotion].filter((motion) => motion !== undefined);
    for (const motion of motions) {
        if (motion.clipA.length === 0
            || motion.blendWeightMilli < 0
            || motion.blendWeightMilli > 1_000
            || motion.speedMilli <= 0
            || (motion.clipB === null && motion.blendWeightMilli !== 0)) {
            return 'controller motion is invalid';
        }
    }
    const transition = controller.transition;
    if (transition !== null
        && (transition.durationTicks === 0 || transition.elapsedTicks > transition.durationTicks)) {
        return 'controller transition progress is invalid';
    }
    return null;
}
function controllerWeights(controller) {
    const transition = controller.transition;
    if (transition === null) {
        return motionWeights(controller.motion);
    }
    const progress = transition.elapsedTicks / transition.durationTicks;
    return mergeWeights([
        ...motionWeights(controller.motion).map((clip) => ({ ...clip, weight: clip.weight * (1 - progress) })),
        ...motionWeights(transition.targetMotion).map((clip) => ({ ...clip, weight: clip.weight * progress })),
    ]);
}
function motionWeights(motion) {
    const highWeight = motion.clipB === null ? 0 : motion.blendWeightMilli / 1_000;
    const clips = [{
            clip: motion.clipA,
            weight: 1 - highWeight,
            speed: motion.speedMilli / 1_000,
        }];
    if (motion.clipB !== null && highWeight > 0) {
        clips.push({ clip: motion.clipB, weight: highWeight, speed: motion.speedMilli / 1_000 });
    }
    return clips;
}
function mergeWeights(clips) {
    const merged = new Map();
    for (const clip of clips) {
        if (clip.weight <= 0) {
            continue;
        }
        const prior = merged.get(clip.clip);
        merged.set(clip.clip, {
            clip: clip.clip,
            weight: (prior?.weight ?? 0) + clip.weight,
            speed: clip.speed,
        });
    }
    return [...merged.values()].sort((left, right) => left.clip.localeCompare(right.clip));
}
function interpolateWeights(from, to, progress) {
    const clips = new Set([...from.map((clip) => clip.clip), ...to.map((clip) => clip.clip)]);
    return mergeWeights([...clips].map((clip) => {
        const prior = from.find((value) => value.clip === clip);
        const next = to.find((value) => value.clip === clip);
        return {
            clip,
            weight: (prior?.weight ?? 0) + ((next?.weight ?? 0) - (prior?.weight ?? 0)) * progress,
            speed: next?.speed ?? prior?.speed ?? 1,
        };
    }));
}
function hostDiagnostic(cause, sequence, handle, target, origin) {
    const message = errorMessage(cause);
    const code = message.includes('missing clip') ? 'clipMissing' : message.includes('handle') ? 'unknownTarget' : 'hostFailure';
    return animationDiagnostic(code, sequence, handle, target, message, origin);
}
function animationDiagnostic(code, sequence, handle, target, message, origin) {
    return { code, sequence, handle, target, message, origin };
}
function errorMessage(cause) {
    return cause instanceof Error ? cause.message : String(cause);
}
//# sourceMappingURL=animation-host.js.map