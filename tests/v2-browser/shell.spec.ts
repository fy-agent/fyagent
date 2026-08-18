import { expect, test, type Locator } from "@playwright/test";
import path from "node:path";
import { pathToFileURL } from "node:url";

import {
  boxesOverlap,
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openV2Page,
  requiredBox,
} from "./support";

const navigationContract = [
  { path: "/agents", label: "Agent 目录" },
  { path: "/models", label: "模型" },
  { path: "/skills", label: "Skills" },
  { path: "/mcp", label: "MCP" },
  { path: "/prompts", label: "提示词" },
  { path: "/memory", label: "记忆" },
] as const;

const visibleControlTestIds = [
  "search",
  "settings",
  "avatar",
  "window-minimize",
  "window-maximize",
  "window-close",
] as const;

const shellRegionTestIds = [
  "brand",
  "primary-navigation",
  "tool-cluster",
  "window-controls",
] as const;

const primaryControlTestIds = [
  "#/agents",
  "#/models",
  "#/skills",
  "#/mcp",
  "#/prompts",
  "#/memory",
  ...visibleControlTestIds,
] as const;

function routeLink(navigation: Locator, label: string): Locator {
  return navigation.getByRole("link", { name: label, exact: true });
}

function escapedRegularExpression(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

test("keeps the complete shell visible, separate, and overflow-free", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/models");

  await expectNoHorizontalOverflow(page);

  const topBar = page.getByTestId("top-bar");
  const topBarFits = await topBar.evaluate(
    (element) => element.scrollWidth <= element.clientWidth + 1,
  );
  expect(topBarFits, "TopBar must not overflow horizontally").toBe(true);

  const regionBoxes = new Map<
    string,
    Awaited<ReturnType<typeof requiredBox>>
  >();
  for (const testId of shellRegionTestIds) {
    regionBoxes.set(
      testId,
      await requiredBox(page.getByTestId(testId), testId),
    );
  }

  for (
    let firstIndex = 0;
    firstIndex < shellRegionTestIds.length;
    firstIndex += 1
  ) {
    for (
      let secondIndex = firstIndex + 1;
      secondIndex < shellRegionTestIds.length;
      secondIndex += 1
    ) {
      const firstId = shellRegionTestIds[firstIndex];
      const secondId = shellRegionTestIds[secondIndex];
      const firstBox = regionBoxes.get(firstId);
      const secondBox = regionBoxes.get(secondId);

      expect(firstBox).toBeDefined();
      expect(secondBox).toBeDefined();
      expect(
        boxesOverlap(firstBox!, secondBox!),
        `${firstId} must not overlap ${secondId}`,
      ).toBe(false);
    }
  }

  const navigation = page.getByRole("navigation", { name: "主导航" });
  for (const { label } of navigationContract) {
    await expect(routeLink(navigation, label)).toBeVisible();
  }
  for (const testId of visibleControlTestIds) {
    await expect(page.getByTestId(testId)).toBeVisible();
  }

  const contentViewport = page.getByTestId("content-viewport");
  const contentBox = await requiredBox(contentViewport, "content viewport");
  expect(contentBox.width).toBeGreaterThan(0);
  expect(contentBox.height).toBeGreaterThan(0);
  expect(
    await contentViewport.evaluate((element) => element.textContent?.trim()),
  ).toBe("");

  await expectHealthyPage(page, health);
});

test("keeps hash, selected link, and aria-current aligned for every route", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/models");

  const navigation = page.getByRole("navigation", { name: "主导航" });
  for (const { path, label } of navigationContract) {
    const link = routeLink(navigation, label);
    await link.click();

    await expect(page).toHaveURL(
      new RegExp(`${escapedRegularExpression(`#${path}`)}$`),
    );
    await expect(link).toHaveAttribute("aria-current", "page");
    const selectedLinks = navigation.locator('a[aria-current="page"]');
    await expect(selectedLinks).toHaveCount(1);
    await expect(selectedLinks).toHaveText(label);
    if (path === "/prompts") {
      await expect(page.getByTestId("prompts-page")).toBeVisible();
    } else if (path === "/memory") {
      await expect(page.getByTestId("memory-page")).toBeVisible();
    } else {
      await expect(page.getByTestId("content-viewport")).toHaveText("");
    }
  }

  await expectHealthyPage(page, health);
});

