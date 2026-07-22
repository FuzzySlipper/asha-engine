import assert from 'node:assert/strict';
import test from 'node:test';
import type { InputBindingCatalog, RecordedInputAction } from '@asha/contracts';

import {
  BrowserFpsResolvedActionConsumer,
  BrowserInputHost,
  createDefaultBrowserInputCatalog,
  createRuntimeSessionFacade,
  ResolvedPauseContextConsumer,
} from './index.js';
import { MockRuntimeBridge } from './mock.js';

function createHost(initialContexts: readonly string[] = ['gameplay']) {
  const bridge = new MockRuntimeBridge();
  bridge.initializeEngine({ seed: 11 });
  const consumer = new BrowserFpsResolvedActionConsumer();
  const host = new BrowserInputHost({
    session: bridge,
    initialContexts,
    consumers: {
      'gameplay.move.forward': 'fps.camera',
      'gameplay.look': 'fps.camera',
      'runtime.time.pause': 'shell.menu',
      'runtime.time.resume': 'shell.menu',
      'runtime.session.restart': 'runtime.lifecycle',
      'dialog.confirm': 'shell.dialog',
    },
    onResolvedAction: (action) => consumer.accept(action),
  });
  return { bridge, consumer, host };
}

void test('browser host normalizes keyboard and mouse samples before FPS delivery', () => {
  const { consumer, host } = createHost();
  const down = host.handleKeyDown({ code: 'KeyW' });
  host.setPointerLockActive(true);
  const look = host.handleMouseMove({ movementX: 8, movementY: -3 });

  assert.equal(down.receipt.action?.actionId, 'gameplay.move.forward');
  assert.equal(down.consumer, 'fps.camera');
  assert.equal(look?.sample.platformKind, 'mouseDelta');
  assert.deepEqual(consumer.drain(), {
    moveForward: 1,
    moveRight: 0,
    pitchDeltaPixels: -3,
    yawDeltaPixels: 8,
    primaryFirePressed: false,
  });
  assert.deepEqual(host.readout().activeContexts, ['gameplay']);
});

void test('menu and dialog contexts consume gameplay while preserving their own UI actions', () => {
  const { host } = createHost(['gameplay', 'menu']);
  const blockedGameplay = host.handleKeyDown({ code: 'KeyW' });
  const menuAction = host.handleKeyDown({ code: 'Escape' });
  assert.equal(blockedGameplay.receipt.action, null);
  assert.equal(blockedGameplay.receipt.consumed, true);
  assert.equal(blockedGameplay.receipt.diagnostics[0]?.code, 'consumedByContext');
  assert.equal(menuAction.receipt.action?.actionId, 'runtime.time.resume');
  assert.equal(menuAction.consumer, 'shell.menu');
  assert.deepEqual(menuAction.activeContexts, ['gameplay', 'menu']);

  const pushed = host.applyContextCommand({ operation: 'push', contextId: 'dialog' });
  assert.equal(pushed.accepted, true);
  const dialogAction = host.handleKeyDown({ code: 'Enter' });
  const swallowedMenu = host.handleKeyDown({ code: 'ArrowDown' });
  assert.equal(dialogAction.receipt.action?.actionId, 'dialog.confirm');
  assert.equal(dialogAction.consumer, 'shell.dialog');
  assert.deepEqual(dialogAction.activeContexts, ['gameplay', 'menu', 'dialog']);
  assert.equal(swallowedMenu.receipt.action, null);
  assert.equal(swallowedMenu.receipt.consumed, true);
  assert.deepEqual(
    host.readout().recentDeliveries.at(1)?.activeContexts,
    ['gameplay', 'menu'],
    'a later context push does not rewrite the delivery-time snapshot',
  );
});

void test('restart resolves once from KeyR in gameplay and menu while repeat and release stay inert', () => {
  for (const contexts of [['gameplay'], ['gameplay', 'menu']] as const) {
    const { host } = createHost(contexts);
    const pressed = host.handleKeyDown({ code: 'KeyR' });
    const held = host.handleKeyDown({ code: 'KeyR', repeat: true });
    const released = host.handleKeyUp({ code: 'KeyR' });

    assert.equal(pressed.receipt.action?.actionId, 'runtime.session.restart');
    assert.equal(pressed.consumer, 'runtime.lifecycle');
    assert.equal(pressed.receipt.action?.phase, 'pressed');
    assert.equal(held.receipt.action, null);
    assert.equal(released.receipt.action, null);
    assert.equal(
      host.readout().recentDeliveries.filter(delivery => (
        delivery.receipt.action?.actionId === 'runtime.session.restart'
      )).length,
      1,
    );
  }
});

