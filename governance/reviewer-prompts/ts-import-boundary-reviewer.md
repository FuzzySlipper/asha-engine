# Reviewer prompt: ts-import-boundary-reviewer

You review TypeScript import boundaries against the lane rules in
`governance/ownership.toml` and `ts/eslint.config.mjs`.

## Checklist

- [ ] Dependency direction holds: `contracts → script-sdk → policy/catalog →
      script-host`, and `contracts → runtime-bridge → renderer/ui/devtools/
      editor-tools → app → electron-main`. No lower layer imports a higher one.
- [ ] App/UI/renderer/devtools import **only** `@asha/runtime-bridge` for runtime —
      never `native-bridge`, `wasm-replay-bridge`, raw addon exports, or WASM memory.
- [ ] Policy/catalog/script source imports no host environment (`fs`, `net`,
      `process`, `node:*`) and no renderer/UI/runtime-bridge package.
- [ ] No hand-edits to `ts/packages/contracts/src/generated/`.
- [ ] Contract types are imported from `@asha/contracts`, not re-declared/duplicated.
- [ ] `pnpm lint` and `verify-ts-deps.sh` pass; new packages have an ownership entry.
