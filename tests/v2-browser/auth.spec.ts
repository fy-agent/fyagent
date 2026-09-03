import { expect, test } from "@playwright/test";

import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openV2Page,
} from "./support";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "./support/features";

test.beforeEach(async ({ page }) => {
  await installRichTauriFeatureFixture(page);
});

test("renders account identity, software connection and current request source as separate fields", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/auth");

  await expect(page.getByRole("heading", { name: "账号与认证" })).toBeVisible();
  const accountDetail = page.getByRole("region", {
    name: "browser-fixture@example.com 账号详情",
  });
  await expect(accountDetail).toContainText("OpenAI · ChatGPT Plus");
  const codexCard = accountDetail
    .getByRole("heading", { name: "Codex", exact: true })
    .locator("xpath=ancestor::article[1]");
  await expect(codexCard).toContainText("DeepSeek API");
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
  await openV2Page(
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
  await openV2Page(page, "/auth?consumer=codex&view=connections");

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
  await openV2Page(page, "/auth");

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
