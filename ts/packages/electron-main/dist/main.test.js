import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createMainWindow, electronHost, mainWindowOptions, RENDERER_ENTRY, SHARED_SHELL_LAUNCH_COMMAND, } from './index.js';
void test('electron host descriptor enables accessibility for the shared composition root', () => {
    const host = electronHost();
    assert.equal(host.name, 'electron');
    assert.equal(host.accessibility, true);
});
void test('main window options are accessible and keep the renderer sandboxed', () => {
    const opts = mainWindowOptions();
    assert.equal(opts.webPreferences.enableAccessibility, true);
    assert.equal(opts.webPreferences.sandbox, true);
    assert.equal(opts.webPreferences.nodeIntegration, false);
    assert.equal(opts.webPreferences.contextIsolation, true);
    assert.ok(opts.accessibleTitle.length > 0);
});
void test('createMainWindow loads the SHARED app shell entry, not an electron-only fork', () => {
    const loaded = [];
    const usedOptions = [];
    const fakeWindow = {
        loadFile(entry) {
            loaded.push(entry);
        },
    };
    const window = createMainWindow((options) => {
        usedOptions.push(options);
        return fakeWindow;
    });
    assert.equal(window, fakeWindow);
    assert.deepEqual(loaded, [RENDERER_ENTRY]);
    assert.match(RENDERER_ENTRY, /@asha\/app/);
    assert.equal(usedOptions[0].webPreferences.enableAccessibility, true);
    // The host points at the same documented launch entry as the headless/browser targets.
    assert.match(SHARED_SHELL_LAUNCH_COMMAND, /@asha\/app dev:asha-shell/);
});
//# sourceMappingURL=main.test.js.map