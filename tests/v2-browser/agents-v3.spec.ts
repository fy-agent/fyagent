import { expect, test, type Page } from "@playwright/test";

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

async function installAgentV3Overrides(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const originalInvoke = window.__TAURI_INTERNALS__.invoke.bind(
      window.__TAURI_INTERNALS__,
    );

    window.__TAURI_INTERNALS__.invoke = async (command, payload = {}) => {
      if (command === "get_agent_catalog") {
        const catalog = (await originalInvoke(command, payload)) as {
          agents: Array<{
            id: string;
            capabilities: Array<{
              id: string;
              mode: string;
              reasonCode: string;
            }>;
          }>;
        };
        for (const agent of catalog.agents) {
          const modelWrite = agent.capabilities.find(
            (capability) => capability.id === "models.write",
          );
          if (!modelWrite) continue;
          if (agent.id === "qoderwork") {
            modelWrite.mode = "unsupported";
            modelWrite.reasonCode = "vendor_private_storage_unsupported";
          }
          if (agent.id === "trae-work") {
            modelWrite.mode = "assisted";
            modelWrite.reasonCode = "vendor_ui_required";
          }
        }
        return catalog;
      }
      if (command !== "get_agent_install_readiness") {
        return originalInvoke(command, payload);
      }

      window.__FYAGENT_FEATURE_FIXTURE__.calls.push({ command, payload });
      await new Promise((resolve) => window.setTimeout(resolve, 1_000));
      const agentId = String(payload.agentId);
      const installState =
        agentId === "trae-work"
          ? "unknown"
          : agentId === "qoderwork" || agentId === "opencode"
            ? "not_installed"
            : "installed";
      const cliAgent = ["grokbuild", "claude-code", "opencode"].includes(
        agentId,
      );
      return {
        contractVersion: 3,
        agentId,
        reviewedAt: "2026-08-29",
        installState,
        inventoryState:
          installState === "installed" ? "single" : "not_observed",
        requiresTargetSelection: false,
        updateState: installState === "installed" ? "up_to_date" : "unknown",
        releaseId: null,
        localVersion: installState === "installed" ? "1.0.0" : null,
        remoteVersion: null,
        authOwnership: agentId === "codex" ? "fyagent_managed" : "agent_owned",
        authState: "unknown",
        sourceKind:
          agentId === "codex"
            ? "codex_desktop"
            : cliAgent
              ? "cli_tooling"
              : "managed_desktop",
        allowedActions: [],
        reasonCodes: ["auth_state_unknown"],
      };
    };
  });
}

async function installFixture(page: Page): Promise<void> {
  await installRichTauriFeatureFixture(page);
  await installAgentV3Overrides(page);
}

