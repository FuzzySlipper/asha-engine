---
status: current
audience: agent
tags: [policy, expression, typescript]
supersedes: []
see-also: []
---

# Policy authoring

## What policy is

A policy pack is a TypeScript module that receives a read-only `PolicyView` and returns
a list of proposed `PolicyCommand` values. It expresses intent; Rust decides what happens.

## Minimal policy example

```ts
import type { PolicyView, PolicyCommand } from "@asha/contracts";

export function runPolicy(view: PolicyView): PolicyCommand[] {
  const commands: PolicyCommand[] = [];

  if (view.signals.someSignal > view.thresholds.someThreshold) {
    commands.push({
      kind: "RequestStateChange",
      target: view.subject.id,
      requestedMode: "ExampleMode",
    });
  }

  return commands;
}
```

## What policy may do

- Read fields on the `PolicyView` (generated, read-only).
- Return `PolicyCommand[]` from approved command variants.
- Import `@asha/contracts`, `@asha/script-sdk`, and approved catalog packages.
- Use deterministic helpers from `@asha/script-sdk` (seeded random, pure math).

## What policy must never do

| Forbidden | Reason |
|---|---|
| `Date` / `performance.now()` | breaks determinism |
| `Math.random()` | breaks determinism; use script-sdk RNG |
| `document` / `window` / `localStorage` | no DOM access |
| `fetch` / XHR / WebSocket | no network access |
| Import renderer, UI, bridge, or Electron packages | wrong layer |
| Mutate the view object | views are read-only projections |
| Maintain module-level mutable state | policy is stateless per tick |

Violations of the forbidden list are caught by ESLint (`ts/eslint.config.mjs`).

## Testing a policy

Use the test harness from `@asha/script-sdk`:

```ts
import { buildPolicyView, assertCommands } from "@asha/script-sdk/test-harness";
import { runPolicy } from "./myPolicy";

test("emits RequestStateChange when signal exceeds threshold", () => {
  const view = buildPolicyView({ signals: { someSignal: 10 }, thresholds: { someThreshold: 5 } });
  const commands = runPolicy(view);
  assertCommands(commands, [{ kind: "RequestStateChange" }]);
});
```

Save fixture inputs to `harness/fixtures/policy-inputs/` and expected outputs to
`harness/fixtures/policy-outputs/` for golden regression tests.

## Lane assignment

Policy packages belong to the `ts-policy` lane.
See `governance/lanes/ts-policy.md` for the full rule set.
