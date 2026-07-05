// @asha/app — the shared, host-agnostic launch entry for the composition root
// (task #2439). The SAME `composeAppShell` assembly is what every host runs: the
// headless CLI here, a browser entry, and the Electron renderer process. Hosts differ
// only in the injected `HostCapabilities`, renderer port, and bridge boot — never in a
// parallel architecture.
//
// `runHeadlessLaunch` is the documented CI-safe launch target: it composes the shell,
// loads the active fixture, projects authority through the facade, and returns the
// deterministic `ShellReadout`. It never silently downgrades — an authority launch with
// no native addon reports `unavailable`, not a faked mock success.

import {
  createNativeRuntimeBridge,
  RuntimeBridgeError,
} from '@asha/runtime-bridge';
import { createMockRuntimeBridge } from '@asha/runtime-bridge/reference';

import { composeAppShell, threeRendererPort } from './shell.js';
import type {
  AppBridgeBoot,
  AppShell,
  FixtureChoice,
  HostCapabilities,
  ShellReadout,
} from './shell.js';

/** Which runtime the launch targets (mirrors the smoke harness intents). */
export type LaunchMode = 'reference' | 'authority';

/** The documented dev/launch commands (referenced by the host READMEs and Den docs). */
export const SHELL_LAUNCH_COMMAND = 'pnpm --filter @asha/app dev:asha-shell';
export const AUTHORITY_SHELL_LAUNCH_COMMAND =
  'ASHA_SHELL_MODE=authority pnpm --filter @asha/app dev:asha-shell';

/** The headless host descriptor (model-only; no real a11y tree to render into). */
export function headlessHost(): HostCapabilities {
  return { name: 'headless', accessibility: false };
}

/**
 * The reference boot: the deterministic mock facade, while *probing* native
 * availability for an honest readout. The reference path never depends on the addon.
 */
export function referenceBoot(): AppBridgeBoot {
  return {
    bridge: createMockRuntimeBridge(),
    mode: 'mock',
    intent: 'reference',
    nativeAvailable: probeNativeAvailable(),
  };
}

/**
 * The authority boot: attempt the real native path. If the addon is not loadable, the
 * boot fails *closed* with a classified error — the shell reports `unavailable`, never a
 * silent downgrade to the mock.
 */
export function authorityBoot(): AppBridgeBoot {
  try {
    return {
      bridge: createNativeRuntimeBridge(),
      mode: 'native',
      intent: 'authority',
      nativeAvailable: true,
    };
  } catch (cause) {
    const bootError =
      cause instanceof RuntimeBridgeError
        ? cause
        : new RuntimeBridgeError('native_unavailable', cause instanceof Error ? cause.message : String(cause));
    return { bridge: null, mode: 'native', intent: 'authority', nativeAvailable: false, bootError };
  }
}

/** Pick a boot strategy from an explicit launch mode. */
export function bootForMode(mode: LaunchMode): AppBridgeBoot {
  return mode === 'authority' ? authorityBoot() : referenceBoot();
}

/** Whether the native addon is loadable, without depending on it for the run. */
function probeNativeAvailable(): boolean {
  try {
    createNativeRuntimeBridge();
    return true;
  } catch {
    return false;
  }
}

/**
 * The canonical runtime-selectable fixture catalog for the launch. Two fixtures prove
 * selection is data (runtime), not a compile-time switch. `launch-grid` is the seeded
 * launch world (grid 1, materials 1–3 — matching the reference bridge seed); `alt-grid`
 * is a second world with a single material to exercise palette/fixture switching.
 */
export function defaultFixtures(): FixtureChoice[] {
  return [
    {
      id: 'launch-grid',
      label: 'Launch grid',
      materials: [1, 2, 3],
      request: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1001 },
    },
    {
      id: 'alt-grid',
      label: 'Alternate grid',
      materials: [1],
      request: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1002 },
    },
  ];
}

/** Options for {@link runHeadlessLaunch} (all injectable for tests). */
export interface HeadlessLaunchOptions {
  readonly mode?: LaunchMode;
  readonly host?: HostCapabilities;
  readonly fixtures?: readonly FixtureChoice[];
  readonly initialFixtureId?: string;
  /** Override the bridge boot directly (tests inject degraded/unavailable). */
  readonly bootBridge?: () => AppBridgeBoot;
  /** Inject a renderer port; defaults to a real headless three renderer. */
  readonly renderer?: ReturnType<typeof threeRendererPort> | null;
}

/**
 * Compose the shell for a headless launch and drive load → projection so the returned
 * readout reflects a real assembled run. The shell instance is returned alongside the
 * readout so callers (tests) can drive further interactions.
 */
export function launchShell(options: HeadlessLaunchOptions = {}): AppShell {
  const mode = options.mode ?? 'reference';
  // `renderer: null` opts out of any renderer (UI-only); otherwise default to a real
  // headless three renderer. Spread conditionally to satisfy exactOptionalPropertyTypes.
  const renderer = options.renderer === null ? undefined : (options.renderer ?? threeRendererPort());
  const shell = composeAppShell({
    host: options.host ?? headlessHost(),
    bootBridge: options.bootBridge ?? (() => bootForMode(mode)),
    fixtures: options.fixtures ?? defaultFixtures(),
    ...(options.initialFixtureId !== undefined ? { initialFixtureId: options.initialFixtureId } : {}),
    ...(renderer !== undefined ? { renderer } : {}),
  });
  shell.loadActiveFixture();
  shell.projectAuthority();
  return shell;
}

/** Run the headless launch and return the deterministic readout. */
export function runHeadlessLaunch(options: HeadlessLaunchOptions = {}): ShellReadout {
  return launchShell(options).readout();
}
