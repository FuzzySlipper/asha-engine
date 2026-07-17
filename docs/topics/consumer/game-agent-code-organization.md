---
status: current
audience: consumer
tags: [consumer, game, organization]
supersedes: []
see-also: []
---

# Game Agent Code Organization

Status: draft guidance for ASHA game consumer repos such as `asha-demo`.

This document describes how agents should organize TypeScript code for a game
that uses ASHA. It is intentionally stricter than a small demo needs, because
the demo is also a pattern for future game repos.

For downstream games that need compiled game-specific Rust authority, see
`docs/game-rust-authority-extension-model.md`. This TypeScript guide still
applies: game TS describes content, input, and projection; compiled Rust and ASHA
RuntimeSession authority decide accepted state changes.

## Hard Rules

1. `app.ts` is an entry point only.
   It should import a boot function, call it, and handle only top-level fatal
   reporting. Treat it like a non-trivial `Program.cs`: once behavior grows,
   it belongs behind named modules, not in the entry file.
2. Game source is TypeScript only.
   Do not hand-write browser or runtime source as `.js`. Generated build output
   may be JavaScript under `dist/`, and tool config may use `.mjs` only when the
   tool cannot practically load TypeScript. Any hand-written `.js` in game
   source should be treated as migration debt.
3. Demo/game TypeScript does not own gameplay authority.
   It may load authored content, collect browser input, project HUD/render
   view models, and submit typed requests. Rust/ASHA public runtime surfaces
   decide movement, collision, combat, health, lifecycle, replay, and accepted
   state.
4. No single stuffing point.
   App composition, content loading, runtime gateway calls, input collection,
   render projection, HUD/menu projection, and feature registration must live
   in separate modules with names that reveal their authority boundary.
5. Public ASHA imports only.
   Game repos import ASHA package roots or explicitly approved public subpaths.
   They do not import ASHA `src/*`, generated contract files by path, Rust
   crates, private bridges, renderer backends, or proof harness internals.

## Recommended Shape

Use a small composition-root plus ports/adapters structure. This is the
well-known TypeScript version of hexagonal architecture without adding a
framework. The entrypoint wires dependencies; feature modules expose explicit
factories; adapters talk to browser or ASHA public surfaces; domain-facing game
code works with narrow ports.

Recommended layout:

```text
src/
  app.ts
  bootstrap/
    boot-game.ts
    compose-game.ts
    feature-registry.ts
  content/
    load-project-content.ts
    preflight-project-content.ts
    content-types.ts
  runtime/
    asha-runtime-port.ts
    asha-runtime-session.ts
    native-provider.ts
  input/
    browser-input-adapter.ts
    input-intents.ts
  projection/
    hud-view.ts
    menu-view.ts
    render-targets.ts
    telemetry-view.ts
  features/
    generated-tunnel/
      generated-tunnel-feature.ts
      generated-tunnel-content.ts
      generated-tunnel-render.ts
    combat-hud/
      combat-hud-feature.ts
      combat-hud-view.ts
  shell/
    dom-mount.ts
    frame-loop.ts
    browser-host.ts
```

`app.ts` should look boring:

```ts
import { bootGame } from './bootstrap/boot-game.js';

void bootGame().catch((error: unknown) => {
  reportFatalBootError(error);
});
```

If `app.ts` needs to know about a weapon, enemy, HUD row, key binding, render
target, catalog path, or RuntimeSession method, the structure has already
started drifting.

## Module Responsibilities

`bootstrap/` owns composition. It chooses which features/adapters are active,
passes dependencies explicitly, and starts the frame loop. It should not contain
gameplay algorithms.

`content/` owns authored game files and consumer-side preflight diagnostics.
Use words like `preflight`, `read`, and `describe`. Avoid `validate` when the
result could be confused with Rust acceptance authority.

`runtime/` owns the ASHA runtime port. It is the only game source lane that
should call `RuntimeSession` methods directly. It wraps those calls in
game-facing request/read functions such as `requestPrimaryFire`,
`requestRestart`, `readPlayableLoopProjection`, and
`requestCollisionConstrainedCameraMove`.

