import fs from "node:fs";
import path from "node:path";
import { expect, test } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";
import {
  sampleControlBoundaryContrast,
  sampleFilledControlContrast,
  sampleTextContrast,
} from "./support/visual";

test("restores appearance before content and preserves pages and drafts across real theme toggles", async ({
  page,
}, info) => {
  const health = monitorPageHealth(page);
  await page.addInitScript(() => localStorage.setItem("fyagent-theme", "dark"));
  await installRichTauriFeatureFixture(page);
  await openRendererPage(page, "/models?target=codex");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  const input = page
    .getByRole("region", { name: "Codex 模型配置" })
    .getByRole("textbox", { name: "配置名称", exact: true });
  await input.fill("theme-draft-fixture");
  const theme = page.getByRole("button", { name: "切换为清亮蓝色" });
  await expect(theme).toHaveCSS("border-radius", "50%");
  await expect(theme.locator("svg")).toHaveCSS("width", "20px");
  await theme.click({ position: { x: 10, y: 12 } });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(page.locator("html")).not.toHaveAttribute(
    "data-theme-reveal",
    "active",
  );
  await expect(input).toHaveValue("theme-draft-fixture");
  expect(await page.evaluate(() => localStorage.getItem("fyagent-theme"))).toBe(
    "light",
  );
  await page.getByRole("button", { name: "切换为雾蓝暗色" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.locator("html")).not.toHaveAttribute(
    "data-theme-reveal",
    "active",
  );
  await expect(input).toHaveValue("theme-draft-fixture");
  await info.attach("dark-models", {
    body: await page.screenshot(),
    contentType: "image/png",
  });
  await expectHealthyPage(page, health);
});

test("dark blue text and controls remain readable on actual composited page and dialog surfaces", async ({
  page,
}, info) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.addInitScript(() => localStorage.setItem("fyagent-theme", "dark"));
  await installRichTauriFeatureFixture(page);
  for (const route of [
    "agents",
    "health",
    "auth",
    "models?target=codex",
    "skills",
    "mcp",
    "prompts",
    "memory",
  ]) {
    await openRendererPage(page, `/${route}`);
    // Retain contrast coverage with allocated scrollbars, not only macOS's
    // overlay-scrollbar presentation. The product must work with both.
    await page.addStyleTag({
      content: "::-webkit-scrollbar { width: 15px; height: 15px; }",
    });
    const scope = `[data-testid="${route.split("?")[0]}-page"]`;
    const routeScope = page.locator(scope);
    await expect(routeScope).toBeVisible();
    if (route === "agents") {
      // Scan completion removes the progress block and commits the card order.
      // Capture glyph coordinates only after those geometry changes settle.
      await expect(
        routeScope.getByRole("button", { name: "重新扫描", exact: true }),
      ).toBeEnabled();
      await expect(routeScope.getByRole("progressbar")).toHaveCount(0);
    }
    if (route === "health") {
      // Raster sampling records glyph geometry before it hides text and takes
      // the screenshot. Let the initial read and its conditional stop action
      // settle first, otherwise the primary action can move into stale points.
      await expect(routeScope.locator(".fy-health-status")).toHaveText(
        "本机检查正常",
      );
      await expect(
        routeScope.getByRole("button", { name: "检查全部软件" }),
      ).toBeEnabled();
      await expect(
        routeScope.getByRole("button", { name: "停止检查" }),
      ).toHaveCount(0);
    }
    const samples = await sampleTextContrast(page, scope);
    expect(samples.length).toBeGreaterThan(3);
    await info.attach(`dark-${route.split("?")[0]}`, {
      body: JSON.stringify(samples),
      contentType: "application/json",
    });
    if (route === "auth") {
      await page.screenshot({
        path: "node_modules/.cache/fyagent-round6/dark-auth.png",
      });
      const geometry = await page.evaluate(() =>
        Array.from(
          document.querySelectorAll(".fy-auth-list-item-copy strong"),
        ).map((element) => {
          const range = document.createRange();
          range.selectNodeContents(element);
          return {
            text: element.textContent,
            textRect: range.getBoundingClientRect().toJSON(),
            clippedRect: element.getBoundingClientRect().toJSON(),
            overflow: getComputedStyle(element).overflow,
          };
        }),
      );
      await info.attach("clipped-glyph-geometry", {
        body: JSON.stringify(geometry),
        contentType: "application/json",
      });
    }
    expect
      .soft(
        samples.filter((sample) => sample.ratio < 4.5),
        route,
      )
      .toEqual([]);
    await expectNoHorizontalOverflow(page);
  }
  await openRendererPage(page, "/auth");
  await page
    .getByRole("button", { name: "添加账号", exact: true })
    .first()
    .click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toHaveAttribute("data-motion-settled", "true");
  const samples = await sampleTextContrast(page, ".fy-control-dialog");
  expect(samples.length).toBeGreaterThan(3);
  expect.soft(samples.filter((sample) => sample.ratio < 4.5)).toEqual([]);
  const boundaries = await sampleControlBoundaryContrast(
    page,
    ".fy-control-dialog .fy-control-button:not(.fy-control-button-primary)",
  );
  const fills = await sampleFilledControlContrast(
    page,
    ".fy-control-dialog .fy-control-button-primary",
  );
  expect(fills.length).toBeGreaterThan(0);
  expect.soft(fills.filter((sample) => sample.ratio < 3)).toEqual([]);
  expect.soft(boundaries.filter((sample) => sample.ratio < 3)).toEqual([]);
  await info.attach("dark-dialog", {
    body: await page.screenshot(),
    contentType: "image/png",
  });
});

