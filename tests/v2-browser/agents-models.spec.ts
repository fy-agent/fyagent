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
  return page.getByRole("region", { name: "Agent 选择" });
}

function agentItem(
  page: Parameters<typeof openV2Page>[0],
  name: (typeof agentOrder)[number],
): Locator {
  return agentSelector(page)
    .locator(".fy-catalog-list-item")
    .filter({ has: page.getByText(name, { exact: true }) });
}

test("Agent catalog keeps exact native order and accessible master-detail selection", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  await expect(page.getByTestId("agents-page")).toBeVisible();
  const items = agentSelector(page).locator(".fy-catalog-list-item");
  await expect(items).toHaveCount(7);
  expect(
    await items.evaluateAll((elements) =>
      elements.map(
        (element) => element.querySelector("strong")?.textContent?.trim() ?? "",
      ),
    ),
  ).toEqual([...agentOrder]);
  expect(
    await items.evaluateAll((elements) =>
      elements.map((element) => ({
        tagName: element.tagName,
        tabIndex: (element as HTMLElement).tabIndex,
      })),
    ),
  ).toEqual(
    agentOrder.map(() => ({
      tagName: "BUTTON",
      tabIndex: 0,
    })),
  );

  await expect(
    items.filter({ has: page.getByText("QoderWork CN") }),
  ).toHaveAttribute("aria-current", "true");
  await expect(
    page.getByRole("region", { name: "QoderWork CN 详情" }),
  ).toBeVisible();
  const qoderDetailArtwork = page
    .getByRole("region", { name: "QoderWork CN 详情" })
    .locator('[data-size="detail"] img');
  await expect(qoderDetailArtwork).toHaveAttribute("alt", "");
  await expect(qoderDetailArtwork).toHaveAttribute("aria-hidden", "true");

  await items.first().focus();
  await page.keyboard.press("Tab");
  await expect(items.nth(1)).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(items.nth(1)).toHaveAttribute("aria-current", "true");
  await expect(
    page.getByRole("region", { name: "TRAE Work CN 详情" }),
  ).toBeVisible();
  const traeDetail = page.getByRole("region", { name: "TRAE Work CN 详情" });
  const traeDetailFrame = traeDetail.locator('[data-size="detail"]');
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
  await expect(items.locator('[aria-current="true"]')).toHaveCount(0);
  await expect(
    agentSelector(page).locator('[aria-current="true"]'),
  ).toHaveCount(1);

  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("Agents and Models share exact catalog geometry, stable gutters, and the 760px stack", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  const agentRail = agentSelector(page);
  const agentRows = agentRail.locator(".fy-catalog-list-item");
  await expect(agentRows).toHaveCount(7);
  const agentRailBox = await requiredBox(agentRail, "Agent catalog rail");
  const agentRowGeometry = await agentRows.evaluateAll((rows) =>
    rows.map((row) => {
      const frame = row.querySelector('[data-size="list"]');
      const rowBox = row.getBoundingClientRect();
      const frameBox = frame?.getBoundingClientRect();
      return {
        rowHeight: rowBox.height,
        frameWidth: frameBox?.width ?? 0,
        frameHeight: frameBox?.height ?? 0,
      };
    }),
  );
  const agentRowHeights = agentRowGeometry.map(({ rowHeight }) => rowHeight);
  expect(Math.min(...agentRowHeights)).toBeGreaterThanOrEqual(56);
  expect(
    Math.max(...agentRowHeights) - Math.min(...agentRowHeights),
  ).toBeLessThanOrEqual(1);
  for (const geometry of agentRowGeometry) {
    expect(geometry.frameWidth).toBe(36);
    expect(geometry.frameHeight).toBe(36);
  }

  const railHeightBeforeSelection = agentRailBox.height;
  await expect(
    page.getByRole("separator", { name: "调整目录与详情的宽度" }),
  ).toBeVisible();
  expect(
    await agentRail.evaluate((el) => getComputedStyle(el).overflowY),
  ).toMatch(/auto|scroll/);
  expect(
    await page
      .locator(".fy-catalog-pane")
      .evaluate((el) => getComputedStyle(el).overflowY),
  ).toMatch(/auto|scroll/);
  await agentItem(page, "Codex").click();
  const railAfterSelection = await requiredBox(
    agentRail,
    "selected Agent rail",
  );
  const paneAfterSelection = await requiredBox(
    page.locator(".fy-catalog-pane"),
    "catalog pane",
  );
  expect(
    Math.abs(railAfterSelection.height - railHeightBeforeSelection),
  ).toBeLessThanOrEqual(1);
  expect(
    Math.abs(paneAfterSelection.height - railAfterSelection.height),
  ).toBeLessThanOrEqual(2);

  await openV2Page(page, "/models");
  const modelRail = page.getByRole("complementary", {
    name: "模型配置目标",
  });
  const modelRows = modelRail.locator(".fy-catalog-list-item");
  await expect(modelRows).toHaveCount(7);
  const modelRailBox = await requiredBox(modelRail, "Models catalog rail");
  expect(Math.abs(modelRailBox.x - agentRailBox.x)).toBeLessThanOrEqual(1);
  expect(Math.abs(modelRailBox.width - agentRailBox.width)).toBeLessThanOrEqual(
    1,
  );
  expect(
    await page
      .getByTestId("content-viewport")
      .evaluate((viewport) => getComputedStyle(viewport).scrollbarGutter),
  ).toContain("stable");

  const modelRowGeometry = await modelRows.evaluateAll((rows) =>
    rows.map((row) => {
      const frame = row.querySelector('[data-size="list"]');
      const rowBox = row.getBoundingClientRect();
      const frameBox = frame?.getBoundingClientRect();
      return {
        rowHeight: rowBox.height,
        frameWidth: frameBox?.width ?? 0,
        frameHeight: frameBox?.height ?? 0,
      };
    }),
  );
  expect(modelRowGeometry).toEqual(agentRowGeometry);

  await page.emulateMedia({ reducedMotion: "reduce" });
  expect(
    await modelRows.first().evaluate((row) => {
      const style = getComputedStyle(row);
      return {
        animationDuration: style.animationDuration,
        transitionDuration: style.transitionDuration,
      };
    }),
  ).toEqual({ animationDuration: "0s", transitionDuration: "0s" });

  await page.setViewportSize({ width: 760, height: 900 });
  await openV2Page(page, "/models");
  const stackedRail = page.getByRole("complementary", {
    name: "模型配置目标",
  });
  const stackedDetail = page.getByRole("region", {
    name: "QoderWork CN 模型设置",
  });
  const stackedRailBox = await requiredBox(stackedRail, "760px rail");
  const stackedDetailBox = await requiredBox(stackedDetail, "760px detail");
  expect(Math.abs(stackedRailBox.x - stackedDetailBox.x)).toBeLessThanOrEqual(
    1,
  );
  expect(stackedDetailBox.y).toBeGreaterThan(stackedRailBox.y);
  await expect(
    page.getByRole("separator", { name: "调整目录与详情的宽度" }),
  ).toHaveCount(0);

  await page.setViewportSize({ width: 761, height: 900 });
  await openV2Page(page, "/models");
  const splitRailBox = await requiredBox(
    page.getByRole("complementary", { name: "模型配置目标" }),
    "761px rail",
  );
  const splitDetailBox = await requiredBox(
    page.getByRole("region", { name: "QoderWork CN 模型设置" }),
    "761px detail",
  );
  expect(splitDetailBox.x).toBeGreaterThan(splitRailBox.x + splitRailBox.width);
  await expect(
    page.getByRole("separator", { name: "调整目录与详情的宽度" }),
  ).toBeVisible();

  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("Agent catalog links invoke exact official URLs and Codex has no external action", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  const qoderDetail = page.getByRole("region", {
    name: "QoderWork CN 详情",
  });
  await qoderDetail
    .getByRole("button", { name: "打开 QoderWork 官方页面" })
    .click();

  await agentItem(page, "TRAE Work CN").click();
  const traeDetail = page.getByRole("region", { name: "TRAE Work CN 详情" });
  await expect(
    traeDetail.getByRole("button", { name: "启动应用" }),
  ).toHaveCount(0);
  await traeDetail
    .getByRole("button", { name: "打开 TRAE Work CN 官方页面" })
    .click();

  await agentItem(page, "WorkBuddy").click();
  await page
    .getByRole("region", { name: "WorkBuddy 详情" })
    .getByRole("button", { name: "打开 WorkBuddy 官方页面" })
    .click();

  await agentItem(page, "Grok Build").click();
  await page
    .getByRole("region", { name: "Grok Build 详情" })
    .getByRole("button", { name: "打开 Grok Build 官方页面" })
    .click();

  await agentItem(page, "Claude Code").click();
  const claudeDetail = page.getByRole("region", {
    name: "Claude Code 详情",
  });
  await claudeDetail
    .getByRole("button", { name: "打开 Claude Code CLI 官网" })
    .click();
  await claudeDetail
    .getByRole("button", { name: "打开 Claude Desktop 官网" })
    .click();

  await agentItem(page, "Codex").click();
  const codexDetail = page.getByRole("region", { name: "Codex 详情" });
  await expect(codexDetail.getByRole("button", { name: /官方/ })).toHaveCount(
    0,
  );

  await expect
    .poll(async () =>
      (await featureFixtureCalls(page)).filter(
        (call) => call.command === "open_external",
      ),
    )
    .toEqual([
      {
        command: "open_external",
        payload: { url: "https://qoder.com.cn/qoderwork" },
      },
      {
        command: "open_external",
        payload: { url: "https://www.trae.cn/sem-work" },
      },
      {
        command: "open_external",
        payload: { url: "https://www.workbuddy.cn/" },
      },
      {
        command: "open_external",
        payload: { url: "https://x.ai/grok" },
      },
      {
        command: "open_external",
        payload: {
          url: "https://docs.anthropic.com/en/docs/claude-code/getting-started",
        },
      },
      {
        command: "open_external",
        payload: { url: "https://claude.com/download" },
      },
    ]);
  const commands = (await featureFixtureCalls(page)).map(
    (call) => call.command,
  );
  expect(commands).not.toContain("apply_provider_quick_setup_with_result");
  expect(commands).not.toContain("switch_provider_with_result");
  expect(commands).not.toContain("save_workbuddy_models");

  await expectHealthyPage(page, health);
});

test("Codex Desktop fixture reads safely and starts only after the explicit install action", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  const callsBeforeCodex = await featureFixtureCalls(page);
  expect(
    callsBeforeCodex.filter(
      (call) =>
        call.command.startsWith("codex_desktop_") ||
        call.payload.event === "codex-desktop-installer://job-updated",
    ),
  ).toEqual([]);
  const codexCallStartIndex = callsBeforeCodex.length;

  await agentItem(page, "Codex").click();
  const installer = page.getByRole("region", {
    name: "Codex Desktop 安装器",
  });
  const install = installer.getByRole("button", {
    name: "安装 Codex Desktop",
  });
  await expect(install).toBeEnabled({ timeout: 10_000 });

  await expect
    .poll(async () => {
      const commands = (await featureFixtureCalls(page))
        .slice(codexCallStartIndex)
        .map((call) => call.command);
      return {
        activeListeners:
          commands.filter((command) => command === "plugin:event|listen")
            .length -
          commands.filter((command) => command === "plugin:event|unlisten")
            .length,
        jobReads: commands.filter(
          (command) => command === "codex_desktop_get_job",
        ).length,
      };
    })
    .toEqual({ activeListeners: 1, jobReads: 1 });

  const initializationCalls = (await featureFixtureCalls(page))
    .slice(codexCallStartIndex)
    .filter(
      (call) =>
        call.command.startsWith("codex_desktop_") ||
        call.command.startsWith("plugin:event|"),
    );
  const allowedInitializationCommands = new Set([
    "codex_desktop_get_local_status",
    "codex_desktop_check_latest",
    "codex_desktop_get_job",
    "plugin:event|listen",
    "plugin:event|unlisten",
  ]);
  expect(
    initializationCalls.filter(
      (call) => !allowedInitializationCommands.has(call.command),
    ),
  ).toEqual([]);

  const localReads = initializationCalls.filter(
    (call) => call.command === "codex_desktop_get_local_status",
  );
  const latestReads = initializationCalls.filter(
    (call) => call.command === "codex_desktop_check_latest",
  );
  expect(localReads.length).toBeGreaterThanOrEqual(1);
  expect(localReads.length).toBeLessThanOrEqual(2);
  expect(latestReads.length).toBeGreaterThanOrEqual(1);
  expect(latestReads.length).toBeLessThanOrEqual(2);
  expect(
    localReads.every((call) => Object.keys(call.payload).length === 0),
  ).toBe(true);
  expect(latestReads.map((call) => call.payload)).toEqual(
    latestReads.map(() => ({ force: false })),
  );

  const activeListenerIds = new Set<number>();
  for (const call of initializationCalls) {
    if (call.command === "plugin:event|listen") {
      expect(call.payload).toEqual({
        event: "codex-desktop-installer://job-updated",
        target: { kind: "Any" },
        handler: expect.any(Number),
      });
      const handler = call.payload.handler as number;
      expect(activeListenerIds.has(handler)).toBe(false);
      activeListenerIds.add(handler);
    }
    if (call.command === "plugin:event|unlisten") {
      expect(call.payload).toEqual({
        event: "codex-desktop-installer://job-updated",
        eventId: expect.any(Number),
      });
      const eventId = call.payload.eventId as number;
      expect(activeListenerIds.delete(eventId)).toBe(true);
    }
    expect(activeListenerIds.size).toBeLessThanOrEqual(1);
  }
  expect(activeListenerIds.size).toBe(1);
  expect(
    initializationCalls.filter(
      (call) => call.command === "codex_desktop_get_job",
    ),
  ).toEqual([{ command: "codex_desktop_get_job", payload: {} }]);
  expect(initializationCalls.map((call) => call.command)).not.toContain(
    "codex_desktop_start_install",
  );

  await install.click();
  await expect
    .poll(async () =>
      (await featureFixtureCalls(page)).filter(
        (call) => call.command === "codex_desktop_start_install",
      ),
    )
    .toEqual([
      {
        command: "codex_desktop_start_install",
        payload: {
          request: { expectedReleaseId: `v1:${"a".repeat(64)}` },
        },
      },
    ]);
  const startPayload = (await featureFixtureCalls(page)).find(
    (call) => call.command === "codex_desktop_start_install",
  )?.payload;
  expect(JSON.stringify(startPayload)).not.toMatch(
    /url|path|hash|scope|bypass/i,
  );

  await expectHealthyPage(page, health);
});

