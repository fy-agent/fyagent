import { expect, test, type Locator } from "@playwright/test";

import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openV2Page,
  requiredBox,
} from "./support";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "./support/features";

const agentOrder = [
  "QoderWork CN",
  "TRAE Work CN",
  "WorkBuddy",
  "Grok Build",
  "Codex",
  "Claude Code",
  "OpenCode",
] as const;

const modelTargetOrder = [
  "QoderWork CN",
  "TRAE Work CN",
  "WorkBuddy",
  "Grok Build",
  "Codex",
  "Claude Code",
  "OpenCode",
] as const;

const modelTargetIconSources = [
  "qoderwork.png",
  "trae-work.png",
  "workbuddy.png",
  "inline-svg",
  "inline-svg",
  "inline-svg",
  "inline-svg",
] as const;

function agentSelector(page: Parameters<typeof openV2Page>[0]): Locator {
  return page.getByRole("region", { name: "AI 软件目录" });
}

function agentItem(
  page: Parameters<typeof openV2Page>[0],
  name: (typeof agentOrder)[number],
): Locator {
  return agentSelector(page)
    .locator(".fy-agent-directory-card")
    .filter({ has: page.getByRole("heading", { name, exact: true }) });
}

test("Agent directory keeps exact native order and accessible configuration entry points", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  await expect(page.getByTestId("agents-page")).toBeVisible();
  await expect(
    agentSelector(page).locator(".fy-agent-directory-card"),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "开始扫描" }).click();
  await expect(page.getByRole("button", { name: "重新扫描" })).toBeEnabled();

  const items = agentSelector(page).locator(".fy-agent-directory-card");
  await expect(items).toHaveCount(7);
  expect(
    await items.evaluateAll((elements) =>
      elements.map(
        (element) => element.querySelector("h2")?.textContent?.trim() ?? "",
      ),
    ),
  ).toEqual([...agentOrder]);
  expect(
    await items.evaluateAll((elements) =>
      elements.map((element) => ({
        tagName: element.tagName,
        configureTabIndex:
          element.querySelector<HTMLButtonElement>("button")?.tabIndex ?? -1,
      })),
    ),
  ).toEqual(
    agentOrder.map(() => ({
      tagName: "ARTICLE",
      configureTabIndex: 0,
    })),
  );

  await expect(agentSelector(page).getByText("尚未扫描")).toHaveCount(0);
  await expect(agentSelector(page).getByText("未确认")).toHaveCount(0);
  await expect(agentSelector(page).getByText("未安装")).toHaveCount(0);
  await expect(agentSelector(page).getByText("查看完整介绍")).toHaveCount(0);
  const traeDetailFrame = agentItem(page, "TRAE Work CN").locator(
    '[data-size="detail"]',
  );
  expect(
    await traeDetailFrame.evaluate((frame) => {
      const image = frame.querySelector("img") as HTMLImageElement;
      return {
        naturalWidth: image.naturalWidth,
        frameWidth: frame.getBoundingClientRect().width,
        frameHeight: frame.getBoundingClientRect().height,
        artworkWidth: image.getBoundingClientRect().width,
        artworkHeight: image.getBoundingClientRect().height,
      };
    }),
  ).toEqual({
    naturalWidth: 48,
    frameWidth: 64,
    frameHeight: 64,
    artworkWidth: 48,
    artworkHeight: 48,
  });
  await agentItem(page, "TRAE Work CN")
    .getByRole("button", { name: "进行配置" })
    .click();
  await expect(page).toHaveURL(/#\/agents\?target=trae-work&section=models$/);
  await expect(
    page.getByRole("region", { name: "TRAE Work CN 配置" }),
  ).toBeVisible();

  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("Agent directory and Models keep their responsive 760px boundaries", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");
  const ensureScanned = async () => {
    const scanBtn = page.getByRole("button", { name: /^(开始扫描|重新扫描)$/ });
    if (await scanBtn.isVisible()) {
      await scanBtn.click();
      await expect(
        page.getByRole("button", { name: "重新扫描" }),
      ).toBeEnabled();
    }
  };

  await ensureScanned();

  const desktopCard = agentItem(page, "QoderWork CN");
  await expect(desktopCard).toBeVisible();
  expect(
    await desktopCard.evaluate(
      (card) =>
        getComputedStyle(card).gridTemplateColumns.trim().split(/\s+/).length,
    ),
  ).toBe(3);
  const desktopFrame = await requiredBox(
    desktopCard.locator('[data-size="detail"]'),
    "Agent directory artwork",
  );
  expect(desktopFrame.width).toBe(64);
  expect(desktopFrame.height).toBe(64);

  await page.setViewportSize({ width: 760, height: 900 });
  await openV2Page(page, "/agents");
  await ensureScanned();
  const stackedAgentCard = agentItem(page, "QoderWork CN");
  expect(
    await stackedAgentCard.evaluate(
      (card) =>
        getComputedStyle(card).gridTemplateColumns.trim().split(/\s+/).length,
    ),
  ).toBe(2);
  const stackedAgentBox = await requiredBox(
    stackedAgentCard,
    "760px Agent card",
  );
  const stackedAgentActionBox = await requiredBox(
    stackedAgentCard.getByRole("button", { name: "进行配置" }),
    "760px Agent action",
  );
  expect(stackedAgentActionBox.width).toBeGreaterThan(
    stackedAgentBox.width - 40,
  );

  await page.setViewportSize({ width: 761, height: 900 });
  await openV2Page(page, "/agents");
  await ensureScanned();
  expect(
    await agentItem(page, "QoderWork CN").evaluate(
      (card) =>
        getComputedStyle(card).gridTemplateColumns.trim().split(/\s+/).length,
    ),
  ).toBe(3);

  await openV2Page(page, "/models?target=opencode");
  await expect(page.getByRole("heading", { name: "OpenCode" })).toBeVisible();
  const desktopSplit = page.locator(".fy-split-panes");
  await expect(desktopSplit).toBeVisible();
  expect(
    await desktopSplit.evaluate((split) => {
      const computed = getComputedStyle(split).gridTemplateColumns.trim();
      return computed.split(/\s+/).length;
    }),
  ).toBe(3);

  await page.setViewportSize({ width: 760, height: 900 });
  await openV2Page(page, "/models?target=opencode");
  const stackedSplit = page.locator(".fy-split-panes");
  await expect(stackedSplit).toBeVisible();
  expect(
    await stackedSplit.evaluate((split) => {
      const computed = getComputedStyle(split).gridTemplateColumns.trim();
      return computed.split(/\s+/).length;
    }),
  ).toBe(1);
  await expect(
    page.getByRole("separator", { name: "调整目录与详情的宽度" }),
  ).toBeHidden();

  await page.setViewportSize({ width: 761, height: 900 });
  await openV2Page(page, "/models?target=opencode");
  await expect(
    page.getByRole("separator", { name: "调整目录与详情的宽度" }),
  ).toBeVisible();

  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("Agent directory keeps cards clean with no prototype-violating links or disclosures", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");
  await page.getByRole("button", { name: "开始扫描" }).click();
  await expect(page.getByRole("button", { name: "重新扫描" })).toBeEnabled();

  await expect(page.getByText("查看完整介绍")).toHaveCount(0);
  await expect(page.getByText("上次扫描：")).toHaveCount(0);
  await expect(page.getByText("未确认")).toHaveCount(0);
  await expect(page.getByText("未安装")).toHaveCount(0);
  await expect(page.getByText("AI 软件配置")).toHaveCount(0);
  await expectHealthyPage(page, health);
});

test("Codex configuration loads safely without unsolicited mutation", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents?target=codex&section=models");

  await expect(page.getByRole("region", { name: "Codex 配置" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "当前模型" })).toBeVisible();
  await expect(
    page.getByRole("button", { name: "进入模型管理" }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "返回" })).toBeVisible();
  await expect(page.getByText("单 Agent 配置")).toHaveCount(0);
  await expect(page.getByText("安装、登录与启动能力")).toHaveCount(0);

  const calls = await featureFixtureCalls(page);
  expect(
    calls.some((call) =>
      call.command.startsWith("codex_desktop_start_install"),
    ),
  ).toBe(false);
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("Agent directory does not observe WorkBuddy or Provider summaries before configuration", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, {
    observationFailure: "workbuddy",
  });
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  let commands = (await featureFixtureCalls(page)).map((call) => call.command);
  expect(commands).toContain("get_agent_catalog");
  expect(commands).not.toContain("get_workbuddy_status");
  expect(commands).not.toContain("get_providers");

  await page.getByRole("button", { name: "开始扫描" }).click();
  await expect(page.getByRole("button", { name: "重新扫描" })).toBeEnabled();

  commands = (await featureFixtureCalls(page)).map((call) => call.command);
  expect(commands).not.toContain("get_workbuddy_status");
  expect(commands).not.toContain("get_provider_summary");
  expect(commands).not.toContain("get_providers");
  expect(commands).not.toContain("get_current_provider");
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("Agent catalog failure stays explicit and never falls back to a static support list", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, { catalogFailure: true });
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  await expect(
    page.getByRole("heading", { name: "无法加载 Agent 目录" }),
  ).toBeVisible({ timeout: 10_000 });
  await expect(page.getByText("暂时无法获取应用信息，请重试。")).toBeVisible();
  await expect(agentSelector(page)).toHaveCount(0);
  expect(
    (await featureFixtureCalls(page)).filter(
      (call) => call.command === "get_agent_catalog",
    ).length,
  ).toBeGreaterThanOrEqual(1);

  await expectHealthyPage(page, health);
});