test("Agent directory cards preserve text contrast on the bright CI backing", async ({
  page,
}, info) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.addInitScript(() => localStorage.setItem("fyagent-theme", "dark"));
  await installRichTauriFeatureFixture(page);
  await openRendererPage(page, "/agents");
  await expect(
    page.getByRole("button", { name: "重新扫描", exact: true }),
  ).toBeEnabled();
  await expect(page.getByRole("progressbar")).toHaveCount(0);
  const directory = page.locator(".fy-agent-directory-list");
  await expect(
    directory.getByRole("heading", { name: "Grok Build", exact: true }),
  ).toBeVisible();
  // WebKit in CI run 34229637275 sampled this composited backing under
  // white Grok Build text (3.49:1). Exercise that brightness deterministically
  // without depending on the host's font metrics or compositor presentation.
  await directory.evaluate((element) => {
    (element as HTMLElement).style.backgroundColor = "rgb(111, 141, 164)";
  });
  const card = directory.locator('[data-agent-id="grokbuild"]');
  await card.scrollIntoViewIfNeeded();
  const samples = await sampleTextContrast(
    page,
    '.fy-agent-directory-card[data-agent-id="grokbuild"]',
  );
  expect(samples.length).toBeGreaterThan(3);
  expect(samples.some((sample) => sample.text === "Grok Build")).toBe(true);
  await info.attach("agent-directory-bright-backing", {
    body: JSON.stringify(samples),
    contentType: "application/json",
  });
  expect(samples.filter((sample) => sample.ratio < 4.5)).toEqual([]);
});

for (const theme of ["light", "dark"] as const) {
  test(`opened official source disclosures preserve text and badge contrast in ${theme} mode on bright CI backing`, async ({
    page,
  }, info) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.addInitScript(
      (currentTheme) => localStorage.setItem("fyagent-theme", currentTheme),
      theme,
    );
    await installRichTauriFeatureFixture(page);
    await openRendererPage(page, "/agents");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(
      page.getByRole("button", { name: "重新扫描", exact: true }),
    ).toBeEnabled();
    await expect(page.getByRole("progressbar")).toHaveCount(0);
    const directory = page.locator(".fy-agent-directory-list");
    await expect(
      directory.getByRole("heading", { name: "Grok Build", exact: true }),
    ).toBeVisible();

    await directory.evaluate((element) => {
      (element as HTMLElement).style.backgroundColor = "rgb(111, 141, 164)";
    });

    const grokCard = directory.locator('[data-agent-id="grokbuild"]');
    const codexCard = directory.locator('[data-agent-id="codex"]');

    await grokCard.getByText("官方资料与许可", { exact: true }).click();
    await expect(grokCard.locator(".fy-agent-source-links")).toBeVisible();

    await codexCard.getByText("官方资料与许可", { exact: true }).click();
    await expect(codexCard.locator(".fy-agent-source-links")).toBeVisible();

    await grokCard.scrollIntoViewIfNeeded();
    const grokSamples = await sampleTextContrast(
      page,
      '.fy-agent-directory-card[data-agent-id="grokbuild"] .fy-agent-source-links',
    );
    expect(grokSamples.length).toBeGreaterThan(3);

    await codexCard.scrollIntoViewIfNeeded();
    const codexSamples = await sampleTextContrast(
      page,
      '.fy-agent-directory-card[data-agent-id="codex"] .fy-agent-source-links',
    );
    expect(codexSamples.length).toBeGreaterThan(3);

    const allSamples = [...grokSamples, ...codexSamples];
    const badgesSampled = allSamples.map((sample) => sample.text);
    expect(badgesSampled.some((text) => text.includes("官方主页"))).toBe(true);
    expect(badgesSampled.some((text) => text.includes("官方文档"))).toBe(true);
    expect(badgesSampled.some((text) => text.includes("官方下载"))).toBe(true);
    expect(badgesSampled.some((text) => text.includes("开源许可"))).toBe(true);
    expect(badgesSampled.some((text) => text.includes("服务协议"))).toBe(true);

    await info.attach(`open-source-links-${theme}`, {
      body: JSON.stringify(allSamples),
      contentType: "application/json",
    });

    expect(allSamples.filter((sample) => sample.ratio < 4.5)).toEqual([]);

    const reviewDir = path.resolve(
      "artifacts/ui-open-source-review",
      info.project.name,
    );
    fs.mkdirSync(reviewDir, { recursive: true });
    await grokCard.scrollIntoViewIfNeeded();
    await page.screenshot({
      path: path.join(reviewDir, `open-source-links-${theme}.png`),
    });
  });
}

