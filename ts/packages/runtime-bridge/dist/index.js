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
export { ResolvedTimeControlConsumer, TIME_CONTROL_INPUT_ACTIONS, timeControlCommandFromResolvedAction, } from './resolved-time-control.js';
export { buildRuntimeSessionAnimationControllerTargetFrame } from './runtime-session-animation.js';
// Render-diff decode (moved from the former @asha/wasm-bridge). Transport-neutral
// payload -> contract types; backs `readRenderDiffs`. See render-decode.ts.
export { decodeRenderDiff, decodeRenderFrameDiff, RenderDecodeError, RenderDiffStream, FrameMemory, } from './render-decode.js';
export { RUNTIME_BRIDGE_PORT_CONTRACTS, RuntimeBridgeError, frameCursor, runtimeBridgePorts, } from './bridge.js';
export { SelectedBackendGameRuntimeLauncher, createNativeGameRuntimeLauncher, createSelectedBackendGameRuntimeLauncher, nativeBackendProfile, validateGameRuntimeBackendProfile, } from './launcher.js';
export * from './native.js';
export * from './browser-input-host.js';
export * from './browser-fps-resolved-actions.js';
export * from './resolved-time-control.js';
export * from './resolved-camera-navigation.js';
export * from './native-runtime-provider.js';
export * from './playable-encounter-tick.js';
export * from './playable-loop-state.js';
export { createRuntimeSessionFacade, } from './runtime-session-adapter.js';
//# sourceMappingURL=index.js.map