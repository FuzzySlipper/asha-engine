// @asha/runtime-bridge — the public, transport-agnostic runtime facade (ADR 0006).
//
// App / UI / renderer / devtools couple ONLY to this package root for runtime.
// The implementation is split by concern behind this barrel. Reference/mock
// helpers live at @asha/runtime-bridge/reference so production consumers do not
// casually couple to the deterministic fixture backend.
//
// The facade exports generated-compatible contract types and explicit
// buffer-handle APIs — never raw addon exports, WASM memory, or JSON escape
// hatches. The manifest-derived conformance tests keep these re-exports stable.
export { MANIFEST_OPERATIONS } from './generated/operations.js';
// Render-diff decode (moved from the former @asha/wasm-bridge). Transport-neutral
// payload -> contract types; backs `readRenderDiffs`. See render-decode.ts.
export { decodeRenderDiff, decodeRenderFrameDiff, RenderDecodeError, RenderDiffStream, FrameMemory, } from './render-decode.js';
export { RuntimeBridgeError, frameCursor } from './bridge.js';
export { SelectedBackendGameRuntimeLauncher, createNativeGameRuntimeLauncher, createSelectedBackendGameRuntimeLauncher, nativeBackendProfile, validateGameRuntimeBackendProfile, } from './launcher.js';
export * from './native.js';
export * from './browser-fps-input.js';
// Compatibility shim for RuntimeSession semantic readouts/proposal shapes.
// New consumers import these from @asha/runtime-session; keep the bridge root
// re-export only for runtime-bridge.v0 callers while the migration closes.
export * from '@asha/runtime-session';
export * from './native-runtime-provider.js';
export * from './playable-encounter-tick.js';
export * from './playable-loop-state.js';
export * from './runtime-session.js';
//# sourceMappingURL=index.js.map