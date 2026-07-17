---
status: current
audience: consumer
tags: [browser, host, native, consumer]
supersedes: []
see-also: []
---

# Native Browser Host

Status: `browser-host.v0`

`@asha/browser-host` is the public ASHA Game Project and Studio workflow host
surface for browser-like use that needs native Rust RuntimeBridge authority
before the app boots.

The host owns the dev-server/provider boundary that `asha-demo` and
`asha-studio` must not invent locally. Downstream repos consume the package root
and run the documented command shape:

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
upstream ASHA host implementation detail. The endpoint inventory is derived from
the generated RuntimeBridge manifest; downstream repos do not maintain an RPC
method list. Consumers still see only the public RuntimeBridge provider object
and typed RuntimeSession facade.

A Game Project that statically links Rust gameplay modules builds them into the
same RuntimeBridge cell returned by `createRuntimeBridge`. Browser-host accepts
no second gameplay transport and exposes no gameplay-host endpoint. Combat
events, movement/trigger reconciliation, decisions, scheduling, and replay stay
inside that Rust cell.

## Session and resource lifecycle

Every provider script response receives a cryptographically opaque, host-issued
browser Session capability. Every `createRuntimeBridge()` call within that page
receives a bounded client identity inside the browser Session, so another page
cannot guess a Session capability and use client `0` to share or retire its
ProjectBundle, scheduler, camera, buffer, voxel, gameplay-module, or replay state.

The returned bridge is structurally a normal `RuntimeBridge` and also exposes the
typed `browserHostLifecycle` readout from `NativeBrowserHostRuntimeBridge`:

- `compatibilityVersion` is `browser-host.v0`;
- `sessionId` carries the opaque host-issued browser Session capability;
- `status()` reports `active` or `disconnected`;
- `disconnect()` unloads and retires that client cell.

Studio switches projects by disconnecting the active client cell and asking the
same standard provider for a fresh RuntimeBridge cell before loading the next
ProjectBundle. This preserves statically linked composition while making project
resource release explicit; the host does not insert semantic unload/reload calls
into the generated operation stream. Explicit client disconnect, browser
`pagehide`, and host shutdown unload active ProjectBundle authority and release
the bridge reference. A request using
a missing, malformed, unissued, retired, or disconnected Session/client
capability fails closed with a structured `RuntimeBridgeError`; it never
recreates or reaches authority under a guessed or stale identity.

These lifecycle routes are host implementation details, not an authority port or
freeform call API. The provider has no gameplay-host property, raw addon handle,
Studio callback registry, or downstream-maintained method dispatcher.

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

An ASHA Game Project or Studio native-authority workflow should keep its own app
boot as ordinary browser code:

1. build its UI into `dist/ui` or another static root;
2. launch that root with `asha-browser-host`;
3. resolve `globalThis.ashaRuntimeBridge` from `@asha/runtime-bridge` inside the
   app before creating `RuntimeSession`;
4. pass the resolved bridge to `createRuntimeSessionFacade`;
5. fail closed when the resolver does not report native authority or the
   required bridge operations are absent.

The downstream project should not add a local browser/native bridge, JSON method
tunnel, reference RuntimeSession fallback, or private package import. Studio
uses the same provider kind, global, compatibility marker, and cell lifecycle as
Demo; there is no Studio-specific provider contract.

## Non-Claims

`@asha/browser-host` is host plumbing. It does not own gameplay authority,
collision, combat, health, replay, rendering, policies, or Studio authoring. It
only serves/hosts the UI root and installs the public native Rust provider before
downstream app boot.
