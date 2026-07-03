import type { CameraHandle, FirstPersonCameraInputEnvelope } from '@asha/contracts';
import type { RuntimeActionIntentEnvelope } from './runtime-action.js';

export type BrowserFpsKeyCode = 'KeyW' | 'KeyA' | 'KeyS' | 'KeyD' | 'Escape';

export interface BrowserFpsKeyboardInput {
  readonly code: string;
  readonly repeat?: boolean;
  preventDefault?(): void;
}

export interface BrowserFpsMouseMoveInput {
  readonly movementX: number;
  readonly movementY: number;
}

export interface BrowserFpsPointerButtonInput {
  readonly button: number;
  preventDefault?(): void;
}

export type BrowserFpsPointerLockIntent =
  | { readonly kind: 'request_pointer_lock'; readonly reason: 'primary_button' | 'programmatic' }
  | { readonly kind: 'release_pointer_lock'; readonly reason: 'escape_key' | 'programmatic' };

export interface BrowserFpsUnsupportedIntent {
  readonly kind: 'unsupported_primary_fire';
  readonly pressed: boolean;
  readonly triggered: boolean;
  readonly reason: 'no_public_runtime_action_protocol';
}

export interface BrowserFpsInputReadout {
  readonly pointerLocked: boolean;
  readonly releaseRequestedByEscape: boolean;
  readonly pressedKeys: readonly BrowserFpsKeyCode[];
  readonly moveForward: number;
  readonly moveRight: number;
  readonly pendingMouseDelta: readonly [number, number];
  readonly primaryFirePressed: boolean;
  readonly primaryFireTriggered: boolean;
}

export type BrowserFpsRuntimeCommand = {
  readonly kind: 'runtime.apply_first_person_camera_input';
  readonly envelope: FirstPersonCameraInputEnvelope;
};

export type BrowserFpsRuntimeActionCommand = {
  readonly kind: 'runtime.propose_runtime_action_intent';
  readonly envelope: RuntimeActionIntentEnvelope;
};

export interface BrowserFpsCommandFrame {
  readonly tick: number;
  readonly runtimeCommand: BrowserFpsRuntimeCommand;
  readonly runtimeActionIntents: readonly BrowserFpsRuntimeActionCommand[];
  readonly pointerLockIntents: readonly BrowserFpsPointerLockIntent[];
  readonly unsupportedIntents: readonly BrowserFpsUnsupportedIntent[];
  readonly readout: BrowserFpsInputReadout;
}

export interface BrowserFpsInputCollectorOptions {
  readonly camera: CameraHandle;
  readonly moveSpeedUnitsPerSecond: number;
  readonly mouseSensitivityDegreesPerPixel: number;
  readonly pointerLocked?: boolean;
}

export interface BrowserFpsDrainInput {
  readonly tick: number;
  readonly dtSeconds: number;
}

export class BrowserFpsInputCollector {
  readonly #camera: CameraHandle;
  readonly #moveSpeedUnitsPerSecond: number;
  readonly #mouseSensitivityDegreesPerPixel: number;
  readonly #keys = new Set<BrowserFpsKeyCode>();
  readonly #pointerLockIntents: BrowserFpsPointerLockIntent[] = [];
  #pointerLocked: boolean;
  #releaseRequestedByEscape = false;
  #mouseX = 0;
  #mouseY = 0;
  #primaryFirePressed = false;
  #primaryFireTriggered = false;
  #primaryFireReleased = false;

  constructor(options: BrowserFpsInputCollectorOptions) {
    if (options.moveSpeedUnitsPerSecond < 0 || !Number.isFinite(options.moveSpeedUnitsPerSecond)) {
      throw new Error('moveSpeedUnitsPerSecond must be a finite non-negative number');
    }
    if (!Number.isFinite(options.mouseSensitivityDegreesPerPixel)) {
      throw new Error('mouseSensitivityDegreesPerPixel must be finite');
    }
    this.#camera = options.camera;
    this.#moveSpeedUnitsPerSecond = options.moveSpeedUnitsPerSecond;
    this.#mouseSensitivityDegreesPerPixel = options.mouseSensitivityDegreesPerPixel;
    this.#pointerLocked = options.pointerLocked ?? false;
  }

  setPointerLockActive(active: boolean): BrowserFpsInputReadout {
    this.#pointerLocked = active;
    if (active) {
      this.#releaseRequestedByEscape = false;
    }
    return this.readout();
  }

  requestPointerLock(): readonly BrowserFpsPointerLockIntent[] {
    const intent: BrowserFpsPointerLockIntent = { kind: 'request_pointer_lock', reason: 'programmatic' };
    this.#pointerLockIntents.push(intent);
    return [intent];
  }

  releasePointerLock(): readonly BrowserFpsPointerLockIntent[] {
    const intent: BrowserFpsPointerLockIntent = { kind: 'release_pointer_lock', reason: 'programmatic' };
    this.#pointerLockIntents.push(intent);
    return [intent];
  }

  handleKeyDown(event: BrowserFpsKeyboardInput): readonly BrowserFpsPointerLockIntent[] {
    const key = fpsKeyCode(event.code);
    if (key === null) {
      return [];
    }
    event.preventDefault?.();
    if (key === 'Escape') {
      this.#releaseRequestedByEscape = true;
      if (!this.#pointerLocked) {
        return [];
      }
      const intent: BrowserFpsPointerLockIntent = { kind: 'release_pointer_lock', reason: 'escape_key' };
      this.#pointerLockIntents.push(intent);
      return [intent];
    }
    this.#keys.add(key);
    return [];
  }

