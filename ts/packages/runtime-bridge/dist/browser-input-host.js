const RECENT_DELIVERY_LIMIT = 32;
export class BrowserInputHost {
    #session;
    #consumers;
    #onResolvedAction;
    #onContextChanged;
    #deliveries = [];
    #sequence = 0;
    #pointerLocked = false;
    #contextRevision = 0;
    constructor(options) {
        this.#session = options.session;
        this.#consumers = options.consumers ?? {};
        this.#onResolvedAction = options.onResolvedAction;
        const snapshot = this.#session.configureInputSession({
            catalog: options.catalog ?? createDefaultBrowserInputCatalog(),
            initialContexts: options.initialContexts ?? ['gameplay'],
        });
        this.#contextRevision = snapshot.contextState.revision;
        this.#onContextChanged = options.onContextChanged;
    }
    applyContextCommand(command) {
        const receipt = this.#session.applyInputContextCommand(command);
        this.#observeContextState(receipt.state);
        return receipt;
    }
    attachDom(attachment) {
        const mouseTarget = attachment.mouseTarget ?? attachment.keyboardTarget;
        const onPointerDown = (event) => {
            this.handlePointerDown(event);
            for (const intent of this.pointerLockIntentsForButton(event)) {
                attachment.onPointerLockIntent?.(intent, event);
            }
        };
        const onPointerUp = (event) => { this.handlePointerUp(event); };
        const onMouseMove = (event) => { this.handleMouseMove(event); };
        const onWheel = (event) => { this.handleWheel(event); };
        const onKeyDown = (event) => {
            if (attachment.acceptsKeyboard?.() === false)
                return;
            this.handleKeyDown(event);
            for (const intent of this.pointerLockIntentsForKey(event)) {
                attachment.onPointerLockIntent?.(intent, event);
            }
        };
        const onKeyUp = (event) => { this.handleKeyUp(event); };
        attachment.pointerTarget.addEventListener('pointerdown', onPointerDown);
        attachment.pointerTarget.addEventListener('pointerup', onPointerUp);
        mouseTarget.addEventListener('mousemove', onMouseMove);
        mouseTarget.addEventListener('wheel', onWheel);
        attachment.keyboardTarget.addEventListener('keydown', onKeyDown);
        attachment.keyboardTarget.addEventListener('keyup', onKeyUp);
        return () => {
            attachment.pointerTarget.removeEventListener('pointerdown', onPointerDown);
            attachment.pointerTarget.removeEventListener('pointerup', onPointerUp);
            mouseTarget.removeEventListener('mousemove', onMouseMove);
            mouseTarget.removeEventListener('wheel', onWheel);
            attachment.keyboardTarget.removeEventListener('keydown', onKeyDown);
            attachment.keyboardTarget.removeEventListener('keyup', onKeyUp);
        };
    }
    setPointerLockActive(active) {
        this.#pointerLocked = active;
        return this.readout();
    }
    requestPointerLock() {
        return [{ kind: 'requestPointerLock', reason: 'programmatic' }];
    }
    releasePointerLock() {
        return [{ kind: 'releasePointerLock', reason: 'programmatic' }];
    }
    handleKeyDown(event) {
        event.preventDefault?.();
        return this.#submit('keyboardKey', event.code, event.repeat === true ? 'held' : 'pressed', {
            kind: 'button',
            pressed: true,
        });
    }
    handleKeyUp(event) {
        event.preventDefault?.();
        return this.#submit('keyboardKey', event.code, 'released', { kind: 'button', pressed: false });
    }
    handleMouseMove(event) {
        if (!this.#pointerLocked)
            return null;
        if (!Number.isFinite(event.movementX) || !Number.isFinite(event.movementY))
            return null;
        return this.#submit('mouseDelta', 'pointer', 'changed', {
            kind: 'axis2d',
            x: event.movementX,
            y: event.movementY,
        });
    }
    handlePointerDown(event) {
        event.preventDefault?.();
        return this.#submit('mouseButton', `button${event.button}`, 'pressed', {
            kind: 'button',
            pressed: true,
        });
    }
    handlePointerUp(event) {
        event.preventDefault?.();
        return this.#submit('mouseButton', `button${event.button}`, 'released', {
            kind: 'button',
            pressed: false,
        });
    }
    handleWheel(event) {
        if (!Number.isFinite(event.deltaY) || event.deltaY === 0)
            return null;
        event.preventDefault?.();
        return this.#submit('mouseWheel', 'wheel', 'changed', {
            kind: 'axis1d',
            value: event.deltaY,
        });
    }
    pointerLockIntentsForKey(event) {
        return event.code === 'Escape' && this.#pointerLocked
            ? [{ kind: 'releasePointerLock', reason: 'escapeKey' }]
            : [];
    }
    pointerLockIntentsForButton(event) {
        return event.button === 0 && !this.#pointerLocked
            ? [{ kind: 'requestPointerLock', reason: 'primaryButton' }]
            : [];
    }
    readout() {
        const state = this.#session.readInputContextState();
        return {
            activeContexts: state.activeContexts.map((context) => context.contextId),
            pointerLocked: this.#pointerLocked,
            lastDelivery: this.#deliveries.at(-1) ?? null,
            recentDeliveries: [...this.#deliveries],
        };
    }
    #submit(platformKind, control, phase, value) {
        const contextState = this.#session.readInputContextState();
        this.#observeContextState(contextState);
        const sample = {
            sequence: this.#sequence,
            platformKind,
            control,
            phase,
            value,
        };
        this.#sequence += 1;
        const receipt = this.#session.submitRawInput(sample);
        const consumer = receipt.action === null ? null : (this.#consumers[receipt.action.actionId] ?? null);
        const reason = receipt.action !== null
            ? `resolved to ${receipt.action.actionId}`
            : (receipt.diagnostics[0]?.message ?? (receipt.consumed ? 'consumed' : 'unbound'));
        const delivery = {
            sample,
            receipt,
            activeContexts: contextState.activeContexts.map((context) => context.contextId),
            consumer,
            reason,
        };
        this.#deliveries.push(delivery);
        if (this.#deliveries.length > RECENT_DELIVERY_LIMIT)
            this.#deliveries.shift();
        if (receipt.action !== null)
            this.#onResolvedAction?.(receipt.action, consumer);
        return delivery;
    }
    #observeContextState(state) {
        if (state.revision === this.#contextRevision)
            return;
        this.#contextRevision = state.revision;
        this.#onContextChanged?.(state);
    }
}
export function createDefaultBrowserInputCatalog() {
    const buttonPhases = ['pressed', 'held', 'released'];
    return {
        schemaVersion: 1,
        actions: [
            { actionId: 'gameplay.move.forward', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'gameplay.move.backward', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'gameplay.move.left', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'gameplay.move.right', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'gameplay.look', valueKind: 'axis2d', acceptedPhases: ['changed'] },
            { actionId: 'gameplay.primaryFire', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'runtime.time.pause', valueKind: 'button', acceptedPhases: ['pressed'] },
            { actionId: 'runtime.time.resume', valueKind: 'button', acceptedPhases: ['pressed'] },
            { actionId: 'camera.mode.firstPerson', valueKind: 'button', acceptedPhases: ['pressed'] },
            { actionId: 'camera.mode.orbit', valueKind: 'button', acceptedPhases: ['pressed'] },
            { actionId: 'camera.mode.topDown', valueKind: 'button', acceptedPhases: ['pressed'] },
            { actionId: 'camera.navigation.rotate', valueKind: 'axis2d', acceptedPhases: ['changed'] },
            { actionId: 'camera.navigation.zoom', valueKind: 'axis1d', acceptedPhases: ['changed'] },
            { actionId: 'camera.navigation.panForward', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'camera.navigation.panBackward', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'camera.navigation.panLeft', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'camera.navigation.panRight', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'menu.open', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'menu.close', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'menu.navigateUp', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'menu.navigateDown', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'dialog.confirm', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'dialog.cancel', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'editor.camera.forward', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'editor.camera.backward', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'editor.camera.left', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'editor.camera.right', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'editor.camera.look', valueKind: 'axis2d', acceptedPhases: ['changed'] },
            { actionId: 'editor.tool.primary', valueKind: 'button', acceptedPhases: buttonPhases },
            { actionId: 'editor.tool.cancel', valueKind: 'button', acceptedPhases: buttonPhases },
        ],
        contexts: [
            { contextId: 'gameplay', priority: 100, consumesLowerPriority: false },
            { contextId: 'editor', priority: 200, consumesLowerPriority: false },
            { contextId: 'cameraNavigation', priority: 300, consumesLowerPriority: true },
            { contextId: 'menu', priority: 1_000, consumesLowerPriority: true },
            { contextId: 'dialog', priority: 2_000, consumesLowerPriority: true },
        ],
        bindings: [
            binding('gameplay-forward', 'gameplay.move.forward', 'gameplay', 'keyboardKey', 'KeyW'),
            binding('gameplay-backward', 'gameplay.move.backward', 'gameplay', 'keyboardKey', 'KeyS'),
            binding('gameplay-left', 'gameplay.move.left', 'gameplay', 'keyboardKey', 'KeyA'),
            binding('gameplay-right', 'gameplay.move.right', 'gameplay', 'keyboardKey', 'KeyD'),
            binding('gameplay-look', 'gameplay.look', 'gameplay', 'mouseDelta', 'pointer'),
            binding('gameplay-fire', 'gameplay.primaryFire', 'gameplay', 'mouseButton', 'button0'),
            binding('gameplay-menu', 'runtime.time.pause', 'gameplay', 'keyboardKey', 'Escape'),
            binding('gameplay-camera-orbit', 'camera.mode.orbit', 'gameplay', 'keyboardKey', 'KeyO'),
            binding('gameplay-camera-top-down', 'camera.mode.topDown', 'gameplay', 'keyboardKey', 'KeyT'),
            binding('camera-first-person', 'camera.mode.firstPerson', 'cameraNavigation', 'keyboardKey', 'KeyF'),
            binding('camera-orbit', 'camera.mode.orbit', 'cameraNavigation', 'keyboardKey', 'KeyO'),
            binding('camera-top-down', 'camera.mode.topDown', 'cameraNavigation', 'keyboardKey', 'KeyT'),
            binding('camera-rotate', 'camera.navigation.rotate', 'cameraNavigation', 'mouseDelta', 'pointer'),
            binding('camera-zoom', 'camera.navigation.zoom', 'cameraNavigation', 'mouseWheel', 'wheel'),
            binding('camera-pan-forward', 'camera.navigation.panForward', 'cameraNavigation', 'keyboardKey', 'KeyW'),
            binding('camera-pan-backward', 'camera.navigation.panBackward', 'cameraNavigation', 'keyboardKey', 'KeyS'),
            binding('camera-pan-left', 'camera.navigation.panLeft', 'cameraNavigation', 'keyboardKey', 'KeyA'),
            binding('camera-pan-right', 'camera.navigation.panRight', 'cameraNavigation', 'keyboardKey', 'KeyD'),
            binding('menu-close', 'runtime.time.resume', 'menu', 'keyboardKey', 'Escape'),
            binding('menu-up', 'menu.navigateUp', 'menu', 'keyboardKey', 'ArrowUp'),
            binding('menu-down', 'menu.navigateDown', 'menu', 'keyboardKey', 'ArrowDown'),
            binding('dialog-confirm', 'dialog.confirm', 'dialog', 'keyboardKey', 'Enter'),
            binding('dialog-cancel', 'dialog.cancel', 'dialog', 'keyboardKey', 'Escape'),
            binding('editor-forward', 'editor.camera.forward', 'editor', 'keyboardKey', 'KeyW'),
            binding('editor-backward', 'editor.camera.backward', 'editor', 'keyboardKey', 'KeyS'),
            binding('editor-left', 'editor.camera.left', 'editor', 'keyboardKey', 'KeyA'),
            binding('editor-right', 'editor.camera.right', 'editor', 'keyboardKey', 'KeyD'),
            binding('editor-look', 'editor.camera.look', 'editor', 'mouseDelta', 'pointer'),
            binding('editor-primary', 'editor.tool.primary', 'editor', 'mouseButton', 'button0'),
            binding('editor-cancel', 'editor.tool.cancel', 'editor', 'keyboardKey', 'Escape'),
        ],
    };
}
function binding(bindingId, actionId, contextId, platformKind, control) {
    return { bindingId, actionId, contextId, platformKind, control, scale: 1, extension: null };
}
//# sourceMappingURL=browser-input-host.js.map