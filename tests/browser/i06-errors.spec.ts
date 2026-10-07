import {
  expect,
  test,
  type Locator,
  type Page,
  type TestInfo,
} from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";

// Controlled Tauri-boundary failures only. No native files/accounts are accessed.
test.use({ viewport: { width: 900, height: 600 } });

async function installErrors(
  page: Page,
  kind: "mcp" | "prompt",
  promptError = "",
) {
  await installRichTauriFeatureFixture(page);
  await page.addInitScript(
    ({ kind, promptError }) => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      const calls = window.__FYAGENT_FEATURE_FIXTURE__.calls;
      const servers = {
        "i06-existing": {
          id: "i06-existing",
          name: "I06 existing MCP",
          server: { type: "stdio", command: "fixture-original-command" },
          apps: {
            claude: true,
            codex: false,
            opencode: false,
            workbuddy: false,
            qoderwork: false,
            "trae-work": false,
            grokbuild: false,
          },
        },
      };
      const prompts = {
        "i06-existing": {
          id: "i06-existing",
          name: "I06 existing prompt",
          content: "Original library text",
          description: "Controlled fixture",
          enabled: false,
        },
      };
      window.__TAURI_INTERNALS__.invoke = async (command, payload = {}) => {
        if (kind === "mcp" && command === "get_mcp_servers") {
          calls.push({ command, payload });
          return structuredClone(servers);
        }
        if (
          kind === "mcp" &&
          /^(upsert_mcp_server|delete_mcp_server)$/.test(command)
        ) {
          calls.push({ command, payload });
          throw new Error(
            "IO 错误: fixture-private-path: fixture-private-content TOKEN=fixture-private-token",
          );
        }
        if (kind === "prompt" && command === "get_prompts") {
          calls.push({ command, payload });
          return structuredClone(prompts);
        }
        if (
          kind === "prompt" &&
          command === "get_current_prompt_file_content"
        ) {
          calls.push({ command, payload });
          return "Controlled current-use text";
        }
        if (kind === "prompt" && command === "import_prompt_from_file") {
          calls.push({ command, payload });
          // Public source-read errors already arrive as closed native messages.
          // The unknown scenario also proves raw details are suppressed by Page.
          throw promptError;
        }
        if (
          kind === "prompt" &&
          /^(upsert_prompt|enable_prompt|delete_prompt)$/.test(command)
        ) {
          calls.push({ command, payload });
          throw new Error(
            "Unexpected mutation in I06 source-import failure test",
          );
        }
        return invoke(command, payload);
      };
    },
    { kind, promptError },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
}

async function expectPhysicallyVisible(control: Locator) {
  await expect(control).toBeVisible();
  await expect(control).toBeInViewport();
  const visible = await control.evaluate((node) => {
    const rect = node.getBoundingClientRect();
    let left = Math.max(0, rect.left),
      right = Math.min(innerWidth, rect.right);
    let top = Math.max(0, rect.top),
      bottom = Math.min(innerHeight, rect.bottom);
    for (
      let parent = node.parentElement;
      parent;
      parent = parent.parentElement
    ) {
      const style = getComputedStyle(parent),
        box = parent.getBoundingClientRect();
      if (/auto|scroll|hidden|clip/.test(style.overflowX)) {
        left = Math.max(left, box.left);
        right = Math.min(right, box.right);
      }
      if (/auto|scroll|hidden|clip/.test(style.overflowY)) {
        top = Math.max(top, box.top);
        bottom = Math.min(bottom, box.bottom);
      }
    }
    const hit = document.elementFromPoint(
      (left + right) / 2,
      (top + bottom) / 2,
    );
    return {
      width: right - left,
      height: bottom - top,
      unobscured: hit !== null && (hit === node || node.contains(hit)),
    };
  });
  expect(visible.width).toBeGreaterThan(10);
  expect(visible.height).toBeGreaterThan(10);
  expect(visible.unobscured).toBe(true);
}

async function capture(page: Page, info: TestInfo, state: string) {
  const file = info.outputPath(`i06-errors-900x600-${state}.png`);
  expect(page.viewportSize()).toEqual({ width: 900, height: 600 });
  await page.screenshot({ path: file, fullPage: false });
  await info.attach(state, { path: file, contentType: "image/png" });
  await info.attach(`${state}-scope`, {
    body: JSON.stringify({
      evidenceKind: "runtime_screenshot",
      viewport: page.viewportSize(),
      scope:
        "controlled browser invoke fixture; real native/account acceptance remains pending",
    }),
    contentType: "application/json",
  });
}

