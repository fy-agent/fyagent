import os from "node:os";
import path from "node:path";
import { defineConfig, devices } from "@playwright/test";

const artifactRoot = path.join(os.tmpdir(), "fyagent-v2-playwright");

const viewports = [
  { name: "900x600", width: 900, height: 600 },
  { name: "1152x640", width: 1152, height: 640 },
  { name: "1232x700", width: 1232, height: 700 },
  { name: "1440x900", width: 1440, height: 900 },
] as const;

export default defineConfig({
  testDir: "./tests/v2-browser",
  outputDir: path.join(artifactRoot, "artifacts"),
  fullyParallel: true,
  forbidOnly: true,
  retries: 0,
  reporter: "list",
  timeout: 30_000,
  expect: {
    timeout: 5_000,
  },
  use: {
    baseURL: "http://127.0.0.1:4173",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    video: "off",
  },
  projects: viewports.map(({ name, width, height }) => ({
    name: `chromium-${name}`,
    use: {
      ...devices["Desktop Chrome"],
      channel:
        process.env.FYAGENT_PLAYWRIGHT_CHANNEL === "chrome"
          ? "chrome"
          : undefined,
      viewport: { width, height },
    },
  })),
  webServer: {
    command: "pnpm dev:renderer --host 127.0.0.1 --port 4173 --strictPort",
    url: "http://127.0.0.1:4173/#/models",
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
