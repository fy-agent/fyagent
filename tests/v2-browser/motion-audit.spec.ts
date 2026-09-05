import { expect, test, type Locator } from "@playwright/test";

import { expectHealthyPage, monitorPageHealth, openV2Page } from "./support";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "./support/features";

test("resize settles a moving dialog and hidden routes remove its portal immediately", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await openV2Page(page, "/auth");
  await page
    .getByRole("button", { name: "添加账号", exact: true })
    .first()
    .click();
  await page.setViewportSize({ width: 900, height: 600 });
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await expect(
    dialog.getByRole("button", { name: "下一步", exact: true }),
  ).toBeInViewport();
  const box = await dialog.boundingBox();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(900);
  // A native/deep-link route change is not an ordinary close animation.
  await page.evaluate(() => {
    location.hash = "#/agents";
  });
  await expect(page.getByTestId("agents-page")).toBeVisible();
  await expect(
    page.locator(".fy-control-dialog-overlay, .fy-control-dialog"),
  ).toHaveCount(0);
  expect(
    await page.evaluate(() => getComputedStyle(document.body).pointerEvents),
  ).not.toBe("none");
});

async function renderedWidth(control: Locator): Promise<number> {
  return control.evaluate((node) => node.getBoundingClientRect().width);
}

test("shared press feedback is bounded, leaves adjacent layout still and preserves native click semantics", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/__dev/ui-lab");
  const button = page.getByTestId("ui-lab-focus-target");
  const neighbour = page.getByTestId("ui-lab-tooltip-trigger");
  await expect(button).toBeVisible();
  const width = await renderedWidth(button);
  const neighbourBefore = await neighbour.boundingBox();
  const layoutWidth = await button.evaluate(
    (node) => (node as HTMLElement).offsetWidth,
  );
  await button.evaluate((node) => {
    node.setAttribute("data-test-clicks", "0");
    node.addEventListener("click", () => {
      node.setAttribute(
        "data-test-clicks",
        String(Number(node.getAttribute("data-test-clicks")) + 1),
      );
    });
  });
  await button.hover();
  await page.mouse.down();
  await expect
    .poll(async () => (await renderedWidth(button)) / width)
    .toBeLessThan(0.99);
  expect((await renderedWidth(button)) / width).toBeGreaterThanOrEqual(0.965);
  expect(
    await button.evaluate((node) => (node as HTMLElement).offsetWidth),
  ).toBe(layoutWidth);
  expect(await neighbour.boundingBox()).toEqual(neighbourBefore);
  await page.mouse.up();
  const scales = await button.evaluate(async (node, originalWidth) => {
    const samples: number[] = [];
    const start = performance.now();
    await new Promise<void>((resolve) => {
      const sample = () => {
        samples.push(node.getBoundingClientRect().width / originalWidth);
        if (performance.now() - start < 700) requestAnimationFrame(sample);
        else resolve();
      };
      requestAnimationFrame(sample);
    });
    return samples;
  }, width);
  expect(Math.max(...scales)).toBeLessThanOrEqual(1.005);
  await expect
    .poll(async () => Math.abs((await renderedWidth(button)) / width - 1))
    .toBeLessThan(0.001);
  await button.focus();
  await page.keyboard.press("Enter");
  await page.keyboard.press("Space");
  await expect(button).toHaveAttribute("data-test-clicks", "3");
  await button.click({ button: "right" });
  await expect(button).toHaveAttribute("data-test-clicks", "3");
  await expectHealthyPage(page, health);
});