test("Agent V3 shows the full catalog, auto-scans, and reuses existing Skill and MCP assignments", async ({
  page,
}) => {
  await installFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  const directory = page.getByRole("region", { name: "AI 软件目录" });
  await expect(directory).toBeVisible();
  await expect(directory.getByRole("article")).toHaveCount(7);
  await expect
    .poll(
      async () =>
        (await featureFixtureCalls(page)).filter(
          (call) => call.command === "get_agent_install_readiness",
        ).length,
    )
    .toBeGreaterThanOrEqual(7);
  await expect(
    directory.getByRole("button", { name: "重新扫描" }),
  ).toBeEnabled();
  await expect(directory.getByRole("button", { name: /取消扫描/ })).toHaveCount(
    0,
  );

  const brandFrames = directory.locator(
    '.fy-agent-directory-card .fy-catalog-brand-frame[data-size="detail"]',
  );
  await expect(brandFrames).toHaveCount(7);
  expect(
    await brandFrames.evaluateAll((frames) =>
      frames.map((frame) => ({
        width: frame.getBoundingClientRect().width,
        height: frame.getBoundingClientRect().height,
      })),
    ),
  ).toEqual([0, 1, 2, 3, 4, 5, 6].map(() => ({ width: 64, height: 64 })));

  const configurable = ["workbuddy", "grokbuild", "codex", "claude-code"];
  const blocked = ["qoderwork", "trae-work", "opencode"];
  for (const agentId of configurable) {
    await expect(
      directory.locator(`[data-agent-id="${agentId}"]`).getByRole("button", {
        name: "进行配置",
      }),
    ).toBeEnabled();
  }
  for (const agentId of blocked) {
    await expect(
      directory.locator(`[data-agent-id="${agentId}"]`).getByRole("button", {
        name: "进行配置",
      }),
    ).toBeDisabled();
  }

  await expect(directory.getByText(/“未确认”不等于“未安装”/)).toHaveCount(0);
  await expect(directory.getByText(/上次扫描：/)).toHaveCount(0);
  await expect(directory.getByText("查看完整介绍")).toHaveCount(0);

  await directory
    .locator('[data-agent-id="workbuddy"]')
    .getByRole("button", { name: "进行配置" })
    .click();
  await expect(page).toHaveURL(/#\/agents\?target=workbuddy&section=models$/);
  const configuration = page.getByRole("region", {
    name: "WorkBuddy 配置",
  });
  await expect(configuration).toBeVisible();

  await configuration.getByRole("tab", { name: "Skills" }).click();
  await expect(page).toHaveURL(/#\/agents\?target=workbuddy&section=skills$/);
  const skillSwitch = configuration
    .getByRole("switch", {
      name: "WorkBuddy Skill 分配",
    })
    .first();
  await expect(skillSwitch).not.toBeChecked();
  await skillSwitch.click();
  await expect(skillSwitch).toBeChecked();
  await expect(
    configuration.getByText(/已从真实配置回读：WorkBuddy 已启用此 Skill/),
  ).toBeVisible();

  await configuration.getByRole("tab", { name: "MCP" }).click();
  await expect(page).toHaveURL(/#\/agents\?target=workbuddy&section=mcp$/);
  const mcpSwitch = configuration
    .getByRole("switch", {
      name: "WorkBuddy MCP 分配",
    })
    .first();
  await expect(mcpSwitch).not.toBeChecked();
  await mcpSwitch.click();
  const trustDialog = page.getByRole("dialog", {
    name: "需要在 WorkBuddy 中信任 MCP",
  });
  await expect(trustDialog).toBeVisible();
  await expect(trustDialog).toContainText("连接器 → 自定义连接器");
  await trustDialog.getByRole("button", { name: "知道了" }).click();
  await expect(trustDialog).toHaveCount(0);
  await expect(mcpSwitch).toBeChecked();
  await expect(
    configuration.getByText(/已从真实配置回读：WorkBuddy 已分配此 MCP/),
  ).toBeVisible();

  const calls = await featureFixtureCalls(page);
  expect(calls.filter((call) => call.command === "toggle_skill_app")).toEqual([
    {
      command: "toggle_skill_app",
      payload: { id: "fixture-review", app: "workbuddy", enabled: true },
    },
  ]);
  expect(calls.filter((call) => call.command === "toggle_mcp_app")).toEqual([
    {
      command: "toggle_mcp_app",
      payload: {
        serverId: "fixture-context",
        app: "workbuddy",
        enabled: true,
      },
    },
  ]);

  await configuration.getByRole("button", { name: "进入 MCP 管理" }).click();
  await expect(page).toHaveURL(
    /#\/mcp\?agentReturn=workbuddy&agentSection=mcp$/,
  );
  const agentsLink = page.getByRole("link", {
    name: "AI软件配置",
    exact: true,
  });
  await expect(agentsLink).toHaveAttribute(
    "href",
    "#/agents?target=workbuddy&section=mcp",
  );
  await agentsLink.click();
  await expect(page).toHaveURL(/#\/agents\?target=workbuddy&section=mcp$/);
  await expect(configuration).toBeVisible();

  await configuration.getByRole("button", { name: "返回" }).click();
  await expect(page).toHaveURL(/#\/agents$/);
  await expect(page.getByRole("button", { name: "重新扫描" })).toBeEnabled();
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("Agent V3 restores deep links and keeps model and prompt capability boundaries honest", async ({
  page,
}) => {
  await installFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents?target=qoderwork&section=models");

  const qoder = page.getByRole("region", { name: "QoderWork CN 配置" });
  await expect(qoder).toBeVisible();
  await expect(page).toHaveURL(/#\/agents\?target=qoderwork&section=models$/);
  await expect(
    qoder.getByText(/当前官方能力不支持第三方模型配置/),
  ).toBeVisible();
  await expect(qoder.getByRole("switch")).toHaveCount(0);

  await qoder.getByRole("tab", { name: "提示词" }).click();
  await expect(page).toHaveURL(/#\/agents\?target=qoderwork&section=prompts$/);
  await expect(qoder.getByText(/当前未接入提示词管理/)).toBeVisible();
  expect(
    (await featureFixtureCalls(page)).some(
      (call) => call.command === "get_prompts",
    ),
  ).toBe(false);

  await openV2Page(page, "/agents?target=trae-work&section=models");
  const trae = page.getByRole("region", { name: "TRAE Work CN 配置" });
  await expect(trae.getByText(/TRAE Work CN 已观测模型/)).toBeVisible();
  await expect(trae.getByText("fixture-model", { exact: true })).toHaveCount(1);
  await expect(trae.getByRole("switch")).toHaveCount(0);
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("Agent Auth distinguishes verified sessions, handoff-only flows, and Auth Center ownership", async ({
  page,
}) => {
  await installFixture(page);
  const health = monitorPageHealth(page);

  await openV2Page(page, "/agents?target=claude-code&section=models");
  const claude = page.getByRole("region", { name: "Claude Code 配置" });
  await expect(claude.getByText("已验证退出")).toBeVisible();
  await claude.getByRole("button", { name: "登录", exact: true }).click();
  await expect(claude.getByText("等待你完成官方认证")).toBeVisible();
  await expect(claude.getByText("认证结果已验证")).toBeVisible();
  await expect(claude.getByText("已验证登录")).toBeVisible();

  await openV2Page(page, "/agents?target=grokbuild&section=models");
  const grok = page.getByRole("region", { name: "Grok Build 配置" });
  await grok.getByRole("button", { name: "登录", exact: true }).click();
  await expect(grok.getByText("已交给官方认证入口")).toBeVisible();
  await expect(grok.getByText("认证结果已验证")).toHaveCount(0);

  await openV2Page(page, "/agents?target=codex&section=models");
  const codex = page.getByRole("region", { name: "Codex 配置" });
  await expect(codex.getByText("由 FyAgent 认证中心管理")).toBeVisible();
  await expect(
    codex.getByRole("button", { name: "登录", exact: true }),
  ).toHaveCount(0);

  const calls = await featureFixtureCalls(page);
  expect(
    calls.some(
      (call) =>
        call.command === "start_agent_auth_session" &&
        (call.payload.request as { agentId?: string }).agentId ===
          "claude-code",
    ),
  ).toBe(true);
  expect(
    calls.some(
      (call) =>
        call.command === "start_agent_auth_session" &&
        (call.payload.request as { agentId?: string }).agentId === "grokbuild",
    ),
  ).toBe(true);
  expect(
    calls.some(
      (call) =>
        call.command === "start_agent_auth_session" &&
        (call.payload.request as { agentId?: string }).agentId === "codex",
    ),
  ).toBe(false);
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});
