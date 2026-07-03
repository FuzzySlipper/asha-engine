# Browser FPS Input

Status: task #4030 upstream input surface for future `asha-demo` controls.

Public import path:

```ts
import {
  BrowserFpsInputCollector,
  createMockRuntimeSession,
} from '@asha/runtime-bridge';
```

Typed runtime command emitted per drain:

```ts
{
  kind: 'runtime.apply_first_person_camera_input',
  envelope: FirstPersonCameraInputEnvelope
}
```

The envelope is accepted by `RuntimeSessionFacade.applyFirstPersonCameraInput`.
The collector also emits typed shell intents:

- `{ kind: 'request_pointer_lock', reason: 'primary_button' | 'programmatic' }`
- `{ kind: 'release_pointer_lock', reason: 'escape_key' | 'programmatic' }`

Input mapping:

- `KeyW` / `KeyS` map to `moveForward` `1` / `-1`.
- `KeyD` / `KeyA` map to `moveRight` `1` / `-1`.
- Mouse movement is accumulated only while pointer lock is active.
- `yawDeltaDegrees = movementX * mouseSensitivityDegreesPerPixel`.
- `pitchDeltaDegrees = -movementY * mouseSensitivityDegreesPerPixel`.
- `Escape` emits pointer-lock release intent and records `releaseRequestedByEscape`.

Non-claims:

- No gameplay movement, collision, or physics.
- No authority mutation from browser input.
- No demo wiring yet.
- Primary fire is reported as `unsupported_primary_fire` because no public runtime action/fire protocol exists yet.
