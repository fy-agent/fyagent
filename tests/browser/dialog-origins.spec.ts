import { expect, test } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  expectHealthyPage,
  monitorPageHealth,
  openRendererPage,
} from "./support";

test("a chained MCP overwrite dialog opens from its confirmation and returns to catalog", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, payload) => {
      const result = await invoke(command, payload);
      if (command !== "get_mcp_servers") return result;
      return {
        ...(result as Record<string, unknown>),
        time: {
          id: "time",
          name: "Custom time",
          apps: { claude: true },
          server: { type: "stdio", command: "python", args: ["time.py"] },
        },
      };
    };
  });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/mcp");
  await page.getByRole("tab", { name: "发现", exact: true }).click();
  const card = page
    .locator("article")
    .filter({ has: page.getByRole("heading", { name: "Time", exact: true }) });
  const trigger = card.getByRole("button", { name: "重新配置", exact: true });
  await trigger.click();
  const confirm = page
    .getByRole("dialog")
    .getByRole("button", { name: "确认", exact: true });
  await confirm.click();
  const next = page
    .getByRole("dialog")
    .filter({ has: page.getByRole("button", { name: "下一步", exact: true }) });
  await expect(next).toHaveAttribute("data-motion-origin", "trigger");
  await expect(next).toHaveAttribute("data-motion-settled", "true");
  await next.getByRole("button", { name: "取消", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(trigger).toBeFocused();
  expect(
    await page.evaluate(() =>
      window.__FYAGENT_FEATURE_FIXTURE__.calls.filter(
        (call) => call.command === "upsert_mcp_server",
      ),
    ),
  ).toEqual([]);
  await expectHealthyPage(page, health);
});

declare global {
  interface Window {
    __dialogOriginTracks: Array<{ elapsed: number; outcome: string }>;
    __releaseRemovalPreview?: () => void;
  }
}

test("transient Skill menu dialogs return to the explicitly owned persistent trigger", async ({
  page,
}, info) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/skills");
  const trigger = page.getByRole("button", { name: "更多", exact: true });
  for (const label of ["Skill 设置", "备份恢复", "导入本地 Skill"]) {
    await trigger.click();
    const source = page.getByRole("button", { name: label, exact: true });
    await source.click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toHaveAttribute("data-motion-origin", "trigger");
    await expect(dialog).toHaveAttribute("data-motion-settled", "true");
    await expect(source).toHaveCount(0);
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveAttribute("data-motion-origin", "trigger");
    await expect(dialog)
      .toHaveCount(0)
      .catch(async (error) => {
        await info.attach("exit-tracks", {
          contentType: "application/json",
          body: JSON.stringify(
            await dialog.evaluate((root) => ({
              phase: (root as HTMLElement).dataset.motionPhase,
              settled: (root as HTMLElement).dataset.motionSettled,
              tracks: root
                .getAnimations({ subtree: true })
                .map((animation) => ({
                  state: animation.playState,
                  time: animation.currentTime,
                  duration: animation.effect?.getTiming().duration,
                  target: (
                    (animation.effect as KeyframeEffect).target as HTMLElement
                  )?.className,
                })),
            })),
          ),
        });
        throw error;
      });
    await expect(trigger).toBeFocused();
  }
  await expectHealthyPage(page, health);
});

for (const route of ["/agents?target=workbuddy&section=mcp", "/mcp"]) {
  test(`WorkBuddy trust preserves the actual asynchronous switch source at ${route}`, async ({
    page,
  }, info) => {
    await installRichTauriFeatureFixture(page);
    const health = monitorPageHealth(page);
    await openRendererPage(page, route);
    const source = page
      .getByRole("switch", {
        name: route === "/mcp" ? "WorkBuddy MCP 分配" : /^在 WorkBuddy 中使用 /,
      })
      .first();
    await expect(source).not.toBeChecked();
    const before = await source.boundingBox();
    const sourceHandle = await source.elementHandle();
    await source.click();
    const dialog = page.getByRole("dialog", {
      name: "需要在 WorkBuddy 中信任 MCP",
    });
    await expect(dialog)
      .toHaveAttribute("data-motion-origin", "trigger")
      .catch(async (error) => {
        await info.attach("async-source-geometry", {
          contentType: "application/json",
          body: JSON.stringify({
            before,
            after: await sourceHandle?.evaluate((node) => {
              const ancestors = [];
              for (
                let element: HTMLElement | null = node as HTMLElement;
                element;
                element = element.parentElement
              ) {
                const style = getComputedStyle(element);
                ancestors.push({
                  tag: element.tagName,
                  class: element.className,
                  box: element.getBoundingClientRect().toJSON(),
                  overflow: style.overflow,
                  opacity: style.opacity,
                  inert: element.inert,
                });
              }
              return ancestors;
            }),
          }),
        });
        throw error;
      });
    await expect(dialog).toHaveAttribute("data-motion-settled", "true");
    await dialog.getByRole("button", { name: "知道了" }).click();
    await expect(dialog).toHaveCount(0);
    await expect(source).toBeChecked();
    await expect(source).toBeFocused();
    await expectHealthyPage(page, health);
  });
}

