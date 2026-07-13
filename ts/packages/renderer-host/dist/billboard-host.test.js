import assert from 'node:assert/strict';
import test from 'node:test';
import { billboardHandle, } from '@asha/contracts';
import { AshaBillboardHost, } from './billboard-host.js';
import { applyAshaRuntimeProjectionFrame } from './audio-host.js';
class FakeElement {
    style = {};
    textContent = null;
    attributes = new Map();
    removed = false;
    setAttribute(name, value) {
        this.attributes.set(name, value);
    }
    remove() {
        this.removed = true;
    }
}
class FakeContainer {
    elements = [];
    appendChild(element) {
        this.elements.push(element);
    }
}
function descriptor(entity, layer = 'occluded') {
    return {
        anchor: { kind: 'entityAttached', entity, offset: [0, 2, 0] },
        content: {
            kind: 'value',
            labelKey: 'enemy.health',
            fallbackLabel: 'Enemy health',
            value: '80/100',
            unitKey: null,
            fallbackUnit: null,
        },
        font: { kind: 'system', family: 'sans-serif' },
        heightPixels: 24,
        color: [1, 1, 1, 1],
        background: [0, 0, 0, 0.7],
        maxDistance: 40,
        layer,
        visible: true,
    };
}
function patch(overrides) {
    return {
        anchor: null,
        content: null,
        font: null,
        heightPixels: null,
        color: null,
        background: null,
        maxDistance: null,
        layer: null,
        visible: null,
        ...overrides,
    };
}
function operation(sequence, op) {
    return {
        domain: 'billboard',
        meta: {
            sequence,
            origin: {
                kind: 'capabilityState',
                id: `health:${sequence}`,
                authorityTick: 7,
                causationId: null,
                correlationId: 'session:1',
            },
        },
        op,
    };
}
function presentation(ops) {
    return { replayScope: 'excludedFromReplayTruth', ops };
}
void test('billboard host creates updates localizes lays out and destroys multiple entity cues', async () => {
    const container = new FakeContainer();
    const positions = new Map([
        [10, [1, 0, 3]],
        [20, [-1, 0, 4]],
    ]);
    const host = new AshaBillboardHost({
        container,
        createElement: () => new FakeElement(),
        localize: (key, fallback, argumentsByName) => {
            const localized = key === 'enemy.health' ? 'Vitality' : fallback;
            return Object.entries(argumentsByName).reduce((text, [name, value]) => text.replaceAll(`{${name}}`, value), localized);
        },
        resolveEntityPosition: (entity) => positions.get(entity) ?? null,
        projectWorld: (position) => ({
            xPixels: 400 + position[0] * 10,
            yPixels: 220 - position[1] * 10,
            depth: position[2] / 10,
            distance: position[2],
            insideViewport: true,
            occluded: false,
        }),
    });
    const first = await host.applyPresentation(presentation([
        operation(0, { op: 'create', handle: billboardHandle(1), descriptor: descriptor(10) }),
        operation(1, { op: 'create', handle: billboardHandle(2), descriptor: descriptor(20) }),
    ]));
    assert.equal(first.applied, 2);
    assert.equal(first.readout.activeBillboards, 2);
    assert.equal(container.elements[0]?.textContent, 'Vitality: 80/100');
    assert.equal(container.elements[0]?.style.left, '410px');
    const second = await host.applyPresentation(presentation([
        operation(0, {
            op: 'update',
            handle: billboardHandle(2),
            patch: patch({
                content: {
                    kind: 'text',
                    localizationKey: 'enemy.defeated',
                    fallbackText: 'Target {state}',
                    arguments: [{ name: 'state', value: 'defeated' }],
                },
            }),
        }),
        operation(1, { op: 'destroy', handle: billboardHandle(1) }),
    ]));
    assert.equal(second.applied, 2);
    assert.equal(second.readout.activeBillboards, 1);
    assert.equal(container.elements[1]?.textContent, 'Target defeated');
    assert.equal(container.elements[0]?.removed, true);
});
void test('billboard layers and distance culling are renderer-owned and do not alter descriptors', async () => {
    const container = new FakeContainer();
    let occluded = true;
    let distance = 12;
    const host = new AshaBillboardHost({
        container,
        createElement: () => new FakeElement(),
        resolveEntityPosition: () => [0, 0, 0],
        projectWorld: () => ({
            xPixels: 100,
            yPixels: 50,
            depth: 0.5,
            distance,
            insideViewport: true,
            occluded,
        }),
    });
    await host.applyPresentation(presentation([
        operation(0, { op: 'create', handle: billboardHandle(1), descriptor: descriptor(10, 'occluded') }),
        operation(1, { op: 'create', handle: billboardHandle(2), descriptor: descriptor(20, 'alwaysOnTop') }),
    ]));
    assert.equal(container.elements[0]?.style.display, 'none');
    assert.equal(container.elements[1]?.style.display, 'block');
    assert.equal(container.elements[1]?.style.zIndex, '30000');
    distance = 50;
    occluded = false;
    host.refreshLayout();
    assert.equal(host.readout().culledBillboards, 2);
});
void test('font and icon resources are SHA-256 validated cached and fail with typed diagnostics', async () => {
    const fontBytes = new Uint8Array([1, 2, 3]).buffer;
    const iconBytes = new Uint8Array([4, 5, 6]).buffer;
    const fontHash = await sha256(fontBytes);
    const iconHash = await sha256(iconBytes);
    let fontLoads = 0;
    const host = new AshaBillboardHost({
        container: new FakeContainer(),
        createElement: () => new FakeElement(),
        loadFont: async () => { fontLoads += 1; },
        resolveEntityPosition: () => [0, 0, 0],
        projectWorld: () => ({ xPixels: 0, yPixels: 0, depth: 0, distance: 0, insideViewport: true, occluded: false }),
        resolveResource: async (asset) => asset.startsWith('font/')
            ? { bytes: fontBytes }
            : { bytes: iconBytes, url: '/fixture-icon.png' },
    });
    const assetDescriptor = {
        ...descriptor(10),
        font: { kind: 'asset', asset: 'font/ui-sans', contentHash: fontHash, family: 'Asha UI' },
        content: {
            kind: 'icon',
            texture: { asset: 'texture/alert', contentHash: iconHash },
            altKey: 'alert',
            fallbackAlt: 'Alert',
        },
    };
    const receipt = await host.applyPresentation(presentation([
        operation(0, { op: 'create', handle: billboardHandle(1), descriptor: assetDescriptor }),
        operation(1, { op: 'create', handle: billboardHandle(2), descriptor: { ...assetDescriptor, anchor: { kind: 'world', position: [0, 1, 0] } } }),
    ]));
    assert.equal(receipt.diagnostics.length, 0);
    assert.equal(receipt.readout.loadedFonts, 1);
    assert.equal(receipt.readout.loadedIcons, 1);
    assert.equal(fontLoads, 1);
    const bad = await host.applyPresentation(presentation([
        operation(0, {
            op: 'create',
            handle: billboardHandle(3),
            descriptor: {
                ...assetDescriptor,
                font: { kind: 'asset', asset: 'font/ui-sans', contentHash: '00', family: 'Asha UI' },
            },
        }),
    ]));
    assert.equal(bad.applied, 0);
    assert.equal(bad.diagnostics[0]?.code, 'contentHashMismatch');
    const missingFontHost = new AshaBillboardHost({
        container: new FakeContainer(),
        createElement: () => new FakeElement(),
        loadFont: async () => undefined,
        resolveEntityPosition: () => [0, 0, 0],
        projectWorld: () => ({ xPixels: 0, yPixels: 0, depth: 0, distance: 0, insideViewport: true, occluded: false }),
        resolveResource: async () => null,
    });
    const missingFont = await missingFontHost.applyPresentation(presentation([
        operation(0, {
            op: 'create',
            handle: billboardHandle(4),
            descriptor: assetDescriptor,
        }),
    ]));
    assert.equal(missingFont.applied, 0);
    assert.equal(missingFont.diagnostics[0]?.code, 'fontLoadFailed');
    assert.equal(missingFont.diagnostics[0]?.origin?.id, 'health:0');
    assert.equal(missingFont.readout.activeBillboards, 0);
});
void test('a missing billboard host is isolated after scene application', async () => {
    let sceneApplied = 0;
    const frame = {
        schemaVersion: 1,
        authorityTick: 9,
        scene: { ops: [] },
        presentation: presentation([
            operation(0, { op: 'create', handle: billboardHandle(1), descriptor: descriptor(10) }),
        ]),
    };
    const receipt = await applyAshaRuntimeProjectionFrame(frame, {
        applyScene: () => { sceneApplied += 1; },
    });
    assert.equal(sceneApplied, 1);
    assert.equal(receipt.audio.diagnostics.length, 0);
    assert.equal(receipt.billboard.applied, 0);
    assert.equal(receipt.billboard.diagnostics[0]?.code, 'unavailableHost');
});
async function sha256(bytes) {
    const digest = await globalThis.crypto.subtle.digest('SHA-256', bytes);
    return Array.from(new Uint8Array(digest))
        .map((byte) => byte.toString(16).padStart(2, '0'))
        .join('');
}
//# sourceMappingURL=billboard-host.test.js.map