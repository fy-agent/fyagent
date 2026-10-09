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

test("Codex aggregation supports membership, default, refresh and return to direct", async ({
  page,
}, testInfo) => {
  await installRichTauriFeatureFixture(page, { aggregation: true });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/models?target=codex");
  await page.getByRole("tab", { name: "聚合", exact: true }).click();
  const panel = page.locator(".fy-aggregation");
  await expect(panel.getByText("当前：直连", { exact: false })).toBeVisible();
  expect(
    (await featureFixtureCalls(page)).filter(
      (call) => call.command === "set_proxy_takeover_for_app",
    ),
  ).toHaveLength(0);

  const first = panel
    .locator(".fy-aggregation-provider")
    .filter({ hasText: "Fixture Codex Current" });
  const second = panel
    .locator(".fy-aggregation-provider")
    .filter({ hasText: "第二家供应商" });
  await first.getByRole("button", { name: "加入聚合" }).click();
  await second.getByRole("button", { name: "加入聚合" }).click();
  await panel.getByRole("button", { name: "启用聚合…" }).click();
  const enable = page.getByRole("dialog", { name: "启用聚合", exact: true });
  await expect(enable.getByLabel("默认供应商")).toHaveValue(
    "fixture-codex-current",
  );
  await enable.getByRole("button", { name: "确认启用" }).click();
  await expect(enable).toBeHidden();
  await expect(panel.getByRole("button", { name: "已启用" })).toBeDisabled();
  await second.getByRole("button", { name: "设为默认供应商" }).click();
  await expect(
    second.getByRole("button", { name: "默认供应商", exact: true }),
  ).toBeDisabled();
  await expect(second.getByRole("button", { name: "移出聚合" })).toBeDisabled();
  await expect(first.getByRole("button", { name: "移出聚合" })).toBeEnabled();

  await panel.getByRole("button", { name: "重启 Codex 命令行服务…" }).click();
  const restart = page.getByRole("dialog", { name: "重启 Codex 命令行服务" });
  await expect(restart).toContainText("重启会中断服务中正在运行的任务");
  expect(
    (await featureFixtureCalls(page)).filter(
      (call) => call.command === "restart_codex_app_server_daemon",
    ),
  ).toHaveLength(0);
  await restart.getByRole("button", { name: "确认重启" }).click();
  await expect(restart).toBeHidden();
  await expect(
    panel.getByRole("button", { name: "重启 Codex 命令行服务…" }),
  ).toHaveCount(0);
  await expectNoHorizontalOverflow(page);
  await page.screenshot({
    path: testInfo.outputPath("mock-ipc-codex-aggregation-enabled.png"),
    fullPage: true,
  });

  await page.getByRole("tab", { name: "直连", exact: true }).click();
  await panel.getByRole("button", { name: "回到直连" }).click();
  await expect(panel.getByText("当前：直连", { exact: false })).toBeVisible();
  const modeCalls = (await featureFixtureCalls(page)).filter(
    (call) => call.command === "set_proxy_takeover_for_app",
  );
  expect(modeCalls.map((call) => call.payload.enabled)).toEqual([true, false]);
  expect(modeCalls[0].payload).toMatchObject({
    appType: "codex",
    stack: true,
    route: "fixture-codex-current",
  });
  await expectHealthyPage(page, health);
});

test("Claude adds an independent supplier with a batch of published models", async ({
  page,
}, testInfo) => {
  await installRichTauriFeatureFixture(page, { aggregation: true });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/models?target=claude");
  await page.getByRole("tab", { name: "聚合", exact: true }).click();
  await page.getByRole("button", { name: "添加供应商", exact: true }).click();
  const editor = page.getByRole("dialog", { name: "添加供应商", exact: true });
  await expect(editor).toHaveAttribute("data-motion-settled", "true");
  await editor.getByLabel("供应商名称").fill("新的聚合供应商");
  await editor.getByLabel("API 地址").fill("https://claude.example.test");
  await editor.getByLabel("API Key", { exact: true }).fill("fixture-new-key");
  await editor.getByRole("button", { name: "获取模型" }).click();
  await editor.getByRole("button", { name: "选择全部搜索结果" }).click();
  await editor.getByRole("button", { name: "添加选中的 2 个模型" }).click();
  await expect(editor).toHaveAttribute("data-motion-settled", "true");
  await editor.getByRole("button", { name: "保存供应商" }).scrollIntoViewIfNeeded();
  await page.screenshot({
    path: testInfo.outputPath("mock-ipc-claude-model-editor.png"),
    fullPage: true,
  });
  await editor.getByRole("button", { name: "保存供应商" }).click();
  await expect(editor).toBeHidden();
  await expect(page.getByText("模型已保存。", { exact: true })).toBeVisible();
  const saved = (await featureFixtureCalls(page)).find(
    (call) => call.command === "add_provider",
  );
  expect(saved?.payload).toMatchObject({
    app: "claude",
    addToLive: false,
    provider: {
      name: "新的聚合供应商",
      meta: {
        stackModels: [
          { model: "gpt-fixture" },
          { model: "gpt-fixture-second" },
        ],
      },
    },
  });
  await expect(
    page
      .locator(".fy-aggregation-provider")
      .filter({ hasText: "新的聚合供应商" })
      .getByRole("button", { name: "加入聚合" }),
  ).toBeEnabled();
  await expectNoHorizontalOverflow(page);
  await page.screenshot({
    path: testInfo.outputPath("mock-ipc-claude-supplier-saved.png"),
    fullPage: true,
  });
  await expectHealthyPage(page, health);
});
