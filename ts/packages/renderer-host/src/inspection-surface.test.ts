import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  renderHandle,
  type EditorGridProjectionReadout,
  type RenderFrameDiff,
} from '@asha/contracts';
import {
  createAshaRendererEditorViewportWithBackend,
  type AshaRendererEditorViewportBackendPort,
  type AshaRendererEditorViewportSize,
} from './editor-viewport.js';
import { createAshaRendererInspectionSurfaceWithViewport } from './inspection-surface.js';

void test('inspection surface owns projection-only mouse orbit and focused WASD movement', () => {
  const harness = createInspectionHarness({ autoStart: false });
  const beforeOrbit = harness.surface.camera();

  harness.canvas.emit('pointerdown', pointerEvent(0));
  assert.equal(harness.surface.readout().dragging, true);
  harness.document.emit('mousemove', mouseMoveEvent(40, -20));
  harness.document.emit('pointerup', pointerEvent(0));

  const afterOrbit = harness.surface.camera();
  assert.notDeepEqual(afterOrbit.pose.position, beforeOrbit.pose.position);
  assert.notDeepEqual(afterOrbit.basis.forward, beforeOrbit.basis.forward);
  assert.equal(harness.surface.readout().dragging, false);

  let beforeMovement = afterOrbit;
  for (const [index, code] of ['KeyW', 'KeyA', 'KeyS', 'KeyD'].entries()) {
    harness.document.emit('keydown', keyboardEvent(code));
    assert.deepEqual(harness.surface.readout().pressedMovementKeys, [code]);
    harness.surface.renderOnce((index + 1) * 1000);
    const afterMovement = harness.surface.camera();
    assert.notDeepEqual(afterMovement.pose.position, beforeMovement.pose.position);
    harness.document.emit('keyup', keyboardEvent(code));
    beforeMovement = afterMovement;
  }
  assert.deepEqual(harness.surface.readout().pressedMovementKeys, []);

  assert.equal(harness.surface.authority, 'projection_only_inspection');
  assert.equal(harness.surface.readout().authority, 'projection_only_inspection');
  assert.equal(harness.surface.readout().camera.source, 'stored_editor');
});

void test('inspection surface retains accepted frames, fails closed on malformed replacement, and owns resize', () => {
  const harness = createInspectionHarness({ autoStart: false, frame: primitiveFrame(7) });
  const accepted = harness.surface.readout();
  assert.equal(accepted.retainedOpCount, 1);

  const malformed = harness.surface.replaceFrame({ ops: null } as unknown as RenderFrameDiff);
  assert.equal(malformed.applied, false);
  assert.equal(malformed.diagnostics[0]?.code, 'invalid_frame');
  assert.equal(harness.surface.readout().retainedFrameHash, accepted.retainedFrameHash);
  assert.equal(harness.surface.readout().retainedOpCount, 1);

  harness.canvas.clientWidth = 900;
  harness.canvas.clientHeight = 500;
  const canvasResize = harness.surface.resizeToCanvas();
  assert.equal(canvasResize.applied, true);
  assert.deepEqual(harness.backend.sizes.at(-1), { width: 900, height: 500, pixelRatio: 1.5 });

  const explicitResize = harness.surface.resize({ width: 1200, height: 700, pixelRatio: 2 });
  assert.equal(explicitResize.applied, true);
  assert.deepEqual(harness.backend.sizes.at(-1), { width: 1200, height: 700, pixelRatio: 2 });
});

void test('inspection surface start stop and disposal release animation input renderer and resize resources', () => {
  const harness = createInspectionHarness({ autoStart: true });
  assert.equal(harness.surface.readout().status, 'running');
  assert.equal(harness.animation.pendingCount(), 1);

  harness.animation.runNext(16);
  assert.equal(harness.animation.pendingCount(), 1);
  harness.surface.stop();
  assert.equal(harness.surface.readout().status, 'stopped');
  assert.equal(harness.animation.pendingCount(), 0);

  harness.surface.start();
  assert.equal(harness.surface.readout().status, 'running');
  const cameraBeforeDispose = harness.surface.camera();
  harness.surface.dispose();
  harness.surface.dispose();
  assert.equal(harness.surface.readout().status, 'disposed');
  assert.equal(harness.animation.pendingCount(), 0);
  assert.equal(harness.backend.disposals, 1);
  assert.equal(harness.resizeObserver.disconnects, 1);

  harness.canvas.emit('pointerdown', pointerEvent(0));
  harness.document.emit('mousemove', mouseMoveEvent(100, 100));
  harness.document.emit('keydown', keyboardEvent('KeyW'));
  harness.surface.renderOnce(1000);
  assert.deepEqual(harness.surface.camera(), cameraBeforeDispose);
  assert.equal(harness.canvas.listenerCount(), 0);
  assert.equal(harness.document.listenerCount(), 0);
});

void test('inspection surface rejects a malformed initial frame and disposes the prepared viewport', () => {
  const backend = new FakeEditorViewportBackend();
  const viewport = createAshaRendererEditorViewportWithBackend(backend, { autoStart: false });
  const document = new FakeDocument();
  const canvas = new FakeCanvas(document);
  const animation = new FakeAnimationScheduler();
  const resizeObserver = new FakeResizeObserver();

  assert.throws(
    () => createAshaRendererInspectionSurfaceWithViewport(
      canvas as unknown as HTMLCanvasElement,
      viewport,
      { autoStart: false, frame: { ops: null } as unknown as RenderFrameDiff },
      inspectionEnvironment(animation, resizeObserver),
    ),
    /render frame ops must be an array/,
  );
  assert.equal(backend.disposals, 1);
  assert.equal(resizeObserver.disconnects, 1);
  assert.equal(canvas.listenerCount(), 0);
  assert.equal(document.listenerCount(), 0);
});

