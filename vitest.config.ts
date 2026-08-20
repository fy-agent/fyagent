import path from "node:path";
import { configDefaults, defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./tests/setupGlobals.ts", "./tests/setupTests.ts"],
    globals: true,
    // The version contract uses Node's native test runner. Keep those tests
    // out of Vitest discovery so both runners can coexist in the project-level
    // test command.
    // V2 owns dedicated Vitest and Playwright projects. Keep both suites out of
    // the legacy aggregate so their environment setup and runner globals cannot
    // leak into one another.
    exclude: [
      ...configDefaults.exclude,
      // Git-ignored local worktrees contain complete dependency graphs and
      // test suites. Discovering them duplicates tests and React runtimes.
      "**/.worktrees/**",
      "**/*.test.mjs",
      "tests/v2/**",
      "tests/v2-browser/**",
    ],
    coverage: {
      reporter: ["text", "lcov"],
    },
  },
});