test("modal material has a real origin, exit drops controls immediately and focus returns only after dismissal", async ({
  page,
}, info) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/auth");
  const trigger = page
    .getByRole("button", { name: "添加账号", exact: true })
    .first();
  await expect(trigger).toBeEnabled();
  const samplesPromise = page.evaluate(async () => {
    const samples: Array<{
      time: number;
      x: number;
      y: number;
      scaleX: number;
      scaleY: number;
    }> = [];
    const started = performance.now();
    await new Promise<void>((resolve) => {
      const sample = () => {
        const material = document.querySelector(".fy-dialog-material");
        if (material) {
          const matrix = new DOMMatrixReadOnly(
            getComputedStyle(material).transform,
          );
          samples.push({
            time: performance.now() - started,
            x: matrix.e,
            y: matrix.f,
            scaleX: matrix.a,
            scaleY: matrix.d,
          });
        }
        if (performance.now() - started < 1100) requestAnimationFrame(sample);
        else resolve();
      };
      requestAnimationFrame(sample);
    });
    return samples;
  });
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "添加官方账号" });
  await expect(dialog).toHaveAttribute("data-motion-origin", "trigger");
  const material = dialog.locator(".fy-dialog-material");
  await expect
    .poll(async () =>
      material.evaluate((node) => {
        const matrix = new DOMMatrixReadOnly(getComputedStyle(node).transform);
        return Math.max(
          Math.abs(matrix.a - 1),
          Math.abs(matrix.d - 1),
          Math.abs(matrix.e),
          Math.abs(matrix.f),
        );
      }),
    )
    .toBeLessThan(0.01);
  const samples = await samplesPromise;
  expect(
    samples.some(
      (sample) => Math.abs(sample.x) > 10 || Math.abs(sample.y) > 10,
    ),
  ).toBe(true);
  expect(
    samples.some((sample) => sample.scaleX < 0.9 && sample.scaleY < 0.9),
  ).toBe(true);
  await info.attach("modal-material-frames", {
    body: JSON.stringify(samples),
    contentType: "application/json",
  });
  // Stretching belongs to the material plane, not the text/form itself.
  expect(
    await dialog.locator(".fy-dialog-foreground").evaluate((node) => {
      const matrix = new DOMMatrixReadOnly(getComputedStyle(node).transform);
      return [matrix.a, matrix.d];
    }),
  ).toEqual([1, 1]);
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await expect(
    page.locator(
      '.fy-control-dialog[data-motion-phase="exit"] .fy-control-dialog-body',
    ),
  ).toHaveCount(0);
  await expect(
    page.locator('.fy-control-dialog[data-motion-phase="exit"] button'),
  ).toHaveCount(0);
  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
  await expectHealthyPage(page, health);
});

test("removing a modal source uses neutral exit and a live reduced-motion change settles the current animation", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/auth");
  await page.evaluate(() =>
    document.documentElement.style.setProperty(
      "--fy-motion-dialog-exit",
      "900ms",
    ),
  );
  const trigger = page
    .getByRole("button", { name: "添加账号", exact: true })
    .first();
  await trigger.evaluate((node) =>
    node.setAttribute("data-test-removable-origin", "true"),
  );
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "添加官方账号" });
  await expect(dialog).toHaveAttribute("data-motion-origin", "trigger");
  // Radix correctly removes outside controls from the accessibility tree.
  // This fault injection targets the original DOM node, not an accessible role.
  await page
    .locator('[data-test-removable-origin="true"]')
    .evaluate((node) => node.remove());
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await expect(dialog).toHaveAttribute("data-motion-origin", "neutral");
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(dialog).toHaveCount(0);
  expect(await page.evaluate(() => document.body.style.pointerEvents)).not.toBe(
    "none",
  );
  await expectHealthyPage(page, health);
});

test("reduced motion disables press travel without disabling native buttons", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await openV2Page(page, "/__dev/ui-lab");
  const button = page.getByTestId("ui-lab-focus-target");
  const before = await button.boundingBox();
  await button.hover();
  await page.mouse.down();
  expect(await button.boundingBox()).toEqual(before);
  await page.mouse.up();
  await expect(button).toBeEnabled();
  await expect(button).toBeFocused();
});

test("a conditional editor gets a fresh form on programmatic rapid reopen while its previous material exits", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/mcp");
  const trigger = page
    .getByRole("button", { name: "添加 MCP", exact: true })
    .first();
  await trigger.evaluate((node) =>
    node.setAttribute("data-test-reopen-source", "true"),
  );
  await trigger.click();
  const dialog = page.locator('.fy-control-dialog[data-motion-phase="open"]');
  await expect(dialog).toHaveAttribute("data-motion-origin", "trigger");
  await dialog
    .getByLabel("名称", { exact: true })
    .fill("discarded draft value");
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  const exiting = page.locator('.fy-control-dialog[data-motion-phase="exit"]');
  await expect(exiting).toHaveCount(1);
  await expect(exiting.locator("input, textarea, button")).toHaveCount(0);
  // Controller-level reopen: physical outside clicks stay blocked while
  // Radix still owns the old modal's decorative exit.
  await page
    .locator('[data-test-reopen-source="true"]')
    .evaluate((node) => (node as HTMLButtonElement).click());
  await expect(dialog.getByLabel("名称", { exact: true })).toHaveValue("");
  expect(
    await page
      .locator("input,textarea")
      .evaluateAll((elements) =>
        elements.some((element) =>
          (element as HTMLInputElement).value.includes("discarded draft value"),
        ),
      ),
  ).toBe(false);
  await expect(dialog).toHaveAttribute("data-motion-settled", "true");
  await expect(
    page.locator('.fy-control-dialog[data-motion-phase="exit"]'),
  ).toHaveCount(0);
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(
    (await featureFixtureCalls(page)).filter((call) =>
      /^(upsert|delete)_mcp/.test(call.command),
    ),
  ).toEqual([]);
  await expectHealthyPage(page, health);
});

