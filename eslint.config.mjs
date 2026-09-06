import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import globals from "globals";
import tseslint from "typescript-eslint";

const files = [
  "src/**/*.{ts,tsx}",
  "tests/renderer/**/*.{ts,tsx}",
  "tests/browser/**/*.{ts,tsx}",
  "config/**/*.ts",
];
const patterns = [
  {
    group: [
      "@/v2/**",
      "@/v3/**",
      "@/App",
      "@/components/**",
      "@/hooks/**",
      "@/lib/**",
      "@/i18n/**",
      "@/index.css",
    ],
    message:
      "Use the single renderer's domain, shared, page or widget owner; retired generation imports are not compatibility APIs.",
  },
];
const imports = {
  paths: [
    { name: "lucide-react", message: "Use the adopted Phosphor Icons." },
    { name: "glasscn-ui", message: "Use the shared Radix/material adapter." },
  ],
  patterns,
};
export default [
  {
    ignores: [
      "dist/**",
      "node_modules/**",
      "playwright-report/**",
      "test-results/**",
    ],
  },
  { ...js.configs.recommended, files },
  {
    ...js.configs.recommended,
    files: ["config/**/*.cjs"],
    languageOptions: { sourceType: "commonjs", globals: globals.node },
  },
  ...tseslint.configs.recommended.map((config) => ({ ...config, files })),
  {
    files,
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: { ...globals.browser, ...globals.node },
    },
  },
  { ...reactHooks.configs.flat.recommended, files },
  { ...reactRefresh.configs.vite, files: ["src/{dev,pages,widgets}/**/*.tsx"] },
  {
    files: ["src/**/*.{ts,tsx}"],
    rules: { "no-restricted-imports": ["error", imports] },
  },
  {
    files: ["src/**/*.{ts,tsx}"],
    ignores: ["src/shared/platform/tauri/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          ...imports,
          patterns: [
            ...patterns,
            {
              group: ["@tauri-apps/*", "@tauri-apps/**"],
              message: "Tauri imports belong to shared/platform/tauri only.",
            },
          ],
        },
      ],
    },
  },
];
