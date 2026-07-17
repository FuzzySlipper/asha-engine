---
status: current
audience: agent
tags: [projection, hud, menu]
supersedes: []
see-also: []
---

# HUD Menu Projection

Status: task #4043 reusable UI projection surface.

Public import path:

```ts
import {
  buildGameHudProjection,
  buildHudProjection,
  hudControlToIntent,
  type GameHudProjection,
  type HudProjection,
  type HudMenuIntent,
} from '@asha/ui-dom';
```

The HUD projection is a pure rusty-view-style model: data in, render-agnostic
control descriptors and typed intents out. It is suitable for Angular/Studio/demo
bindings because no state or authority is hidden in DOM components.

`buildHudProjection()` projects:

- player health: current, max, dead, ratio, accessible label
- status lines
- runtime non-claim text
- menu controls for resume, restart, options, and exit

`buildGameHudProjection()` is the broader FPS/game HUD descriptor surface for
human-facing demos. It projects only readout-shaped data:

- multiple health bars, such as player and current target
- combat counters, accuracy ratio, and optional damage/restart/tick counters
- pointer-lock, movement, fire, and pause status labels
- pose labels for position, facing, and active camera
- status rows and runtime event rows
- menu controls for pause, resume, restart, options, and exit

`hudControlToIntent()` returns typed proposals only:

- `ui.pause_intent`
- `runtime.restart_session_intent`
- `ui.open_options_intent`
- `ui.exit_to_menu_intent`
- `ui.resume_intent`

The restart intent is a UI proposal. Runtime/session code validates and executes
restarts through `RuntimeSessionFacade.requestSessionRestart`, which returns a
typed restart receipt and rejects stale or non-terminal-gated requests without UI
authority.

Non-claims:

- No gameplay authority.
- No restart execution.
- No options or exit implementation.
- No DOM framework requirement.
- No arbitrary JSON payloads.
