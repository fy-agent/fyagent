import { expect, test } from "@playwright/test";

import {
  expectHealthyPage,
  monitorPageHealth,
  openRendererPage,
} from "./support";
import { installRichTauriFeatureFixture } from "./support/features";

test("keeps the selection frame on the current link after real navigation clicks", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  const health = monitorPageHealth(page);
  await installRichTauriFeatureFixture(page);
  await openRendererPage(page, "/agents");

  const navigation = page.getByRole("navigation", { name: "主导航" });
  const track = navigation.locator(".fy-side-navigation-track");
  const destinations = [
    ["/projects", "客户项目"],
    ["/agents", "AI软件配置"],
    ["/health", "运行状态"],
    ["/auth", "账号与认证"],
    ["/models", "模型管理"],
    ["/skills", "Skills 管理"],
    ["/mcp", "MCP 管理"],
    ["/prompts", "提示词管理"],
    ["/memory", "记忆模块"],
    ["/agents", "AI软件配置"],
    ["/projects", "客户项目"],
  ] as const;

  for (const [path, label] of destinations) {
    await test.step(label, async () => {
      await navigation.getByRole("link", { name: label, exact: true }).click();
      await expect(page).toHaveURL(new RegExp(`#${path}$`));
      const currentLink = navigation.locator('a[aria-current="page"]');
      await expect(currentLink).toHaveCount(1);
      await expect(currentLink).toHaveText(label);
      await expect(navigation.getByTestId("selection-lens")).toHaveCount(1);

      // Keep one document alive: fresh route loads only test initial placement.
      await expect
        .poll(
          () =>
            track.evaluate((element) => {
              const selected = element.querySelector('[aria-current="page"]');
              const lens = element.querySelector(":scope > .fy-selection-lens");
              if (!selected || !lens) return Infinity;
              const target = selected.getBoundingClientRect();
              const frame = lens.getBoundingClientRect();
              // SideNavigation's shared frame is inset by 1px on every edge.
              return Math.max(
                Math.abs(target.left + 1 - frame.left),
                Math.abs(target.top + 1 - frame.top),
                Math.abs(target.width - 2 - frame.width),
                Math.abs(target.height - 2 - frame.height),
              );
            }),
          { message: `The selection frame must align with current ${label}` },
        )
        .toBeLessThan(1.5);
    });
  }

  await expectHealthyPage(page, health);
});