test("resizing during modal entry settles real geometry without leaving a stuck exit", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/auth");
  const trigger = page
    .getByRole("button", { name: "添加账号", exact: true })
    .first();
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "添加官方账号" });
  await expect(dialog).toHaveAttribute("data-motion-origin", "trigger");
  const viewport = page.viewportSize()!;
  await page.setViewportSize({
    width: viewport.width - 64,
    height: viewport.height - 24,
  });
  await expect(dialog).toHaveAttribute("data-motion-settled", "true");
  const box = await dialog.boundingBox();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(viewport.width - 64);
  expect(box!.y + box!.height).toBeLessThanOrEqual(viewport.height - 24);
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
  await expectHealthyPage(page, health);
});

test("a dirty-page confirmation uses the actual sidebar source without changing cancel or navigation behavior", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");
  await page.evaluate(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = (command, payload) => {
      if (command === "read_workspace_file")
        return Promise.resolve("Saved fixture memory");
      if (command === "get_hermes_memory_limits")
        return Promise.resolve({
          memory: 2200,
          user: 1375,
          memoryEnabled: true,
          userEnabled: true,
        });
      return invoke(command, payload);
    };
  });
  const navigation = page.getByRole("navigation", { name: "主导航" });
  await navigation.getByRole("link", { name: "记忆模块", exact: true }).click();
  const editor = page.getByRole("textbox", { name: "记忆内容", exact: true });
  await expect(editor).toHaveValue("Saved fixture memory");
  await editor.fill("Unsaved fixture memory");
  const destination = navigation.getByRole("link", {
    name: "账号与认证",
    exact: true,
  });
  await destination.click();
  const dialog = page.getByRole("dialog", { name: "放弃未保存的更改？" });
  await expect(dialog).toHaveAttribute("data-motion-origin", "trigger");
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page).toHaveURL(/#\/memory$/);
  await expect(editor).toHaveValue("Unsaved fixture memory");
  await expect(destination).toBeFocused();
  await destination.click();
  await dialog.getByRole("button", { name: "确认", exact: true }).click();
  await expect(page).toHaveURL(/#\/auth$/);
  await expect(
    page.locator(".fy-control-dialog, .fy-control-dialog-overlay"),
  ).toHaveCount(0);
  expect(
    (await featureFixtureCalls(page)).filter((call) =>
      ["write_workspace_file", "set_hermes_memory"].includes(call.command),
    ),
  ).toEqual([]);
  await expectHealthyPage(page, health);
});

test.describe("touch feedback", () => {
  test.use({ hasTouch: true });
  test("a primary touch compresses the same control and produces one native click", async ({
    page,
  }) => {
    await openV2Page(page, "/__dev/ui-lab");
    const button = page.getByTestId("ui-lab-focus-target");
    const box = await button.boundingBox();
    expect(box).not.toBeNull();
    await button.evaluate((node) => {
      node.setAttribute("data-test-touch-clicks", "0");
      node.addEventListener("click", () =>
        node.setAttribute(
          "data-test-touch-clicks",
          String(Number(node.getAttribute("data-test-touch-clicks")) + 1),
        ),
      );
    });
    const cdp = await page.context().newCDPSession(page);
    try {
      await cdp.send("Input.dispatchTouchEvent", {
        type: "touchStart",
        touchPoints: [
          { x: box!.x + box!.width / 2, y: box!.y + box!.height / 2, id: 1 },
        ],
      });
      await expect
        .poll(async () => (await renderedWidth(button)) / box!.width)
        .toBeLessThan(0.99);
      await cdp.send("Input.dispatchTouchEvent", {
        type: "touchEnd",
        touchPoints: [],
      });
      await expect(button).toHaveAttribute("data-test-touch-clicks", "1");
      await expect
        .poll(async () =>
          Math.abs((await renderedWidth(button)) / box!.width - 1),
        )
        .toBeLessThan(0.001);
    } finally {
      await cdp.detach();
    }
  });
});