test("Models keeps seven targets and saves TRAE models natively", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/models");

  const modelPage = page.getByTestId("models-page");
  await expect(modelPage).toBeVisible();
  const targetButtons = modelPage.locator('[data-testid^="model-target-"]');
  await expect(targetButtons).toHaveCount(7);
  expect(
    await targetButtons.evaluateAll((elements) =>
      elements.map(
        (element) => element.querySelector("strong")?.textContent?.trim() ?? "",
      ),
    ),
  ).toEqual([...modelTargetOrder]);
  const targetIcons = targetButtons.locator("img");
  await expect(targetIcons).toHaveCount(7);
  expect(
    await targetIcons.evaluateAll((elements) =>
      elements.map((element) => ({
        source: (element as HTMLImageElement).src.startsWith(
          "data:image/svg+xml",
        )
          ? "inline-svg"
          : new URL((element as HTMLImageElement).src).pathname
              .split("/")
              .at(-1),
        alt: element.getAttribute("alt"),
        ariaHidden: element.getAttribute("aria-hidden"),
        local:
          (element as HTMLImageElement).src.startsWith("data:image/svg+xml") ||
          new URL((element as HTMLImageElement).src).origin === location.origin,
      })),
    ),
  ).toEqual(
    modelTargetIconSources.map((source) => ({
      source,
      alt: "",
      ariaHidden: "true",
      local: true,
    })),
  );
  await expect(page.getByTestId("model-target-qoderwork")).toHaveAttribute(
    "aria-current",
    "true",
  );
  await expect(
    page.getByRole("region", { name: "QoderWork CN 模型设置" }),
  ).toBeVisible();

  await page.getByTestId("model-target-qoderwork").click();
  await expect(modelPage).toContainText("官方不支持第三方模型配置");
  await expect(page.getByRole("button", { name: "管理 MCP" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "打开官方设置" })).toHaveCount(
    0,
  );
  await page.getByTestId("model-target-trae").click();
  await expect(
    page.getByRole("region", { name: "TRAE Work CN 模型设置" }),
  ).toBeVisible();
  await expect(page.locator("body")).toContainText(
    "自定义模型需在 TRAE Work CN 中添加",
  );
  await expect(page.getByRole("textbox", { name: "API Key" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "拉取模型" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "测试连通" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "保存并应用" })).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "打开 TRAE 官方模型设置" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: /TRAE 当前第三方模型 ID/ }).click();
  await expect(page.getByText("fixture-model")).toBeVisible();
  await page.getByTestId("model-target-qoderwork").click();
  await expect(modelPage).not.toContainText("配置成功");

  const calls = await featureFixtureCalls(page);
  expect(calls.filter((call) => call.command === "open_external")).toEqual([]);
  expect(
    calls.filter((call) => call.command === "get_traework_model_ids"),
  ).toHaveLength(1);
  expect(
    calls.filter((call) => call.command === "fetch_traework_models"),
  ).toHaveLength(0);
  expect(
    calls.filter((call) => call.command === "save_traework_models"),
  ).toHaveLength(0);
  expect(
    calls.filter((call) =>
      [
        "apply_provider_quick_setup_with_result",
        "switch_provider_with_result",
        "save_workbuddy_models",
        "test_traework_model_endpoint",
      ].includes(call.command),
    ),
  ).toEqual([]);

  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("Provider read failure disables writes and remains an unknown observation", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, {
    observationFailure: "codex",
  });
  const health = monitorPageHealth(page);
  await openV2Page(page, "/models?target=codex");

  await expect(page.locator("body")).toContainText(
    "暂时无法读取当前配置，请稍后重试。",
    { timeout: 10_000 },
  );
  await expect(
    page.getByRole("button", { name: "保存并设为当前配置" }),
  ).toBeDisabled();
  await expect(page.locator("body")).not.toContainText("未安装");
  const calls = await featureFixtureCalls(page);
  expect(
    calls.filter((call) =>
      [
        "apply_provider_quick_setup_with_result",
        "switch_provider_with_result",
      ].includes(call.command),
    ),
  ).toEqual([]);

  await expectHealthyPage(page, health);
});

