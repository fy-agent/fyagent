import { expect, test, type Page } from "@playwright/test";
import {
  expectHealthyPage,
  monitorPageHealth,
  openRendererPage,
} from "./support";
import { installRichTauriFeatureFixture } from "./support/features";

async function login(page: Page) {
  await installRichTauriFeatureFixture(page);
  await openRendererPage(page, "/auth");
  await page.getByRole("button", { name: "添加账号", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveAttribute(
    "data-motion-settled",
    "true",
  );
}

async function stepFrames(page: Page, label: string) {
  return page.evaluate(async (label) => {
    const root = document.querySelector<HTMLElement>(".fy-control-dialog")!;
    const button = Array.from(root.querySelectorAll("button")).find(
      (node) => node.textContent?.trim() === label,
    );
    if (!button) throw new Error(`Missing step ${label}`);
    const before = root.getBoundingClientRect().height;
    const frames: Array<{
      height: number;
      contentTransform: string;
      footerInside: boolean;
    }> = [];
    const start = performance.now();
    let seen = false;
    button.click();
    await new Promise<void>((resolve, reject) => {
      const sample = () => {
        seen ||= root.dataset.contentMotion === "resizing";
        const bounds = root.getBoundingClientRect();
        const footer = root
          .querySelector(".fy-control-dialog-actions")!
          .getBoundingClientRect();
        frames.push({
          height: bounds.height,
          contentTransform: getComputedStyle(
            root.querySelector(".fy-dialog-foreground")!,
          ).transform,
          footerInside:
            footer.top >= bounds.top - 1 && footer.bottom <= bounds.bottom + 1,
        });
        if (seen && !root.dataset.contentMotion) resolve();
        else if (performance.now() - start > 1600)
          reject(new Error("Step never animated and settled"));
        else requestAnimationFrame(sample);
      };
      requestAnimationFrame(sample);
    });
    return { before, after: root.getBoundingClientRect().height, frames };
  }, label);
}

test("login steps resize one real dialog through intermediate frames without scaling forms", async ({
  page,
}, info) => {
  const health = monitorPageHealth(page);
  await login(page);
  await page.getByRole("dialog").evaluate((node) => {
    (node as HTMLElement).dataset.testSession = "original";
  });
  for (const label of ["下一步", "上一步", "下一步"]) {
    const result = await stepFrames(page, label);
    await info.attach(`step-${label}-${result.before}`, {
      body: JSON.stringify(result),
      contentType: "application/json",
    });
    const low = Math.min(result.before, result.after);
    const high = Math.max(result.before, result.after);
    expect(high - low).toBeGreaterThan(40);
    expect(
      result.frames.filter(
        (frame) => frame.height > low + 2 && frame.height < high - 2,
      ).length,
    ).toBeGreaterThan(2);
    expect(
      result.frames.every(
        (frame) => frame.contentTransform === "none" && frame.footerInside,
      ),
    ).toBe(true);
    await expect(page.getByRole("dialog")).toHaveAttribute(
      "data-test-session",
      "original",
    );
    if (label === "下一步")
      await page
        .getByRole("radio", { name: "连接 Codex", exact: true })
        .check();
  }
  await expect(
    page.getByRole("radio", { name: "连接 Codex", exact: true }),
  ).toBeChecked();
  await expectHealthyPage(page, health);
});

test("a resizing step can close, reopen, resize and adopt reduced motion without a stuck modal", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await login(page);
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "下一步", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toHaveAttribute(
    "data-content-motion",
    "resizing",
  );
  await page.keyboard.press("Escape");
  await expect(page.locator(".fy-control-dialog-actions")).toHaveCount(0);
  await expect(page.locator(".fy-control-dialog")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "添加账号", exact: true }),
  ).toBeFocused();
  await page.getByRole("button", { name: "添加账号", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveAttribute(
    "data-motion-settled",
    "true",
  );
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "下一步", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toHaveAttribute(
    "data-content-motion",
    "resizing",
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.getByRole("dialog")).toHaveAttribute(
    "data-motion-settled",
    "true",
  );
  await expect(page.getByRole("dialog")).not.toHaveAttribute(
    "data-content-motion",
    "resizing",
  );
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "上一步" })
    .click();
  await page.setViewportSize({ width: 760, height: 600 });
  await page.keyboard.press("Escape");
  await expect(page.locator(".fy-control-dialog-overlay")).toHaveCount(0);
  expect(await page.evaluate(() => document.body.style.pointerEvents)).not.toBe(
    "none",
  );
  await expectHealthyPage(page, health);
});

