import { readFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { expect, test, type Dialog, type Page } from "@playwright/test";

import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openV2Page,
} from "./support";

function primaryNavigation(page: Page) {
  return page.getByRole("navigation", { name: "主导航" });
}

function handleNextDialog(
  page: Page,
  action: "accept" | "dismiss",
): Promise<void> {
  let resolveHandled: () => void;
  const handled = new Promise<void>((resolve) => {
    resolveHandled = resolve;
  });

  page.once("dialog", async (dialog: Dialog) => {
    expect(dialog.message()).toContain("尚未保存");
    if (action === "accept") {
      await dialog.accept();
    } else {
      await dialog.dismiss();
    }
    resolveHandled();
  });

  return handled;
}

test("persists Prompt targets, keeps several rules enabled, and guards dirty navigation", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/prompts");

  await expectNoHorizontalOverflow(page);
  await expect(
    page.getByText("前端原型 · 未读取或写入本机文件"),
  ).toBeVisible();
  await expect(page.getByText("2 条已启用")).toBeVisible();
  await expect(page.getByText("7 个目标文件")).toBeVisible();
  await expect(page.getByText("8 个 Agent")).toBeVisible();

  await page.getByRole("button", { name: /^代码审查/ }).click();
  const openClawDefault = page.getByRole("checkbox", {
    name: "注入到OpenClaw默认工作区 · main + utility",
  });
  await openClawDefault.click();
  await expect(page.getByText("5 个目标文件")).toBeVisible();
  await expect(page.getByText("6 个 Agent")).toBeVisible();
  await page.getByRole("button", { name: "保存", exact: true }).click();

  await page.getByRole("button", { name: /^中文与回复风格/ }).click();
  await page.getByRole("button", { name: /^代码审查/ }).click();
  await expect(
    page.getByRole("checkbox", {
      name: "取消注入到OpenClaw默认工作区 · main + utility",
    }),
  ).toBeChecked();

  await page.getByRole("switch", { name: "启用代码审查" }).click();
  await expect(page.getByText("3 条已启用")).toBeVisible();
  await expect(
    page.getByRole("switch", { name: "停用中文与回复风格" }),
  ).toBeChecked();
  await expect(
    page.getByRole("switch", { name: "停用目标、边界与完成证据" }),
  ).toBeChecked();

  const description = page.getByRole("textbox", { name: "描述" });
  await description.fill("仍在编辑的浏览器草稿");
  const memoryLink = primaryNavigation(page).getByRole("link", {
    name: "记忆",
    exact: true,
  });

  const dismissed = handleNextDialog(page, "dismiss");
  await Promise.all([dismissed, memoryLink.click()]);
  await expect(page).toHaveURL(/#\/prompts$/);
  await expect(description).toHaveValue("仍在编辑的浏览器草稿");

  const accepted = handleNextDialog(page, "accept");
  await Promise.all([accepted, memoryLink.click()]);
  await expect(page).toHaveURL(/#\/memory$/);
  await expect(page.getByTestId("memory-page")).toBeVisible();

  await expectHealthyPage(page, health);
});

test("creates saved Memory revisions and only pending per-target preview tasks", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/memory");

  await expectNoHorizontalOverflow(page);
  await expect(
    page.getByText("前端原型 · 未读取或写入本机文件"),
  ).toBeVisible();
  await page
    .getByRole("button", { name: /Claude Code · 长期记忆/ })
    .click();
  await expect(page.getByRole("checkbox")).toHaveCount(4);

  const content = page.getByRole("textbox", { name: "记忆内容" });
  await content.fill(`${await content.inputValue()}\n- 浏览器验收经验`);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("r2")).toBeVisible();

  await page
    .getByRole("checkbox", {
      name: "同步到OpenClaw默认工作区 · main + utility",
    })
    .click();
  await expect(
    page.getByRole("button", { name: "生成 2 个同步预览任务" }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("r3")).toBeVisible();
  await page
    .getByRole("button", { name: "生成 2 个同步预览任务" })
    .click();

  await expect(
    page.getByText(
      "前端预览：已生成 2 个待执行任务；未写入本机文件",
    ),
  ).toBeVisible();
  const tasks = page.getByTestId("memory-preview-tasks");
  await expect(tasks.getByRole("listitem")).toHaveCount(2);
  for (const task of await tasks.getByRole("listitem").all()) {
    await expect(task).toHaveAttribute("data-preview-state", "pending");
    await expect(task).toHaveAttribute("data-durable-state", "not-run");
    await expect(task).toContainText("待执行 · 未写入");
    await expect(task).toContainText("基于修订 r3");
  }
  await expect(page.getByText("已同步")).toHaveCount(0);

  const title = page.getByRole("textbox", { name: "记忆标题" });
  await title.fill("未保存的浏览器标题");
  const promptLink = primaryNavigation(page).getByRole("link", {
    name: "提示词",
    exact: true,
  });
  const dismissed = handleNextDialog(page, "dismiss");
  await Promise.all([dismissed, promptLink.click()]);
  await expect(page).toHaveURL(/#\/memory$/);
  await expect(title).toHaveValue("未保存的浏览器标题");

  const accepted = handleNextDialog(page, "accept");
  await Promise.all([accepted, promptLink.click()]);
  await expect(page).toHaveURL(/#\/prompts$/);

  await expectHealthyPage(page, health);
});

test("promotes a read-only Daily source with complete visible provenance", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/memory");

  await page.getByRole("tab", { name: "每日记录" }).click();
  await expect(page.getByRole("textbox", { name: "记忆内容" })).toHaveAttribute(
    "readonly",
  );
  await expect(page.getByRole("button", { name: "只读来源" })).toBeDisabled();
  await page.getByRole("button", { name: "提炼为长期记忆" }).click();

  await expect(
    page.getByRole("tab", { name: "长期记忆", selected: true }),
  ).toBeVisible();
  await expect(page.getByRole("textbox", { name: "记忆标题" })).toHaveValue(
    "OpenClaw · 今日记录 · 提炼草稿",
  );
  await expect(page.getByText("前端草稿 · 未创建文件")).toBeVisible();
  await expect(page.getByText("r0")).toBeVisible();
  await expect(
    page.getByText("已生成长期记忆草稿；原始记录保持不变"),
  ).toBeVisible();

  const provenance = page.getByTestId("memory-provenance");
  await expect(provenance).toContainText("OpenClaw · 今日记录");
  await expect(provenance).toContainText("ID: openclaw-daily-latest");
  await expect(provenance).toContainText("toolId: openclaw");
  await expect(provenance).toContainText("targetId: openclaw-default");
  await expect(provenance).toContainText(
    "~/.openclaw/workspace/memory/2026-08-12.md",
  );
  await expect(provenance).toContainText("提炼时间");
  await expect(page.getByRole("checkbox")).toHaveCount(4);
  await expect(
    page.getByRole("button", { name: "生成 0 个同步预览任务" }),
  ).toBeDisabled();

  await page
    .getByRole("checkbox", { name: "同步到Claude Code全局" })
    .click();
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("r1")).toBeVisible();
  await page
    .getByRole("button", { name: "生成 1 个同步预览任务" })
    .click();
  const tasks = page.getByTestId("memory-preview-tasks");
  await expect(tasks.getByRole("listitem")).toHaveCount(1);
  await expect(tasks.getByRole("listitem")).toHaveAttribute(
    "data-preview-state",
    "pending",
  );
  await expect(tasks.getByRole("listitem")).toHaveAttribute(
    "data-durable-state",
    "not-run",
  );
  await expect(tasks).toContainText("基于修订 r1");

  await page.getByRole("tab", { name: "会话记录" }).click();
  await expect(page.getByRole("textbox", { name: "记忆内容" })).toHaveAttribute(
    "readonly",
  );
  await expect(page.getByRole("button", { name: "只读来源" })).toBeDisabled();

  await expectHealthyPage(page, health);
});