test("WorkBuddy save preview uses Change Plan and does not expose overwrite tokens", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, {
    workBuddySave: "overwrite_then_saved",
  });
  const health = monitorPageHealth(page);
  const apiKey = "browser-workbuddy-secret";
  await openV2Page(page, "/models?target=workbuddy");

  await page.getByLabel("服务地址").fill("https://workbuddy.example.test/v1");
  await page.getByLabel("API Key", { exact: true }).fill(apiKey);
  await page.getByLabel("自定义模型 ID").fill("manual-browser-model");
  await page.getByRole("button", { name: "保存并应用" }).click();
  await expect(page.getByRole("dialog", { name: "保存前确认" })).toBeVisible();
  await expect(page.getByText("将修改")).toBeVisible();
  await page.getByRole("button", { name: "确认保存" }).click();

  await expect(page.getByRole("button", { name: "确认应用" })).toBeVisible();
  await expect(
    page.getByRole("dialog", { name: "确认覆盖已有模型" }),
  ).toHaveCount(0);
  await expect(page.getByRole("button", { name: "取消" })).toHaveCount(0);
  await expect(page.locator("body")).not.toContainText(apiKey);
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue(apiKey);

  await page.getByRole("button", { name: "确认应用" }).click();
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");
  await expect(page.locator("body")).toContainText("WorkBuddy 模型配置已保存");

  const calls = await featureFixtureCalls(page);
  expect(
    calls.filter((call) => call.command === "save_workbuddy_models"),
  ).toEqual([]);
  const createCalls = calls.filter(
    (call) => call.command === "create_workbuddy_save_plan",
  );
  expect(createCalls).toHaveLength(1);
  expect(createCalls[0].payload.request).toMatchObject({
    baseUrl: "https://workbuddy.example.test/v1",
    apiKey,
    manualModelIds: ["manual-browser-model"],
    expectedRevision: "fixture-revision-1",
  });
  expect(createCalls[0].payload.request).not.toHaveProperty("overwriteToken");
  expect(calls.filter((call) => call.command === "apply_change_plan")).toEqual([
    expect.objectContaining({
      command: "apply_change_plan",
      payload: {
        planId: "plan-workbuddy-save",
        planDigest: "c".repeat(64),
      },
    }),
  ]);
  expect(JSON.stringify(calls)).not.toContain("fixture-opaque-overwrite-token");

  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("WorkBuddy write failures stay redacted and clear the submitted credential", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, { workBuddySave: "failure" });
  const health = monitorPageHealth(page);
  const apiKey = "browser-workbuddy-error-secret";
  await openV2Page(page, "/models?target=workbuddy");

  await page.getByLabel("服务地址").fill("https://failure.example.test/v1");
  await page.getByLabel("API Key", { exact: true }).fill(apiKey);
  await page.getByLabel("自定义模型 ID").fill("failure-model");
  await page.getByRole("button", { name: "保存并应用" }).click();
  await page.getByRole("button", { name: "确认保存" }).click();
  await page.getByRole("button", { name: "确认应用" }).click();

  await expect(page.locator("body")).toContainText("保存失败，已恢复原配置");
  await expect(page.locator("body")).not.toContainText(apiKey);
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");
  const calls = await featureFixtureCalls(page);
  expect(
    calls.filter((call) => call.command === "save_workbuddy_models"),
  ).toEqual([]);
  expect(
    calls.filter((call) => call.command === "create_workbuddy_save_plan"),
  ).toHaveLength(1);
  expect(
    calls.filter((call) => call.command === "apply_change_plan"),
  ).toHaveLength(1);

  await expectHealthyPage(page, health);
});

