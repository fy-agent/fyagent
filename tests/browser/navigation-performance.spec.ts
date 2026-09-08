import { expect, test } from "@playwright/test";
import { writeFile } from "node:fs/promises";
import { installRichTauriFeatureFixture } from "./support/features";

const routes = [
  "auth",
  "models",
  "skills",
  "mcp",
  "prompts",
  "memory",
  "agents",
];

test("production boots all seven primary routes without initialization errors", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await installRichTauriFeatureFixture(page);
  await page.goto("/#/agents");
  await expect(page.locator(".fy-agent-directory-card")).toHaveCount(7);
  for (const route of routes) {
    await page.locator(`.fy-side-navigation a[href="#/${route}"]`).click();
    await expect(page.getByTestId(`${route}-page`)).toBeVisible();
  }
  expect(errors).toEqual([]);
});

for (const cpuRate of [1, 4]) {
  test(`production navigation at ${cpuRate}x CPU cost`, async ({
    page,
  }, info) => {
    await installRichTauriFeatureFixture(page);
    const session = await page.context().newCDPSession(page);
    await session.send("Emulation.setCPUThrottlingRate", { rate: cpuRate });
    await page.goto("/#/agents");
    await expect(page.locator(".fy-agent-directory-card")).toHaveCount(7);
    await page.evaluate(() => document.fonts.ready);

    const samples: {
      route: string;
      visit: "first" | "return";
      frameMs: number;
      scriptMs: number;
      layoutMs: number;
    }[] = [];
    await session.send("Performance.enable");
    await session.send("Profiler.enable");
    await session.send("Profiler.start");
    await page.evaluate(() => {
      const scope = window as typeof window & { __perfLongTasks?: number[] };
      scope.__perfLongTasks = [];
      new PerformanceObserver((entries) => {
        for (const entry of entries.getEntries())
          scope.__perfLongTasks?.push(entry.duration);
      }).observe({ type: "longtask" });
    });
    for (let round = 0; round < 7; round += 1) {
      for (const route of routes) {
        const before = await session.send("Performance.getMetrics");
        // Measure from the synchronous activation of the same semantic NavLink
        // to the next frame after its destination's DOM is visible. Deliberately
        // excludes Playwright/OS input dispatch and does not wait for animation.
        const frameMs = await page.evaluate(async (target) => {
          const link = document.querySelector<HTMLAnchorElement>(
            `.fy-side-navigation a[href="#/${target}"]`,
          );
          if (!link) throw new Error(`Missing navigation link: ${target}`);
          const start = performance.now();
          link.click();
          return new Promise<number>((resolve, reject) => {
            const frame = () => {
              const page = document.querySelector<HTMLElement>(
                `[data-testid="${target}-page"]`,
              );
              const selected = link.getAttribute("aria-current") === "page";
              if (
                selected &&
                page &&
                page.getClientRects().length &&
                !page.closest("[hidden]")
              ) {
                requestAnimationFrame(() => resolve(performance.now() - start));
              } else if (performance.now() - start > 10_000) {
                reject(new Error(`Navigation did not settle: ${target}`));
              } else requestAnimationFrame(frame);
            };
            requestAnimationFrame(frame);
          });
        }, route);
        const after = await session.send("Performance.getMetrics");
        const delta = (name: string) =>
          1000 *
          ((after.metrics.find((metric) => metric.name === name)?.value ?? 0) -
            (before.metrics.find((metric) => metric.name === name)?.value ??
              0));
        samples.push({
          route,
          visit: round === 0 ? "first" : "return",
          frameMs,
          scriptMs: delta("ScriptDuration"),
          layoutMs: delta("LayoutDuration"),
        });
      }
    }
    const profile = await session.send("Profiler.stop");
    const profilePath = info.outputPath("navigation.cpuprofile");
    await writeFile(profilePath, JSON.stringify(profile.profile));
    await info.attach("navigation.cpuprofile", {
      path: profilePath,
      contentType: "application/json",
    });
    const returns = samples.filter((sample) => sample.visit === "return");
    const ordered = returns
      .map((sample) => sample.frameMs)
      .sort((a, b) => a - b);
    const result = {
      label: process.env.FYAGENT_PERF_LABEL ?? "current",
      cpuRate,
      viewport: page.viewportSize(),
      definition:
        "semantic NavLink activation to frame after visible destination; excludes OS/driver input latency and animation settling",
      count: returns.length,
      longTasks: await page.evaluate(
        () =>
          (window as typeof window & { __perfLongTasks?: number[] })
            .__perfLongTasks ?? [],
      ),
      p50: ordered[Math.ceil(ordered.length * 0.5) - 1],
      p95: ordered[Math.ceil(ordered.length * 0.95) - 1],
      samples,
    };
    console.log("FYAGENT_NAV_PERF", JSON.stringify(result));
    await info.attach("navigation-metrics", {
      body: JSON.stringify(result, null, 2),
      contentType: "application/json",
    });
    expect(returns).toHaveLength(42);
    if (cpuRate === 1) expect(result.p95).toBeLessThanOrEqual(100);
  });
}
