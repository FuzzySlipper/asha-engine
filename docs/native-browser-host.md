# Native Browser Host

Status: `browser-host.v0`

`@asha/browser-host` is the public ASHA Game Project host surface for browser-like
human play that needs native Rust RuntimeBridge authority before the app boots.

The host owns the dev-server/provider boundary that `asha-demo` must not invent
locally. Downstream repos consume the package root and run the documented command
shape:

```sh
asha-browser-host --ui-root dist/ui --host 0.0.0.0 --port 5173
```

## Contract

The host injects `/asha/browser-host/native-provider.js` into served HTML before
the app entry imports or boots. That script installs `globalThis.ashaRuntimeBridge`
in the browser page. The installed provider uses:

- provider kind: `asha.runtime_bridge.native_rust_provider.v1`
- provider global: `globalThis.ashaRuntimeBridge`
- backend: `native_rust`
- product authority: `true`
- reference fallback: `false`

The provider is installed through the public `@asha/runtime-bridge` package root.
Game projects do not import `@asha/native-bridge`, private runtime-bridge files,
engine Rust crates, or raw transports.

The host owns the browser-to-native method transport behind bounded
`/asha/browser-host/runtime-bridge/<method>` endpoints. Those endpoints are an
upstream ASHA host implementation detail; game projects still see only the public
RuntimeBridge provider object and typed RuntimeSession facade.

## Status Readout

The static host exposes:

- `/health`
- `/asha/browser-host/runtime-provider.json`
- `/asha/browser-host/native-provider.js`

The provider readout reports `status: "rust_authority"` only after the public
provider resolver accepts the installed native provider and verifies the required
RuntimeBridge operations. Missing, spoofed, reference-backed, or incomplete
providers report `status: "missing_rust_backend"` with typed diagnostics.

## Downstream Shape

An ASHA Game Project should keep its own app boot as ordinary browser code:

1. build its UI into `dist/ui` or another static root;
2. launch that root with `asha-browser-host`;
3. resolve `globalThis.ashaRuntimeBridge` from `@asha/runtime-bridge` inside the
   app before creating `RuntimeSession`;
4. fail closed when the resolver does not report native authority.

The game project should not add a local browser/native bridge, JSON method
tunnel, reference RuntimeSession fallback, or private package import.

## Non-Claims

`@asha/browser-host` is host plumbing. It does not own gameplay authority,
collision, combat, health, replay, rendering, policies, or Studio authoring. It
only serves/hosts the UI root and installs the public native Rust provider before
downstream app boot.
