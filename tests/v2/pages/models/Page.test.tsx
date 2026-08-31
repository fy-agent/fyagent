import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { ModelsPage } from "@/v2/pages/models/Page";
import { QUICK_SETUP_PROVIDER_IDS } from "@/v2/pages/models/quickSetup";
import { getAgentIcon } from "@/v2/shared/assets/agents";
import type { FeaturePorts } from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import type { AgentCatalogResult } from "@/v2/shared/features/types";
import type { ChangeJobSnapshot } from "@/v2/shared/features/change-plans";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import { TooltipProvider } from "@/v2/shared/ui/primitives";
import {
  changeJobWire,
  changeJobWorkBuddyWire,
  changePlanUpsertWire,
  changePlanWorkBuddyWire,
} from "../../fixtures/changePlans";

function renderPage(ports: FeaturePorts, target?: string) {
  const initialEntry = target ? `/models?target=${target}` : "/models";
  return render(
    <StrictMode>
      <MemoryRouter initialEntries={[initialEntry]}>
        <TooltipProvider delayDuration={0} skipDelayDuration={0}>
          <FeatureProvider ports={ports}>
            <ModelsPage />
          </FeatureProvider>
        </TooltipProvider>
      </MemoryRouter>
    </StrictMode>,
  );
}

async function confirmWriteDisclosure(
  user: ReturnType<typeof userEvent.setup>,
) {
  const dialog = await screen.findByRole("dialog", { name: "保存前确认" });
  expect(within(dialog).getByText("将修改")).toBeVisible();
  expect(within(dialog).getByText("备份位置")).toBeVisible();
  await user.click(within(dialog).getByRole("button", { name: "确认保存" }));
}

function succeededCodexJob(): ChangeJobSnapshot {
  return {
    ...changeJobWire,
    jobId: "job-save",
    executionId: "job-save",
    planId: changePlanUpsertWire.planId,
    idempotencyKey: changePlanUpsertWire.planId,
    targetProviderId: changePlanUpsertWire.targetProviderId,
    status: "succeeded",
    resultCode: "applied",
    revision: 5,
    eventSeq: 5,
    steps: [
      { kind: "precheck", status: "succeeded", code: "ok" },
      { kind: "snapshot", status: "succeeded", code: "ok" },
      { kind: "managed_write", status: "succeeded", code: "ok" },
      { kind: "readback", status: "succeeded", code: "ok" },
      { kind: "finalize", status: "succeeded", code: "ok" },
    ],
    resources: [
      { kind: "provider_db_current", status: "matched", code: "ok" },
      { kind: "device_current", status: "matched", code: "ok" },
      { kind: "target_definition", status: "matched", code: "ok" },
      { kind: "codex_live_projection", status: "matched", code: "ok" },
    ],
    events: [
      { sequence: 1, phase: "precheck", reasonCode: "ok", createdAt: 1 },
      { sequence: 2, phase: "snapshot", reasonCode: "ok", createdAt: 2 },
      { sequence: 3, phase: "managed_write", reasonCode: "ok", createdAt: 3 },
      { sequence: 4, phase: "readback", reasonCode: "ok", createdAt: 4 },
      { sequence: 5, phase: "finalize", reasonCode: "ok", createdAt: 5 },
    ],
    liveConfigChanged: true,
    recoveryState: "not_needed",
  };
}

function succeededWorkBuddyJob(): ChangeJobSnapshot {
  return {
    ...changeJobWorkBuddyWire,
    jobId: "job-workbuddy-save",
    executionId: "job-workbuddy-save",
    planId: changePlanWorkBuddyWire.planId,
    idempotencyKey: changePlanWorkBuddyWire.planId,
    targetProviderId: changePlanWorkBuddyWire.targetProviderId,
    status: "succeeded",
    resultCode: "applied",
    revision: 5,
    eventSeq: 5,
    steps: [
      { kind: "precheck", status: "succeeded", code: "ok" },
      { kind: "snapshot", status: "succeeded", code: "ok" },
      { kind: "managed_write", status: "succeeded", code: "ok" },
      { kind: "readback", status: "succeeded", code: "ok" },
      { kind: "finalize", status: "succeeded", code: "ok" },
    ],
    resources: [
      { kind: "work_buddy_models_config", status: "matched", code: "ok" },
      { kind: "work_buddy_backup", status: "matched", code: "ok" },
    ],
    events: [
      { sequence: 1, phase: "precheck", reasonCode: "ok", createdAt: 1 },
      { sequence: 2, phase: "snapshot", reasonCode: "ok", createdAt: 2 },
      { sequence: 3, phase: "managed_write", reasonCode: "ok", createdAt: 3 },
      { sequence: 4, phase: "readback", reasonCode: "ok", createdAt: 4 },
      { sequence: 5, phase: "finalize", reasonCode: "ok", createdAt: 5 },
    ],
    liveConfigChanged: false,
    restartRequirement: "not_required",
    recoveryState: "not_needed",
  };
}

function stubWorkBuddySavePlan(
  ports: FeaturePorts,
  job: ChangeJobSnapshot = succeededWorkBuddyJob(),
  plan = changePlanWorkBuddyWire,
) {
  ports.workbuddy.saveModels = vi.fn();
  ports.changePlans.createWorkBuddySavePlan = vi.fn(async () => ({ ...plan }));
  ports.changePlans.applyChangePlan = vi.fn(async () => ({
    kind: "admitted" as const,
    job,
  }));
  ports.changePlans.getChangeJob = vi.fn(async () => job);
}

function stubCodexSavePlan(
  ports: FeaturePorts,
  job: ChangeJobSnapshot = succeededCodexJob(),
) {
  ports.providers.applyQuickSetupWithResult = vi.fn();
  ports.changePlans.createCodexProviderUpsertPlan = vi.fn(async () => ({
    ...changePlanUpsertWire,
  }));
  ports.changePlans.applyChangePlan = vi.fn(async () => ({
    kind: "admitted" as const,
    job,
  }));
  ports.changePlans.getChangeJob = vi.fn(async () => job);
}

function catalog(): AgentCatalogResult {
  const capabilities: AgentCatalogResult["agents"][number]["capabilities"] = [
    "product.open",
    "app.detect",
    "app.launch",
    "skills.read",
    "skills.write",
    "hooks.read",
    "hooks.write",
    "models.validate",
    "models.write",
    "mcp.validate",
    "mcp.write",
  ].map((id) => ({
    id: id as AgentCatalogResult["agents"][number]["capabilities"][number]["id"],
    mode: id === "product.open" ? "direct" : "assisted",
    reasonCode:
      id === "product.open" ? "official_link_reviewed" : "vendor_ui_required",
    evidenceIds: ["p0_scope"],
  }));
  return {
    contractVersion: 5,
    reviewedAt: "2026-08-20",
    agents: [
      {
        id: "qoderwork",
        variantId: "qoderwork-cn",
        displayName: "QoderWork CN",
        description: "QoderWork CN 官方辅助设置",
        officialLinks: [
          {
            id: "product",
            label: "打开 QoderWork 官方页面",
            url: "https://qoder.com.cn/qoderwork",
          },
        ],
        capabilities,
      },
      {
        id: "trae-work",
        variantId: "trae-work-cn",
        displayName: "TRAE Work CN",
        description: "TRAE Work CN 官方辅助设置",
        officialLinks: [
          {
            id: "desktop",
            label: "非产品链接应被忽略",
            url: "https://ignored.example.test/trae",
          },
          {
            id: "product",
            label: "打开 TRAE Work CN 官方页面",
            url: "https://www.trae.cn/sem-work",
          },
        ],
        capabilities,
      },
    ],
  };
}

