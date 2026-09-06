import { fileURLToPath } from "node:url";
import { configDefaults, defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// One runner, split only by environment ownership. Product renderer tests are
// part of the normal unit/check aggregate, not a parallel generation opt-in.
export default defineConfig({
  plugins: [react()],
  build: { assetsInlineLimit: 0 },
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  test: {
    projects: [
      {
        extends: true,
        test: {
          name: "contracts",
          environment: "jsdom",
          setupFiles: ["./tests/setupGlobals.ts", "./tests/setupTests.ts"],
          globals: true,
          include: [
            "tests/**/*.{test,spec}.{ts,tsx}",
            "src/domain/**/*.{test,spec}.{ts,tsx}",
          ],
          exclude: [
            ...configDefaults.exclude,
            "**/.worktrees/**",
            "**/*.test.mjs",
            "tests/renderer/**",
            "tests/browser/**",
          ],
        },
      },
      {
        extends: true,
        test: {
          name: "renderer",
          environment: "jsdom",
          environmentOptions: { jsdom: { url: "http://localhost/" } },
          setupFiles: [
            "./tests/setupGlobals.ts",
            "./tests/renderer/app/setup.ts",
          ],
          include: ["tests/renderer/**/*.{test,spec}.{ts,tsx}"],
          exclude: [...configDefaults.exclude, "tests/browser/**"],
          globals: true,
          clearMocks: true,
          restoreMocks: true,
        },
      },
    ],
    coverage: { reporter: ["text", "lcov"] },
  },
});
