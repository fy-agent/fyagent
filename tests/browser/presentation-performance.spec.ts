import { expect, test } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";

test("production boots with unit-correct presentation timing", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await page.goto("/#/auth");
  const trigger = page
    .getByRole("button", { name: "添加账号", exact: true })
    .first();
  await expect(trigger).toBeVisible();
  const start = await trigger.evaluate((node) => {
    const now = performance.now();
    (node as HTMLButtonElement).click();
    return now;
  });
  const dialog = page.getByRole("dialog", { name: "添加官方账号" });
  const nativeTiming = await dialog.evaluate((node) =>
    node
      .getAnimations({ subtree: true })
      .filter((animation) => animation.effect instanceof KeyframeEffect)
      .map((animation) => {
        const timing = animation.effect!.getTiming();
        return Number(timing.duration) + (timing.delay ?? 0);
      }),
  );
  expect(Math.max(...nativeTiming)).toBeCloseTo(420, 0);
  await expect(dialog).toHaveAttribute("data-motion-settled", "true");
  expect(
    (await page.evaluate(() => performance.now())) - start,
  ).toBeGreaterThanOrEqual(400);
  const close = await dialog
    .getByRole("button", { name: "取消", exact: true })
    .evaluate((node) => {
      const now = performance.now();
      (node as HTMLButtonElement).click();
      return now;
    });
  await expect(dialog).toHaveCount(0);
  expect(
    (await page.evaluate(() => performance.now())) - close,
  ).toBeGreaterThanOrEqual(340);
});

// The same production harness measures cold entry separately from 20 warm
// cycles. This is browser composition evidence, not a native GPU benchmark.
for (const cpuRate of [1, 4]) {
  test(`production presentation at ${cpuRate}x CPU cost`, async ({
    page,
  }, info) => {
    await installRichTauriFeatureFixture(page);
    const session = await page.context().newCDPSession(page);
    await session.send("Emulation.setCPUThrottlingRate", { rate: cpuRate });
    await session.send("Performance.enable");
    await page.goto("/#/auth");
    await expect(
      page.getByRole("button", { name: "添加账号", exact: true }).first(),
    ).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    const metrics = await page.evaluate(async () => {
      const intervals: number[] = [];
      const cycles: number[] = [];
      const longTasks: number[] = [];
      let sampling = false;
      let previous = 0;
      let frameId: number;
      const observer = new PerformanceObserver((entries) => {
        entries.getEntries().forEach((entry) => longTasks.push(entry.duration));
      });
      observer.observe({ type: "longtask" });
      const frame = (time: number) => {
        if (sampling && previous) intervals.push(time - previous);
        previous = sampling ? time : 0;
        frameId = requestAnimationFrame(frame);
      };
      frameId = requestAnimationFrame(frame);
      const waitFor = (condition: () => boolean) =>
        new Promise<void>((resolve, reject) => {
          const start = performance.now();
          const check = () => {
            if (condition()) resolve();
            else if (performance.now() - start > 5000)
              reject(new Error("Presentation failed to settle"));
            else requestAnimationFrame(check);
          };
          check();
        });
      const trigger = Array.from(
        document.querySelectorAll<HTMLButtonElement>("button"),
      ).find((button) => button.textContent?.trim() === "添加账号");
      if (!trigger) throw new Error("Missing account trigger");
      try {
        for (let index = 0; index < 21; index += 1) {
          sampling = index > 0;
          const start = performance.now();
          trigger.click();
          await waitFor(() =>
            Boolean(
              document.querySelector(
                '.fy-control-dialog[data-motion-settled="true"]',
              ),
            ),
          );
          const cancel = Array.from(
            document.querySelectorAll<HTMLButtonElement>(
              ".fy-control-dialog button",
            ),
          ).find((button) => button.textContent?.trim() === "取消");
          if (!cancel) throw new Error("Missing cancel action");
          cancel.click();
          await waitFor(() => !document.querySelector(".fy-control-dialog"));
          cycles.push(performance.now() - start);
          sampling = false;
          previous = 0;
          await new Promise<void>((resolve) =>
            requestAnimationFrame(() => resolve()),
          );
        }
        return { intervals, cycles, longTasks };
      } finally {
        cancelAnimationFrame(frameId);
        observer.disconnect();
      }
    });
    const after = await session.send("Performance.getMetrics");
    const sorted = [...metrics.intervals].sort((left, right) => left - right);
    const result = {
      label: process.env.FYAGENT_PERF_LABEL ?? "current",
      cpuRate,
      viewport: page.viewportSize(),
      coldCycleMs: metrics.cycles[0],
      warmCycles: metrics.cycles.slice(1),
      frameCount: sorted.length,
      frameP95: sorted[Math.ceil(sorted.length * 0.95) - 1],
      longTasks: metrics.longTasks,
      rendererMetrics: after.metrics,
    };
    console.log("FYAGENT_PRESENTATION_PERF", JSON.stringify(result));
    await info.attach("presentation-metrics", {
      body: JSON.stringify(result),
      contentType: "application/json",
    });
    expect(result.warmCycles).toHaveLength(20);
    expect(result.frameCount).toBeGreaterThan(100);
    // Subtracting large rAF timestamps can yield 33.400000000001ms. Normalize
    // only sub-nanosecond floating-point noise, not the 33.4ms frame budget.
    if (cpuRate === 1)
      expect(Number(result.frameP95.toFixed(6))).toBeLessThanOrEqual(33.4);
    await expect(
      page.locator(
        ".fy-control-dialog, .fy-control-dialog-overlay, .fy-presentation-proxy",
      ),
    ).toHaveCount(0);
    expect(
      await page.evaluate(() => document.body.style.pointerEvents),
    ).not.toBe("none");
    await session.detach();
  });
}