test("WorkBuddy concurrent modification rereads authority instead of claiming success", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, {
    workBuddySave: "concurrent_modification",
  });
  const health = monitorPageHealth(page);
  await openV2Page(page, "/models?target=workbuddy");

  await page.getByLabel("服务地址").fill("https://conflict.example.test/v1");
  await page
    .getByLabel("API Key", { exact: true })
    .fill("browser-conflict-secret");
  await page.getByLabel("自定义模型 ID").fill("conflict-model");
  await page.getByRole("button", { name: "保存并应用" }).click();
  await page.getByRole("button", { name: "确认保存" }).click();
  await page.getByRole("button", { name: "确认应用" }).click();

  await expect(page.locator("body")).toContainText("计划已失效");
  await expect(page.locator("body")).not.toContainText(
    "WorkBuddy 模型配置已保存",
  );
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue(
    "browser-conflict-secret",
  );
  const calls = await featureFixtureCalls(page);
  expect(
    calls.filter((call) => call.command === "save_workbuddy_models"),
  ).toEqual([]);
  expect(
    calls.filter((call) => call.command === "create_workbuddy_save_plan"),
  ).toHaveLength(1);
  expect(
    calls.filter((call) => call.command === "apply_change_plan"),
  ).toHaveLength(1);

  await expectHealthyPage(page, health);
});

