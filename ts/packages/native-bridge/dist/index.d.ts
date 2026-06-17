/**
 * The typed surface the compiled addon exports. Mirrors the `#[napi]` functions in
 * `native-bridge/src/lib.rs`. Kept in lockstep with the bridge manifest's stable
 * operations; the generated `#[napi]` wrappers (one-in/one-out) replace the
 * hand-written stubs once the codegen emitter lands.
 */
export interface NativeAddon {
    initializeEngine(seed: number): number;
    loadWorldBundle(handle: number, bundleSchemaVersion: number, protocolVersion: number, sceneId: number): {
        loadedWorld: number | null;
        fatalCount: number;
        totalCount: number;
        blocksLoad: boolean;
    };
    submitCommands(handle: number, commandsJson: string): {
        accepted: number;
        rejected: number;
        rejections: unknown[];
    };
    stepSimulation(handle: number, tick: number): number;
    readRenderDiffs(handle: number, cursor: number): {
        ops: unknown[];
    };
    saveCurrentWorld(handle: number): {
        artifactsWritten: number;
        compactedEdits: number;
        retainedEdits: number;
    };
    getCompositionStatus(handle: number): {
        loadedWorld: number | null;
        fatalCount: number;
        totalCount: number;
        blocksLoad: boolean;
    };
}
/** Raised when the native addon cannot be loaded (missing build / ABI mismatch). */
export declare class NativeAddonUnavailable extends Error {
    constructor(message: string);
}
/**
 * Attempt to load the compiled addon. Returns a typed handle or throws a
 * classified {@link NativeAddonUnavailable} — never a raw module-resolution error,
 * so `@asha/runtime-bridge` can re-map it to a `native_unavailable` bridge error.
 *
 * Build the addon with `napi build --platform --release` in the native-bridge crate.
 */
export declare function loadNativeAddon(modulePath?: string): NativeAddon;
//# sourceMappingURL=index.d.ts.map