// @asha/electron-main — the Electron MAIN-process host (task #2439).
//
// Per the ts-shell lane boundary, the main process owns window/process/IPC only and
// does NOT import the runtime/renderer/app packages. The application itself — the shared
// `@asha/app` composition root (`composeAppShell` / the `dev:asha-shell` entry) — runs in
// the Electron *renderer* process, exactly like the browser and headless launch targets.
// So all hosts share one composition root; the main process just opens an accessible
// window pointed at that shared entry.
//
// `electron` is intentionally not a build dependency (it ships a platform binary and is
// excluded from the offline build), so this module is structured around injected
// capabilities: pure builders return plain option objects, and `createMainWindow` takes
// an injected window factory. That keeps the main-process logic fully unit-testable
// without launching a real Electron runtime.

/**
 * The host capability descriptor the renderer-process composition reads (structurally
 * matches `@asha/app`'s `HostCapabilities` — duplicated, not imported, to respect the
 * main-process import boundary). Electron exposes a real accessibility tree, so the
 * accessible control model renders into actual ARIA nodes a screen reader / Playwright
 * can drive.
 */
export interface ElectronHostDescriptor {
  readonly name: 'electron';
  readonly accessibility: true;
}

/** The Electron host descriptor injected into the shared composition root. */
export function electronHost(): ElectronHostDescriptor {
  return { name: 'electron', accessibility: true };
}

/** The documented launch commands (shared with the headless/browser targets). */
export const ELECTRON_LAUNCH_COMMAND = 'pnpm --filter @asha/electron-main start';
export const SHARED_SHELL_LAUNCH_COMMAND = 'pnpm --filter @asha/app dev:asha-shell';

/**
 * The renderer-process entry the window loads — the SHARED app shell, not an
 * Electron-only fork. Resolved relative to the built app package so the same bundle
 * backs every host.
 */
export const RENDERER_ENTRY = '@asha/app/dist/cli.js';

/**
 * BrowserWindow construction options with accessibility enabled and node integration
 * kept out of the renderer (the renderer runs the sandboxed app composition; authority
 * stays in the runtime bridge, never in the privileged main process). Returned as a
 * plain object so it can be asserted in tests without importing `electron`.
 */
export interface MainWindowOptions {
  readonly width: number;
  readonly height: number;
  readonly title: string;
  /** Accessibility support is on by default and explicitly recorded for tests. */
  readonly accessibleTitle: string;
  readonly webPreferences: {
    readonly sandbox: true;
    readonly nodeIntegration: false;
    readonly contextIsolation: true;
    /** Render the accessibility tree so screen readers / automation can navigate. */
    readonly enableAccessibility: true;
  };
}

/** Build the accessible main-window options. */
export function mainWindowOptions(overrides: Partial<MainWindowOptions> = {}): MainWindowOptions {
  return {
    width: 1280,
    height: 800,
    title: 'ASHA',
    accessibleTitle: 'ASHA voxel editor shell',
    webPreferences: {
      sandbox: true,
      nodeIntegration: false,
      contextIsolation: true,
      enableAccessibility: true,
    },
    ...overrides,
  };
}

/** A minimal structural view of the Electron objects the host needs (injected in tests). */
export interface ElectronWindowLike {
  loadFile(entry: string): void | Promise<void>;
}
export type WindowFactory = (options: MainWindowOptions) => ElectronWindowLike;

/**
 * Open the accessible main window and load the shared renderer entry. The window factory
 * is injected (the real one wraps `new BrowserWindow(...)` when running under Electron),
 * so this is testable without an Electron runtime. Authority/accessibility settings are
 * applied through {@link mainWindowOptions}.
 */
export function createMainWindow(
  createWindow: WindowFactory,
  options: MainWindowOptions = mainWindowOptions(),
): ElectronWindowLike {
  const window = createWindow(options);
  void window.loadFile(RENDERER_ENTRY);
  return window;
}