test("Codex quick setup locks duplicate submission and sends exact provider payload", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, {
    providerWriteDelayMs: 250,
  });
  const health = monitorPageHealth(page);
  const apiKey = "browser-codex-secret";
  await openV2Page(page, "/models");
  await page.getByTestId("model-target-codex").click();

  await page.getByLabel("配置名称").fill("Browser Codex");
  await page.getByLabel("服务地址").fill("https://codex.example.test/v1");
  await page.getByLabel("API Key", { exact: true }).fill(apiKey);
  await page.getByLabel("模型 ID").fill("gpt-browser");
  const providerPanel = page.getByRole("region", { name: "Codex 模型配置" });
  const submit = providerPanel.locator("button.fy-control-button-primary");
  await submit.click();
  await expect(submit).toBeDisabled();
  await submit.dispatchEvent("click");
  const saveWorkspace = page.getByRole("region", {
    name: "Change Plan Provider 保存",
  });
  const confirm = saveWorkspace.getByRole("button", { name: "确认应用" });
  await expect(confirm).toBeEnabled();
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue(apiKey);

  await expect
    .poll(
      async () =>
        (await featureFixtureCalls(page)).filter(
          (call) => call.command === "create_codex_provider_upsert_plan",
        ).length,
    )
    .toBe(1);
  await confirm.click();
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");
  await expect(page.locator("body")).toContainText(
    "模型设置已保存并设为当前配置",
  );

  const calls = await featureFixtureCalls(page);
  const createCalls = calls.filter(
    (call) => call.command === "create_codex_provider_upsert_plan",
  );
  expect(createCalls).toHaveLength(1);
  expect(createCalls[0].payload).toMatchObject({
    request: {
      name: "Browser Codex",
      baseUrl: "https://codex.example.test/v1",
      apiKey,
      modelId: "gpt-browser",
    },
  });
  const applyCalls = calls.filter(
    (call) => call.command === "apply_change_plan",
  );
  expect(applyCalls).toHaveLength(1);
  expect(Object.keys(applyCalls[0].payload).sort()).toEqual([
    "planDigest",
    "planId",
  ]);
  expect(applyCalls[0].payload).toEqual({
    planId: "plan-codex-upsert",
    planDigest: "a".repeat(64),
  });
  expect(
    calls.filter(
      (call) => call.command === "apply_provider_quick_setup_with_result",
    ),
  ).toEqual([]);
  expect(
    calls.filter((call) => call.command === "switch_provider_with_result"),
  ).toEqual([]);
  expect(
    calls.filter(
      (call) =>
        call.command === "get_provider_summary" && call.payload.app === "codex",
    ).length,
  ).toBeGreaterThanOrEqual(2);

  const rendered = await page.locator("body").innerText();
  expect(rendered).not.toContain(apiKey);
  expect(page.url()).not.toContain(apiKey);
  expect(await page.evaluate(() => Object.values(localStorage))).not.toContain(
    apiKey,
  );
  expect(
    await page.evaluate(() => Object.values(sessionStorage)),
  ).not.toContain(apiKey);
  await expectHealthyPage(page, health);
});