void test('project interaction resolves once beside protected Engine movement through the normal host readout', () => {
  const baseCatalog = createDefaultBrowserInputCatalog();
  const catalog: InputBindingCatalog = {
    ...baseCatalog,
    actions: [
      ...baseCatalog.actions,
      { actionId: 'demo.interact', valueKind: 'button', acceptedPhases: ['pressed'] },
    ],
    bindings: [
      ...baseCatalog.bindings,
      {
        bindingId: 'demo.interact.primary',
        actionId: 'demo.interact',
        contextId: 'gameplay',
        platformKind: 'keyboardKey',
        control: 'KeyE',
        scale: 1,
        extension: null,
      },
    ],
  };
  const bridge = new MockRuntimeBridge();
  bridge.initializeEngine({ seed: 18 });
  const resolvedActions: string[] = [];
  const host = new BrowserInputHost({
    session: bridge,
    catalog,
    initialContexts: ['gameplay'],
    onResolvedAction: (action) => { resolvedActions.push(action.actionId); },
  });

  const movement = host.handleKeyDown({ code: 'KeyW' });
  const pressed = host.handleKeyDown({ code: 'KeyE' });
  const repeated = host.handleKeyDown({ code: 'KeyE', repeat: true });
  const released = host.handleKeyUp({ code: 'KeyE' });

  assert.equal(movement.receipt.action?.actionId, 'gameplay.move.forward');
  assert.equal(pressed.receipt.action?.actionId, 'demo.interact');
  assert.equal(repeated.receipt.action, null);
  assert.equal(released.receipt.action, null);
  assert.deepEqual(resolvedActions, ['gameplay.move.forward', 'demo.interact']);
  assert.equal(host.readout().recentDeliveries.filter((delivery) => (
    delivery.receipt.action?.actionId === 'demo.interact'
  )).length, 1);

  host.applyContextCommand({ operation: 'push', contextId: 'menu' });
  const inactive = host.handleKeyDown({ code: 'KeyE' });
  assert.equal(inactive.receipt.action, null);
  assert.equal(inactive.receipt.consumed, true);
  assert.deepEqual(resolvedActions, ['gameplay.move.forward', 'demo.interact']);
});

void test('keyboard focus exclusion blocks standalone release actions but completes accepted key lifecycles', () => {
  const baseCatalog = createDefaultBrowserInputCatalog();
  const catalog: InputBindingCatalog = {
    ...baseCatalog,
    actions: [
      ...baseCatalog.actions,
      { actionId: 'demo.interact', valueKind: 'button', acceptedPhases: ['released'] },
    ],
    bindings: [
      ...baseCatalog.bindings,
      {
        bindingId: 'demo.interact.primary', actionId: 'demo.interact', contextId: 'gameplay',
        platformKind: 'keyboardKey', control: 'KeyE', scale: 1, extension: null,
      },
    ],
  };
  const bridge = new MockRuntimeBridge();
  bridge.initializeEngine({ seed: 20 });
  const host = new BrowserInputHost({ session: bridge, catalog });
  const listeners = new Map<string, EventListener>();
  const eventTarget = {
    addEventListener(type: string, listener: EventListenerOrEventListenerObject): void {
      if (typeof listener === 'function') listeners.set(type, listener);
    },
    removeEventListener(type: string): void { listeners.delete(type); },
  };
  let acceptsKeyboard = false;
  const detach = host.attachDom({
    pointerTarget: eventTarget as unknown as HTMLElement,
    keyboardTarget: eventTarget as unknown as Document,
    acceptsKeyboard: () => acceptsKeyboard,
  });

  listeners.get('keydown')?.({ code: 'KeyE' } as KeyboardEvent);
  listeners.get('keyup')?.({ code: 'KeyE' } as KeyboardEvent);
  assert.equal(host.readout().recentDeliveries.length, 0);
  acceptsKeyboard = true;
  listeners.get('keydown')?.({ code: 'KeyE' } as KeyboardEvent);
  assert.equal(host.readout().lastDelivery?.receipt.action, null);
  acceptsKeyboard = false;
  listeners.get('keyup')?.({ code: 'KeyE' } as KeyboardEvent);
  assert.equal(host.readout().lastDelivery?.receipt.action?.actionId, 'demo.interact');
  assert.equal(host.readout().recentDeliveries.length, 2);
  listeners.get('keyup')?.({ code: 'KeyE' } as KeyboardEvent);
  assert.equal(host.readout().recentDeliveries.length, 2);
  detach();
});

