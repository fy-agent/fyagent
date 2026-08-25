import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { AgentsPage } from "@/v2/pages/agents/Page";
import type {
  AgentInstallReadiness,
  AgentInstallState,
} from "@/v2/shared/features/agent-install-readiness";
import type {
  CodexDesktopPort,
  FeaturePorts,
} from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import {
  createMcpAssignments,
  createSkillAssignments,
  type AgentCapabilityId,
  type AgentCatalogEntry,
  type AgentCatalogId,
  type AgentCatalogResult,
  type ManagedPrompt,
  type PromptAppId,
} from "@/v2/shared/features/types";
import { AGENT_CATALOG_IDS, PROMPT_APP_IDS } from "@/v2/shared/features/types";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";

const capabilityIds: readonly AgentCapabilityId[] = [
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
];

const variantById = {
  qoderwork: "qoderwork-cn",
  "trae-work": "trae-work-cn",
  workbuddy: "workbuddy",
  grokbuild: "grokbuild",
  codex: "codex",
  "claude-code": "claude-code",
  opencode: "opencode",
} as const;

function entry(id: AgentCatalogId, displayName: string): AgentCatalogEntry {
  const officialLinks: AgentCatalogEntry["officialLinks"] =
    id === "codex"
      ? []
      : [
          {
            id: "product",
            label: `打开 ${displayName} 官方页面`,
            url: `https://example.test/${id}`,
          },
        ];
  return {
    id,
    variantId: variantById[id],
    displayName,
    description: `${displayName} 的完整目录说明，用于验证两行摘要与完整介绍访问方式。`,
    officialLinks,
    capabilities: capabilityIds.map((capabilityId) => {
      const qoderModel =
        id === "qoderwork" &&
        (capabilityId === "models.validate" || capabilityId === "models.write");
      const traeModel =
        id === "trae-work" &&
        (capabilityId === "models.validate" || capabilityId === "models.write");
      const codexProduct = id === "codex" && capabilityId === "product.open";
      const runtime =
        capabilityId === "app.detect" || capabilityId === "app.launch";
      return {
        id: capabilityId,
        mode:
          qoderModel || codexProduct
            ? "unsupported"
            : traeModel
              ? "assisted"
              : runtime
                ? "unverified"
                : "direct",
        reasonCode: qoderModel
          ? "vendor_private_storage_unsupported"
          : traeModel
            ? "vendor_ui_required"
            : codexProduct
              ? "no_catalog_product_link"
              : runtime
                ? "trusted_runtime_identity_unavailable"
                : "dedicated_native_contract",
        evidenceIds: ["p0_scope"],
      };
    }),
  };
}

function catalog(): AgentCatalogResult {
  return {
    contractVersion: 4,
    reviewedAt: "2026-08-26",
    agents: [
      entry("qoderwork", "QoderWork CN"),
      entry("trae-work", "TRAE Work CN"),
      entry("workbuddy", "WorkBuddy"),
      entry("grokbuild", "Grok Build"),
      entry("codex", "Codex"),
      entry("claude-code", "Claude Code"),
      entry("opencode", "OpenCode"),
    ],
  };
}

function readiness(
  agentId: AgentCatalogId,
  installState: AgentInstallState = "installed",
): AgentInstallReadiness {
  return {
    contractVersion: 2,
    agentId,
    reviewedAt: "2026-08-26",
    installState,
    updateState: installState === "installed" ? "up_to_date" : "unknown",
    releaseId: null,
    localVersion: installState === "installed" ? "1.0.0" : null,
    remoteVersion: null,
    authOwnership: "agent_owned",
    authState: "unknown",
    sourceKind: "managed_desktop",
    allowedActions: [],
    reasonCodes: ["auth_state_unknown"],
  };
}

function promptStores(): Record<PromptAppId, ManagedPrompt[]> {
  const stores = {} as Record<PromptAppId, ManagedPrompt[]>;
  for (const app of PROMPT_APP_IDS) {
    stores[app] = [];
  }
  return stores;
}