  handleKeyUp(event: BrowserFpsKeyboardInput): void {
    const key = fpsKeyCode(event.code);
    if (key === null || key === 'Escape') {
      return;
    }
    event.preventDefault?.();
    this.#keys.delete(key);
  }

  handleMouseMove(event: BrowserFpsMouseMoveInput): void {
    if (!this.#pointerLocked) {
      return;
    }
    if (!Number.isFinite(event.movementX) || !Number.isFinite(event.movementY)) {
      return;
    }
    this.#mouseX += event.movementX;
    this.#mouseY += event.movementY;
  }

  handlePointerDown(event: BrowserFpsPointerButtonInput): readonly BrowserFpsPointerLockIntent[] {
    event.preventDefault?.();
    if (event.button !== 0) {
      return [];
    }
    this.#primaryFirePressed = true;
    this.#primaryFireTriggered = true;
    if (this.#pointerLocked) {
      return [];
    }
    const intent: BrowserFpsPointerLockIntent = { kind: 'request_pointer_lock', reason: 'primary_button' };
    this.#pointerLockIntents.push(intent);
    return [intent];
  }

  handlePointerUp(event: BrowserFpsPointerButtonInput): void {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault?.();
    const wasPressed = this.#primaryFirePressed;
    this.#primaryFirePressed = false;
    if (wasPressed) {
      this.#primaryFireReleased = true;
    }
  }

  drainFrame(input: BrowserFpsDrainInput): BrowserFpsCommandFrame {
    validateDrainInput(input);
    const moveForward = directional(this.#keys.has('KeyW'), this.#keys.has('KeyS'));
    const moveRight = directional(this.#keys.has('KeyD'), this.#keys.has('KeyA'));
    const mouseX = this.#mouseX;
    const mouseY = this.#mouseY;
    const readoutBeforeReset = this.readout();
    const runtimeCommand: BrowserFpsRuntimeCommand = {
      kind: 'runtime.apply_first_person_camera_input',
      envelope: {
        camera: this.#camera,
        tick: input.tick,
        input: {
          moveForward,
          moveRight,
          moveUp: 0,
          yawDeltaDegrees: mouseX * this.#mouseSensitivityDegreesPerPixel,
          pitchDeltaDegrees: -mouseY * this.#mouseSensitivityDegreesPerPixel,
          dtSeconds: input.dtSeconds,
          moveSpeedUnitsPerSecond: this.#moveSpeedUnitsPerSecond,
        },
      },
    };
    const runtimeActionIntents = this.#drainRuntimeActionIntents(input.tick);
    const frame: BrowserFpsCommandFrame = {
      tick: input.tick,
      runtimeCommand,
      runtimeActionIntents,
      pointerLockIntents: [...this.#pointerLockIntents],
      unsupportedIntents: [],
      readout: readoutBeforeReset,
    };
    this.#pointerLockIntents.length = 0;
    this.#mouseX = 0;
    this.#mouseY = 0;
    this.#primaryFireTriggered = false;
    this.#primaryFireReleased = false;
    return frame;
  }

  readout(): BrowserFpsInputReadout {
    return {
      pointerLocked: this.#pointerLocked,
      releaseRequestedByEscape: this.#releaseRequestedByEscape,
      pressedKeys: [...this.#keys].sort(),
      moveForward: directional(this.#keys.has('KeyW'), this.#keys.has('KeyS')),
      moveRight: directional(this.#keys.has('KeyD'), this.#keys.has('KeyA')),
      pendingMouseDelta: [this.#mouseX, this.#mouseY],
      primaryFirePressed: this.#primaryFirePressed,
      primaryFireTriggered: this.#primaryFireTriggered,
    };
  }

  #drainRuntimeActionIntents(tick: number): readonly BrowserFpsRuntimeActionCommand[] {
    const intents: BrowserFpsRuntimeActionCommand[] = [];
    if (this.#primaryFireTriggered) {
      intents.push({
        kind: 'runtime.propose_runtime_action_intent',
        envelope: {
          kind: 'runtime_action_intent.v0',
          action: 'primary_fire',
          phase: 'pressed',
          camera: this.#camera,
          tick,
          source: 'browser_fps_pointer',
          pressed: true,
        },
      });
    }
    if (this.#primaryFireReleased) {
      intents.push({
        kind: 'runtime.propose_runtime_action_intent',
        envelope: {
          kind: 'runtime_action_intent.v0',
          action: 'primary_fire',
          phase: 'released',
          camera: this.#camera,
          tick,
          source: 'browser_fps_pointer',
          pressed: false,
        },
      });
    }
    return intents;
  }
}

function fpsKeyCode(code: string): BrowserFpsKeyCode | null {
  switch (code) {
    case 'KeyW':
    case 'KeyA':
    case 'KeyS':
    case 'KeyD':
    case 'Escape':
      return code;
    default:
      return null;
  }
}

function directional(positive: boolean, negative: boolean): number {
  if (positive === negative) {
    return 0;
  }
  return positive ? 1 : -1;
}

function validateDrainInput(input: BrowserFpsDrainInput): void {
  if (!Number.isSafeInteger(input.tick) || input.tick < 0) {
    throw new Error('tick must be a non-negative safe integer');
  }
  if (!Number.isFinite(input.dtSeconds) || input.dtSeconds < 0) {
    throw new Error('dtSeconds must be a finite non-negative number');
  }
}
