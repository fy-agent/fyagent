import { expect, test } from "@playwright/test";

import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "./support/features";

test("OpenCode applies an existing ChatGPT subscription through its own revisioned configuration", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/models?target=opencode");
  const section = page.getByRole("region", { name: "账号订阅设置" });
  await section
    .getByRole("radio", { name: "ChatGPT · Browser Fixture" })
    .check();
  await expect(section.getByText("已读取 2 个 Codex 模型")).toBeVisible();
  await section
    .getByRole("button", { name: "chatgpt-fixture-model", exact: true })
    .click();
  await section.getByRole("button", { name: "应用到 OpenCode" }).click();
  const dialog = page.getByRole("dialog", {
    name: "确认应用 OpenCode 订阅配置",
  });
  await expect(
    dialog.getByText("~/.config/opencode/opencode.json", { exact: true }),
  ).toBeVisible();
  await dialog.getByRole("button", { name: "确认应用" }).click();
  await expect(section.getByText("已将账号订阅应用到 OpenCode")).toBeVisible();
  const calls = await featureFixtureCalls(page);
  expect(
    calls
      .filter((call) => call.command === "bind_opencode_managed_proxy")
      .map((call) => call.payload),
  ).toEqual([
    {
      request: {
        accountId: `ma1:${"1".repeat(32)}`,
        modelId: "chatgpt-fixture-model",
        expectedRevision: null,
      },
    },
  ]);
  expect(
    calls.filter((call) =>
      [
        "bind_managed_proxy_provider",
        "save_opencode_models",
        "apply_provider_quick_setup_with_result",
        "fetch_models_for_config",
      ].includes(call.command),
    ),
  ).toEqual([]);
  expect(
    calls.filter((call) => call.command === "get_opencode_model_snapshot")
      .length,
  ).toBeGreaterThanOrEqual(2);
  await expect(
    page.getByRole("button", { name: "保存并应用", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "恢复之前的模型配置" }).click();
  const restoreDialog = page.getByRole("dialog", {
    name: "恢复 OpenCode 模型配置",
  });
  await expect(
    restoreDialog.getByText("~/.config/opencode/opencode.json", {
      exact: true,
    }),
  ).toBeVisible();
  await restoreDialog.getByRole("button", { name: "确认恢复" }).click();
  await expect(
    page.getByRole("button", { name: "恢复之前的模型配置" }),
  ).toHaveCount(0);
  await expect(section.getByText("已将账号订阅应用到 OpenCode")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "保存并应用", exact: true }),
  ).toBeEnabled();
  expect(
    (await featureFixtureCalls(page))
      .filter((call) => call.command === "set_proxy_takeover_for_app")
      .map((call) => call.payload),
  ).toEqual([{ appType: "opencode", enabled: false }]);
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("saved Grok subscription can be selected for Claude and continued through the Codex source plan", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/models?target=claude");
  let section = page
    .getByRole("region", { name: "Claude Code 模型配置", exact: true })
    .getByRole("region", { name: "账号订阅设置" });
  await expect(
    section.getByRole("button", { name: "应用到 Claude Code" }),
  ).toBeDisabled();
  await section
    .getByRole("radio", { name: "xAI 设备码 · browser-xai@example.com" })
    .check();
  await section
    .getByRole("button", { name: "grok-subscription-fixture-2" })
    .click();
  await section.getByRole("button", { name: "应用到 Claude Code" }).click();
  let dialog = page.getByRole("dialog", {
    name: "确认应用 Claude Code 订阅配置",
  });
  await expect(dialog.getByText(/grok-subscription-fixture-2/)).toBeVisible();
  await dialog.getByRole("button", { name: "确认应用" }).click();
  await expect(
    section.getByText("已将账号订阅应用到 Claude Code"),
  ).toBeVisible();
  await expect(section.getByText(/保持 FyAgent 在后台运行/)).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await page.getByRole("button", { name: "Codex", exact: true }).click();
  section = page
    .getByRole("region", { name: "Codex 模型配置", exact: true })
    .getByRole("region", { name: "账号订阅设置" });
  await expect(
    section.getByRole("button", { name: "保存 Codex 订阅配置" }),
  ).toBeDisabled();
  await section
    .getByRole("radio", { name: "xAI 设备码 · browser-xai@example.com" })
    .check();
  await section
    .getByRole("button", { name: "grok-subscription-fixture-1" })
    .click();
  await section.getByRole("button", { name: "保存 Codex 订阅配置" }).click();
  dialog = page.getByRole("dialog", { name: "确认保存 Codex 订阅配置" });
  await dialog.getByRole("button", { name: "确认保存" }).click();
  await expect(section.getByText(/当前请求来源尚未切换/)).toBeVisible();
  await section.getByRole("button", { name: "继续预览 Codex 配置" }).click();
  await expect(page).toHaveURL(/#\/auth\?consumer=codex&view=connections$/);
  await page
    .getByRole("combobox", { name: "切换到" })
    .selectOption("subscription-fixture-codex");
  await page.getByRole("button", { name: "预览更改" }).click();
  await page.getByRole("button", { name: "应用更改" }).click();
  await expect(page.getByRole("button", { name: "应用更改" })).toHaveCount(0);

  const calls = await featureFixtureCalls(page);
  expect(
    calls
      .filter((call) => call.command === "bind_managed_proxy_provider")
      .map((call) => call.payload),
  ).toEqual([
    {
      request: {
        app: "claude",
        accountId: `ma1:${"2".repeat(32)}`,
        modelId: "grok-subscription-fixture-2",
      },
    },
    {
      request: {
        app: "codex",
        accountId: `ma1:${"2".repeat(32)}`,
        modelId: "grok-subscription-fixture-1",
      },
    },
  ]);
  expect(
    calls.filter((call) => call.command === "apply_change_plan"),
  ).toHaveLength(1);
  expect(calls.some((call) => call.command === "auth_get_status")).toBe(false);
  await expectHealthyPage(page, health);
});

