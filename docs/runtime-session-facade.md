# RuntimeSession Facade Status

Status: initial public semantic facade for task #4028.

## Public Import Path

Consumers import from the package root:

```ts
import { createMockRuntimeSession, type RuntimeSessionFacade } from '@asha/runtime-bridge';
```

No consumer should import package internals, raw native transports, generated file paths, or Rust crate paths.

## Current API

`RuntimeSessionFacade` exposes:

- `initialize(input)`: validates semantic session/project input, initializes the bridge, and loads a ProjectBundle-shaped request.
- `submitCommands(batch)`: submits generated `CommandBatch` values only.
- `tick(input?)`: advances deterministic runtime ticks through the bridge.
- `readProjection()`: returns a render/projection summary from public render diff contracts.
- `readTelemetry()`: returns sequence/tick/composition/command/replay/hash summary.
- `restart()`: unloads/reinitializes/reloads the same ProjectBundle input and resets tick/command counters.

The first implementation is `createMockRuntimeSession`, a reference/mock facade over the existing public `RuntimeBridge` mock. It is sufficient for downstream skeleton boot/readout tests and Studio contract work, but it does not claim native authority, renderer ownership, or gameplay behavior.

## Runtime Vocabulary

The public facade uses `RuntimeSession` and `ProjectBundle` vocabulary. Internally, the current bridge still wraps older WorldBundle-shaped DTOs for compatibility (`WorldLoadRequest`), as documented in `docs/vocabulary-compatibility.md`.

## Non-Claims

The reference RuntimeSession reports these non-claims:

- `not_native_runtime`
- `not_raw_state_store`
- `not_arbitrary_json_bridge`
- `not_gameplay_loop`
- `not_renderer`

These non-claims are intentional until native runtime/session attach and renderer/gameplay tasks land.
