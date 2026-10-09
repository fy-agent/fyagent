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

test.beforeEach(async ({ page }, testInfo) => {
  await installRichTauriFeatureFixture(page, {
    authSummaryScenario:
      testInfo.title ===
      "summarizes mixed connection states at compact desktop width"
        ? "mixed"
        : undefined,
  });
});

test("renders account identity, software connection and current request source as separate fields", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/auth");

  await expect(page.getByRole("heading", { name: "账号与认证" })).toBeVisible();
  const accountDetail = page.getByRole("region", {
    name: "browser-fixture@example.com 账号详情",
  });
  await expect(accountDetail).toContainText("OpenAI · ChatGPT Plus");
  const codexCard = accountDetail
    .getByRole("heading", { name: "Codex", exact: true })
    .locator("xpath=ancestor::article[1]");
  await expect(codexCard).toContainText("DeepSeek API");
  await expect(codexCard.getByText("Codex", { exact: true })).toHaveCount(1);
  await expect(codexCard).toContainText("已保留");
  await expect(codexCard).toContainText("由 Codex 自动续期");
  await expect(page.getByText(/access[_ ]?token/iu)).toHaveCount(0);
  await expect(page.getByText(/refresh[_ ]?token/iu)).toHaveCount(0);
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("opens a Codex deep link and keeps return context while switching views", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openRendererPage(
    page,
    "/auth?consumer=codex&view=connections&agentReturn=codex&agentSection=models",
  );

  const detail = page.getByRole("region", { name: "Codex 连接详情" });
  await expect(detail).toContainText("OpenAI · browser-fixture@example.com");
  await expect(detail).toContainText("DeepSeek API");
  await expect(detail).toContainText("官方登录");
  await expect(detail).toContainText("已保留");
  await page.getByRole("tab", { name: /账号 2/ }).click();
  await expect(page).toHaveURL(/agentReturn=codex/);
  await expect(page).toHaveURL(/view=accounts/);
  await expectHealthyPage(page, health);
});

test("completes the device-code interaction with official-host copy and a cancellable backend session", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/auth?consumer=codex&view=connections");

  await page.getByRole("button", { name: "添加账号" }).click();
  const dialog = page.getByRole("dialog", { name: "添加官方账号" });
  await dialog.getByLabel("设备码登录").check();
  await dialog.getByRole("button", { name: "下一步" }).click();
  await expect(dialog).toContainText("auth.openai.com / chatgpt.com");
  await dialog.getByRole("button", { name: "继续" }).click();
  await expect(dialog.getByText("BROWSER-2026")).toBeVisible();
  await expect(dialog).toContainText("auth.openai.com");
  await expect(dialog).not.toContainText("localhost");
  await dialog.getByRole("button", { name: "取消登录" }).click();
  await expect(dialog).toContainText("登录已取消");

  const calls = await featureFixtureCalls(page);
  expect(
    calls.some(
      (call) =>
        call.command === "managed_auth_start_login" &&
        (call.payload.request as { method?: string }).method === "device_code",
    ),
  ).toBe(true);
  expect(
    calls.some((call) => call.command === "managed_auth_cancel_login"),
  ).toBe(true);
  await expectHealthyPage(page, health);
});

test("uses one-pane mobile navigation without horizontal overflow", async ({
  page,
}) => {
  await page.setViewportSize({ width: 740, height: 640 });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/auth");

  const pageRoot = page.getByTestId("auth-page");
  await expect(pageRoot).toHaveAttribute("data-mobile-detail", "false");
  await expect(
    page.getByRole("region", { name: "官方账号列表" }),
  ).toBeVisible();
  await page.getByTestId(`managed-auth-account-ma1:${"1".repeat(32)}`).click();
  await expect(pageRoot).toHaveAttribute("data-mobile-detail", "true");
  await expect(
    page.getByRole("button", { name: "返回账号列表" }),
  ).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await page.getByRole("button", { name: "返回账号列表" }).click();
  await expect(pageRoot).toHaveAttribute("data-mobile-detail", "false");
  await expectHealthyPage(page, health);
});

test("restores focus to add account after Escape closes the login dialog", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/auth");

  const addAccount = page.getByRole("button", { name: "添加账号" }).first();
  await addAccount.click();
  const dialog = page.getByRole("dialog", { name: "添加官方账号" });
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "取消" }).focus();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(addAccount).toBeFocused();
  await expectHealthyPage(page, health);
});

test("summarizes mixed connection states at compact desktop width", async ({
  page,
}, testInfo) => {
  await page.setViewportSize({ width: 900, height: 600 });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/auth?view=connections");
  await expect(page.getByRole("tab", { name: "软件连接 1/6" })).toBeVisible();
  const summary = page.getByTestId("managed-auth-overview-summary");
  await expect(summary).toHaveText("需要重新登录");

  const codexItem = page.getByTestId("managed-auth-consumer-codex");
  await expect(codexItem).toContainText("部分连接 1/3");
  await expect(codexItem).toContainText("账号已保存");
  await expect(codexItem).toContainText("正在确认");
  await codexItem.click();
  const codexDetail = page.getByRole("region", { name: "Codex 连接详情" });
  await expect(codexDetail.getByText("已连接", { exact: true })).toBeVisible();
  await expect(
    codexDetail.getByText("账号已保存", { exact: true }),
  ).toBeVisible();
  await expect(
    codexDetail.getByText("正在确认", { exact: true }),
  ).toBeVisible();

  await page.getByTestId("managed-auth-consumer-fyagent_proxy").click();
  await expect(
    page.getByRole("region", { name: "FyAgent Local Proxy 连接详情" }),
  ).toContainText("等待重启");
  await page.getByTestId("managed-auth-consumer-grokbuild").click();
  await expect(
    page.getByRole("region", { name: "Grok Build 连接详情" }),
  ).toContainText("需要重新登录");
  await page.getByTestId("managed-auth-consumer-opencode").click();
  const openCodeDetail = page.getByRole("region", {
    name: "OpenCode Desktop 连接详情",
  });
  await expect(openCodeDetail).toContainText("状态不可用");
  await expect(
    openCodeDetail.getByText("暂时无法确认", { exact: true }),
  ).toBeVisible();
  await expect(summary).toHaveAttribute("data-attention", "true");
  await expectNoHorizontalOverflow(page);
  const box = await summary.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(900);
  await page.screenshot({
    path: testInfo.outputPath("i18-mixed-summary-900x600.png"),
  });
  await expectHealthyPage(page, health);
});
