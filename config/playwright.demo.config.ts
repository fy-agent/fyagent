import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "@playwright/test";

const root = fileURLToPath(new URL("..", import.meta.url));

export default defineConfig({
  testDir: path.join(root, "tests/demo"),
  testMatch: "*.demo.ts",
  outputDir: path.join(root, "artifacts/current-demo/test-results"),
  workers: 1,
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  timeout: 120_000,
  reporter: "list",
  use: { baseURL: "http://127.0.0.1:4198", browserName: "chromium" },
  webServer: {
    cwd: root,
    command: "pnpm dev:renderer --host 127.0.0.1 --port 4198 --strictPort",
    url: "http://127.0.0.1:4198/#/agents",
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