function configuredPorts(): FeaturePorts {
  const ports = createBrowserFeaturePorts();
  ports.catalog.get = vi.fn(async () => catalog());
  ports.agentInstallReadiness.get = vi.fn(async (agentId) =>
    readiness(agentId),
  );
  ports.agentInstallReadiness.startAction = vi.fn();
  ports.agentInstallReadiness.cancelAction = vi.fn();
  ports.agentInstallReadiness.getActionJob = vi.fn();

  const skills = [
    {
      id: "review",
      name: "Review Companion",
      description: "审查交互与状态反馈",
      directory: "review-companion",
      repoOwner: "fyagent",
      repoName: "skills-review",
      apps: createSkillAssignments(["claude"]),
      installedAt: 1,
      updatedAt: 2,
    },
    {
      id: "release-notes",
      name: "Release Notes",
      description: "整理发布说明",
      directory: "release-notes",
      apps: createSkillAssignments(["codex"]),
      installedAt: 3,
      updatedAt: 4,
    },
  ];
  ports.skills.getInstalled = vi.fn(async () => structuredClone(skills));
  ports.skills.toggleApp = vi.fn(async (id, app, enabled) => {
    const skill = skills.find((item) => item.id === id);
    if (!skill) return false;
    skill.apps[app] = enabled;
    return true;
  });

  const mcpServers = {
    context: {
      id: "context",
      name: "Context Server",
      description: "提供受控上下文",
      source: "fixture",
      server: { type: "stdio" as const, command: "context-server" },
      apps: createMcpAssignments(["claude"]),
    },
    browser: {
      id: "browser",
      name: "Browser Server",
      description: "浏览器控制",
      source: "fixture",
      server: { type: "http" as const, url: "https://example.test/mcp" },
      apps: createMcpAssignments(["codex"]),
    },
  };
  ports.mcp.getAll = vi.fn(async () => structuredClone(mcpServers));
  ports.mcp.toggleApp = vi.fn(async (serverId, app, enabled) => {
    const server = mcpServers[serverId as keyof typeof mcpServers];
    if (!server) throw new Error("missing MCP");
    server.apps[app] = enabled;
  });

  const prompts = promptStores();
  prompts.codex = [
    {
      id: "active",
      name: "Current prompt",
      content: "当前内容",
      description: "当前启用",
      enabled: true,
    },
    {
      id: "review",
      name: "Review prompt",
      content: "核对状态反馈与真实回读。",
      description: "交互审查",
      enabled: false,
    },
  ];
  ports.prompts.getAll = vi.fn(async (app: PromptAppId) =>
    structuredClone(prompts[app]),
  );
  ports.prompts.enable = vi.fn(async (app: PromptAppId, id: string) => {
    prompts[app] = prompts[app].map((prompt) => ({
      ...prompt,
      enabled: prompt.id === id,
    }));
  });

  ports.traeWork.getModelIds = vi.fn(async () => ({
    modelIds: ["trae-observed-model"],
    revision: "trae-revision",
    truncated: false,
  }));
  ports.workbuddy.getStatus = vi.fn(async () => ({
    path: "~/.workbuddy/models.json",
    backupPath: "~/.workbuddy/models.json.backup",
    exists: true,
    modelCount: 1,
    revision: "workbuddy-revision",
    backupExists: true,
    format: "objectRoot" as const,
  }));
  ports.workbuddy.getModelIds = vi.fn(async () => ({
    ids: ["workbuddy-model"],
    revision: "workbuddy-revision",
  }));
  ports.opencodeModels.getSnapshot = vi.fn(async () => ({
    providers: [{ id: "openai", name: "OpenAI", modelIds: ["opencode-model"] }],
    revision: "opencode-revision",
    path: "~/.config/opencode/opencode.json",
    backupPath: "~/.config/opencode/opencode.json.backup",
    exists: true,
  }));
  ports.providers.getSummary = vi.fn(async (app) => ({
    providers: {
      current: {
        id: "current",
        name: `${app} provider`,
        modelId: `${app}-model`,
      },
    },
    currentId: "current",
    writeTargets: [],
  }));
  ports.codexDesktop = {
    getLocalStatus: async () => ({
      state: "not_installed",
      platform: "windows",
      architecture: "x86_64",
    }),
    checkLatest: async () => ({
      releaseId: `v1:${"a".repeat(64)}`,
      displayVersion: "1.2.3.4",
      platformVersion: {
        kind: "windows_msix",
        major: 1,
        minor: 2,
        build: 3,
        revision: 4,
      },
      downloadSizeHint: 4096,
      checkedAt: "2026-08-26T00:00:00.000Z",
    }),
    getJob: async () => null,
    startInstall: vi.fn(),
    cancelInstall: vi.fn(),
    launch: vi.fn(),
    openLogDirectory: vi.fn(),
    subscribeJobUpdates: async () => () => undefined,
  } satisfies CodexDesktopPort;
  return ports;
}

function LocationProbe() {
  const location = useLocation();
  return (
    <output data-testid="location">
      {location.pathname}
      {location.search}
    </output>
  );
}