test("Agent directory does not observe WorkBuddy or Provider summaries", async ({
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

  await agentItem(page, "WorkBuddy").click();
  await expect(
    page.getByRole("region", { name: "WorkBuddy 配置概览" }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("region", { name: "WorkBuddy 详情" }),
  ).toBeVisible();

  await agentItem(page, "Claude Code").click();
  await expect(
    page.getByRole("region", { name: "Claude Code 模型配置" }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("region", { name: "Claude Code 详情" }),
  ).toBeVisible();

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
  await expect(
    page.getByRole("button", { name: "打开官方设置" }),
  ).toHaveCount(0);
  await page.getByTestId("model-target-trae").click();
  await expect(
    page.getByRole("region", { name: "TRAE Work CN 模型设置" }),
  ).toBeVisible();
  await expect(page.locator("body")).toContainText(
    "自定义模型需在 TRAE Work CN 中添加",
  );
  await expect(page.getByRole("textbox", { name: "API Key" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "拉取模型" })).toHaveCount(0);
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

test("WorkBuddy freezes overwrite input, sends revision, rereads, and clears credentials", async ({
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

  const dialog = page.getByRole("dialog", { name: "确认覆盖已有模型" });
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "确认覆盖" }).click();
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");

  await expect
    .poll(
      async () =>
        (await featureFixtureCalls(page)).filter(
          (call) => call.command === "save_workbuddy_models",
        ).length,
    )
    .toBe(2);
  const calls = await featureFixtureCalls(page);
  const saveCalls = calls.filter(
    (call) => call.command === "save_workbuddy_models",
  );
  const firstRequest = saveCalls[0].payload.request as Record<string, unknown>;
  const secondRequest = saveCalls[1].payload.request as Record<string, unknown>;
  expect(firstRequest).toMatchObject({
    baseUrl: "https://workbuddy.example.test/v1",
    apiKey,
    manualModelIds: ["manual-browser-model"],
    expectedRevision: "fixture-revision-1",
  });
  expect(secondRequest).toEqual({
    ...firstRequest,
    overwriteToken: "fixture-opaque-overwrite-token",
  });
  expect(
    calls.filter((call) => call.command === "get_workbuddy_status").length,
  ).toBeGreaterThanOrEqual(2);
  expect(
    calls.filter((call) => call.command === "get_workbuddy_model_ids").length,
  ).toBeGreaterThanOrEqual(2);

  const secretSurfaces = await page.evaluate(
    (secret) => ({
      body: document.body.textContent ?? "",
      hash: window.location.hash,
      localStorage: Object.values(window.localStorage),
      sessionStorage: Object.values(window.sessionStorage),
      secret,
    }),
    apiKey,
  );
  expect(secretSurfaces.body).not.toContain(apiKey);
  expect(secretSurfaces.hash).not.toContain(apiKey);
  expect(secretSurfaces.localStorage).not.toContain(apiKey);
  expect(secretSurfaces.sessionStorage).not.toContain(apiKey);

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

  await expect(page.locator("body")).toContainText("保存失败");
  await expect(page.locator("body")).toContainText(
    "请刷新当前设置、检查输入后重试。",
  );
  await expect(page.locator("body")).not.toContainText(apiKey);
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");
  expect(
    (await featureFixtureCalls(page)).filter(
      (call) => call.command === "save_workbuddy_models",
    ),
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

  await expect(page.locator("body")).toContainText("配置已被其他操作修改");
  await expect(page.locator("body")).not.toContainText(
    "WorkBuddy 模型配置已保存",
  );
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");
  const calls = await featureFixtureCalls(page);
  expect(
    calls.filter((call) => call.command === "get_workbuddy_status").length,
  ).toBeGreaterThanOrEqual(2);
  expect(
    calls.filter((call) => call.command === "get_workbuddy_model_ids").length,
  ).toBeGreaterThanOrEqual(2);

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
    app: "codex",
    request: {
      name: "Browser Codex",
      baseUrl: "https://codex.example.test/v1",
      apiKey,
      modelId: "gpt-browser",
    },
  });
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

  await expect(page.locator("body")).toContainText(
    "未能保存设置，已还原之前的状态",
  );
  await expect(page.locator("body")).not.toContainText("partial-secret");
  const calls = await featureFixtureCalls(page);
  expect(
    calls.filter(
      (call) => call.command === "apply_provider_quick_setup_with_result",
    ),
  ).toHaveLength(1);
  expect(
    calls.filter((call) => call.command === "switch_provider_with_result"),
  ).toHaveLength(0);
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");
  await expectHealthyPage(page, health);
});
