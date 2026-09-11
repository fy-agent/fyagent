import { readFile } from "node:fs/promises";
import path from "node:path";
import { expect, test, type Locator, type Page } from "@playwright/test";

import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";

const navigationContract = [
  { path: "/agents", label: "AI软件配置" },
  { path: "/health", label: "运行状态" },
  { path: "/auth", label: "账号与认证" },
  { path: "/models", label: "模型管理" },
  { path: "/skills", label: "Skills 管理" },
  { path: "/mcp", label: "MCP 管理" },
  { path: "/prompts", label: "提示词管理" },
  { path: "/memory", label: "记忆模块" },
] as const;

const retiredPrototypeCopy = [
  "前端原型",
  "未读取或写入本机文件",
  "注入目标",
  "同步预览任务",
  "会话记录",
  "提炼为长期记忆",
  "Claude Code · 长期记忆",
  "已保存到前端预览",
] as const;

function primaryNavigation(page: Page): Locator {
  return page.getByRole("navigation", { name: "主导航" });
}

function escapedRegularExpression(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function monitorReactWarnings(page: Page): string[] {
  const warnings: string[] = [];
  page.on("console", (message) => {
    const text = message.text();
    if (
      message.type() === "warning" &&
      /react|warning:|maximum update depth|error boundary/i.test(text)
    ) {
      warnings.push(text);
    }
  });
  return warnings;
}

async function expectNoReactWarnings(warnings: readonly string[]) {
  expect(
    warnings,
    `Unexpected React console warnings:\n${warnings.join("\n")}`,
  ).toEqual([]);
}

async function expectReachable(control: Locator): Promise<void> {
  await control.scrollIntoViewIfNeeded();
  await expect(control).toBeVisible();
  await expect(control).toBeInViewport();
}

async function expectNoRetiredPrototype(page: Page): Promise<void> {
  for (const copy of retiredPrototypeCopy) {
    await expect(page.getByText(copy, { exact: false })).toHaveCount(0);
  }
  await expect(page.locator('[data-data-source="prototype"]')).toHaveCount(0);
  await expect(page.getByTestId("memory-preview-tasks")).toHaveCount(0);
}

async function expectPromptNativeOnly(page: Page): Promise<void> {
  const promptPage = page.getByTestId("prompts-page");
  await expect(promptPage).toBeVisible();
  await expect(promptPage).toHaveClass(/\bfy-feature-page\b/);
  await expect(promptPage).toHaveClass(/\bfy-prompts-page\b/);
  await expect(promptPage).toHaveAttribute("data-data-source", "native");
  await expect(
    page.getByText("请使用 FyAgent 桌面应用", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("网页版不能管理提示词。", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(promptPage.locator(".fy-control-empty")).toBeVisible();
  await expect(page.getByRole("button", { name: "新建提示词" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "从文件导入" })).toBeDisabled();
  await expect(
    page.getByRole("searchbox", { name: "搜索提示词" }),
  ).toBeDisabled();
  await expect(page.locator('section[aria-label="提示词列表"]')).toHaveCount(0);
  await expect(page.getByRole("textbox", { name: "提示词内容" })).toHaveCount(
    0,
  );
  await expect(
    page.getByRole("textbox", { name: "当前使用的内容" }),
  ).toHaveCount(0);
  await expectNoRetiredPrototype(page);
}

async function expectMemoryNativeOnly(
  page: Page,
  feature: "长期记忆" | "每日记忆",
): Promise<void> {
  const memoryPage = page.getByTestId("memory-page");
  await expect(memoryPage).toBeVisible();
  await expect(memoryPage).toHaveClass(/\bfy-feature-page\b/);
  await expect(memoryPage).toHaveClass(/\bfy-memory-page\b/);
  await expect(
    page.getByText("需要 FyAgent 桌面应用", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText(`${feature}仅在 FyAgent 桌面应用中可用。`, {
      exact: true,
    }),
  ).toBeVisible();
  await expect(memoryPage.locator(".fy-control-empty")).toBeVisible();
  await expect(page.locator('section[aria-label="长期记忆资源"]')).toHaveCount(
    0,
  );
  await expect(page.locator('section[aria-label="每日记忆列表"]')).toHaveCount(
    0,
  );
  await expect(page.locator(".fy-memory-editor-textarea")).toHaveCount(0);
  await expectNoRetiredPrototype(page);
}

test("shows truthful native-only Prompt and Memory states without seeded data", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  const reactWarnings = monitorReactWarnings(page);
  await openRendererPage(page, "/prompts");

  await expectPromptNativeOnly(page);
  await expectNoHorizontalOverflow(page);

  await primaryNavigation(page)
    .getByRole("link", { name: "记忆模块", exact: true })
    .click();
  await expect(page).toHaveURL(/#\/memory$/);
  await expectMemoryNativeOnly(page, "长期记忆");

  await page.getByRole("tab", { name: "每日记忆" }).click();
  await expect(
    page.getByRole("tab", { name: "每日记忆", selected: true }),
  ).toBeVisible();
  await expectMemoryNativeOnly(page, "每日记忆");
  await expectNoHorizontalOverflow(page);

  await expectHealthyPage(page, health);
  await expectNoReactWarnings(reactWarnings);
});

test("uses the shared feature and control visual language without page-local theme colors", async () => {
  const sourceRoot = path.resolve(process.cwd(), "src/pages");
  const [promptSource, memorySource, promptStyles, memoryStyles] =
    await Promise.all([
      readFile(path.join(sourceRoot, "prompts/Page.tsx"), "utf8"),
      readFile(path.join(sourceRoot, "memory/Page.tsx"), "utf8"),
      readFile(path.join(sourceRoot, "prompts/page.css"), "utf8"),
      readFile(path.join(sourceRoot, "memory/page.css"), "utf8"),
    ]);

  for (const source of [promptSource, memorySource]) {
    expect(source).toContain("fy-feature-page");
    expect(source).toMatch(/fy-feature-(?:header|tabs|toolbar|master|panel)/);
    expect(source).toMatch(/fy-control-(?:field|select|textarea|button)/);
    expect(source).not.toContain('data-data-source="prototype"');
  }

  const pageLocalThemeLiteral =
    /(?:linear|radial)-gradient|#[0-9a-f]{3,8}\b|rgba?\s*\(/i;
  for (const styles of [promptStyles, memoryStyles]) {
    expect(styles).not.toMatch(pageLocalThemeLiteral);
  }
});

test("switches all eight routes and keeps Prompt and Memory controls reachable", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  const reactWarnings = monitorReactWarnings(page);
  await openRendererPage(page, "/prompts");

  const navigation = primaryNavigation(page);
  for (const { path: routePath, label } of navigationContract) {
    const link = navigation.getByRole("link", { name: label, exact: true });
    await expectReachable(link);
    await link.click();
    await expect(page).toHaveURL(
      new RegExp(`${escapedRegularExpression(`#${routePath}`)}$`),
    );
    await expect(link).toHaveAttribute("aria-current", "page");
    await expect(navigation.locator('a[aria-current="page"]')).toHaveCount(1);
    await expectNoHorizontalOverflow(page);
  }

  await navigation
    .getByRole("link", { name: "提示词管理", exact: true })
    .click();
  await expectPromptNativeOnly(page);
  for (const control of [
    page.getByTestId("prompt-app-claude"),
    page.getByRole("searchbox", { name: "搜索提示词" }),
    page.getByRole("button", { name: "从文件导入" }),
    page.getByRole("button", { name: "新建提示词" }),
    page.getByText("请使用 FyAgent 桌面应用", { exact: true }),
  ]) {
    await expectReachable(control);
  }
  await expectNoHorizontalOverflow(page);

  await navigation.getByRole("link", { name: "记忆模块", exact: true }).click();
  await expectMemoryNativeOnly(page, "长期记忆");
  for (const control of [
    page.getByRole("tab", { name: "长期记忆" }),
    page.getByRole("tab", { name: "每日记忆" }),
    page.getByText("需要 FyAgent 桌面应用", { exact: true }),
  ]) {
    await expectReachable(control);
  }
  await page.getByRole("tab", { name: "每日记忆" }).click();
  await expectMemoryNativeOnly(page, "每日记忆");
  await expectReachable(
    page.getByText("需要 FyAgent 桌面应用", { exact: true }),
  );
  await expectNoHorizontalOverflow(page);

  await expectHealthyPage(page, health);
  await expectNoReactWarnings(reactWarnings);
});

test("serves native-only pages through the normal HTTP entry without external requests", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  const reactWarnings = monitorReactWarnings(page);
  const externalRequests: string[] = [];

  page.on("request", (request) => {
    const requestUrl = request.url();
    if (
      !["127.0.0.1", "localhost"].includes(new URL(requestUrl).hostname) &&
      !requestUrl.startsWith("data:") &&
      !requestUrl.startsWith("blob:")
    ) {
      externalRequests.push(requestUrl);
    }
  });

  await openRendererPage(page, "/prompts");
  await expect(page).toHaveURL(/^http:\/\/127\.0\.0\.1:\d+\/#\/prompts$/);
  await expectPromptNativeOnly(page);
  await expectNoHorizontalOverflow(page);

  await primaryNavigation(page)
    .getByRole("link", { name: "记忆模块", exact: true })
    .click();
  await expect(page).toHaveURL(/#\/memory$/);
  await expectMemoryNativeOnly(page, "长期记忆");
  await page.getByRole("tab", { name: "每日记忆" }).click();
  await expectMemoryNativeOnly(page, "每日记忆");
  await expectNoHorizontalOverflow(page);

  await primaryNavigation(page)
    .getByRole("link", { name: "提示词管理", exact: true })
    .click();
  await expectPromptNativeOnly(page);
  expect(externalRequests).toEqual([]);

  await expectHealthyPage(page, health);
  await expectNoReactWarnings(reactWarnings);
});
