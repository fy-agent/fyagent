import { expect, test } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";

for (const cpuRate of [1, 4]) {
  test(`production content resize at ${cpuRate}x CPU cost`, async ({
    page,
  }, info) => {
    await installRichTauriFeatureFixture(page);
    const session = await page.context().newCDPSession(page);
    await session.send("Emulation.setCPUThrottlingRate", { rate: cpuRate });
    await session.send("Performance.enable");
    await page.goto("/#/auth");
    await page.getByRole("button", { name: "添加账号", exact: true }).click();
    await expect(page.getByRole("dialog")).toHaveAttribute(
      "data-motion-settled",
      "true",
    );
    const before = await session.send("Performance.getMetrics");
    const results = await page.evaluate(async () => {
      const root = document.querySelector<HTMLElement>(".fy-control-dialog")!;
      const longTasks: number[] = [];
      const observer = new PerformanceObserver((list) =>
        longTasks.push(...list.getEntries().map((entry) => entry.duration)),
      );
      observer.observe({ type: "longtask", buffered: false });
      const cycles: Array<{
        elapsed: number;
        intervals: number[];
        heights: number[];
      }> = [];
      try {
        for (let cycle = 0; cycle < 21; cycle++) {
          const intervals: number[] = [];
          const heights: number[] = [];
          const started = performance.now();
          for (const label of ["下一步", "上一步"]) {
            const button = Array.from(
              root.querySelectorAll<HTMLButtonElement>(
                ".fy-control-dialog-actions button",
              ),
            ).find((node) => node.textContent?.trim() === label);
            if (!button) throw new Error(`Missing step action: ${label}`);
            let seen = false;
            let last = 0;
            const start = performance.now();
            button.click();
            await new Promise<void>((resolve, reject) => {
              const frame = (now: number) => {
                seen ||= root.dataset.contentMotion === "resizing";
                if (seen) {
                  if (last) intervals.push(now - last);
                  last = now;
                  heights.push(root.getBoundingClientRect().height);
                }
                if (seen && !root.dataset.contentMotion) resolve();
                else if (performance.now() - start > 2000)
                  reject(new Error("Content resize failed to start or settle"));
                else requestAnimationFrame(frame);
              };
              requestAnimationFrame(frame);
            });
          }
          cycles.push({
            elapsed: performance.now() - started,
            intervals,
            heights,
          });
        }
        return { cycles, longTasks };
      } finally {
        observer.disconnect();
      }
    });
    const p95 = (values: number[]) =>
      [...values].sort((a, b) => a - b)[Math.ceil(values.length * 0.95) - 1];
    const warm = results.cycles.slice(1);
    const intervals = warm.flatMap((cycle) => cycle.intervals);
    const report = {
      cpuRate,
      cold: results.cycles[0],
      warmCycles: warm.length,
      frameP95: p95(intervals),
      frameMax: Math.max(...intervals),
      samples: intervals.length,
      durationP95: p95(warm.map((cycle) => cycle.elapsed)),
      longTasks: results.longTasks,
      before: before.metrics,
      after: (await session.send("Performance.getMetrics")).metrics,
    };
    console.log("FYAGENT_STATE_PERF", JSON.stringify(report));
    await info.attach("state-performance", {
      body: JSON.stringify(report),
      contentType: "application/json",
    });
    expect(warm).toHaveLength(20);
    expect(intervals.length).toBeGreaterThan(300);
    expect(
      warm.every(
        (cycle) => Math.max(...cycle.heights) - Math.min(...cycle.heights) > 50,
      ),
    ).toBe(true);
    if (cpuRate === 1)
      expect(Number(report.frameP95.toFixed(6))).toBeLessThanOrEqual(33.4);
    await page.keyboard.press("Escape");
    await expect(page.locator(".fy-control-dialog")).toHaveCount(0);
    expect(
      await page.evaluate(() => document.body.style.pointerEvents),
    ).not.toBe("none");
    await session.detach();
  });
}