test("subscription rejection remains local to its target", async ({ page }) => {
  await installRichTauriFeatureFixture(page, { xaiBindFailure: true });
  await openRendererPage(
    page,
    "/models?target=claude&agentReturn=grokbuild&agentSection=models",
  );
  const section = page.getByRole("region", { name: "账号订阅设置" });
  await section
    .getByRole("radio", { name: "xAI 设备码 · browser-xai@example.com" })
    .check();
  await section
    .getByRole("button", { name: "grok-subscription-fixture-1" })
    .click();
  await section.getByRole("button", { name: "应用到 Claude Code" }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "确认应用" })
    .click();
  await expect(section.getByText("未能应用订阅配置")).toBeVisible();
  await expect(section.getByText(/所选账号暂时不能用于本机转发/)).toBeVisible();
  await section.getByRole("button", { name: "管理订阅账号" }).click();
  await expect(page).toHaveURL(
    /#\/auth\?view=accounts&agentReturn=grokbuild&agentSection=models$/,
  );
  const calls = await featureFixtureCalls(page);
  expect(
    calls.filter(
      (call) =>
        call.command === "apply_provider_quick_setup_with_result" ||
        call.command === "apply_change_plan",
    ),
  ).toEqual([]);
});

test("ChatGPT account can be bound to Grok Build through the existing model target", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/models?target=grokbuild");
  const section = page
    .getByRole("region", { name: "Grok Build 模型配置", exact: true })
    .getByRole("region", { name: "账号订阅设置" });
  await section
    .getByRole("radio", { name: "ChatGPT · Browser Fixture" })
    .check();
  await section
    .getByRole("button", { name: "chatgpt-fixture-model", exact: true })
    .click();
  await section.getByRole("button", { name: "应用到 Grok Build" }).click();
  const dialog = page.getByRole("dialog", {
    name: "确认应用 Grok Build 订阅配置",
  });
  await expect(
    dialog.getByText("~/.config/grokbuild/config", { exact: true }),
  ).toBeVisible();
  await dialog.getByRole("button", { name: "确认应用" }).click();
  await expect(
    section.getByText("已将账号订阅应用到 Grok Build"),
  ).toBeVisible();
  const calls = await featureFixtureCalls(page);
  expect(
    calls
      .filter((call) => call.command === "bind_managed_proxy_provider")
      .map((call) => call.payload),
  ).toEqual([
    {
      request: {
        app: "grokbuild",
        accountId: `ma1:${"1".repeat(32)}`,
        modelId: "chatgpt-fixture-model",
      },
    },
  ]);
  expect(
    calls.filter(
      (call) =>
        call.command === "get_xai_oauth_models" ||
        call.command === "fetch_models_for_config",
    ),
  ).toEqual([]);
  expect(
    calls
      .filter((call) => call.command === "managed_auth_get_openai_models")
      .map((call) => call.payload),
  ).toEqual([{ accountId: `ma1:${"1".repeat(32)}` }]);
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("ChatGPT catalog failure names the reason and keeps manual entry and retry", async ({
  page,
}, testInfo) => {
  await installRichTauriFeatureFixture(page, {
    openAiModelsFailOnce: "requires_reauth",
  });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/models?target=grokbuild");
  const section = page
    .getByRole("region", { name: "Grok Build 模型配置", exact: true })
    .getByRole("region", { name: "账号订阅设置" });
  await section
    .getByRole("radio", { name: "ChatGPT · Browser Fixture" })
    .check();
  await expect(section.getByText("需要重新登录此 ChatGPT 账号")).toBeVisible();
  await expect(section.getByLabel("订阅模型 ID")).toBeVisible();
  await section.screenshot({
    path: testInfo.outputPath("openai-catalog-requires-reauth.png"),
  });
  await section.getByRole("button", { name: "查看模型选项" }).click();
  await expect(section.getByText("已读取 2 个 Codex 模型")).toBeVisible();
  await expect(
    section.getByRole("button", { name: "gpt-fixture-codex", exact: true }),
  ).toBeVisible();
  await section.screenshot({
    path: testInfo.outputPath("openai-catalog-loaded.png"),
  });
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

for (const source of [
  {
    provider: "OpenAI",
    label: "ChatGPT · Browser Fixture",
    accountId: `ma1:${"1".repeat(32)}`,
    modelId: "chatgpt-fixture-model",
  },
  {
    provider: "xAI",
    label: "xAI 设备码 · browser-xai@example.com",
    accountId: `ma1:${"2".repeat(32)}`,
    modelId: "grok-subscription-fixture-1",
  },
]) {
  for (const target of [
    { app: "claude", name: "Claude Code" },
    { app: "codex", name: "Codex" },
    { app: "grokbuild", name: "Grok Build" },
    { app: "opencode", name: "OpenCode" },
  ]) {
    test(`${target.name} ${source.provider}: explicit binding retains target semantics and never writes WorkBuddy`, async ({
      page,
    }) => {
      await installRichTauriFeatureFixture(page);
      await openRendererPage(page, `/models?target=${target.app}`);
      const section = page.getByRole("region", { name: "账号订阅设置" });
      const draft = target.app === "codex";
      const action = draft ? "保存 Codex 订阅配置" : `应用到 ${target.name}`;
      await expect(
        section.getByRole("button", { name: action, exact: true }),
      ).toBeDisabled();
      await section
        .getByRole("radio", { name: source.label, exact: true })
        .check();
      await section
        .getByRole("button", { name: source.modelId, exact: true })
        .click();
      await section.getByRole("button", { name: action, exact: true }).click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: draft ? "确认保存" : "确认应用",
          exact: true,
        })
        .click();
      await expect(
        section.getByText(
          draft ? /当前请求来源尚未切换/ : `已将账号订阅应用到 ${target.name}`,
        ),
      ).toBeVisible();
      const calls = await featureFixtureCalls(page);
      const command =
        target.app === "opencode"
          ? "bind_opencode_managed_proxy"
          : "bind_managed_proxy_provider";
      expect(
        calls.filter((c) => c.command === command).map((c) => c.payload),
      ).toEqual([
        {
          request: {
            accountId: source.accountId,
            modelId: source.modelId,
            ...(target.app === "opencode"
              ? { expectedRevision: null }
              : { app: target.app }),
          },
        },
      ]);
      expect(
        calls.filter((c) =>
          [
            "create_workbuddy_subscription_plan",
            "create_workbuddy_save_plan",
            "save_workbuddy_models",
            "apply_change_plan",
          ].includes(c.command),
        ),
      ).toEqual([]);
    });
  }
}