void test('resolved pause context records and replays without browser events or double delivery', () => {
  const createSession = (sessionId: string) => {
    const bridge = new MockRuntimeBridge();
    const session = createRuntimeSessionFacade({ bridge, mode: 'reference' });
    session.initialize({
      sessionId,
      seed: 19,
      project: { gameId: 'input-replay', workspaceId: 'workspace.input-replay' },
    });
    return session;
  };

  const sourceSession = createSession('input-replay.source');
  const sourceFps = new BrowserFpsResolvedActionConsumer();
  const sourcePause = new ResolvedPauseContextConsumer(sourceSession);
  const sourceActions: string[] = [];
  const host = new BrowserInputHost({
    session: sourceSession,
    onResolvedAction: (action) => {
      sourceActions.push(action.actionId);
      sourcePause.consume(action);
      sourceFps.accept(action);
    },
  });
  host.setPointerLockActive(true);

  const records: RecordedInputAction[] = [];
  const capture = (record: ReturnType<typeof host.handleKeyDown>['receipt']['record']): void => {
    if (record !== null) records.push(record);
  };
  capture(host.handleKeyDown({ code: 'KeyW' }).receipt.record);
  capture(host.handleKeyDown({ code: 'KeyW', repeat: true }).receipt.record);
  const initialLook = host.handleMouseMove({ movementX: 5, movementY: -2 });
  capture(initialLook!.receipt.record);
  assert.equal(sourceSession.tick({ tick: 1 }).tick, 1);

  const pause = host.handleKeyDown({ code: 'Escape' });
  capture(pause.receipt.record);
  assert.equal(pause.receipt.action?.actionId, 'runtime.time.pause');
  assert.equal(sourceSession.readTimeControlState().mode, 'paused');
  assert.deepEqual(host.readout().activeContexts, ['gameplay', 'menu']);
  assert.equal(sourceSession.tick({ tick: 2 }).tick, 1);
  assert.ok(sourceSession.readProjection().runtimeFrame);

  const blockedMovement = host.handleKeyDown({ code: 'KeyW' });
  const blockedCamera = host.handleMouseMove({ movementX: 100, movementY: 100 });
  assert.equal(blockedMovement.receipt.action, null);
  assert.equal(blockedMovement.receipt.consumed, true);
  assert.equal(blockedCamera?.receipt.action, null);
  assert.equal(blockedCamera?.receipt.consumed, true);
  capture(host.handleKeyDown({ code: 'ArrowDown' }).receipt.record);

  const resume = host.handleKeyDown({ code: 'Escape' });
  capture(resume.receipt.record);
  assert.equal(resume.receipt.action?.actionId, 'runtime.time.resume');
  assert.equal(sourceSession.readTimeControlState().mode, 'running');
  assert.deepEqual(host.readout().activeContexts, ['gameplay']);
  capture(host.handleKeyUp({ code: 'KeyW' }).receipt.record);
  assert.equal(sourceSession.tick({ tick: 2 }).tick, 2);
  const sourceFpsOutcome = sourceFps.drain();

  const replaySession = createSession('input-replay.target');
  replaySession.configureInputSession({
    catalog: createDefaultBrowserInputCatalog(), initialContexts: ['gameplay'],
  });
  const replayFps = new BrowserFpsResolvedActionConsumer();
  const replayPause = new ResolvedPauseContextConsumer(replaySession);
  const replayActions: string[] = [];
  for (const record of records) {
    const receipt = replaySession.replayResolvedInputAction(record);
    assert.equal(receipt.accepted, true);
    assert.ok(receipt.replayHash.startsWith('fnv1a64:'));
    replayActions.push(receipt.action!.actionId);
    replayPause.consume(receipt.action!);
    replayFps.accept(receipt.action!);
  }
  assert.deepEqual(replayActions, sourceActions);
  assert.deepEqual(replayFps.drain(), sourceFpsOutcome);
  assert.equal(replaySession.readTimeControlState().mode, 'running');
  assert.deepEqual(
    replaySession.readInputContextState().activeContexts.map((context) => context.contextId),
    ['gameplay'],
  );
  const duplicate = replaySession.replayResolvedInputAction(records.at(-1)!);
  assert.equal(duplicate.accepted, false);
  assert.equal(duplicate.diagnostics[0]?.code, 'replayAlreadyDelivered');
  assert.equal(JSON.stringify(records).includes('KeyW'), false);
  assert.equal(JSON.stringify(records).includes('keyboardKey'), false);
});

void test('editor context resolves shared host samples without raw DOM knowledge in editor consumers', () => {
  const { host } = createHost(['editor']);
  const movement = host.handleKeyDown({ code: 'KeyD' });
  const tool = host.handlePointerDown({ button: 0 });
  assert.equal(movement.receipt.action?.actionId, 'editor.camera.right');
  assert.equal(tool.receipt.action?.actionId, 'editor.tool.primary');
  assert.equal(host.readout().lastDelivery?.sample.control, 'button0');
});

void test('public RuntimeSession facade carries browser input to the shared resolver', () => {
  const bridge = new MockRuntimeBridge();
  const session = createRuntimeSessionFacade({ bridge, mode: 'reference' });
  session.initialize({
    sessionId: 'input-facade-test',
    seed: 12,
    project: { gameId: 'input-test', workspaceId: 'workspace.input-test' },
  });
  const host = new BrowserInputHost({ session, initialContexts: ['gameplay'] });
  const delivery = host.handleKeyDown({ code: 'KeyA' });
  assert.equal(delivery.receipt.action?.actionId, 'gameplay.move.left');
  assert.equal(session.readInputContextState().activeContexts[0]?.contextId, 'gameplay');
});
