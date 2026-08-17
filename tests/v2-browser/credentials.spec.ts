import { test } from "@playwright/test";

// Phase 2C lands the panel and vitest gate only. Models Page.tsx is
// intentionally not wired, so this spec stays a focused skeleton.
test.describe("credentials four viewports", () => {
  test.skip("list / status / candidate / impact are not mounted on Models Page yet", () => {
    // Viewport 1 list, 2 status card, 3 candidate plan, 4 impact confirmation.
  });
});
