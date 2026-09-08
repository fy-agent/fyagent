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

test("saved Grok subscription can be selected for Claude and continued through the Codex source plan", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/models?target=claude");
  let section = page
    .getByRole("region", { name: "Claude Code 模型配置", exact: true })
    .getByRole("region", { name: "SuperGrok 订阅设置" });
  await expect(
    section.getByRole("button", { name: "应用到 Claude Code" }),
  ).toBeDisabled();
  await section.getByRole("radio", { name: "browser-xai@example.com" }).check();
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
    section.getByText("已将 SuperGrok 应用到 Claude Code"),
  ).toBeVisible();
  await expect(section.getByText(/保持 FyAgent 在后台运行/)).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await page.getByRole("button", { name: "Codex", exact: true }).click();
  section = page
    .getByRole("region", { name: "Codex 模型配置", exact: true })
    .getByRole("region", { name: "SuperGrok 订阅设置" });
  await expect(
    section.getByRole("button", { name: "保存 Codex 订阅配置" }),
  ).toBeDisabled();
  await section.getByRole("radio", { name: "browser-xai@example.com" }).check();
  await section
    .getByRole("button", { name: "grok-subscription-fixture-1" })
    .click();
  await section.getByRole("button", { name: "保存 Codex 订阅配置" }).click();
  dialog = page.getByRole("dialog", { name: "确认保存 Codex 订阅配置" });
  await dialog.getByRole("button", { name: "确认保存" }).click();
  await expect(section.getByText(/当前请求来源尚未切换/)).toBeVisible();
  await section.getByRole("button", { name: "继续预览 Codex 配置" }).click();
  await expect(page).toHaveURL(/#\/auth\?consumer=codex&view=connections$/);
  await page.getByRole("combobox").selectOption("subscription-fixture-codex");
  await page.getByRole("button", { name: "预览更改" }).click();
  await page.getByRole("button", { name: "应用更改" }).click();
  await expect(page.getByRole("button", { name: "应用更改" })).toHaveCount(0);

  const calls = await featureFixtureCalls(page);
  expect(
    calls
      .filter((call) => call.command === "bind_xai_managed_provider")
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

test("subscription rejection remains local to its target and exposes the account recovery entry", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, { xaiBindFailure: true });
  await openRendererPage(
    page,
    "/models?target=claude&agentReturn=grokbuild&agentSection=models",
  );
  const section = page.getByRole("region", { name: "SuperGrok 订阅设置" });
  await section.getByRole("radio", { name: "browser-xai@example.com" }).check();
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
  await section.getByRole("button", { name: "管理 Grok 账号" }).click();
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