test("Models shared API key reveal toggle stays anchored inside the input", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/models?target=codex");

  const input = page.getByLabel("API Key", { exact: true });
  await input.fill("fixture-secret-".repeat(24));
  const hiddenToggle = page.getByRole("button", { name: "显示 API Key" });
  const inputBox = await requiredBox(input, "Codex API key input");
  const hiddenBox = await requiredBox(hiddenToggle, "hidden secret toggle");
  expect(hiddenBox.x).toBeGreaterThanOrEqual(inputBox.x);
  expect(hiddenBox.x + hiddenBox.width).toBeLessThanOrEqual(
    inputBox.x + inputBox.width + 1,
  );

  await hiddenToggle.click();
  const visibleToggle = page.getByRole("button", { name: "隐藏 API Key" });
  const visibleInputBox = await requiredBox(
    input,
    "visible Codex API key input",
  );
  const visibleBox = await requiredBox(visibleToggle, "visible secret toggle");
  expect(
    Math.abs(visibleBox.x - visibleInputBox.x - (hiddenBox.x - inputBox.x)),
  ).toBeLessThanOrEqual(1);
  expect(
    Math.abs(visibleBox.y - visibleInputBox.y - (hiddenBox.y - inputBox.y)),
  ).toBeLessThanOrEqual(1);
  expect(visibleBox.x + visibleBox.width).toBeLessThanOrEqual(
    visibleInputBox.x + visibleInputBox.width + 1,
  );

  await visibleToggle.click();
  const hiddenAgain = await requiredBox(
    page.getByRole("button", { name: "显示 API Key" }),
    "hidden secret toggle after round trip",
  );
  const hiddenAgainInputBox = await requiredBox(
    input,
    "hidden Codex API key input after round trip",
  );
  expect(
    Math.abs(
      hiddenAgain.x - hiddenAgainInputBox.x - (hiddenBox.x - inputBox.x),
    ),
  ).toBeLessThanOrEqual(1);
  expect(
    Math.abs(
      hiddenAgain.y - hiddenAgainInputBox.y - (hiddenBox.y - inputBox.y),
    ),
  ).toBeLessThanOrEqual(1);
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("Claude quick setup updates its reserved row with exact settings and switches it", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, {
    existingQuickSetup: "claude",
  });
  const health = monitorPageHealth(page);
  const apiKey = "browser-claude-secret";
  await openV2Page(page, "/models?target=claude");

  await page.getByLabel("配置名称").fill("Browser Claude");
  await page.getByLabel("服务地址").fill("https://claude.example.test/v1");
  await page.getByLabel("API Key", { exact: true }).fill(apiKey);
  await page.getByLabel("模型 ID").fill("claude-browser");
  await page.getByRole("button", { name: "保存并设为当前配置" }).click();
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");

  await expect
    .poll(
      async () =>
        (await featureFixtureCalls(page)).filter(
          (call) => call.command === "apply_provider_quick_setup_with_result",
        ).length,
    )
    .toBe(1);
  const calls = await featureFixtureCalls(page);
  const applyCalls = calls.filter(
    (call) => call.command === "apply_provider_quick_setup_with_result",
  );
  expect(applyCalls).toHaveLength(1);
  expect(applyCalls[0].payload).toMatchObject({
    app: "claude",
    request: {
      name: "Browser Claude",
      baseUrl: "https://claude.example.test/v1",
      apiKey,
      modelId: "claude-browser",
    },
  });
  expect(
    calls.filter((call) => call.command === "switch_provider_with_result"),
  ).toEqual([]);
  await expectHealthyPage(page, health);
});

test("Provider atomic failure reports rollback instead of a partial result", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, {
    providerMutation: "switch_failure",
  });
  const health = monitorPageHealth(page);
  await openV2Page(page, "/models?target=codex");

  await page.getByLabel("配置名称").fill("Partial Codex");
  await page.getByLabel("服务地址").fill("https://partial.example.test/v1");
  await page.getByLabel("API Key", { exact: true }).fill("partial-secret");
  await page.getByLabel("模型 ID").fill("partial-model");
  await page.getByRole("button", { name: "保存并设为当前配置" }).click();
  await page
    .getByRole("region", { name: "Change Plan Provider 保存" })
    .getByRole("button", { name: "确认应用" })
    .click();

  await expect(page.locator("body")).toContainText(
    "未能保存设置，已还原之前的状态",
  );
  await expect(page.locator("body")).not.toContainText("partial-secret");
  const calls = await featureFixtureCalls(page);
  expect(
    calls.filter(
      (call) => call.command === "create_codex_provider_upsert_plan",
    ),
  ).toHaveLength(1);
  expect(
    calls.filter((call) => call.command === "apply_change_plan"),
  ).toHaveLength(1);
  expect(
    calls.filter(
      (call) => call.command === "apply_provider_quick_setup_with_result",
    ),
  ).toHaveLength(0);
  expect(
    calls.filter((call) => call.command === "switch_provider_with_result"),
  ).toHaveLength(0);
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");
  await expectHealthyPage(page, health);
});