function renderPage(ports: FeaturePorts, initialEntry = "/agents") {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <FeatureProvider ports={ports}>
        <AgentsPage />
        <LocationProbe />
      </FeatureProvider>
    </MemoryRouter>,
  );
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe("V3 Agent directory and configuration shell", () => {
  it("aggregates seven readiness queries with progress, unknown, and no fake cancel", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const reads = {} as Record<
      AgentCatalogId,
      ReturnType<typeof deferred<AgentInstallReadiness>>
    >;
    for (const agentId of AGENT_CATALOG_IDS) {
      reads[agentId] = deferred<AgentInstallReadiness>();
    }
    ports.agentInstallReadiness.get = vi.fn(
      async (agentId: AgentCatalogId) => reads[agentId].promise,
    );
    renderPage(ports);

    expect(
      await screen.findByRole("button", { name: "开始扫描" }),
    ).toBeVisible();
    expect(screen.getByTestId("agents-page")).toHaveAttribute(
      "data-view",
      "directory",
    );
    expect(
      screen
        .getAllByRole("article")
        .map(
          (article) =>
            within(article).getByRole("heading", { level: 2 }).textContent,
        ),
    ).toEqual([
      "QoderWork CN",
      "TRAE Work CN",
      "WorkBuddy",
      "Grok Build",
      "Codex",
      "Claude Code",
      "OpenCode",
    ]);

    await user.click(screen.getByRole("button", { name: "开始扫描" }));
    expect(screen.getByRole("button", { name: "扫描中…" })).toBeDisabled();
    expect(screen.getByText(/已完成 0 \/ 7/)).toBeVisible();
    expect(
      screen.queryByRole("button", { name: /取消扫描/ }),
    ).not.toBeInTheDocument();

    reads.qoderwork.resolve(readiness("qoderwork", "installed"));
    await waitFor(() =>
      expect(screen.getByText(/已完成 1 \/ 7/)).toBeVisible(),
    );
    reads["trae-work"].resolve(readiness("trae-work", "unknown"));
    for (const agentId of AGENT_CATALOG_IDS.slice(2)) {
      reads[agentId].resolve(readiness(agentId, "not_installed"));
    }

    expect(
      await screen.findByRole("button", { name: "重新扫描" }),
    ).toBeEnabled();
    expect(screen.getByText("未确认")).toBeVisible();
    expect(screen.getByText(/“未确认”不等于“未安装”/)).toBeVisible();
    expect(screen.getAllByText(/上次扫描：/)).toHaveLength(7);
    expect(ports.agentInstallReadiness.get).toHaveBeenCalledTimes(7);
  });

  it("shows a true empty result and preserves the last success when a rescan fails", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.agentInstallReadiness.get = vi.fn(async (agentId) =>
      readiness(agentId, "not_installed"),
    );
    renderPage(ports);

    await user.click(await screen.findByRole("button", { name: "开始扫描" }));
    expect(await screen.findByText(/本次未发现已安装的 AI 软件/)).toBeVisible();
    expect(screen.getAllByText("未安装")).toHaveLength(7);

    vi.mocked(ports.agentInstallReadiness.get).mockRejectedValue(
      new Error("readiness offline"),
    );
    await user.click(screen.getByRole("button", { name: "重新扫描" }));
    expect(
      await screen.findByText(/本次扫描未能读取任何软件状态/),
    ).toHaveTextContent("正在保留上次成功结果");
    expect(screen.getAllByText(/本次读取失败 · 未安装/)).toHaveLength(7);
    expect(screen.getByRole("button", { name: "重新扫描" })).toBeEnabled();
  });

  it("restores target and section from the query, supports back, and enters global management", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const view = renderPage(ports, "/agents?target=trae-work&section=skills");

    const configuration = await screen.findByRole("region", {
      name: "TRAE Work CN 配置",
    });
    expect(
      within(configuration).getByRole("tab", { name: "Skills" }),
    ).toHaveAttribute("aria-selected", "true");
    expect(screen.getByTestId("location")).toHaveTextContent(
      "/agents?target=trae-work&section=skills",
    );
    await user.click(within(configuration).getByRole("tab", { name: "MCP" }));
    expect(screen.getByTestId("location")).toHaveTextContent(
      "/agents?target=trae-work&section=mcp",
    );
    await user.click(
      within(configuration).getByRole("button", { name: "返回软件目录" }),
    );
    expect(screen.getByTestId("location")).toHaveTextContent(/^\/agents$/);
    expect(screen.getByRole("region", { name: "AI 软件目录" })).toBeVisible();

    view.unmount();
    renderPage(ports, "/agents?target=workbuddy&section=mcp");
    await user.click(
      await screen.findByRole("button", { name: "进入 MCP 管理" }),
    );
    expect(screen.getByTestId("location")).toHaveTextContent(/^\/mcp$/);
  });

  it("writes Skills and MCP only through their existing assignment owners and authoritative readback", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    renderPage(ports, "/agents?target=workbuddy&section=skills");

    const skillSwitch = await screen.findByRole("switch", {
      name: "WorkBuddy Skill 分配",
    });
    expect(skillSwitch).not.toBeChecked();
    await user.click(skillSwitch);
    await waitFor(() => expect(skillSwitch).toBeChecked());
    expect(ports.skills.toggleApp).toHaveBeenCalledWith(
      "review",
      "workbuddy",
      true,
    );
    expect(
      await screen.findByText(/已从真实配置回读：WorkBuddy 已启用此 Skill/),
    ).toBeVisible();

    await user.click(screen.getByRole("tab", { name: "MCP" }));
    const mcpSwitch = await screen.findByRole("switch", {
      name: "WorkBuddy MCP 分配",
    });
    expect(mcpSwitch).not.toBeChecked();
    await user.click(mcpSwitch);
    await waitFor(() => expect(mcpSwitch).toBeChecked());
    expect(ports.mcp.toggleApp).toHaveBeenCalledWith(
      "context",
      "workbuddy",
      true,
    );
    expect(
      await screen.findByText(/已从真实配置回读：WorkBuddy 已分配此 MCP/),
    ).toBeVisible();
    const trustDialog = await screen.findByRole("dialog", {
      name: "需要在 WorkBuddy 中信任 MCP",
    });
    expect(trustDialog).toHaveTextContent("连接器 → 自定义连接器");
    await user.click(
      within(trustDialog).getByRole("button", { name: "知道了" }),
    );
    await waitFor(() => {
      expect(
        screen.queryByRole("dialog", {
          name: "需要在 WorkBuddy 中信任 MCP",
        }),
      ).not.toBeInTheDocument();
    });
  });

  it("fails closed when Skill or MCP assignment readback does not match", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.skills.toggleApp = vi.fn(async () => true);
    ports.mcp.toggleApp = vi.fn(async () => undefined);
    renderPage(ports, "/agents?target=workbuddy&section=skills");

    const skillSwitch = await screen.findByRole("switch", {
      name: "WorkBuddy Skill 分配",
    });
    await user.click(skillSwitch);
    expect(
      await screen.findByText(/Skill 分配未能完成或回读不一致/),
    ).toBeVisible();
    expect(skillSwitch).not.toBeChecked();

    await user.click(screen.getByRole("tab", { name: "MCP" }));
    const mcpSwitch = await screen.findByRole("switch", {
      name: "WorkBuddy MCP 分配",
    });
    await user.click(mcpSwitch);
    expect(
      await screen.findByText(/MCP 分配未能完成或回读不一致/),
    ).toBeVisible();
    expect(mcpSwitch).not.toBeChecked();
  });

  it("keeps model capability honest and uses PromptAppId only where an owner exists", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const qoder = renderPage(ports, "/agents?target=qoderwork&section=models");

    expect(
      await screen.findByText(/当前官方能力不支持第三方模型配置/),
    ).toBeVisible();
    expect(screen.queryByRole("switch")).not.toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "提示词" }));
    expect(await screen.findByText(/当前没有 promptAppId/)).toBeVisible();
    expect(ports.prompts.getAll).not.toHaveBeenCalled();

    qoder.unmount();
    const trae = renderPage(ports, "/agents?target=trae-work&section=models");
    expect(await screen.findByText(/需要在供应商界面完成/)).toBeVisible();
    expect(await screen.findAllByText("trae-observed-model")).toHaveLength(2);
    expect(screen.queryByRole("switch")).not.toBeInTheDocument();

    trae.unmount();
    renderPage(ports, "/agents?target=codex&section=prompts");
    await user.click(await screen.findByText("Review prompt"));
    await user.click(screen.getByRole("button", { name: "设为启用" }));
    expect(ports.prompts.enable).toHaveBeenCalledWith("codex", "review");
    expect(
      await screen.findByText(/已从真实配置回读：Codex 当前使用此提示词/),
    ).toBeVisible();
  });

  it("keeps catalog failure explicit instead of inventing a static directory", async () => {
    const ports = configuredPorts();
    ports.catalog.get = vi.fn(async () => {
      throw new Error("catalog unavailable");
    });
    renderPage(ports);

    expect(
      await screen.findByRole(
        "heading",
        { name: "无法加载 Agent 目录" },
        { timeout: 5_000 },
      ),
    ).toBeVisible();
    expect(screen.queryByRole("article")).not.toBeInTheDocument();
    expect(ports.catalog.get).toHaveBeenCalledTimes(2);
  });
});
