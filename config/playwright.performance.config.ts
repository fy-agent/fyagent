import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, devices } from "@playwright/test";

const repositoryRoot = fileURLToPath(new URL("..", import.meta.url));

// A production build, one worker and a fixed viewport keep comparisons useful.
// This is supplemental profiling, not a substitute for native WebView evidence.
export default defineConfig({
  testDir: path.join(repositoryRoot, "tests/browser"),
  testMatch: [
    "mcp-followup-origins.spec.ts",
    "navigation-performance.spec.ts",
    "presentation-performance.spec.ts",
    "theme-performance.spec.ts",
    "state-performance.spec.ts",
    "dialog-origins.spec.ts",
  ],
  outputDir: path.join(os.tmpdir(), "fyagent-performance"),
  workers: 1,
  retries: 0,
  timeout: 120_000,
  use: {
    ...devices["Desktop Chrome"],
    baseURL: "http://127.0.0.1:4175",
    viewport: { width: 1232, height: 700 },
    trace: "retain-on-failure",
  },
  webServer: {
    cwd: repositoryRoot,
    command:
      "pnpm build:renderer && pnpm exec vite preview --config config/vite.config.ts --host 127.0.0.1 --port 4175 --strictPort",
    url: "http://127.0.0.1:4175",
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
