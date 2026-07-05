import tseslint from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import generatedBoundaryConfigs from "./eslint-boundaries.generated.mjs";

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
      "@typescript-eslint/consistent-type-imports": [
        "error",
        {
          prefer: "type-imports",
          disallowTypeAnnotations: false,
        },
      ],
      "@typescript-eslint/explicit-module-boundary-types": "error",
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-floating-promises": "error",
      "@typescript-eslint/no-misused-promises": "error",
      "@typescript-eslint/no-unsafe-argument": "error",
      "@typescript-eslint/no-unsafe-assignment": "error",
      "@typescript-eslint/no-unsafe-call": "error",
      "@typescript-eslint/no-unsafe-member-access": "error",
      "@typescript-eslint/no-unsafe-return": "error",
    },
  },
  ...generatedBoundaryConfigs,
  // Policy/catalog sandbox: forbid dangerous globals everywhere in the package,
  // including tests — determinism must hold for fixtures too.
  {
    files: [
      "packages/policy-*/**/*.ts",
      "packages/catalog-*/**/*.ts",
      "packages/script-sdk/**/*.ts",
      "packages/script-host/**/*.ts",
    ],
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
    files: [
      "packages/policy-*/**/*.ts",
      "packages/catalog-*/**/*.ts",
      "packages/script-sdk/**/*.ts",
      "packages/script-host/**/*.ts",
    ],
    ignores: ["**/*.test.ts"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          paths: [
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
