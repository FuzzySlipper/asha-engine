---
status: current
audience: consumer
tags: [consumer, game, organization, typescript, architecture]
supersedes: []
see-also: [consumer-compatibility.md, runtime-session-facade.md]
---

# Game Agent Code Organization

Guidance for ASHA game consumer repos such as `asha-demo`. Stricter than a small demo needs because the demo is a pattern for future game repos.

## Hard Rules

1. `app.ts` is an entry point only.
2. Game source is TypeScript only.
3. Demo/game TypeScript does not own gameplay authority.
4. No single stuffing point.
5. Public ASHA imports only.

## Recommended Shape

Small composition-root plus ports/adapters structure. Entrypoint wires dependencies; feature modules expose explicit factories; adapters talk to browser or ASHA public surfaces; domain-facing game code works with narrow ports.

See `docs/game-agent-code-organization.md` for the full layout, naming guidance, and review checklist.