test("theme reveal has one real circular track and survives quick reversal and resize", async ({
  page,
}) => {
  await openRendererPage(page, "/agents");
  const trigger = page.locator(".fy-theme-toggle");
  await expect(trigger).toBeVisible();
  const supported = await page.evaluate(
    () => typeof document.startViewTransition === "function",
  );
  await trigger.evaluate((element) =>
    element.addEventListener(
      "click",
      (event) => {
        const click = event as MouseEvent;
        (element as HTMLElement).dataset.testPointer =
          `${click.clientX}px ${click.clientY}px`;
      },
      { once: true },
    ),
  );
  // Observe and interrupt the real track inside the browser. Separate protocol
  // polls can miss the entire 560 ms animation on a busy test host.
  const nativeReveal = supported
    ? page.evaluate(async () => {
        const deadline = performance.now() + 5_000;
        while (performance.now() < deadline) {
          const tracks = document
            .getAnimations()
            .filter(
              (animation) =>
                (animation.effect as KeyframeEffect | null)?.pseudoElement ===
                "::view-transition-new(root)",
            );
          const animation = tracks[0];
          if (animation) {
            const effect = animation.effect as KeyframeEffect;
            const observed = {
              trackCount: tracks.length,
              active: document.documentElement.dataset.themeReveal,
              frames: effect.getKeyframes().map((frame) => frame.clipPath),
              duration: effect.getTiming().duration,
              easing: effect.getTiming().easing,
            };
            const trigger =
              document.querySelector<HTMLButtonElement>(".fy-theme-toggle");
            if (!trigger) throw new Error("Missing theme trigger");
            trigger.click();
            return observed;
          }
          await new Promise<void>((resolve) =>
            requestAnimationFrame(() => resolve()),
          );
        }
        throw new Error("No native reveal track within 5 seconds");
      })
    : null;
  await trigger.click({ position: { x: 10, y: 12 } });
  if (nativeReveal) {
    const frames = await nativeReveal;
    expect(frames.trackCount).toBe(1);
    expect(frames.active).toBe("active");
    expect(frames.duration).toBe(560);
    expect(String(frames.easing).replace(/\s/g, "")).toBe(
      "cubic-bezier(0.25,0.08,0.25,1)",
    );
    expect(String(frames.frames[0])).toMatch(/^circle\(0% at /);
    const pointer = await trigger.getAttribute("data-test-pointer");
    if (pointer === null)
      throw new Error("Theme trigger did not record a pointer");
    const expected = await page.evaluate((pointerText) => {
      const [x, y] = pointerText
        .split(" ")
        .map((part) => Number.parseFloat(part));
      return {
        xPercent: (x / window.innerWidth) * 100,
        yPercent: (y / window.innerHeight) * 100,
      };
    }, pointer);
    const startClip = String(frames.frames[0]);
    const parsed = /^circle\(0% at ([\d.]+)% ([\d.]+)%\)/.exec(startClip);
    expect(parsed).toBeTruthy();
    expect(Number(parsed?.[1])).toBeCloseTo(expected.xPercent, 1);
    expect(Number(parsed?.[2])).toBeCloseTo(expected.yPercent, 1);
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await page.setViewportSize({ width: 1000, height: 700 });
  }
  await expect(page.locator("html")).not.toHaveAttribute(
    "data-theme-reveal",
    "active",
  );
  await expect(trigger).toBeEnabled();
  expect(await page.evaluate(() => document.body.style.pointerEvents)).not.toBe(
    "none",
  );
});

test("reduced motion, denied storage and unavailable transitions still switch appearance", async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(document, "startViewTransition", {
      configurable: true,
      value: undefined,
    });
    Storage.prototype.setItem = () => {
      throw new Error("blocked storage");
    };
  });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await openRendererPage(page, "/agents");
  await page.getByRole("button", { name: "切换为雾蓝暗色" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.locator("html")).not.toHaveAttribute(
    "data-theme-reveal",
    "active",
  );
});

test("live motion preference and external preference interrupt capture without stale writes", async ({
  page,
}) => {
  await openRendererPage(page, "/agents");
  const trigger = page.getByTestId("theme-toggle");
  await trigger.click();
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.locator("html")).not.toHaveAttribute(
    "data-theme-reveal",
    "active",
  );
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await trigger.click();
  await page.evaluate(() => {
    localStorage.setItem("fyagent-theme", "dark");
    window.dispatchEvent(
      new StorageEvent("storage", { key: "fyagent-theme", newValue: "dark" }),
    );
  });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.locator("html")).not.toHaveAttribute(
    "data-theme-reveal",
    "active",
  );
  expect(await page.evaluate(() => localStorage.getItem("fyagent-theme"))).toBe(
    "dark",
  );
  await expect(trigger).toBeEnabled();
});
