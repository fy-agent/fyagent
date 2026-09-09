import { expect, test, type Page } from "@playwright/test";
import {
  expectHealthyPage,
  monitorPageHealth,
  openRendererPage,
} from "./support";
import { installRichTauriFeatureFixture } from "./support/features";

async function startSampling(page: Page, selector: string) {
  await page.evaluate((selector) => {
    const element = document.querySelector<HTMLElement>(selector)!;
    const neighbour = document.querySelector<HTMLElement>(
      '[data-testid="ui-lab-tooltip-trigger"]',
    )!;
    const before = neighbour.getBoundingClientRect();
    // offsetWidth rounds to integer CSS pixels; WebKit's fractional text
    // metrics can exceed the entire bounded rebound. Compare actual geometry.
    const width = element.getBoundingClientRect().width;
    element.dataset.testClicks = "0";
    element.addEventListener("click", () => {
      element.dataset.testClicks = String(
        Number(element.dataset.testClicks) + 1,
      );
    });
    const probe = window as Window & {
      pressProbe?: Promise<{
        min: number;
        max: number;
        clicks: number;
        neighbourMoved: boolean;
      }>;
    };
    probe.pressProbe = new Promise((resolve) => {
      const start = performance.now();
      const values: number[] = [];
      let moved = false;
      const frame = () => {
        values.push(element.getBoundingClientRect().width / width);
        const bounds = neighbour.getBoundingClientRect();
        moved ||=
          before.x !== bounds.x ||
          before.y !== bounds.y ||
          before.width !== bounds.width ||
          before.height !== bounds.height;
        if (performance.now() - start < 950) requestAnimationFrame(frame);
        else
          resolve({
            min: Math.min(...values),
            max: Math.max(...values),
            clicks: Number(element.dataset.testClicks),
            neighbourMoved: moved,
          });
      };
      requestAnimationFrame(frame);
    });
  }, selector);
}

for (const method of ["pointer", "Enter", "Space"] as const) {
  test(`a fast ${method} activation has perceptible bounded rebound and one action`, async ({
    page,
  }, info) => {
    await page.clock.install();
    const health = monitorPageHealth(page);
    await openRendererPage(page, "/__dev/ui-lab");
    const button = page.getByTestId("ui-lab-focus-target");
    await button.focus();
    await page.clock.pauseAt(await page.evaluate(() => Date.now() + 1000));
    await startSampling(page, '[data-testid="ui-lab-focus-target"]');
    if (method === "pointer") await button.click({ delay: 12 });
    else await page.keyboard.press(method, { delay: 12 });
    // Functional spring geometry uses controlled RAF delivery, not the CI
    // runner's frame rate. Real-time performance remains in its production gate.
    await page.clock.runFor(1000);
    const result = (await page.evaluate(
      () => (window as Window & { pressProbe?: Promise<unknown> }).pressProbe,
    )) as { min: number; max: number; clicks: number; neighbourMoved: boolean };
    await page.clock.resume();
    await info.attach("quick-press", {
      body: JSON.stringify(result),
      contentType: "application/json",
    });
    expect(result.min).toBeLessThan(0.974);
    expect(result.min).toBeGreaterThanOrEqual(0.95);
    expect(result.max).toBeGreaterThan(1.002);
    expect(result.max).toBeLessThanOrEqual(1.005);
    expect(result.clicks).toBe(1);
    expect(result.neighbourMoved).toBe(false);
    await expectHealthyPage(page, health);
  });
}

test("positioned search/reveal controls animate their child without losing centering", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await openRendererPage(page, "/models?target=workbuddy");
  const control = page
    .getByRole("region", { name: "WorkBuddy 模型配置" })
    .getByRole("button", { name: "显示 API Key", exact: true });
  await expect(control).toBeVisible();
  await expect(control).toBeEnabled();
  await control.scrollIntoViewIfNeeded();
  await expect(control).toBeInViewport();
  const before = await control.boundingBox();
  const visual = control.locator(".fy-control-icon-feedback");
  const visualWidth = (await visual.boundingBox())!.width;
  await control.hover();
  await page.mouse.down();
  await expect
    .poll(async () => (await visual.boundingBox())!.width / visualWidth)
    .toBeLessThan(0.98);
  expect(await control.boundingBox()).toEqual(before);
  await page.mouse.up();
});