function workBuddyPorts(): FeaturePorts {
  const ports = createBrowserFeaturePorts();
  ports.workbuddy.getStatus = vi.fn<FeaturePorts["workbuddy"]["getStatus"]>(
    async () => ({
      path: "C:/redacted/models.json",
      backupPath: "C:/redacted/models.json.backup",
      exists: true,
      modelCount: 1,
      revision: "revision-1",
      backupExists: true,
      format: "objectRoot",
    }),
  );
  ports.workbuddy.getModelIds = vi.fn<FeaturePorts["workbuddy"]["getModelIds"]>(
    async () => ({
      ids: ["existing-model"],
      revision: "revision-1",
    }),
  );
  return ports;
}

const TEST_PROVIDER_WRITE_TARGETS = [
  {
    path: "~/.config/provider/config.json",
    backupPath: "~/.config/provider/config.json.fyagent.backup",
    exists: true,
  },
] as const;

const TEST_OPENCODE_SNAPSHOT_META = {
  path: "~/.config/opencode/opencode.json",
  backupPath: "~/.config/opencode/opencode.json.backup",
  exists: true,
} as const;

describe("V2 Models page", () => {
  it("renders the exact selector order, local decorative icons, and QoderWork default", () => {
    const ports = createBrowserFeaturePorts();
    renderPage(ports);

    const selector = screen.getByRole("complementary", {
      name: "模型配置目标",
    });
    const buttons = within(selector).getAllByRole("button");
    expect(buttons.map((button) => button.textContent)).toEqual([
      "QoderWork CN",
      "TRAE Work CN",
      "WorkBuddy",
      "Grok Build",
      "Codex",
      "Claude Code",
      "OpenCode",
    ]);

    const expectedIcons = [
      getAgentIcon("qoderwork"),
      getAgentIcon("trae-work"),
      getAgentIcon("workbuddy"),
      getAgentIcon("grokbuild"),
      getAgentIcon("codex"),
      getAgentIcon("claude-code"),
      getAgentIcon("opencode"),
    ];
    buttons.forEach((button, index) => {
      const icon = button.querySelector("img");
      expect(icon).toHaveAttribute("src", expectedIcons[index]);
      expect(icon).toHaveAttribute("alt", "");
      expect(icon).toHaveAttribute("aria-hidden", "true");
      expect(button.querySelector('[data-size="list"]')).toBeInTheDocument();
    });
    expect(screen.getByTestId("model-target-qoderwork")).toHaveAttribute(
      "aria-current",
      "true",
    );
    const qoderRegion = screen.getByRole("region", {
      name: "QoderWork CN 模型设置",
    });
    expect(qoderRegion).toBeVisible();
    expect(qoderRegion.querySelector(".fy-control-badge")).toBeNull();
    expect(qoderRegion.querySelector(".fy-control-button-primary")).toBeNull();
    expect(
      qoderRegion.querySelector(".fy-models-commit-heading"),
    ).not.toBeNull();
    expect(qoderRegion.querySelector(".fy-models-existing")).toBeNull();
    expect(qoderRegion).toHaveTextContent("官方不支持第三方模型配置");
  });

  it("does not expose official settings buttons on QoderWork or TRAE model details", async () => {
    const ports = createBrowserFeaturePorts();
    ports.catalog.get = vi.fn(async () => catalog());
    renderPage(ports, "qoderwork");

    const qoderRegion = await screen.findByRole("region", {
      name: "QoderWork CN 模型设置",
    });
    expect(qoderRegion).toHaveTextContent("官方不支持第三方模型配置");
    expect(
      screen.queryByRole("button", { name: "管理 MCP" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "测试连通" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "打开官方设置" }),
    ).not.toBeInTheDocument();
    expect(
      screen
        .getByRole("region", { name: "QoderWork CN 模型设置" })
        .querySelector(".fy-control-badge"),
    ).toBeNull();

    const view = renderPage(ports, "trae");
    expect(
      await screen.findByRole("region", { name: "TRAE Work CN 模型设置" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "打开 TRAE 官方模型设置" }),
    ).not.toBeInTheDocument();
    view.unmount();
  });

  it("shows TRAE guidance and observed IDs without fetch or save controls", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.catalog.get = vi.fn(async () => catalog());
    ports.traeWork.getModelIds = vi.fn(async () => ({
      modelIds: ["custom-a"],
      revision: "revision-1",
      truncated: false,
    }));
    renderPage(ports, "trae");

    expect(
      await screen.findByRole("region", { name: "TRAE Work CN 模型设置" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "自定义模型需在 TRAE Work CN 中添加。FyAgent 不会写入其本地模型配置。",
      ),
    ).toBeVisible();
    expect(screen.getByText(/以云端模型列表为准/)).toBeVisible();
    expect(screen.queryByLabelText("服务地址")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("API Key")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "拉取模型" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "测试连通" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "保存并应用" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("checkbox", { name: "允许本机地址" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("checkbox", { name: "允许私有网络地址" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("不使用 API Key")).not.toBeInTheDocument();
    expect(screen.queryByText("TRAE 模型配置已保存")).not.toBeInTheDocument();
    expect(screen.queryByText("请回 TRAE 保存")).not.toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: /TRAE 当前第三方模型 ID/ }),
    );
    expect(await screen.findByText("custom-a")).toBeVisible();
    expect(ports.traeWork.getModelIds).toHaveBeenCalled();
    expect("fetchModels" in ports.traeWork).toBe(false);
    expect("saveModels" in ports.traeWork).toBe(false);
  });

  it("fetches OpenCode models without clearing the key, then saves natively and clears it", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    const secret = "OPENCODE-UI-SECRET-SENTINEL-814";
    ports.opencodeModels.getSnapshot = vi.fn(async () => ({
      providers: [],
      revision: "revision-1",
      ...TEST_OPENCODE_SNAPSHOT_META,
    }));
    ports.opencodeModels.fetchProviderModels = vi.fn(async () => ({
      models: [{ id: "gpt-4o", ownedBy: "openai" }],
      truncated: false,
    }));
    ports.opencodeModels.saveModels = vi.fn(async () => ({
      state: "saved" as const,
      revision: "revision-2",
      modelCount: 1,
      createdEntries: 1,
      updatedEntries: 0,
    }));
    localStorage.clear();
    sessionStorage.clear();
    renderPage(ports, "opencode");

    await user.type(await screen.findByLabelText("供应商名称"), "Gateway");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://gateway.example.test/v1",
    );
    await user.type(screen.getByLabelText("API Key"), secret);
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    await waitFor(() =>
      expect(ports.opencodeModels.fetchProviderModels).toHaveBeenCalledWith({
        baseUrl: "https://gateway.example.test/v1",
        apiKey: secret,
        allowNoApiKey: false,
      }),
    );
    expect(screen.getByLabelText("API Key")).toHaveValue(secret);
    expect(await screen.findByText("gpt-4o")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "保存并应用" }));
    await confirmWriteDisclosure(user);
    await waitFor(() =>
      expect(ports.opencodeModels.saveModels).toHaveBeenCalledWith({
        providerName: "Gateway",
        baseUrl: "https://gateway.example.test/v1",
        apiKey: secret,
        selectedModelIds: ["gpt-4o"],
        removedModelIds: [],
        expectedRevision: "revision-1",
      }),
    );
    expect(await screen.findByText("OpenCode 模型配置已保存")).toBeVisible();
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    const openCodeHeader = screen
      .getByRole("heading", { name: "OpenCode" })
      .closest("header");
    expect(openCodeHeader).not.toBeNull();
    expect(
      within(openCodeHeader as HTMLElement).queryByText("待保存"),
    ).not.toBeInTheDocument();
    expect(document.body.innerHTML).not.toContain(secret);
    expect(JSON.stringify(localStorage)).not.toContain(secret);
    expect(JSON.stringify(sessionStorage)).not.toContain(secret);
  });

  it("fetches Claude models as chips with local icons then saves the selected id", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    const secret = "CLAUDE-UI-SECRET-SENTINEL-814";
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: "",
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    ports.providers.fetchModels = vi.fn(async () => [
      { id: "claude-sonnet-4", ownedBy: "anthropic" },
    ]);
    ports.providers.applyQuickSetupWithResult = vi.fn(async () => ({
      value: { warnings: [] },
      liveConfigChanged: true,
      app: "claude" as const,
    }));
    renderPage(ports, "claude");

    await screen.findByRole("heading", { name: "Claude Code" });
    await user.clear(screen.getByLabelText("配置名称"));
    await user.type(screen.getByLabelText("配置名称"), "Claude Gateway");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://claude.example.test/v1",
    );
    await user.type(screen.getByLabelText("API Key"), secret);
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(await screen.findByText("claude-sonnet-4")).toBeVisible();
    const chipIcon = screen
      .getByText("claude-sonnet-4")
      .closest("li")
      ?.querySelector("img");
    expect(chipIcon).toHaveAttribute(
      "src",
      expect.stringMatching(/\/src\/v2\/shared\/assets\/models\//),
    );
    expect(chipIcon?.getAttribute("src") ?? "").not.toMatch(/^https?:/i);
    await user.click(screen.getByRole("button", { name: "claude-sonnet-4" }));
    await user.click(
      screen.getByRole("button", { name: "保存并设为当前配置" }),
    );
    await confirmWriteDisclosure(user);
    await waitFor(() =>
      expect(ports.providers.applyQuickSetupWithResult).toHaveBeenCalledWith(
        expect.objectContaining({
          name: "Claude Gateway",
          baseUrl: "https://claude.example.test/v1",
          apiKey: secret,
          modelId: "claude-sonnet-4",
        }),
        "claude",
      ),
    );
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(document.body.innerHTML).not.toContain(secret);
  });

  it("keeps the WorkBuddy save action in the sticky panel heading", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    const heading = screen.getByRole("heading", { name: "WorkBuddy" });
    const header = heading.closest("header");
    expect(header).not.toBeNull();
    expect(
      within(header as HTMLElement).getByRole("button", { name: "保存并应用" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("heading", { name: "保存" }),
    ).not.toBeInTheDocument();

    await user.type(
      screen.getByLabelText("服务地址"),
      "https://pending.example/v1",
    );
    expect(header).toHaveAttribute("data-pending", "true");
    expect(within(header as HTMLElement).getByText("待保存")).toBeVisible();

    await user.type(screen.getByLabelText("自定义模型 ID"), "pending-model");
    expect(within(header as HTMLElement).getByText("待保存")).toBeVisible();
  });

  it("previews WorkBuddy overwrite as a Change Plan risk and confirms with plan identity only", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    stubWorkBuddySavePlan(ports);
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://workbuddy.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "first-secret");
    await user.type(screen.getByLabelText("自定义模型 ID"), "manual-model");
    await user.click(screen.getByRole("button", { name: "保存并应用" }));
    await confirmWriteDisclosure(user);

    expect(await screen.findByText("保存 WorkBuddy 模型设置")).toBeVisible();
    expect(screen.queryByRole("button", { name: "确认覆盖" })).toBeNull();
    expect(screen.queryByRole("button", { name: "取消" })).toBeNull();
    expect(document.body).not.toHaveTextContent("first-secret");
    expect(screen.getByLabelText("API Key")).toHaveValue("first-secret");
    expect(ports.workbuddy.saveModels).not.toHaveBeenCalled();
    expect(ports.changePlans.createWorkBuddySavePlan).toHaveBeenCalledWith({
      baseUrl: "https://workbuddy.example/v1",
      apiKey: "first-secret",
      allowNoApiKey: false,
      selectedModelIds: [],
      manualModelIds: ["manual-model"],
      removedModelIds: [],
      clearExistingApiKeys: false,
      expectedRevision: "revision-1",
    });

    await user.click(screen.getByRole("button", { name: "应用更改" }));
    await screen.findByText("WorkBuddy 模型配置已保存");
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledWith({
      planId: changePlanWorkBuddyWire.planId,
      planDigest: changePlanWorkBuddyWire.planDigest,
    });
    expect(ports.workbuddy.saveModels).not.toHaveBeenCalled();
    expect(screen.getByLabelText("API Key")).toHaveValue("");
  });

  it("locks duplicate WorkBuddy fetches, preserves truncation, and keeps the key", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    type FetchResult = Awaited<
      ReturnType<FeaturePorts["workbuddy"]["fetchModels"]>
    >;
    let resolveFetch!: (result: FetchResult) => void;
    const pendingFetch = new Promise<FetchResult>((resolve) => {
      resolveFetch = resolve;
    });
    ports.workbuddy.fetchModels = vi.fn(() => pendingFetch);
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://fetch.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "fetch-secret");
    const fetchButton = screen.getByRole("button", { name: "拉取模型" });
    fireEvent.click(fetchButton);
    fireEvent.click(fetchButton);

    expect(ports.workbuddy.fetchModels).toHaveBeenCalledTimes(1);
    expect(ports.workbuddy.fetchModels).toHaveBeenCalledWith({
      baseUrl: "https://fetch.example/v1",
      apiKey: "fetch-secret",
      allowNoApiKey: false,
    });
    resolveFetch({ models: ["model-a", "model-b"], truncated: true });

    expect(await screen.findByText("已达到可显示的模型数量上限")).toBeVisible();
    expect(screen.getByText("model-a")).toBeVisible();
    expect(screen.getByLabelText("API Key")).toHaveValue("fetch-secret");
    expect(document.body).not.toHaveTextContent("fetch-secret");
  });

  it("redacts WorkBuddy fetch failures and keeps the submitted key", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    ports.workbuddy.fetchModels = vi.fn(async () => {
      throw new Error("fetch-secret must not escape");
    });
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://failure.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "fetch-secret");
    await user.click(screen.getByRole("button", { name: "拉取模型" }));

    expect(await screen.findByText("模型读取失败")).toBeVisible();
    expect(screen.getByLabelText("API Key")).toHaveValue("fetch-secret");
    expect(document.body).not.toHaveTextContent("fetch-secret");
  });

  it("rejects a WorkBuddy success response whose model ID contains the submitted key", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    ports.workbuddy.fetchModels = vi.fn(async () => ({
      models: ["safe-model", "prefix-fetch-secret-suffix"],
      truncated: false,
    }));
    ports.workbuddy.saveModels = vi.fn();
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://hostile.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "fetch-secret");
    await user.click(screen.getByRole("button", { name: "拉取模型" }));

    expect(await screen.findByText("模型读取失败")).toBeVisible();
    expect(screen.queryByText("safe-model")).not.toBeInTheDocument();
    expect(document.body).not.toHaveTextContent("fetch-secret");
    expect(screen.getByLabelText("API Key")).toHaveValue("fetch-secret");
    expect(ports.workbuddy.saveModels).not.toHaveBeenCalled();
  });

  it("blocks a WorkBuddy save whose model ID contains the submitted key", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    ports.workbuddy.saveModels = vi.fn();
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://conflict.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "conflict-secret");
    await user.type(
      screen.getByLabelText("自定义模型 ID"),
      "prefix-conflict-secret-suffix",
    );
    await user.click(screen.getByRole("button", { name: "保存并应用" }));

    expect(await screen.findByText("模型 ID 不能包含 API Key")).toBeVisible();
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(ports.workbuddy.saveModels).not.toHaveBeenCalled();
  });

  it("does not claim a WorkBuddy authoritative reread when either refresh fails", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    stubWorkBuddySavePlan(ports);
    vi.mocked(ports.workbuddy.getStatus)
      .mockResolvedValueOnce({
        path: "C:/redacted/models.json",
        exists: true,
        backupPath: "C:/redacted/models.json.backup",
        modelCount: 1,
        revision: "revision-1",
        backupExists: true,
        format: "objectRoot",
      })
      .mockRejectedValue(new Error("status refresh failed"));
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://conflict.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "conflict-secret");
    await user.type(screen.getByLabelText("自定义模型 ID"), "conflict-model");
    await user.click(screen.getByRole("button", { name: "保存并应用" }));
    await confirmWriteDisclosure(user);
    await user.click(await screen.findByRole("button", { name: "应用更改" }));

    expect(
      await screen.findByText(
        "设置已保存，但暂时无法刷新页面。请重新打开后检查。",
      ),
    ).toBeVisible();
    expect(document.body).not.toHaveTextContent("权威状态");
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(ports.workbuddy.saveModels).not.toHaveBeenCalled();
  });

  it("treats a stale WorkBuddy apply as regenerate instead of an overwrite dialog", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    stubWorkBuddySavePlan(ports);
    ports.changePlans.applyChangePlan = vi.fn(async () => ({
      kind: "rejected" as const,
      errorCode: "stale" as const,
    }));
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://expired.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "expired-secret");
    await user.type(screen.getByLabelText("自定义模型 ID"), "expired-model");
    await user.click(screen.getByRole("button", { name: "保存并应用" }));
    await confirmWriteDisclosure(user);
    await user.click(await screen.findByRole("button", { name: "应用更改" }));

    expect(await screen.findByText("预览已过期")).toBeVisible();
    expect(screen.getByRole("button", { name: "重新生成预览" })).toBeVisible();
    expect(screen.queryByRole("button", { name: "确认覆盖" })).toBeNull();
    expect(ports.workbuddy.saveModels).not.toHaveBeenCalled();
    expect(screen.getByLabelText("API Key")).toHaveValue("expired-secret");
  });

  it("blocks WorkBuddy saves while authoritative local state is unavailable", async () => {
    const ports = workBuddyPorts();
    ports.workbuddy.getStatus = vi.fn(async () => {
      throw new Error("status unavailable");
    });
    ports.workbuddy.getModelIds = vi.fn(async () => {
      throw new Error("model IDs unavailable");
    });
    ports.workbuddy.saveModels = vi.fn();
    renderPage(ports, "workbuddy");

    expect(
      await screen.findByText(
        "暂时无法读取 WorkBuddy 配置，请重试。",
        undefined,
        { timeout: 5_000 },
      ),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "保存并应用" })).toBeDisabled();
    expect(ports.workbuddy.saveModels).not.toHaveBeenCalled();
  });

  it("hides WorkBuddy backup and config-file status, and groups existing IDs", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    ports.workbuddy.getModelIds = vi.fn(async () => ({
      ids: ["gpt-4o", "gemini-2.5-pro", "grok-4.6"],
      revision: "revision-1",
    }));
    renderPage(ports, "workbuddy");

    await screen.findByText("当前已有的第三方模型 ID");
    const existingToggle = await screen.findByTestId("workbuddy-status");
    expect(existingToggle).toHaveTextContent("已有第三方模型数量3");
    expect(existingToggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("配置状态")).not.toBeInTheDocument();
    expect(screen.queryByText("备份")).not.toBeInTheDocument();
    expect(screen.queryByText("gpt-4o")).not.toBeInTheDocument();

    await user.click(
      screen.getByRole("heading", { name: "当前已有的第三方模型 ID" }),
    );
    expect(existingToggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("button", { name: "gpt 分组" })).toBeVisible();
    expect(screen.getByRole("button", { name: "gemini 分组" })).toBeVisible();
    expect(screen.getByRole("button", { name: "grok 分组" })).toBeVisible();
    expect(screen.getByText("gpt-4o")).toBeVisible();
    expect(screen.getByText("gemini-2.5-pro")).toBeVisible();
    expect(
      screen.queryByText("更新时清除已保存的 API Key"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText("出错时提示会出现在对应输入旁边。"),
    ).not.toBeInTheDocument();
  });

  it("filters existing and draft WorkBuddy model IDs from each list", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    ports.workbuddy.getModelIds = vi.fn(async () => ({
      ids: ["gpt-4o", "gemini-2.5-pro"],
      revision: "revision-1",
    }));
    ports.workbuddy.fetchModels = vi.fn(async () => ({
      models: ["gpt-4o", "claude-sonnet-4"],
      truncated: false,
    }));
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.click(
      screen.getByRole("heading", { name: "当前已有的第三方模型 ID" }),
    );
    await user.type(screen.getByLabelText("搜索已有模型"), "gemini");
    expect(screen.getByText("gemini-2.5-pro")).toBeVisible();
    expect(screen.queryByText("gpt-4o")).not.toBeInTheDocument();

    await user.type(
      screen.getByLabelText("服务地址"),
      "https://search.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "search-secret");
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(await screen.findByText("已读取 2 个模型")).toBeVisible();
    await user.type(screen.getByLabelText("搜索待保存模型"), "claude");
    expect(screen.getByText("claude-sonnet-4")).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "移除模型 gpt-4o" }),
    ).not.toBeInTheDocument();
  });

  it("deletes an existing WorkBuddy model immediately after confirmation", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    let ids = ["existing-model"];
    ports.workbuddy.getModelIds = vi.fn(async () => ({
      ids: [...ids],
      revision: ids.length === 0 ? "revision-2" : "revision-1",
    }));
    ports.workbuddy.saveModels = vi
      .fn()
      .mockImplementation(async (request: { overwriteToken?: string }) => {
        if (!request.overwriteToken) {
          return {
            state: "overwrite_confirmation_required",
            token: "opaque-delete-token",
            existingIds: ["existing-model"],
          };
        }
        ids = [];
        return {
          state: "saved",
          revision: "revision-2",
          modelCount: 0,
          createdEntries: 0,
          updatedEntries: 0,
        };
      });
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.type(screen.getByLabelText("API Key"), "keep-secret");
    await user.click(
      screen.getByRole("heading", { name: "当前已有的第三方模型 ID" }),
    );
    await user.click(
      screen.getByRole("button", { name: "移除模型 existing-model" }),
    );
    expect(ports.workbuddy.saveModels).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog", { name: "确认删除模型" });
    expect(dialog).toHaveTextContent(
      "此操作将会删除该模型配置，不可恢复，是否确认删除",
    );
    expect(within(dialog).getByText("existing-model")).toBeVisible();
    await user.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(
      screen.queryByRole("dialog", { name: "确认删除模型" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "移除模型 existing-model" }),
    ).toBeVisible();

    await user.click(
      screen.getByRole("button", { name: "移除模型 existing-model" }),
    );
    await user.click(await screen.findByRole("button", { name: "确认删除" }));

    expect(await screen.findByText("已删除该模型配置")).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "移除模型 existing-model" }),
    ).not.toBeInTheDocument();
    expect(screen.getByLabelText("API Key")).toHaveValue("keep-secret");
    expect(ports.workbuddy.saveModels).toHaveBeenCalledTimes(2);
    expect(ports.workbuddy.saveModels).toHaveBeenCalledWith({
      baseUrl: "",
      apiKey: "",
      allowNoApiKey: false,
      selectedModelIds: [],
      manualModelIds: [],
      removedModelIds: ["existing-model"],
      clearExistingApiKeys: false,
      expectedRevision: "revision-1",
    });
    expect(ports.workbuddy.saveModels).toHaveBeenCalledWith({
      baseUrl: "",
      apiKey: "",
      allowNoApiKey: false,
      selectedModelIds: [],
      manualModelIds: [],
      removedModelIds: ["existing-model"],
      clearExistingApiKeys: false,
      expectedRevision: "revision-1",
      overwriteToken: "opaque-delete-token",
    });
  });

  it("toggles WorkBuddy API key visibility without leaving the input", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    const apiKey = screen.getByLabelText("API Key");
    await user.type(apiKey, "visible-secret");
    expect(apiKey).toHaveAttribute("type", "password");
    await user.click(screen.getByRole("button", { name: "显示 API Key" }));
    expect(apiKey).toHaveAttribute("type", "text");
    expect(apiKey).toHaveValue("visible-secret");
    await user.click(screen.getByRole("button", { name: "隐藏 API Key" }));
    expect(apiKey).toHaveAttribute("type", "password");
    expect(document.body).not.toHaveTextContent("visible-secret");
  });

  it("explains the WorkBuddy option for local models without an API key", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    renderPage(ports, "workbuddy");

    await screen.findByText("当前已有的第三方模型 ID");
    await user.hover(
      screen.getByRole("button", { name: "不使用 API Key 说明" }),
    );
    expect(await screen.findByRole("tooltip")).toHaveTextContent(
      "给不需要鉴权的本地模型使用，例如本机的 Ollama、LM Studio。勾选后请求不会携带 API Key。",
    );
  });

  it("manages fetched and manual WorkBuddy models in one draft", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    ports.workbuddy.fetchModels = vi.fn(async () => ({
      models: ["gpt-4o", "gemini-2.5-pro"],
      truncated: false,
    }));
    stubWorkBuddySavePlan(ports);
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://draft.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "draft-secret");
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(await screen.findByText("已读取 2 个模型")).toBeVisible();
    await user.type(screen.getByLabelText("自定义模型 ID"), "custom-router");
    await user.click(screen.getByRole("button", { name: "填入" }));
    await user.click(
      screen.getByRole("button", { name: "移除模型 gemini-2.5-pro" }),
    );
    await user.click(screen.getByRole("button", { name: "保存并应用" }));
    await confirmWriteDisclosure(user);
    await user.click(await screen.findByRole("button", { name: "应用更改" }));

    await screen.findByText("WorkBuddy 模型配置已保存");
    expect(ports.changePlans.createWorkBuddySavePlan).toHaveBeenCalledWith({
      baseUrl: "https://draft.example/v1",
      apiKey: "draft-secret",
      allowNoApiKey: false,
      selectedModelIds: ["gpt-4o"],
      manualModelIds: ["custom-router"],
      removedModelIds: [],
      clearExistingApiKeys: false,
      expectedRevision: "revision-1",
    });
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledWith({
      planId: changePlanWorkBuddyWire.planId,
      planDigest: changePlanWorkBuddyWire.planDigest,
    });
    expect(ports.workbuddy.saveModels).not.toHaveBeenCalled();
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    const savedHeader = screen
      .getByRole("heading", { name: "WorkBuddy" })
      .closest("header");
    expect(savedHeader).not.toBeNull();
    expect(
      within(savedHeader as HTMLElement).queryByText("待保存"),
    ).not.toBeInTheDocument();
  });

  it("atomically applies Codex once with the exact provider payload", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    let currentProviderId = "current-codex";
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: currentProviderId,
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    ports.providers.applyQuickSetupWithResult = vi.fn();
    let resolveCreate!: (plan: typeof changePlanUpsertWire) => void;
    const pendingCreate = new Promise<typeof changePlanUpsertWire>(
      (resolve) => {
        resolveCreate = resolve;
      },
    );
    ports.changePlans.createCodexProviderUpsertPlan = vi.fn(
      () => pendingCreate,
    );
    const job = succeededCodexJob();
    ports.changePlans.applyChangePlan = vi.fn(async () => ({
      kind: "admitted" as const,
      job,
    }));
    ports.changePlans.getChangeJob = vi.fn(async () => job);
    renderPage(ports, "codex");

    await screen.findByTestId("provider-status");
    const codexHeader = screen
      .getByRole("heading", { name: "Codex" })
      .closest("header");
    expect(codexHeader).not.toBeNull();
    expect(
      within(codexHeader as HTMLElement).getByRole("button", {
        name: "保存并设为当前配置",
      }),
    ).toBeVisible();
    await user.clear(screen.getByLabelText("配置名称"));
    await user.type(screen.getByLabelText("配置名称"), "Codex Gateway");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://codex.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "codex-secret");
    await user.type(screen.getByLabelText("模型 ID"), "gpt-5");
    expect(
      within(codexHeader as HTMLElement).getByText("待保存"),
    ).toBeVisible();
    expect(screen.queryByText("将修改")).not.toBeInTheDocument();
    const submit = screen.getByRole("button", { name: "保存并设为当前配置" });
    await user.click(submit);
    expect(
      ports.changePlans.createCodexProviderUpsertPlan,
    ).not.toHaveBeenCalled();
    await confirmWriteDisclosure(user);
    expect(submit).toBeDisabled();
    fireEvent.click(submit);
    expect(
      ports.changePlans.createCodexProviderUpsertPlan,
    ).toHaveBeenCalledTimes(1);
    expect(ports.providers.applyQuickSetupWithResult).not.toHaveBeenCalled();
    currentProviderId = QUICK_SETUP_PROVIDER_IDS.codex;
    resolveCreate({
      ...changePlanUpsertWire,
      risks: [
        ...changePlanUpsertWire.risks,
        {
          code: "CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED",
          severity: "warning",
        },
      ],
    });

    expect(
      await screen.findByRole("button", { name: "应用更改" }),
    ).toBeEnabled();
    expect(
      screen.getByText("当前网络代理可能影响连接，请确认后使用。"),
    ).toBeVisible();
    expect(
      ports.changePlans.createCodexProviderUpsertPlan,
    ).toHaveBeenCalledWith({
      name: "Codex Gateway",
      baseUrl: "https://codex.example/v1",
      apiKey: "codex-secret",
      modelId: "gpt-5",
      codexFeatures: { imageExtension: false, websockets: false },
    });
    await user.click(screen.getByRole("button", { name: "应用更改" }));
    await screen.findByText("模型设置已保存并设为当前配置");
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledWith({
      planId: changePlanUpsertWire.planId,
      planDigest: changePlanUpsertWire.planDigest,
    });
    expect(
      screen.getByText("重启或新建会话后即可使用新的设置。"),
    ).toBeVisible();
    expect(document.body).not.toHaveTextContent("Quick Setup Provider ID");
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(
      within(codexHeader as HTMLElement).queryByText("待保存"),
    ).not.toBeInTheDocument();
    expect(ports.providers.getSummary).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("button", { name: "取消" })).toBeNull();
  });

  it("keeps write targets in a save-confirm dialog instead of the page layout", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: "",
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    stubCodexSavePlan(ports);
    renderPage(ports, "codex");

    await screen.findByTestId("provider-status");
    expect(
      screen.queryByRole("dialog", { name: "保存前确认" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("将修改")).not.toBeInTheDocument();
    await user.clear(screen.getByLabelText("配置名称"));
    await user.type(screen.getByLabelText("配置名称"), "Codex Gateway");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://codex.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "codex-secret");
    await user.type(screen.getByLabelText("模型 ID"), "gpt-5");
    await user.click(
      screen.getByRole("button", { name: "保存并设为当前配置" }),
    );

    const dialog = await screen.findByRole("dialog", { name: "保存前确认" });
    expect(within(dialog).getByText("将修改")).toBeVisible();
    expect(within(dialog).getByText("备份位置")).toBeVisible();
    expect(
      ports.changePlans.createCodexProviderUpsertPlan,
    ).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(
      screen.queryByRole("dialog", { name: "保存前确认" }),
    ).not.toBeInTheDocument();
    expect(
      ports.changePlans.createCodexProviderUpsertPlan,
    ).not.toHaveBeenCalled();
    expect(ports.providers.applyQuickSetupWithResult).not.toHaveBeenCalled();
  });

  it("sends Codex image-extension and websocket toggles in the quick setup payload", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: "",
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    stubCodexSavePlan(ports);
    renderPage(ports, "codex");

    await screen.findByTestId("provider-status");
    await user.clear(screen.getByLabelText("配置名称"));
    await user.type(screen.getByLabelText("配置名称"), "Codex Gateway");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://codex.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "codex-secret");
    await user.type(screen.getByLabelText("模型 ID"), "gpt-5");

    await user.click(
      screen.getByRole("checkbox", { name: "启用内置生图扩展" }),
    );
    await user.click(
      screen.getByRole("checkbox", { name: "启用 WebSocket 传输" }),
    );

    await user.click(
      screen.getByRole("button", { name: "保存并设为当前配置" }),
    );
    await confirmWriteDisclosure(user);

    await waitFor(() =>
      expect(
        ports.changePlans.createCodexProviderUpsertPlan,
      ).toHaveBeenCalledTimes(1),
    );
    expect(ports.providers.applyQuickSetupWithResult).not.toHaveBeenCalled();
    expect(
      ports.changePlans.createCodexProviderUpsertPlan,
    ).toHaveBeenCalledWith({
      name: "Codex Gateway",
      baseUrl: "https://codex.example/v1",
      apiKey: "codex-secret",
      modelId: "gpt-5",
      codexFeatures: { imageExtension: true, websockets: true },
    });
  });

  it("treats an unclassified apply failure as unknown and stops writes", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {
        [QUICK_SETUP_PROVIDER_IDS.claude]: {
          id: QUICK_SETUP_PROVIDER_IDS.claude,
          name: "Sanitized existing Provider",
        },
      },
      currentId: "another-provider",
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    ports.providers.applyQuickSetupWithResult = vi.fn(async () => {
      throw new Error("atomic response contains claude-secret");
    });
    renderPage(ports, "claude");

    await screen.findByText("已有设置，将更新");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://claude.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "claude-secret");
    await user.type(screen.getByLabelText("模型 ID"), "claude-model");
    await user.click(
      screen.getByRole("button", { name: "保存并设为当前配置" }),
    );
    await confirmWriteDisclosure(user);

    await screen.findByText("无法确认当前设置");
    expect(ports.providers.applyQuickSetupWithResult).toHaveBeenCalledTimes(1);
    expect(ports.providers.applyQuickSetupWithResult).toHaveBeenCalledWith(
      expect.objectContaining({
        name: expect.any(String),
        baseUrl: "https://claude.example/v1",
        apiKey: "claude-secret",
        modelId: "claude-model",
      }),
      "claude",
    );
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(document.body).not.toHaveTextContent("claude-secret");
    expect(ports.providers.getSummary).toHaveBeenCalledTimes(2);
    expect(
      screen.getByRole("button", { name: "暂时无法确认当前设置" }),
    ).toBeDisabled();
  });

  it("stops further writes when the backend reports partial rollback", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: "",
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    const recoveryJob: ChangeJobSnapshot = {
      ...succeededCodexJob(),
      status: "failed",
      resultCode: "recovery_required",
      recoveryState: "recovery_required",
      resources: [
        { kind: "provider_db_current", status: "matched", code: "ok" },
        { kind: "device_current", status: "matched", code: "ok" },
        { kind: "target_definition", status: "unavailable", code: "unknown" },
        {
          kind: "codex_live_projection",
          status: "unavailable",
          code: "unknown",
        },
      ],
    };
    stubCodexSavePlan(ports, recoveryJob);
    renderPage(ports, "codex");

    await screen.findByTestId("provider-status");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://partial.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "partial-secret");
    await user.type(screen.getByLabelText("模型 ID"), "gpt-partial");
    await user.click(
      screen.getByRole("button", { name: "保存并设为当前配置" }),
    );
    await confirmWriteDisclosure(user);
    await user.click(await screen.findByRole("button", { name: "应用更改" }));

    await screen.findByText("无法确认当前设置");
    expect(document.body).not.toHaveTextContent("partial-secret");
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(ports.providers.applyQuickSetupWithResult).not.toHaveBeenCalled();
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledTimes(1);
    const blockedButton = screen.getByRole("button", {
      name: "暂时无法确认当前设置",
    });
    expect(blockedButton).toBeDisabled();
    await user.click(blockedButton);
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledTimes(1);

    await user.click(screen.getByTestId("model-target-claude"));
    await screen.findByRole("heading", { name: "Claude Code" });
    await user.click(screen.getByTestId("model-target-codex"));
    expect(
      await screen.findByRole("button", {
        name: "暂时无法确认当前设置",
      }),
    ).toBeDisabled();
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledTimes(1);
  });

  it("treats a confirmed baseline restore as a failed save, not unknown authority", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {
        [QUICK_SETUP_PROVIDER_IDS.codex]: {
          id: QUICK_SETUP_PROVIDER_IDS.codex,
          name: "FyAgent Codex",
        },
      },
      currentId: QUICK_SETUP_PROVIDER_IDS.codex,
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    const restoredJob: ChangeJobSnapshot = {
      ...succeededCodexJob(),
      status: "failed",
      resultCode: "writer_failed_baseline_restored",
      recoveryState: "succeeded",
      liveConfigChanged: false,
      resources: [
        {
          kind: "provider_db_current",
          status: "mismatched",
          code: "target_not_current",
        },
        {
          kind: "device_current",
          status: "mismatched",
          code: "target_not_current",
        },
        {
          kind: "target_definition",
          status: "matched",
          code: "definition_matched",
        },
        {
          kind: "codex_live_projection",
          status: "mismatched",
          code: "live_mismatched",
        },
      ],
    };
    stubCodexSavePlan(ports, restoredJob);
    renderPage(ports, "codex");

    await screen.findByTestId("provider-status");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://restored.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "restored-secret");
    await user.type(screen.getByLabelText("模型 ID"), "gpt-restored");
    await user.click(
      screen.getByRole("button", { name: "保存并设为当前配置" }),
    );
    await confirmWriteDisclosure(user);
    expect(ports.providers.applyQuickSetupWithResult).not.toHaveBeenCalled();
    expect(
      ports.changePlans.createCodexProviderUpsertPlan,
    ).toHaveBeenCalledTimes(1);
    await user.click(await screen.findByRole("button", { name: "应用更改" }));

    expect(
      await screen.findByText("未能保存设置，已还原之前的状态"),
    ).toBeVisible();
    expect(screen.queryByText("无法确认当前设置")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "暂时无法确认当前设置" }),
    ).toBeNull();
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    const header = screen
      .getByRole("heading", { name: "Codex" })
      .closest("header");
    expect(header).not.toBeNull();
    expect(within(header as HTMLElement).getByText("待保存")).toBeVisible();
  });

  it("surfaces only a generic partial warning from an atomic apply", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: QUICK_SETUP_PROVIDER_IDS.codex,
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    const warningJob: ChangeJobSnapshot = {
      ...succeededCodexJob(),
      status: "warning",
      resultCode: "applied_with_warning",
    };
    stubCodexSavePlan(ports, warningJob);
    ports.changePlans.createCodexProviderUpsertPlan = vi.fn(async () => ({
      ...changePlanUpsertWire,
      risks: [
        ...changePlanUpsertWire.risks,
        { code: "CODEX_WEBSOCKET_NON_GPT_MODEL", severity: "warning" },
      ],
    }));
    renderPage(ports, "codex");

    await screen.findByTestId("provider-status");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://partial.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "partial-secret");
    await user.type(screen.getByLabelText("模型 ID"), "gpt-partial");
    await user.click(
      screen.getByRole("button", { name: "保存并设为当前配置" }),
    );
    await confirmWriteDisclosure(user);
    expect(
      await screen.findByText("当前模型可能与此连接方式不兼容，请确认后使用。"),
    ).toBeVisible();
    await user.click(screen.getByRole("button", { name: "应用更改" }));

    await screen.findByText("模型设置已保存并设为当前配置");
    expect(screen.getByText(/部分设置仍需确认/)).toBeVisible();
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(document.body).not.toHaveTextContent("partial-secret");
    expect(ports.providers.applyQuickSetupWithResult).not.toHaveBeenCalled();
    expect(ports.providers.getSummary).toHaveBeenCalledTimes(2);
  });

  it("does not claim current Provider when the authoritative reread disagrees", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: "another-provider",
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    stubCodexSavePlan(ports);
    renderPage(ports, "codex");

    await screen.findByTestId("provider-status");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://unconfirmed.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "unconfirmed-secret");
    await user.type(screen.getByLabelText("模型 ID"), "gpt-unconfirmed");
    await user.click(
      screen.getByRole("button", { name: "保存并设为当前配置" }),
    );
    await confirmWriteDisclosure(user);
    await user.click(await screen.findByRole("button", { name: "应用更改" }));

    expect(await screen.findByText("模型设置已保存，待确认")).toBeVisible();
    expect(
      screen.queryByText("模型设置已保存并设为当前配置"),
    ).not.toBeInTheDocument();
    expect(screen.getByLabelText("API Key")).toHaveValue("");
  });

  it("clears unsaved WorkBuddy fields when the target panel unmounts", async () => {
    const user = userEvent.setup();
    const ports = workBuddyPorts();
    renderPage(ports, "workbuddy");

    await screen.findByText("已有第三方模型数量");
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://keep.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "keep-secret");
    await user.click(screen.getByTestId("model-target-qoderwork"));
    expect(
      await screen.findByRole("heading", { name: "QoderWork CN" }),
    ).toBeVisible();
    expect(document.body).not.toHaveTextContent("keep-secret");

    await user.click(screen.getByTestId("model-target-workbuddy"));
    expect(screen.getByLabelText("服务地址")).toHaveValue("");
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(JSON.stringify(localStorage)).not.toContain("keep-secret");
    expect(JSON.stringify(sessionStorage)).not.toContain("keep-secret");
  });

  it("remounts the TRAE observation panel from authoritative data", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.catalog.get = vi.fn(async () => catalog());
    ports.traeWork.getModelIds = vi.fn(async () => ({
      modelIds: ["custom-keep"],
      revision: "revision-1",
      truncated: false,
    }));
    localStorage.clear();
    sessionStorage.clear();
    const view = renderPage(ports, "trae");

    expect(
      await screen.findByRole("region", { name: "TRAE Work CN 模型设置" }),
    ).toBeVisible();
    expect(screen.queryByLabelText("API Key")).not.toBeInTheDocument();

    await user.click(screen.getByTestId("model-target-qoderwork"));
    expect(screen.queryByLabelText("API Key")).not.toBeInTheDocument();

    await user.click(screen.getByTestId("model-target-trae"));
    expect(
      await screen.findByRole("region", { name: "TRAE Work CN 模型设置" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "自定义模型需在 TRAE Work CN 中添加。FyAgent 不会写入其本地模型配置。",
      ),
    ).toBeVisible();
    expect(screen.queryByLabelText("服务地址")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("API Key")).not.toBeInTheDocument();
    expect(ports.traeWork.getModelIds).toHaveBeenCalled();
    view.unmount();
  });

  it("warns only when Claude pathname has an explicit v1 segment", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: "",
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    renderPage(ports, "claude");

    await screen.findByTestId("provider-status");
    const url = screen.getByLabelText("服务地址");
    expect(url).toHaveAttribute("placeholder", "https://gateway.example");
    await user.type(url, "https://v1.example.com/anthropic");
    expect(screen.queryByText(/\/v1\/v1\/XXXX/)).not.toBeInTheDocument();
    await user.clear(url);
    await user.type(url, "https://gateway.example/v1");
    expect(
      screen.getByText(
        "服务地址已包含 /v1。Claude 会继续追加版本路径，可能请求到 /v1/v1/…。通常应填写不含末尾 /v1 的地址；请按服务商文档确认。",
      ),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "保存并设为当前配置" }),
    ).toBeEnabled();
  });

  it("probes a selected model after IDs exist on WorkBuddy, Provider, and OpenCode", async () => {
    const user = userEvent.setup();
    const probed = {
      success: false,
      status: "failed" as const,
      message: 'HTTP 401: {"error":{"message":"invalid api key"}}',
      responseTimeMs: 22,
      httpStatus: 401,
      modelUsed: "gpt-test",
      errorCategory: null,
    };
    const ports = createBrowserFeaturePorts();
    ports.workbuddy.getStatus = vi.fn<FeaturePorts["workbuddy"]["getStatus"]>(
      async () => ({
        path: "C:/redacted/models.json",
        exists: true,
        backupPath: "C:/redacted/models.json.backup",
        modelCount: 0,
        revision: "revision-1",
        backupExists: false,
        format: "objectRoot",
      }),
    );
    ports.workbuddy.getModelIds = vi.fn(async () => ({
      ids: [],
      revision: "revision-1",
    }));
    ports.workbuddy.checkModel = vi.fn(async () => probed);
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: "",
      writeTargets: [...TEST_PROVIDER_WRITE_TARGETS],
    }));
    ports.providers.checkModel = vi.fn(async () => probed);
    ports.opencodeModels.getSnapshot = vi.fn(async () => ({
      providers: [],
      revision: "revision-1",
      ...TEST_OPENCODE_SNAPSHOT_META,
    }));
    ports.opencodeModels.checkModel = vi.fn(async () => probed);

    const workbuddyView = renderPage(ports, "workbuddy");
    await screen.findByText("已有第三方模型数量");
    expect(
      screen.queryByRole("button", { name: "测试连通" }),
    ).not.toBeInTheDocument();
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://draft.example/anthropic",
    );
    await user.type(screen.getByLabelText("API Key"), "wb-key");
    await user.type(screen.getByLabelText("自定义模型 ID"), "gpt-test");
    await user.click(screen.getByRole("button", { name: "填入" }));
    await user.click(screen.getByRole("button", { name: "测试连通" }));
    const workbuddyDialog = await screen.findByRole("dialog");
    await user.click(
      within(workbuddyDialog).getByRole("button", { name: "gpt-test" }),
    );
    await user.click(
      within(workbuddyDialog).getByRole("button", { name: "开始测试" }),
    );
    expect(ports.workbuddy.checkModel).toHaveBeenCalledWith({
      app: "workbuddy",
      baseUrl: "https://draft.example/anthropic",
      apiKey: "wb-key",
      modelId: "gpt-test",
    });
    expect(await screen.findByText("连通测试失败")).toBeVisible();
    expect(screen.getByText(/invalid api key/)).toBeVisible();
    workbuddyView.unmount();

    const codexView = renderPage(ports, "codex");
    await screen.findByTestId("provider-status");
    expect(screen.getByRole("button", { name: "拉取模型" })).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "测试连通" }),
    ).not.toBeInTheDocument();
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://codex.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "codex-key");
    await user.type(screen.getByLabelText("模型 ID"), "gpt-test");
    await user.click(screen.getByRole("button", { name: "测试连通" }));
    const codexDialog = await screen.findByRole("dialog");
    await user.click(
      within(codexDialog).getByRole("button", { name: "gpt-test" }),
    );
    await user.click(
      within(codexDialog).getByRole("button", { name: "开始测试" }),
    );
    expect(ports.providers.checkModel).toHaveBeenCalledWith({
      app: "codex",
      baseUrl: "https://codex.example/v1",
      apiKey: "codex-key",
      modelId: "gpt-test",
      codexImageExtension: false,
    });
    expect(screen.queryByText(/\/v1\/v1\/XXXX/)).not.toBeInTheDocument();
    codexView.unmount();

    renderPage(ports, "opencode");
    await screen.findByRole("region", { name: "OpenCode 模型设置" });
    expect(
      screen.queryByRole("button", { name: "测试连通" }),
    ).not.toBeInTheDocument();
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://opencode.example/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "oc-key");
    await user.type(screen.getByLabelText("自定义模型 ID"), "gpt-test");
    await user.click(screen.getByRole("button", { name: "填入" }));
    await user.click(screen.getByRole("button", { name: "测试连通" }));
    const opencodeDialog = await screen.findByRole("dialog");
    await user.click(
      within(opencodeDialog).getByRole("button", { name: "gpt-test" }),
    );
    await user.click(
      within(opencodeDialog).getByRole("button", { name: "开始测试" }),
    );
    expect(ports.opencodeModels.checkModel).toHaveBeenCalledWith({
      app: "opencode",
      baseUrl: "https://opencode.example/v1",
      apiKey: "oc-key",
      modelId: "gpt-test",
    });
  });
});
