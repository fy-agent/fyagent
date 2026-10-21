import { expect, test, type Locator } from "@playwright/test";

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

const CONNECTION_RESULTS_TEST =
  "keeps connection results compact and account actions on the first screen";
const CONNECT_ENTRY_TEST =
  "keeps the main connect entry hittable on the first screen of both views";

test.beforeEach(async ({ page }, testInfo) => {
  await installRichTauriFeatureFixture(page, {
    authSummaryScenario:
      testInfo.title ===
      "summarizes mixed connection states at compact desktop width"
        ? "mixed"
        : undefined,
    authConnectionFailOnce:
      testInfo.title === CONNECTION_RESULTS_TEST ||
      testInfo.title === CONNECT_ENTRY_TEST
        ? ["opencode"]
        : undefined,
    authCodexConnectable: testInfo.title === CONNECT_ENTRY_TEST,
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

test(CONNECTION_RESULTS_TEST, async ({ page }, testInfo) => {
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/auth?view=connections&consumer=opencode");

  const viewports = [
    { width: 900, height: 600 },
    { width: 1232, height: 700 },
  ];
  const results = page.getByRole("region", { name: "软件连接操作结果" });
  const detail = page.getByRole("region", {
    name: "OpenCode Desktop 连接详情",
  });
  const detailHeading = detail.getByRole("heading", {
    name: "OpenCode Desktop",
  });
  const detailAction = detail.getByRole("button", {
    name: "断开",
    exact: true,
  });
  const listItems = [
    page.getByTestId("managed-auth-consumer-codex"),
    page.getByTestId("managed-auth-consumer-opencode"),
  ];
  const expectReachable = async (control: Locator) => {
    await expect(control).toBeInViewport({ ratio: 0.95 });
    await control.click({ trial: true });
  };
  const expectFirstScreen = async ({ expanded = false } = {}) => {
    await detailHeading.scrollIntoViewIfNeeded();
    await listItems[0].scrollIntoViewIfNeeded();
    for (const control of [
      page.getByRole("button", { name: "添加账号" }).first(),
      page.getByRole("tab", { selected: true }),
      page.getByRole("heading", { name: "软件连接", exact: true }),
      detailHeading,
      ...(expanded ? [] : listItems),
    ])
      await expectReachable(control);
    // Expanded details cap their own height; the panes below scroll
    // independently instead of the page.
    const pageHeading = page.getByRole("heading", { name: "账号与认证" });
    const headingTop = (await pageHeading.boundingBox())!.y;
    for (const control of [...listItems, detailAction]) {
      await control.scrollIntoViewIfNeeded();
      await expectReachable(control);
    }
    expect((await pageHeading.boundingBox())!.y).toBe(headingTop);
    await expectNoHorizontalOverflow(page);
    await detailHeading.scrollIntoViewIfNeeded();
    await page
      .getByRole("heading", { name: "软件连接", exact: true })
      .scrollIntoViewIfNeeded();
  };
  const confirmDisconnect = async () => {
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("button", { name: "断开", exact: true }).click();
    await expect(dialog).toHaveCount(0);
  };
  const disconnect = async (consumer: string, label: string) => {
    await page.getByTestId(`managed-auth-consumer-${consumer}`).click();
    await page
      .getByRole("region", { name: `${label} 连接详情` })
      .getByRole("button", { name: "断开", exact: true })
      .click();
    await confirmDisconnect();
  };

  for (const viewport of viewports) {
    await page.setViewportSize(viewport);
    await expectFirstScreen();
    await page.screenshot({
      path: testInfo.outputPath(
        `connection-results-before-${viewport.width}x${viewport.height}.png`,
      ),
    });
  }

  await disconnect("codex", "Codex");
  await disconnect("opencode", "OpenCode Desktop");
  const failed = page.getByRole("article", {
    name: /^OpenCode Desktop · .+ 操作结果$/u,
  });
  const completed = page.getByRole("article", {
    name: "Codex · OpenAI 操作结果",
  });
  const toggle = results.getByRole("button", {
    name: /查看已完成|收起已完成/u,
  });
  await expect(results).toContainText("1 个需要处理，1 个已完成");
  await expect(failed).toContainText("检测到软件在 FyAgent 外部修改了登录信息");

  for (const viewport of viewports) {
    await page.setViewportSize(viewport);
    const size = `${viewport.width}x${viewport.height}`;

    await expect(completed).toBeHidden();
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    await expect(
      failed.getByRole("button", { name: "重试此连接" }),
    ).toBeInViewport({ ratio: 1 });
    await expectFirstScreen();
    await page.screenshot({
      path: testInfo.outputPath(`connection-results-collapsed-${size}.png`),
    });

    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    await expect(completed).toContainText("此连接操作已完成并回读。");
    const box = await results.boundingBox();
    expect(box!.height).toBeLessThanOrEqual(viewport.height * 0.38 + 1);
    await expectFirstScreen({ expanded: true });
    await page.screenshot({
      path: testInfo.outputPath(`connection-results-expanded-${size}.png`),
    });
    await toggle.click();
    await expect(toggle).toBeFocused();
  }

  await results.getByRole("button", { name: "关闭已完成" }).click();
  await expect(completed).toHaveCount(0);
  await expect(failed).toBeVisible();
  await expect(page.getByRole("tab", { selected: true })).toBeFocused();

  await failed.getByRole("button", { name: "重试此连接" }).click();
  await confirmDisconnect();
  await expect(results).toContainText("1 个已完成");
  await expect(failed).toBeHidden();
  await expectHealthyPage(page, health);
});

test(CONNECT_ENTRY_TEST, async ({ page }, testInfo) => {
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/auth?view=connections&consumer=grokbuild");
  const disconnect = async (consumer: string, label: string) => {
    await page.getByTestId(`managed-auth-consumer-${consumer}`).click();
    await page
      .getByRole("region", { name: `${label} 连接详情` })
      .getByRole("button", { name: "断开", exact: true })
      .click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("button", { name: "断开", exact: true }).click();
    await expect(dialog).toHaveCount(0);
  };
  await disconnect("grokbuild", "Grok Build");
  await disconnect("opencode", "OpenCode Desktop");
  const results = page.getByRole("region", { name: "软件连接操作结果" });
  await expect(results).toContainText("1 个需要处理，1 个已完成");
  await expect(
    page.getByRole("article", { name: /^OpenCode Desktop · .+ 操作结果$/u }),
  ).toContainText("检测到软件在 FyAgent 外部修改了登录信息");
  // Checked right after the operations, while a toast they raised would
  // still show: targeted connection results live in the page instead.
  await expect(page.locator(".fy-toast")).toHaveCount(0);

  const viewports = [
    { width: 900, height: 600 },
    { width: 1232, height: 700 },
  ];
  // No scrolling: the entry must sit fully inside the first screen and be
  // the topmost element at its centre.
  const expectHittable = async (control: Locator, viewportHeight: number) => {
    await expect(control).toBeInViewport({ ratio: 1 });
    const box = (await control.boundingBox())!;
    expect(box.y + box.height).toBeLessThanOrEqual(viewportHeight);
    const hit = await control.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      const target = document.elementFromPoint(
        rect.x + rect.width / 2,
        rect.y + rect.height / 2,
      );
      return target !== null && element.contains(target);
    });
    expect(hit).toBe(true);
    // Toasts ignore pointer events, so hit-testing alone cannot see them.
    const covered = await control.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      return [...document.querySelectorAll(".fy-toast")].some((toast) => {
        const other = toast.getBoundingClientRect();
        return (
          other.left < rect.right &&
          rect.left < other.right &&
          other.top < rect.bottom &&
          rect.top < other.bottom
        );
      });
    });
    expect(covered).toBe(false);
    await control.click({ trial: true });
  };
  const expectConnectDialog = async (entry: Locator) => {
    await entry.click();
    const dialog = page.getByRole("dialog", { name: "连接 Codex 账号" });
    await expect(dialog).toBeVisible();
    await dialog.getByRole("button", { name: "取消" }).click();
    await expect(dialog).toHaveCount(0);
  };

  const capture = (name: string) =>
    page.screenshot({ path: testInfo.outputPath(name) });
  const connectionEntry = page
    .getByRole("region", { name: "Codex 连接详情" })
    .getByRole("button", { name: "连接账号", exact: true });
  const accountEntry = page
    .getByRole("region", { name: "连接到软件" })
    .getByRole("button", { name: "用此账号连接", exact: true });

  await page.getByTestId("managed-auth-consumer-codex").click();
  for (const viewport of viewports) {
    await page.setViewportSize(viewport);
    await expectHittable(connectionEntry, viewport.height);
    await capture(
      `connect-entry-connections-${viewport.width}x${viewport.height}.png`,
    );
  }
  await page.getByRole("tab", { name: /^账号/u }).click();
  for (const viewport of viewports) {
    await page.setViewportSize(viewport);
    await expectHittable(accountEntry, viewport.height);
    await capture(
      `connect-entry-accounts-${viewport.width}x${viewport.height}.png`,
    );
  }

  await expectConnectDialog(accountEntry);
  await page.getByRole("tab", { name: /^软件连接/u }).click();
  await expectConnectDialog(connectionEntry);
  await expect(results).toContainText("1 个需要处理");
  await expectHealthyPage(page, health);
});

