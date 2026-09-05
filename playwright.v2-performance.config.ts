import os from "node:os";
import path from "node:path";
import { defineConfig, devices } from "@playwright/test";

// A production build, one worker and a fixed viewport keep comparisons useful.
// This is supplemental profiling, not a substitute for native WebView evidence.
export default defineConfig({
  testDir: "./tests/v2-browser",
  testMatch: "navigation-performance.spec.ts",
  outputDir: path.join(os.tmpdir(), "fyagent-v2-performance"),
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
    command:
      "pnpm exec vite build && node scripts/verify-v2-route-chunks.mjs && pnpm exec vite preview --host 127.0.0.1 --port 4175 --strictPort",
    url: "http://127.0.0.1:4175",
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
