import { expect, test } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";

for (const cpuRate of [1, 4]) {
  test(`production theme reveal at ${cpuRate}x CPU cost`, async ({
    page,
  }, info) => {
    await installRichTauriFeatureFixture(page);
    const session = await page.context().newCDPSession(page);
    await session.send("Emulation.setCPUThrottlingRate", { rate: cpuRate });
    await session.send("Performance.enable");
    await page.goto("/#/auth");
    await expect(page.locator(".fy-theme-toggle")).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    const before = await session.send("Performance.getMetrics");
    const result = await page.evaluate(async () => {
      const trigger =
        document.querySelector<HTMLButtonElement>(".fy-theme-toggle");
      if (!trigger) throw new Error("Missing theme control");
      const cycles: Array<{
        prepare: number;
        total: number;
        segments: number[][];
      }> = [];
      for (let index = 0; index < 21; index++) {
        const start = performance.now();
        const segments: number[][] = [[], [], []];
        let prepare = 0;
        let previous = 0;
        let seen = false;
        trigger.click();
        await new Promise<void>((resolve, reject) => {
          const frame = (now: number) => {
            const animation = document
              .getAnimations()
              .find(
                (a) =>
                  (a.effect as KeyframeEffect | null)?.pseudoElement ===
                  "::view-transition-new(root)",
              );
            if (animation) {
              if (!seen) prepare = performance.now() - start;
              seen = true;
              const elapsed = Number(animation.currentTime);
              const duration = Number(animation.effect!.getTiming().duration);
              const segment = Math.min(2, Math.floor((elapsed / duration) * 3));
              if (previous) segments[segment].push(now - previous);
              previous = now;
            }
            if (
              seen &&
              !document.documentElement.hasAttribute("data-theme-reveal")
            )
              resolve();
            else if (performance.now() - start > 5000)
              reject(new Error("Theme reveal did not animate and settle"));
            else requestAnimationFrame(frame);
          };
          requestAnimationFrame(frame);
        });
        cycles.push({ prepare, total: performance.now() - start, segments });
        await new Promise<void>((resolve) =>
          requestAnimationFrame(() => resolve()),
        );
      }
      return cycles;
    });
    const p95 = (values: number[]) =>
      [...values].sort((a, b) => a - b)[Math.ceil(values.length * 0.95) - 1];
    const warm = result.slice(1);
    const report = {
      cpuRate,
      cold: result[0],
      cycles: warm.length,
      prepareP95: p95(warm.map((cycle) => cycle.prepare)),
      segmentsP95: [0, 1, 2].map((segment) =>
        p95(warm.flatMap((cycle) => cycle.segments[segment])),
      ),
      segmentSamples: [0, 1, 2].map(
        (segment) => warm.flatMap((cycle) => cycle.segments[segment]).length,
      ),
      cyclesData: warm,
      before: before.metrics,
      after: (await session.send("Performance.getMetrics")).metrics,
    };
    console.log("FYAGENT_THEME_PERF", JSON.stringify(report));
    await info.attach("theme-performance", {
      body: JSON.stringify(report),
      contentType: "application/json",
    });
    expect(warm).toHaveLength(20);
    report.segmentSamples.forEach((count) => expect(count).toBeGreaterThan(20));
    if (cpuRate === 1)
      report.segmentsP95.forEach((value) =>
        expect(Number(value.toFixed(6))).toBeLessThanOrEqual(33.4),
      );
    await expect(page.locator("html")).not.toHaveAttribute(
      "data-theme-reveal",
      "active",
    );
    await session.detach();
  });
}
