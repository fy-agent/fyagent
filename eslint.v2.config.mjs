import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import globals from "globals";
import tseslint from "typescript-eslint";

const v2Files = [
  "src/v2/**/*.{ts,tsx}",
  "tests/v2/**/*.{ts,tsx}",
  "tests/v2-browser/**/*.{ts,tsx}",
  "vitest.v2.config.ts",
  "playwright.v2.config.ts",
];

const legacyImportPatterns = [
  {
    group: [
      "@/App",
      "@/App.*",
      "@/main",
      "@/main.*",
      "@/components",
      "@/components/**",
      "@/hooks",
      "@/hooks/**",
      "@/lib",
      "@/lib/**",
      "@/i18n",
      "@/i18n/**",
      "@/index.css",
    ],
    message:
      "FyAgent V2 owns an isolated renderer boundary and must not import legacy renderer modules.",
  },
];

const v2ImportRestrictions = {
  paths: [
    {
      name: "lucide-react",
      message: "FyAgent V2 uses Phosphor Icons; Lucide remains legacy-only.",
    },
    {
      name: "glasscn-ui",
      message:
        "FyAgent V2 uses its own Radix-backed primitives and semantic glass tokens.",
    },
  ],
  patterns: legacyImportPatterns,
};

const nonTauriBoundaryRestrictions = {
  ...v2ImportRestrictions,
  patterns: [
    ...legacyImportPatterns,
    {
      group: ["@tauri-apps/*", "@tauri-apps/**"],
      message:
        "Direct Tauri imports belong only in src/v2/shared/platform/tauri/**.",
    },
  ],
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
  {
    ...js.configs.recommended,
    files: v2Files,
  },
  ...tseslint.configs.recommended.map((config) => ({
    ...config,
    files: v2Files,
  })),
  {
    files: v2Files,
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
  {
    ...reactHooks.configs.flat.recommended,
    files: v2Files,
  },
  {
    ...reactRefresh.configs.vite,
    files: ["src/v2/{dev,pages,widgets}/**/*.tsx"],
  },
  {
    files: ["src/v2/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-imports": ["error", v2ImportRestrictions],
    },
  },
  {
    files: ["src/v2/**/*.{ts,tsx}"],
    ignores: ["src/v2/shared/platform/tauri/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-imports": ["error", nonTauriBoundaryRestrictions],
    },
  },
];
