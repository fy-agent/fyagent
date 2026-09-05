import { expect, test } from "@playwright/test";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "./support/features";

test("prioritizes the initial module and never emits readiness from its loading shell", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  let release!: () => void;
  let intercepted = false;
  const moduleGate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/v2/pages/agents/Page.tsx", async (route) => {
    intercepted = true;
    await moduleGate;
    await route.continue();
  });
  try {
    await page.goto("/#/agents", { waitUntil: "domcontentloaded" });
    await expect.poll(() => intercepted).toBe(true);
    expect(
      (await featureFixtureCalls(page)).filter(
        (call) => call.command === "plugin:event|emit",
      ),
    ).toEqual([]);
    await expect(page.getByText("正在加载页面", { exact: true })).toHaveCount(
      0,
    );
    await expect(page.getByTestId("app-shell")).toHaveCount(0);
    release();
    await expect(page.locator(".fy-agent-directory-card")).toHaveCount(7);
    await expect
      .poll(
        async () =>
          (await featureFixtureCalls(page)).filter(
            (call) =>
              call.command === "plugin:event|emit" &&
              call.payload.event === "frontend-deeplink-ready",
          ).length,
      )
      .toBe(1);
    expect(
      await page
        .locator(".fy-agent-directory-card img")
        .evaluateAll((images) =>
          images.every((image) => (image as HTMLImageElement).naturalWidth > 0),
        ),
    ).toBe(true);
  } finally {
    release();
  }
});

test("optional module preload failure does not break the initial page or become unhandled", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  let aborted = false;
  await page.route("**/v2/pages/models/Page.tsx", async (route) => {
    aborted = true;
    await route.abort();
  });
  await page.goto("/#/agents", { waitUntil: "domcontentloaded" });
  await expect.poll(() => aborted).toBe(true);
  await expect(page.locator(".fy-agent-directory-card")).toHaveCount(7);
  await expect(page.getByRole("button", { name: "重新扫描" })).toBeEnabled();
  expect(errors).toEqual([]);
  await expect(
    page.getByRole("heading", { name: "页面暂时无法打开" }),
  ).toHaveCount(0);
});

test("a failed initial module shows a recoverable error rather than a permanent hidden window", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await page.route("**/v2/pages/agents/Page.tsx", (route) => route.abort());
  await page.goto("/#/agents", { waitUntil: "domcontentloaded" });
  await expect(
    page.getByRole("heading", { name: "页面暂时无法打开" }),
  ).toBeVisible();
  await expect(page.getByRole("alert")).toContainText(
    "界面加载未完成。请重新加载后重试。",
  );
  await expect(
    page.getByRole("button", { name: "重新加载界面" }),
  ).toBeVisible();
  await expect
    .poll(
      async () =>
        (await featureFixtureCalls(page)).filter(
          (call) =>
            call.command === "plugin:event|emit" &&
            call.payload.event === "frontend-deeplink-ready",
        ).length,
    )
    .toBe(1);
});
