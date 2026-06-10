import tseslint from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";

// Shell/UI/bridge packages a constrained policy or catalog package may never
// import. The dependency graph (harness/depgraph/verify-ts-deps.sh) is the
// canonical enforcement; these lint rules give the same boundary faster, local
// feedback with a lane-routing message.
const FORBIDDEN_SHELL_IMPORTS = [
  {
    name: "@asha/renderer-babylon",
    message:
      "ts-policy/ts-catalog may not import the ts-shell renderer. Route rendering through the generated contract border, not a direct import.",
  },
  {
    name: "@asha/ui-dom",
    message: "ts-policy/ts-catalog may not import ts-shell UI (@asha/ui-dom).",
  },
  {
    name: "@asha/wasm-bridge",
    message:
      "ts-policy/ts-catalog may not import the ts-shell wasm bridge. Policy proposes commands; it never touches WASM memory.",
  },
  {
    name: "@asha/electron-main",
    message: "ts-policy/ts-catalog may not import the Electron main process.",
  },
];

/** @type {import('eslint').Linter.FlatConfig[]} */
export default [
  // Build output is generated, not source — never lint it.
  { ignores: ["**/dist/**"] },
  {
    files: ["packages/**/*.ts"],
    languageOptions: {
      parser: tsParser,
      parserOptions: { project: true },
    },
    plugins: { "@typescript-eslint": tseslint },
    rules: {
      ...tseslint.configs.recommended.rules,
    },
  },
  // Policy/catalog sandbox: forbid dangerous globals everywhere in the package,
  // including tests — determinism must hold for fixtures too.
  {
    files: ["packages/policy-*/**/*.ts", "packages/catalog-*/**/*.ts"],
    rules: {
      "no-restricted-globals": [
        "error",
        { name: "Date",         message: "Policy may not use wall-clock time." },
        { name: "document",     message: "Policy may not access the DOM." },
        { name: "window",       message: "Policy may not access window." },
        { name: "localStorage", message: "Policy may not access localStorage." },
        { name: "fetch",        message: "Policy may not make network calls." },
      ],
      "no-restricted-syntax": [
        "error",
        {
          selector: "MemberExpression[object.name='Math'][property.name='random']",
          message: "Policy may not use Math.random; use deterministic RNG from script-sdk.",
        },
      ],
    },
  },
  // Policy/catalog SOURCE (not tests): forbid host-environment imports. Test
  // files legitimately use Node's built-in test runner and fixture I/O, so they
  // are excluded from this block; policy/catalog *source* must stay pure.
  {
    files: ["packages/policy-*/**/*.ts", "packages/catalog-*/**/*.ts"],
    ignores: ["**/*.test.ts"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          paths: [
            ...FORBIDDEN_SHELL_IMPORTS,
            { name: "fs", message: "Policy source may not touch the filesystem." },
            { name: "net", message: "Policy source may not open sockets." },
            { name: "http", message: "Policy source may not make network calls." },
            { name: "https", message: "Policy source may not make network calls." },
            { name: "child_process", message: "Policy source may not spawn processes." },
            { name: "os", message: "Policy source may not read host environment." },
            { name: "process", message: "Policy source may not read host environment." },
          ],
          patterns: [
            {
              group: ["node:*"],
              message:
                "Policy source may not import Node built-ins (filesystem, network, process, etc.); a policy is a pure function of its view.",
            },
          ],
        },
      ],
    },
  },
];