function createInspectionHarness(options: {
  readonly autoStart: boolean;
  readonly frame?: RenderFrameDiff;
}) {
  const backend = new FakeEditorViewportBackend();
  const viewport = createAshaRendererEditorViewportWithBackend(backend, { autoStart: false });
  const document = new FakeDocument();
  const canvas = new FakeCanvas(document);
  const animation = new FakeAnimationScheduler();
  const resizeObserver = new FakeResizeObserver();
  const surface = createAshaRendererInspectionSurfaceWithViewport(
    canvas as unknown as HTMLCanvasElement,
    viewport,
    {
      autoStart: options.autoStart,
      ...(options.frame === undefined ? {} : { frame: options.frame }),
    },
    inspectionEnvironment(animation, resizeObserver),
  );
  return { animation, backend, canvas, document, resizeObserver, surface };
}

function inspectionEnvironment(
  animation: FakeAnimationScheduler,
  resizeObserver: FakeResizeObserver,
) {
  return {
    animation,
    createResizeObserver: () => resizeObserver,
    devicePixelRatio: () => 1.5,
  };
}

class FakeAnimationScheduler {
  readonly #callbacks = new Map<number, (timeMs: number) => void>();
  #nextHandle = 1;

  request(callback: (timeMs: number) => void): number {
    const handle = this.#nextHandle;
    this.#nextHandle += 1;
    this.#callbacks.set(handle, callback);
    return handle;
  }

  cancel(handle: number): void {
    this.#callbacks.delete(handle);
  }

  now(): number {
    return 0;
  }

  pendingCount(): number {
    return this.#callbacks.size;
  }

  runNext(timeMs: number): void {
    const entry = this.#callbacks.entries().next().value as [number, (timeMs: number) => void] | undefined;
    assert.ok(entry);
    this.#callbacks.delete(entry[0]);
    entry[1](timeMs);
  }
}

class FakeResizeObserver {
  disconnects = 0;
  observations = 0;

  observe(): void {
    this.observations += 1;
  }

  disconnect(): void {
    this.disconnects += 1;
  }
}

type FakeListener = EventListenerOrEventListenerObject;

class FakeEventSource {
  readonly #listeners = new Map<string, Set<FakeListener>>();

  addEventListener(type: string, listener: FakeListener | null): void {
    if (listener === null) return;
    const listeners = this.#listeners.get(type) ?? new Set<FakeListener>();
    listeners.add(listener);
    this.#listeners.set(type, listeners);
  }

  removeEventListener(type: string, listener: FakeListener | null): void {
    if (listener === null) return;
    this.#listeners.get(type)?.delete(listener);
  }

  emit(type: string, event: Event): void {
    for (const listener of this.#listeners.get(type) ?? []) {
      if (typeof listener === 'function') listener(event);
      else listener.handleEvent(event);
    }
  }

  listenerCount(): number {
    return [...this.#listeners.values()].reduce((total, listeners) => total + listeners.size, 0);
  }
}

class FakeDocument extends FakeEventSource {
  activeElement: Element | null = null;
}

class FakeCanvas extends FakeEventSource {
  clientHeight = 360;
  clientWidth = 640;
  height = 360;
  width = 640;
  tabIndex = -1;
  readonly style = { touchAction: '' };

  constructor(readonly fakeDocument: FakeDocument) {
    super();
  }

  get ownerDocument(): Document {
    return this.fakeDocument as unknown as Document;
  }

  focus(): void {
    this.fakeDocument.activeElement = this as unknown as Element;
  }
}

function pointerEvent(button: number): PointerEvent {
  return {
    button,
    preventDefault: () => undefined,
  } as unknown as PointerEvent;
}

function mouseMoveEvent(movementX: number, movementY: number): MouseEvent {
  return {
    movementX,
    movementY,
    preventDefault: () => undefined,
  } as unknown as MouseEvent;
}

function keyboardEvent(code: string): KeyboardEvent {
  return {
    code,
    preventDefault: () => undefined,
  } as unknown as KeyboardEvent;
}

class FakeEditorViewportBackend implements AshaRendererEditorViewportBackendPort {
  readonly frames = new Map<string, RenderFrameDiff>();
  readonly sizes: AshaRendererEditorViewportSize[] = [];
  disposals = 0;

  replaceChannel(channel: 'runtime' | 'authored' | 'overlay', frame: RenderFrameDiff): void {
    this.frames.set(channel, structuredClone(frame));
  }

  setCamera(): void {}

  setGrid(): void {}

  gridReadout(): EditorGridProjectionReadout | null {
    return null;
  }

  resize(size: AshaRendererEditorViewportSize): void {
    this.sizes.push(structuredClone(size));
  }

  pick(): ReturnType<AshaRendererEditorViewportBackendPort['pick']> {
    return { diagnostics: [], hit: null };
  }

  renderOnce(): void {}

  start(): void {}

  stop(): void {}

  snapshot(): string {
    return JSON.stringify([...this.frames]);
  }

  dispose(): void {
    this.disposals += 1;
  }
}

function primitiveFrame(handle: number): RenderFrameDiff {
  return {
    ops: [{
      op: 'create',
      handle: renderHandle(handle),
      parent: null,
      node: {
        layer: 'scene',
        geometry: { shape: 'cube' },
        transform: {
          translation: [0, 0, 0],
          rotation: [0, 0, 0, 1],
          scale: [1, 1, 1],
        },
        material: { color: [0.3, 0.5, 0.8, 1], wireframe: false },
        visible: true,
        metadata: { source: null, tags: [], label: 'inspection-fixture' },
      },
    }],
  };
}
