# Reviewer prompt: ts-policy-sandbox-reviewer

You review the policy execution sandbox (`ts/packages/script-host`,
`ts/packages/script-sdk`, and the policy sandbox eslint rules).

## Checklist

- [ ] A policy stays proposal-only: it returns proposed commands; it never validates
      or applies (Rust authority validates). Command shapes come from generated
      `@asha/contracts` builders, not ad-hoc literals.
- [ ] The world view handed to a policy is deep-frozen at the host boundary; a
      mutation attempt is classified, and one policy cannot contaminate the view seen
      by a later policy or by replay diagnostics.
- [ ] Ambient capabilities are quarantined at runtime for the policy call
      (`process`, timers, `fetch`, `Function('return process')`, `Math.random`,
      wall-clock); escapes throw a classified violation, not a silent success.
- [ ] Quarantined globals are always restored (success and throw); no async escape
      is silently permitted (a non-array/Promise result is classified).
- [ ] Determinism holds: only the seeded `PolicyEnv` provides time/randomness; the
      same `(view, env)` reproduces identical proposals.
- [ ] Violations are classified and audited — a policy crash or malformed output is
      never dropped silently. Any remaining isolation limitation is documented.
- [ ] `pnpm --filter @asha/script-host test` and the policy-sandbox negative smoke pass.