test("reads the native Codex catalog only on request and labels its source", async ({
  page,
}, testInfo) => {
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/auth?consumer=codex&view=connections");

  const detail = page.getByRole("region", { name: "Codex 连接详情" });
  const catalog = detail.getByRole("region", { name: "本机 Codex 模型目录" });
  await expect(catalog).toBeVisible();
  const nativeCalls = async () =>
    (await featureFixtureCalls(page))
      .filter((call) => call.command === "managed_auth_get_codex_native_models")
      .map((call) => call.payload);
  expect(await nativeCalls()).toEqual([]);

  await catalog.getByRole("button", { name: "获取模型", exact: true }).click();
  // The fixture's Codex requests go to a custom provider, so a matched
  // ChatGPT login alone must not earn the official label.
  await expect(catalog).toContainText(
    "本机 Codex app-server 返回的目录 · Codex CLI 0.0.0-fixture",
  );
  await expect(catalog).not.toContainText("原生官方");
  await expect(catalog).toContainText("未确认这些模型对应此 ChatGPT 账号");
  const models = catalog.getByRole("list");
  await expect(models.getByRole("listitem")).toHaveCount(2);
  await expect(models).toContainText("Fixture Codex");
  await expect(models).toContainText("默认");
  await expect(catalog).not.toContainText("DeepSeek");
  expect(await nativeCalls()).toEqual([{ accountId: `ma1:${"1".repeat(32)}` }]);

  await catalog.scrollIntoViewIfNeeded();
  await expectNoHorizontalOverflow(page);
  const size = page.viewportSize();
  await page.screenshot({
    path: testInfo.outputPath(
      `codex-native-catalog-${size?.width}x${size?.height}.png`,
    ),
  });
  await expectHealthyPage(page, health);
});