test("renders and exercises the cross-agent Memory workspace", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/memory");

  await expectNoHorizontalOverflow(page);
  await expect(
    page.getByRole("heading", { name: "记忆", exact: true }),
  ).toBeVisible();
  await expect(page.getByTestId("memory-library")).toBeVisible();
  await expect(page.getByTestId("memory-editor")).toBeVisible();
  await expect(page.getByTestId("memory-inspector")).toBeVisible();

  await page.getByRole("button", { name: /Claude Code · 长期记忆/ }).click();
  await page
    .getByRole("checkbox", {
      name: "同步到OpenClaw默认工作区 · main + utility",
    })
    .click();
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await page
    .getByRole("button", { name: "生成 2 个同步预览任务" })
    .click();
  await expect(
    page.getByText("前端预览：已生成 2 个待执行任务；未写入本机文件"),
  ).toBeVisible();

  await page.getByRole("tab", { name: "会话记录" }).click();
  await expect(
    page.getByRole("tab", { name: "会话记录", selected: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: /OpenCode · 会话数据库/ }).click();
  await page.getByRole("button", { name: "提炼为长期记忆" }).click();
  await expect(
    page.getByRole("tab", { name: "长期记忆", selected: true }),
  ).toBeVisible();
  await expect(page.getByRole("textbox", { name: "记忆标题" })).toHaveValue(
    "OpenCode · 会话数据库 · 提炼草稿",
  );

  await expectHealthyPage(page, health);
});

test("opens the standalone HTML and both file entry points without a blank page", async ({
  page,
}) => {
  const previewUrl = pathToFileURL(
    path.resolve(process.cwd(), "FyAgent-前端交互预览.html"),
  );
  const sourceIndexUrl = pathToFileURL(
    path.resolve(process.cwd(), "src/index.html"),
  );
  const builtIndexUrl = pathToFileURL(
    path.resolve(process.cwd(), "dist/index.html"),
  );

  await page.goto(previewUrl.href, { waitUntil: "load" });
  await expect(page).toHaveURL(/FyAgent-.*\.html#\/prompts$/);
  await expect(page.getByTestId("prompts-page")).toBeVisible();

  await page.goto(sourceIndexUrl.href, { waitUntil: "load" });
  await expect(page).toHaveURL(/FyAgent-.*\.html#\/prompts$/);
  await expect(page.getByTestId("prompts-page")).toBeVisible();

  await page.goto(builtIndexUrl.href, { waitUntil: "load" });
  await expect(page).toHaveURL(/FyAgent-.*\.html#\/prompts$/);
  await expect(page.getByTestId("prompts-page")).toBeVisible();

  const health = monitorPageHealth(page);
  const navigation = page.getByRole("navigation", { name: "主导航" });
  await routeLink(navigation, "记忆").click();
  await expect(page).toHaveURL(/#\/memory$/);
  await expect(page.getByTestId("memory-page")).toBeVisible();
  await page.getByRole("tab", { name: "会话记录" }).click();
  await expect(
    page.getByRole("tab", { name: "会话记录", selected: true }),
  ).toBeVisible();

  await expectHealthyPage(page, health);
});

test("renders the cross-agent Prompt workspace without page overflow", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/prompts");

  await expectNoHorizontalOverflow(page);
  await expect(
    page.getByRole("heading", { name: "提示词", exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "新建提示词" })).toBeVisible();
  await expect(page.getByTestId("prompt-library")).toBeVisible();
  await expect(page.getByTestId("prompt-editor")).toBeVisible();
  await expect(page.getByTestId("prompt-inspector")).toBeVisible();
  await expect(page.getByText("同一应用仅启用一条")).toHaveCount(0);
  await expect(page.getByText("2 条已启用")).toBeVisible();

  await page.getByRole("button", { name: /代码审查/ }).click();
  await expect(page.getByRole("textbox", { name: "名称" })).toHaveValue(
    "代码审查",
  );
  await page.getByRole("switch", { name: "启用代码审查" }).click();
  await expect(
    page.getByRole("switch", { name: "停用中文与回复风格" }),
  ).toBeChecked();
  await expect(
    page.getByRole("switch", { name: "停用代码审查" }),
  ).toBeChecked();

  await expectHealthyPage(page, health);
});

test("reaches every primary control with the keyboard in document order", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/models");

  const focusedControlIds: string[] = [];
  for (let index = 0; index < primaryControlTestIds.length; index += 1) {
    await page.keyboard.press("Tab");
    focusedControlIds.push(
      (await page.evaluate(() => {
        const activeElement = document.activeElement;
        return (
          activeElement?.getAttribute("data-testid") ??
          activeElement?.getAttribute("href")
        );
      })) ?? "",
    );
  }

  expect(focusedControlIds).toEqual([...primaryControlTestIds]);

  await expectHealthyPage(page, health);
});