async function expectNoPrivateError(page: Page) {
  await expect(page.locator("body")).not.toContainText("fixture-private-path");
  await expect(page.locator("body")).not.toContainText(
    "fixture-private-content",
  );
  await expect(page.locator("body")).not.toContainText("fixture-private-token");
}

async function mutationCalls(page: Page) {
  return page.evaluate(() =>
    window.__FYAGENT_FEATURE_FIXTURE__.calls.filter((call) =>
      /^(upsert_mcp_server|delete_mcp_server|import_prompt_from_file|upsert_prompt|enable_prompt|delete_prompt)$/.test(
        call.command,
      ),
    ),
  );
}

test("I06 MCP failed save retains draft, visible footer and correction focus; reopen clears error", async ({
  page,
}, info) => {
  await installErrors(page, "mcp");
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/mcp");
  const add = page
    .getByRole("button", { name: "添加 MCP", exact: true })
    .first();
  await add.click();
  const editor = page.getByRole("dialog", { name: "添加 MCP", exact: true });
  await expect(editor).toHaveAttribute("data-motion-settled", "true");
  await editor
    .getByRole("textbox", { name: "ID", exact: true })
    .fill("i06-draft");
  const name = editor.getByRole("textbox", { name: "名称", exact: true });
  const command = editor.getByRole("textbox", { name: "命令", exact: true });
  await name.fill("I06 retained draft");
  await command.fill("fixture-draft-command");
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  const error = editor.getByRole("alert");
  await expect(error).toContainText("MCP 配置文件读写失败");
  await expect(error).toContainText("部分目标可能已更改");
  await expect(name).toHaveValue("I06 retained draft");
  await expect(command).toHaveValue("fixture-draft-command");
  await expect(name).toBeFocused();
  await expect(name).toBeEditable();
  await expectPhysicallyVisible(name);
  expect(await error.evaluate((node) => node.closest("footer") !== null)).toBe(
    true,
  );
  await expectPhysicallyVisible(error);
  await expectPhysicallyVisible(
    editor.getByRole("button", { name: "保存", exact: true }),
  );
  await expectNoPrivateError(page);
  const calls = await mutationCalls(page);
  expect(calls).toHaveLength(1);
  expect(calls[0]).toMatchObject({
    command: "upsert_mcp_server",
    payload: {
      server: {
        id: "i06-draft",
        name: "I06 retained draft",
        server: { command: "fixture-draft-command" },
      },
    },
  });
  await expect(page.locator(".fy-toast-error")).toHaveCount(0);
  await capture(page, info, "mcp-save-failure");
  await expectPhysicallyVisible(
    editor.getByRole("button", { name: "保存", exact: true }),
  );
  await expectPhysicallyVisible(
    editor.getByRole("button", { name: "取消", exact: true }),
  );
  await editor.getByRole("button", { name: "取消", exact: true }).click();
  await expect(editor).toHaveCount(0);
  await add.click();
  await expect(editor).toBeVisible();
  await expect(editor).toHaveAttribute("data-motion-settled", "true");
  await expect(page.locator(".fy-toast-error")).toHaveCount(0);
  await expect(editor.getByRole("alert")).toHaveCount(0);
  await expect(
    editor.getByRole("textbox", { name: "ID", exact: true }),
  ).toHaveValue("");
  await expect(
    editor.getByRole("textbox", { name: "名称", exact: true }),
  ).toHaveValue("");
  expect(await mutationCalls(page)).toHaveLength(1);
  await capture(page, info, "mcp-reopen-clean");
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("I06 MCP failed delete keeps confirmation and permits cancellation", async ({
  page,
}, info) => {
  await installErrors(page, "mcp");
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/mcp");
  await expect(
    page.getByRole("heading", { name: "I06 existing MCP", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "删除", exact: true }).click();
  const dialog = page.getByRole("dialog", {
    name: "删除 I06 existing MCP",
    exact: true,
  });
  await expect(dialog).toHaveAttribute("data-motion-settled", "true");
  await dialog.getByRole("button", { name: "确认", exact: true }).click();
  await expect(dialog).toContainText("删除未确认完成，部分目标可能已更改");
  await expect(dialog).toContainText("请先核对管理列表和目标配置");
  const cancel = dialog.getByRole("button", { name: "取消", exact: true });
  await expect(cancel).toBeEnabled();
  await expectPhysicallyVisible(cancel);
  await expectNoPrivateError(page);
  const calls = await mutationCalls(page);
  expect(calls).toHaveLength(1);
  expect(calls[0]).toMatchObject({
    command: "delete_mcp_server",
    payload: { id: "i06-existing" },
  });
  await capture(page, info, "mcp-delete-failure");
  await cancel.click();
  await expect(dialog).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "I06 existing MCP", exact: true }),
  ).toBeVisible();
  await expectHealthyPage(page, health);
});