test("opens the generated standalone from file and leaves no local entry requests", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  const previewPath = path.resolve(
    process.cwd(),
    "FyAgent-前端交互预览.html",
  );
  const previewHtml = await readFile(previewPath, "utf8");

  expect(previewHtml).not.toMatch(/<script\b[^>]*\bsrc\s*=/i);
  expect(previewHtml).not.toMatch(
    /<link\b(?=[^>]*\brel=["']stylesheet["'])(?=[^>]*\bhref=)[^>]*>/i,
  );
  expect(previewHtml).not.toMatch(/(?:\.\/)?dist\/assets\//i);

  await page.goto(pathToFileURL(previewPath).href, { waitUntil: "load" });
  await expect(page).toHaveURL(/FyAgent-.*\.html#\/prompts$/);
  await expect(page.getByTestId("prompts-page")).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await primaryNavigation(page)
    .getByRole("link", { name: "记忆", exact: true })
    .click();
  await expect(page).toHaveURL(/#\/memory$/);
  await expect(page.getByTestId("memory-page")).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await expectHealthyPage(page, health);
});

test("keeps critical Prompt and Memory controls reachable at every configured viewport", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/prompts");

  await expectNoHorizontalOverflow(page);
  for (const control of [
    page.getByRole("button", { name: "新建提示词" }),
    page.getByRole("searchbox", { name: "搜索提示词" }),
    page.getByRole("textbox", { name: "名称" }),
    page.getByRole("button", { name: "保存" }),
    page.getByRole("checkbox", { name: "取消注入到Codex全局" }),
  ]) {
    await control.scrollIntoViewIfNeeded();
    await expect(control).toBeVisible();
    await expect(control).toBeInViewport();
  }

  await primaryNavigation(page)
    .getByRole("link", { name: "记忆", exact: true })
    .click();
  await expectNoHorizontalOverflow(page);
  for (const control of [
    page.getByRole("button", { name: "重新扫描本机" }),
    page.getByRole("tab", { name: "每日记录" }),
    page.getByRole("searchbox", { name: "搜索长期记忆" }),
    page.getByRole("button", { name: /Codex · 派生记忆/ }),
    page.getByRole("textbox", { name: "记忆内容" }),
  ]) {
    await control.scrollIntoViewIfNeeded();
    await expect(control).toBeVisible();
    await expect(control).toBeInViewport();
  }

  await expectHealthyPage(page, health);
});