`input/` owns browser/platform events and converts them into typed intents. It
does not resolve movement, damage, cooldowns, or lifecycle.

`projection/` owns read-model conversion: ASHA readouts plus shell state become
HUD, menu, render, telemetry, and diagnostic view models. Projection modules
never submit commands.

`features/` owns game-specific assembly. A feature may declare the content refs,
view projections, and controls it contributes, but it must depend on ports
rather than private ASHA implementations.

`shell/` owns DOM mounting, browser frame loops, pointer lock attachment, and
host lifecycle. It should be replaceable by a standalone host without rewriting
game logic.

## Ports And Adapters

Use small interfaces for dependencies that cross boundaries:

```ts
export interface AshaRuntimePort {
  requestPrimaryFire(input: PrimaryFireRequest): Promise<PrimaryFireReceipt>;
  requestRestart(input: RestartRequest): Promise<RestartReceipt>;
  readPlayableLoopProjection(): PlayableLoopProjection;
}

export interface BrowserInputPort {
  readFrameInput(): BrowserInputFrame;
  requestPointerLock(): Promise<PointerLockReceipt>;
}
```

Adapters implement ports:

- `asha-runtime-session.ts` implements a game-facing port against neutral
  `@asha/runtime-session` contracts and receives concrete construction from
  public `@asha/runtime-bridge` at the composition root.
- `browser-input-adapter.ts` adapts DOM/pointer-lock/keyboard events.
- a standalone host can provide another adapter without changing features.

Avoid hidden global registries, mutable service locators, and broad "manager"
classes. Pass ports explicitly through composition.

## Feature Loading

Prefer explicit feature descriptors over central piles of declarations:

```ts
export interface GameFeature {
  readonly id: string;
  readonly contentRefs: readonly string[];
  mount(context: GameFeatureContext): MountedGameFeature;
}
```

`feature-registry.ts` may gather static imports:

```ts
import { generatedTunnelFeature } from '../features/generated-tunnel/generated-tunnel-feature.js';
import { combatHudFeature } from '../features/combat-hud/combat-hud-feature.js';

export const gameFeatures = [
  generatedTunnelFeature,
  combatHudFeature,
] as const;
```

This keeps declarations close to their feature while still making the loaded set
reviewable. Avoid dynamic plugin discovery until ASHA has a reason and a typed
manifest for it.

## Naming Guidance

Names should make authority posture obvious.

Prefer:

- `preflightProjectContent`
- `loadAuthoredProjectFiles`
- `requestPrimaryFire`
- `projectHudView`
- `readRuntimeHealthProjection`
- `composeGame`
- `mountBrowserShell`

Avoid:

- `validateProjectContent` for consumer-side checks
- `applyDamage` in game TS
- `resolveCollision` in game TS
- `updateHealth` in UI/shell code
- `runEnemyAi` when the module is only submitting policy proposals
- `GameManager`, `RuntimeManager`, or `AppController` as broad catch-alls

## JavaScript Exceptions

The preferred answer is: game source should not need hand-written JavaScript.

Acceptable exceptions are narrow:

- emitted build output under `dist/`;
- generated vendor/browser bundles copied from ASHA packages;
- tool configuration files where the tool does not support TypeScript config in
  this repo yet;
- temporary migration shims with a task to remove them.

Node scripts should also be TypeScript when they become durable product or
release machinery. A tiny `.mjs` script is acceptable for simple plumbing, but
it must not become gameplay structure, runtime authority, or a second app.

## Review Checklist

Reviewers and agents should flag:

- `app.ts` containing feature logic, runtime calls, DOM event logic, or HUD
  declarations;
- hand-written browser/runtime `.js` source;
- modules that both submit runtime requests and project UI;
- local state that shadows RuntimeSession health, collision, combat, lifecycle,
  replay, or generated level truth;
- imports from ASHA private paths or concrete renderer backends;
- "temporary" adapters with no owner task and no removal path;
- feature declarations centralized in one large file instead of colocated with
  feature modules.

The desired failure mode is simple: a game agent can see where new code belongs,
and the codebase makes it awkward to put authority in TypeScript by accident.