const sourceErrors = [
  {
    id: "missing",
    message: "提示词文件不存在，未导入。请先确认目标应用已创建提示词文件。",
    action: "请先确认目标应用已创建提示词文件",
  },
  {
    id: "utf8",
    message:
      "提示词文件不是有效 UTF-8，未导入。请保留原文件，将副本转换为 UTF-8 后重试。",
    action: "将副本转换为 UTF-8 后重试",
  },
  {
    id: "permission",
    message: "无法读取提示词文件，未导入。请检查文件类型和读取权限后重试。",
    action: "请检查文件类型和读取权限后重试",
  },
] as const;

for (const usageExpanded of [false, true]) {
  for (const source of sourceErrors) {
    test(`I06 Prompt ${source.id} source failure with current-use ${usageExpanded ? "expanded" : "collapsed"}`, async ({
      page,
    }, info) => {
      await installErrors(page, "prompt", source.message);
      const health = monitorPageHealth(page);
      await openRendererPage(page, "/prompts?target=claude");
      const scope = page.getByTestId("prompts-page");
      const name = scope.getByRole("textbox", { name: "名称", exact: true });
      const body = scope.getByRole("textbox", { name: "内容", exact: true });
      await expect(name).toHaveValue("I06 existing prompt");
      await name.fill("I06 unsaved prompt draft");
      if (usageExpanded) {
        await scope.locator(".fy-prompts-live summary").click();
        await expect(
          scope.getByRole("textbox", { name: "当前使用的内容", exact: true }),
        ).toHaveValue("Controlled current-use text");
      }
      await scope
        .getByRole("button", { name: "从文件导入", exact: true })
        .click();
      const error = scope.getByText(`提示词导入遇到问题：${source.message}`, {
        exact: true,
      });
      await expectPhysicallyVisible(error);
      await expect(error).toContainText("未导入");
      await expect(error).toContainText(source.action);
      await expect(name).toHaveValue("I06 unsaved prompt draft");
      await expect(body).toHaveValue("Original library text");
      await expect(
        scope.getByText("提示词已从文件导入", { exact: true }),
      ).toHaveCount(0);
      await expect(
        scope.getByRole("button", { name: "从文件导入", exact: true }),
      ).toBeEnabled();
      await expectNoPrivateError(page);
      const calls = await mutationCalls(page);
      expect(calls).toHaveLength(1);
      expect(calls[0]).toMatchObject({
        command: "import_prompt_from_file",
        payload: { app: "claude" },
      });
      await expect(page.locator(".fy-toast-error")).toHaveCount(0);
      await capture(
        page,
        info,
        `prompt-${source.id}-${usageExpanded ? "current-use" : "library"}`,
      );
      if (usageExpanded) {
        const currentUse = scope.getByRole("textbox", {
          name: "当前使用的内容",
          exact: true,
        });
        await currentUse.scrollIntoViewIfNeeded();
        await expectPhysicallyVisible(currentUse);
        await expect(currentUse).toHaveValue("Controlled current-use text");
        await expectPhysicallyVisible(error);
        await capture(page, info, `prompt-${source.id}-live-preserved`);
      }
      await expectNoHorizontalOverflow(page);
      await expectHealthyPage(page, health);
    });
  }
}

test("I06 Prompt unknown import failure hides raw details and asks for result review", async ({
  page,
}, info) => {
  await installErrors(
    page,
    "prompt",
    "fixture-private-path fixture-private-content TOKEN=fixture-private-token",
  );
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/prompts?target=claude");
  const scope = page.getByTestId("prompts-page");
  await expect(
    scope.getByRole("textbox", { name: "名称", exact: true }),
  ).toHaveValue("I06 existing prompt");
  await scope.getByRole("button", { name: "从文件导入", exact: true }).click();
  const error = scope.getByText(
    "提示词导入遇到问题：提示词导入未确认完成。请先刷新管理列表核对结果，再决定是否重试。",
    { exact: true },
  );
  await expectPhysicallyVisible(error);
  await expect(error).not.toContainText("未导入");
  await expectNoPrivateError(page);
  expect(await mutationCalls(page)).toHaveLength(1);
  await expect(page.locator(".fy-toast-error")).toHaveCount(0);
  await capture(page, info, "prompt-unconfirmed");
  await expectHealthyPage(page, health);
});