test("a cancelled removal preview cannot overwrite the next removal session", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await page.addInitScript(() => {
    let requests = 0;
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, payload) => {
      const result = await invoke(command, payload);
      if (
        command !== "managed_auth_preview_account_removal" ||
        ++requests !== 1
      )
        return result;
      await new Promise<void>((resolve) => {
        window.__releaseRemovalPreview = resolve;
      });
      return { ...(result as Record<string, unknown>), canApply: false };
    };
  });
  await openRendererPage(page, "/auth");
  const source = page.getByRole("button", { name: "移除账号", exact: true });
  await source.click();
  await expect
    .poll(() => page.evaluate(() => Boolean(window.__releaseRemovalPreview)))
    .toBe(true);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await source.click();
  const confirm = page
    .getByRole("dialog")
    .getByRole("button", { name: "移除账号", exact: true });
  await expect(confirm).toBeEnabled();
  await page.evaluate(async () => {
    window.__releaseRemovalPreview?.();
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
  });
  await expect(confirm).toBeEnabled();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});

for (const { delay, failure } of [
  ...[0, 16, 80, 120, 252, 300, 600].map((delay) => ({
    delay,
    failure: false,
  })),
  { delay: 120, failure: true },
]) {
  test(`account removal preserves source entry with a ${delay}ms ${failure ? "failed" : "successful"} preview`, async ({
    page,
  }) => {
    await installRichTauriFeatureFixture(page);
    await page.addInitScript(
      ({ delay, failure }) => {
        const invoke = window.__TAURI_INTERNALS__.invoke;
        window.__TAURI_INTERNALS__.invoke = async (command, payload) => {
          const result = await invoke(command, payload);
          if (command === "managed_auth_preview_account_removal" && delay)
            await new Promise((resolve) => setTimeout(resolve, delay));
          if (command === "managed_auth_preview_account_removal" && failure)
            throw new Error("fixture preview unavailable");
          return result;
        };
        window.__dialogOriginTracks = [];
        const animate = Element.prototype.animate;
        Element.prototype.animate = function (frames, options) {
          const animation = animate.call(this, frames, options);
          if (
            this.classList.contains("fy-dialog-material") &&
            Array.isArray(frames) &&
            frames.some((frame) => "width" in frame)
          ) {
            const started = performance.now();
            const row = { elapsed: 0, outcome: "running" };
            window.__dialogOriginTracks.push(row);
            void animation.finished.then(
              () => {
                row.elapsed = performance.now() - started;
                row.outcome = "finished";
              },
              () => {
                row.elapsed = performance.now() - started;
                row.outcome = "cancelled";
              },
            );
          }
          return animation;
        };
      },
      { delay, failure },
    );
    const health = monitorPageHealth(page);
    await openRendererPage(page, "/auth");
    const source = page.getByRole("button", { name: "移除账号", exact: true });
    await source.click();
    const dialog = page.getByRole("dialog", { name: /^移除/ });
    await expect(dialog).toHaveAttribute("data-motion-origin", "trigger");
    await expect(dialog).toHaveAttribute("data-motion-settled", "true");
    const destructiveAction = dialog.getByRole("button", {
      name: "移除账号",
      exact: true,
    });
    if (failure) {
      await expect(dialog.locator(".fy-control-notice")).toBeVisible();
      await expect(destructiveAction).toBeDisabled();
    } else {
      await expect(destructiveAction).toBeEnabled();
    }
    await expect(dialog).toHaveAttribute("data-motion-settled", "true");
    const entry = await page.evaluate(() => window.__dialogOriginTracks[0]);
    expect(entry.outcome).toBe("finished");
    expect(entry.elapsed).toBeGreaterThan(350);
    await dialog.getByRole("button", { name: "取消", exact: true }).click();
    await expect(dialog).toHaveCount(0);
    await expect(source).toBeFocused();
    expect(
      await page.evaluate(() =>
        window.__FYAGENT_FEATURE_FIXTURE__.calls.filter(
          (call) => call.command === "managed_auth_remove_account",
        ),
      ),
    ).toEqual([]);
    await expectHealthyPage(page, health);
  });
}
