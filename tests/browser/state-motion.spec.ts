import { expect, test, type Page } from "@playwright/test";
import {
  expectHealthyPage,
  monitorPageHealth,
  openRendererPage,
} from "./support";
import { installRichTauriFeatureFixture } from "./support/features";

type CapturedDialog = HTMLElement & {
  testResizeAnimations: Animation[];
  restoreTestAnimate: () => void;
};

async function captureDialogResizes(page: Page) {
  await page.locator(".fy-control-dialog").evaluate((node) => {
    const root = node as CapturedDialog;
    const original = root.animate;
    root.testResizeAnimations = [];
    // Keep the real native keyframes. Capture synchronously at creation so a
    // slow rendering loop cannot finish a short animation before the probe.
    root.animate = function (...args) {
      const animation = original.apply(this, args);
      if (
        (animation.effect as KeyframeEffect)
          .getKeyframes()
          .some((frame) => frame.height !== undefined)
      ) {
        animation.pause();
        animation.currentTime = 0;
        root.testResizeAnimations.push(animation);
      }
      return animation;
    };
    root.restoreTestAnimate = () => {
      root.animate = original;
      for (const animation of root.testResizeAnimations) {
        if (animation.playState === "paused") animation.play();
      }
    };
  });
}

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
    const root = document.querySelector<CapturedDialog>(".fy-control-dialog")!;
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
    const count = root.testResizeAnimations.length;
    button.click();
    const deadline = performance.now() + 5000;
    while (root.testResizeAnimations.length === count) {
      if (performance.now() > deadline) throw new Error("Step never animated");
      await new Promise(requestAnimationFrame);
    }
    const animation = root.testResizeAnimations[count];
    await animation.ready;
    const duration = Number(animation.effect!.getTiming().duration);
    if (!Number.isFinite(duration) || duration <= 0)
      throw new Error("Invalid native resize duration");
    for (const fraction of [0, 0.05, 0.1, 0.15, 0.25, 0.4, 0.6, 0.8, 1]) {
      animation.currentTime = fraction * duration;
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
    }
    const after = root.getBoundingClientRect().height;
    animation.finish();
    await animation.finished;
    return { before, after, duration, frames };
  }, label);
}

test("login steps resize one real dialog through intermediate frames without scaling forms", async ({
  page,
}, info) => {
  const health = monitorPageHealth(page);
  await login(page);
  await captureDialogResizes(page);
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
    expect(result.duration).toBe(320);
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
    await expect(page.getByRole("dialog")).toHaveAttribute(
      "data-motion-settled",
      "true",
    );
    if (label === "下一步")
      await page
        .getByRole("radio", { name: "连接 Codex", exact: true })
        .check();
  }
  await expect(
    page.getByRole("radio", { name: "连接 Codex", exact: true }),
  ).toBeChecked();
  await page
    .locator(".fy-control-dialog")
    .evaluate((node) => (node as CapturedDialog).restoreTestAnimate());
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
  await page.clock.install();
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
  await page.clock.pauseAt(await page.evaluate(() => Date.now() + 1000));
  await page.evaluate(() => {
    const scope = document.querySelector('[data-testid="auth-page"]')!;
    const tabs = scope.querySelectorAll<HTMLButtonElement>(".fy-feature-tab");
    const lens = scope.querySelector('[data-testid="selection-lens"]')!;
    const positions = [lens.getBoundingClientRect().left];
    (window as Window & { testLensPositions?: number[] }).testLensPositions =
      positions;
    tabs[1].focus();
    const sample = () => {
      positions.push(lens.getBoundingClientRect().left);
      if (positions.length < 25) requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  });
  await page.clock.runFor(500);
  const positions = await page.evaluate(
    () =>
      (window as Window & { testLensPositions?: number[] }).testLensPositions!,
  );
  await page.clock.resume();
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
  await captureDialogResizes(page);
  const result = await page.evaluate(async () => {
    const root = document.querySelector<CapturedDialog>(".fy-control-dialog")!;
    const original = root.getBoundingClientRect().height;
    const next = Array.from(root.querySelectorAll("button")).find(
      (node) => node.textContent?.trim() === "下一步",
    )!;
    next.click();
    let animation: Animation | undefined;
    for (let i = 0; i < 10 && !animation; i++) {
      await new Promise(requestAnimationFrame);
      animation = root.testResizeAnimations[0];
    }
    if (!animation)
      throw new Error("Step size animation missing without ResizeObserver");
    animation.pause();
    await animation.ready;
    animation.currentTime = 85;
    const intermediate = root.getBoundingClientRect().height;
    const back = Array.from(root.querySelectorAll("button")).find(
      (node) => node.textContent?.trim() === "上一步",
    )!;
    back.click();
    const deadline = performance.now() + 5000;
    while (root.testResizeAnimations.length < 2) {
      if (performance.now() > deadline)
        throw new Error("Reverse resize missing");
      await new Promise(requestAnimationFrame);
    }
    const reverse = root.testResizeAnimations[1];
    if (!reverse) throw new Error("Reverse resize missing");
    const frames = (reverse.effect as KeyframeEffect).getKeyframes();
    root.restoreTestAnimate();
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
  await page.clock.install();
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
  await page.clock.pauseAt(await page.evaluate(() => Date.now() + 1000));
  await scope.evaluate((scope) => {
    const panel = scope.querySelector<HTMLElement>(".fy-collapsible-panel")!;
    const toggle = scope.querySelector<HTMLButtonElement>(
      ".fy-models-existing-toggle",
    )!;
    const heights = [panel.getBoundingClientRect().height];
    (
      window as Window & { testCollapseHeights?: number[] }
    ).testCollapseHeights = heights;
    toggle.click();
    const sample = () => {
      heights.push(panel.getBoundingClientRect().height);
      if (heights.length < 31) requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  });
  await page.clock.runFor(550);
  const heights = await page.evaluate(
    () =>
      (window as Window & { testCollapseHeights?: number[] })
        .testCollapseHeights!,
  );
  await page.clock.resume();
  expect(heights[0]).toBeGreaterThan(25);
  expect(
    heights.filter((height) => height > 2 && height < heights[0] - 2).length,
  ).toBeGreaterThan(2);
  await expect(panel).toHaveAttribute("inert", "");
  await expect(
    scope.getByRole("button", { name: /当前已有的第三方模型 ID/ }),
  ).toHaveAttribute("aria-expanded", "false");
});
