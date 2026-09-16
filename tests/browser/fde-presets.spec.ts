import { expect, test } from "@playwright/test";
import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";

for (const viewport of [
  { width: 1440, height: 900 },
  { width: 1024, height: 768 },
]) {
  test(`FDE preset browsing stays bounded at ${viewport.width}`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize(viewport);
    const health = monitorPageHealth(page);
    await openRendererPage(page, "/prompts");
    await page.getByRole("tab", { name: /FDE 预设/u }).click();
    const content = page.getByRole("textbox", { name: "预设内容" });
    await expect(content).toHaveValue(/前线部署工程师/u);
    await expect(content).toHaveAttribute("readonly", "");
    await expect(
      page.getByRole("button", { name: "使用此预设" }),
    ).toBeDisabled();
    await page
      .getByRole("combobox", { name: "预设领域" })
      .selectOption("public");
    await page.getByRole("searchbox", { name: "搜索 FDE 预设" }).fill("环保");
    await expect(
      page.getByRole("heading", { name: "FDE · 环保监测与验收材料" }),
    ).toBeVisible();
    await expect(content).toHaveValue(/不补造浓度/u);
    await expect
      .poll(async () =>
        content.evaluate((element) => {
          const box = element.getBoundingClientRect();
          const preview = element
            .closest('section[aria-label="FDE 预设预览"]')
            ?.getBoundingClientRect();
          return Boolean(
            preview &&
              box.height >= 160 &&
              box.top >= preview.top &&
              box.bottom <= preview.bottom + 1 &&
              box.right <= preview.right + 1,
          );
        }),
      )
      .toBe(true);
    await expectNoHorizontalOverflow(page);
    await page.screenshot({
      path: testInfo.outputPath(`fde-presets-${viewport.width}.png`),
      fullPage: true,
    });
    await page
      .getByRole("searchbox", { name: "搜索 FDE 预设" })
      .fill("absent-scenario");
    await expect(page.getByText("没有匹配的 FDE 预设")).toBeVisible();
    await page.getByRole("button", { name: "清空筛选" }).click();
    await expect(page.getByText("领域预设 · 30")).toBeVisible();
    await page.getByRole("tab", { name: "我的提示词" }).click();
    await expect(
      page.getByText("请使用 FyAgent 桌面应用", { exact: true }),
    ).toBeVisible();
    await expect(page.getByRole("textbox", { name: "预设内容" })).toHaveCount(
      0,
    );
    await expectHealthyPage(page, health);
  });
}