test("revisiting a page keeps its lens size while real tab changes still interpolate", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await installRichTauriFeatureFixture(page);
  await openRendererPage(page, "/auth");
  const lens = page
    .locator(
      '[data-testid="auth-page"] .fy-feature-tabs [data-testid="selection-lens"]',
    )
    .first();
  await expect(lens).toBeVisible();
  const before = (await lens.boundingBox())!;
  await page
    .getByRole("navigation", { name: "主导航" })
    .getByRole("link", { name: "AI软件配置", exact: true })
    .click();
  await page
    .getByRole("navigation", { name: "主导航" })
    .getByRole("link", { name: "账号与认证", exact: true })
    .click();
  const widths = await lens.evaluate(async (node) => {
    const widths: number[] = [];
    // Router commits asynchronously. Start at the first visible frame, not
    // the hidden previous route's zero-layout box. Do not wait for settling.
    let attempts = 0;
    while (node.closest("[hidden]")) {
      if (++attempts > 120) throw new Error("Auth route did not become active");
      await new Promise(requestAnimationFrame);
    }
    for (let i = 0; i < 24; i++) {
      await new Promise(requestAnimationFrame);
      widths.push(node.getBoundingClientRect().width);
    }
    return widths;
  });
  expect(Math.min(...widths)).toBeGreaterThan(before.width * 0.97);
  const positions = await page.evaluate(async () => {
    const scope = document.querySelector('[data-testid="auth-page"]')!;
    const tabs = scope.querySelectorAll<HTMLButtonElement>(".fy-feature-tab");
    const lens = scope.querySelector('[data-testid="selection-lens"]')!;
    const positions = [lens.getBoundingClientRect().left];
    tabs[1].focus();
    for (let i = 0; i < 24; i++) {
      await new Promise(requestAnimationFrame);
      positions.push(lens.getBoundingClientRect().left);
    }
    return positions;
  });
  await expect(
    page.getByTestId("auth-page").getByRole("tab", { name: /软件连接/ }),
  ).toHaveAttribute("aria-selected", "true");
  expect(
    new Set(positions.map((value) => Math.round(value))).size,
  ).toBeGreaterThan(3);
  await expectHealthyPage(page, health);
});

test("explicit steps remain continuous without ResizeObserver and reverse from the intermediate size", async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, "ResizeObserver", {
      configurable: true,
      value: undefined,
    });
  });
  const health = monitorPageHealth(page);
  await login(page);
  const result = await page.evaluate(async () => {
    const root = document.querySelector<HTMLElement>(".fy-control-dialog")!;
    const original = root.getBoundingClientRect().height;
    const next = Array.from(root.querySelectorAll("button")).find(
      (node) => node.textContent?.trim() === "下一步",
    )!;
    next.click();
    let animation: Animation | undefined;
    for (let i = 0; i < 10 && !animation; i++) {
      await new Promise(requestAnimationFrame);
      animation = root
        .getAnimations()
        .find((item) =>
          (item.effect as KeyframeEffect | null)
            ?.getKeyframes()
            .some((frame) => frame.height),
        );
    }
    if (!animation)
      throw new Error("Step size animation missing without ResizeObserver");
    animation.pause();
    animation.currentTime = 85;
    const intermediate = root.getBoundingClientRect().height;
    const back = Array.from(root.querySelectorAll("button")).find(
      (node) => node.textContent?.trim() === "上一步",
    )!;
    back.click();
    await new Promise(requestAnimationFrame);
    const reverse = root
      .getAnimations()
      .find(
        (item) =>
          item !== animation &&
          (item.effect as KeyframeEffect | null)
            ?.getKeyframes()
            .some((frame) => frame.height),
      );
    if (!reverse) throw new Error("Reverse resize missing");
    const frames = (reverse.effect as KeyframeEffect).getKeyframes();
    return {
      original,
      intermediate,
      reverseFrom: parseFloat(String(frames[0].height)),
      reverseTo: parseFloat(String(frames[1].height)),
    };
  });
  expect(result.intermediate).toBeGreaterThan(result.original + 15);
  expect(Math.abs(result.reverseFrom - result.intermediate)).toBeLessThan(1);
  expect(Math.abs(result.reverseTo - result.original)).toBeLessThan(1);
  await expect(page.getByRole("dialog")).toHaveAttribute(
    "data-motion-settled",
    "true",
  );
  await page.keyboard.press("Escape");
  await expect(page.locator(".fy-control-dialog")).toHaveCount(0);
  await expectHealthyPage(page, health);
});

test("model ID disclosure reuses the shared collapse and closes from the populated height", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await openRendererPage(page, "/models?target=workbuddy");
  const scope = page.getByTestId("workbuddy-model-ids");
  const panel = scope.locator(".fy-collapsible-panel").first();
  await scope.getByRole("button", { name: /当前已有的第三方模型 ID/ }).click();
  await expect(panel).toBeVisible();
  await expect
    .poll(() =>
      panel.evaluate((node) => getComputedStyle(node).height === "0px"),
    )
    .toBe(false);
  await expect
    .poll(() => panel.evaluate((node) => (node as HTMLElement).style.height))
    .toBe("auto");
  const heights = await scope.evaluate(async (scope) => {
    const panel = scope.querySelector<HTMLElement>(".fy-collapsible-panel")!;
    const toggle = scope.querySelector<HTMLButtonElement>(
      ".fy-models-existing-toggle",
    )!;
    const heights = [panel.getBoundingClientRect().height];
    toggle.click();
    for (let i = 0; i < 30; i++) {
      await new Promise(requestAnimationFrame);
      heights.push(panel.getBoundingClientRect().height);
    }
    return heights;
  });
  expect(heights[0]).toBeGreaterThan(25);
  expect(
    heights.filter((height) => height > 2 && height < heights[0] - 2).length,
  ).toBeGreaterThan(2);
  await expect(panel).toHaveAttribute("inert", "");
  await expect(
    scope.getByRole("button", { name: /当前已有的第三方模型 ID/ }),
  ).toHaveAttribute("aria-expanded", "false");
});
