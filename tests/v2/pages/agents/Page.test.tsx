import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { AgentsPage } from "@/v2/pages/agents/Page";
import type {
  CodexDesktopPort,
  FeaturePorts,
} from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import type {
  AgentCapabilityId,
  AgentCatalogEntry,
  AgentCatalogId,
  AgentCatalogResult,
} from "@/v2/shared/features/types";
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
      : id === "claude-code"
        ? [
            {
              id: "cli",
              label: "Claude Code CLI",
              url: "https://docs.anthropic.com/en/docs/claude-code/getting-started",
            },
            {
              id: "desktop",
              label: "Claude Desktop",
              url: "https://claude.com/download",
            },
          ]
        : [
            {
              id: "product",
              label: `打开 ${displayName} 官方页面`,
              url:
                id === "qoderwork"
                  ? "https://qoder.com.cn/qoderwork"
                  : id === "trae-work"
                    ? "https://www.trae.cn/sem-work"
                    : id === "grokbuild"
                      ? "https://x.ai/grok"
                      : id === "opencode"
                        ? "https://opencode.ai"
                        : "https://www.workbuddy.cn/",
            },
          ];
  return {
    id,
    variantId: variantById[id],
    displayName,
    description: `${displayName} 的目录说明`,
    officialLinks,
    capabilities: capabilityIds.map((capabilityId) => ({
      id: capabilityId,
      mode:
        capabilityId === "product.open" && id === "codex"
          ? "unsupported"
          : capabilityId === "app.detect" || capabilityId === "app.launch"
            ? "unverified"
            : id === "trae-work" &&
                (capabilityId === "models.validate" ||
                  capabilityId === "models.write")
              ? "assisted"
              : "direct",
      reasonCode:
        capabilityId === "product.open" && id === "codex"
          ? "no_catalog_product_link"
          : capabilityId === "app.detect" || capabilityId === "app.launch"
            ? "trusted_runtime_identity_unavailable"
            : id === "trae-work" &&
                (capabilityId === "models.validate" ||
                  capabilityId === "models.write")
              ? "vendor_ui_required"
              : "dedicated_native_contract",
      evidenceIds: ["p0_scope"],
    })),
  };
}

function catalog(): AgentCatalogResult {
  return {
    contractVersion: 4,
    reviewedAt: "2026-08-20",
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

function LocationProbe() {
  const location = useLocation();
  return (
    <output data-testid="location">
      {location.pathname}
      {location.search}
    </output>
  );
}

function renderPage(ports: FeaturePorts) {
  return render(
    <MemoryRouter initialEntries={["/agents"]}>
      <FeatureProvider ports={ports}>
        <AgentsPage />
        <LocationProbe />
      </FeatureProvider>
    </MemoryRouter>,
  );
}

function configuredPorts(): FeaturePorts {
  const ports = createBrowserFeaturePorts();
  ports.catalog.get = async () => catalog();
  ports.workbuddy.getStatus = async () => ({
    path: "C:/redacted/models.json",
    backupPath: "C:/redacted/models.json.backup",
    exists: true,
    modelCount: 3,
    revision: "opaque-revision",
    backupExists: true,
    format: "legacyArray",
  });
  ports.providers.getSummary = async (app) => ({
    providers: {
      [`fyagent-${app}`]: {
        id: `fyagent-${app}`,
        name: `${app} current`,
        websiteUrl: "https://provider.example",
        category: "custom",
      },
    },
    currentId: `fyagent-${app}`,
    writeTargets: [
      {
        path: "~/.config/provider/config.json",
        backupPath: "~/.config/provider/config.json.fyagent.backup",
        exists: true,
      },
    ],
  });
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
      checkedAt: "2026-08-14T00:00:00.000Z",
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

describe("V2 Agent directory", () => {
  it("renders the native catalog in order and supports keyboard selection", async () => {
    const user = userEvent.setup();
    renderPage(configuredPorts());

    const selector = await screen.findByRole("region", {
      name: "Agent 选择",
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
    expect(buttons[0]).toHaveAttribute("aria-current", "true");
    expect(
      screen.getByRole("region", { name: "QoderWork CN 详情" }),
    ).toBeVisible();
    const listArtwork = buttons[0].querySelector('[data-size="list"] img');
    expect(listArtwork).toHaveAttribute("alt", "");
    expect(listArtwork).toHaveAttribute("aria-hidden", "true");
    const detail = screen.getByRole("region", {
      name: "QoderWork CN 详情",
    });
    expect(detail.querySelector('[data-size="detail"] img')).toHaveAttribute(
      "aria-hidden",
      "true",
    );

    await user.tab();
    expect(buttons[0]).toHaveFocus();
    await user.tab();
    expect(buttons[1]).toHaveFocus();
    await user.keyboard("{Enter}");

    expect(buttons[1]).toHaveAttribute("aria-current", "true");
    expect(buttons[0]).not.toHaveAttribute("aria-current");
    expect(
      screen.getByRole("region", { name: "TRAE Work CN 详情" }),
    ).toBeVisible();
  });

  it("renders every catalog-owned official link and no Codex external action", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.settings.openExternal = vi.fn(async () => undefined);
    renderPage(ports);

    const qoderDetail = await screen.findByRole("region", {
      name: "QoderWork CN 详情",
    });
    const qoderOfficial = within(qoderDetail).getByRole("button", {
      name: "打开 QoderWork CN 官方页面",
    });
    expect(qoderOfficial).toHaveClass("fy-control-button-primary");
    expect(
      within(qoderDetail).getByRole("group", { name: "官方网站" }),
    ).toBeVisible();
    expect(
      qoderOfficial.compareDocumentPosition(
        within(qoderDetail).getByRole("heading", { name: "支持的功能" }),
      ) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    await user.click(qoderOfficial);
    await user.click(
      await screen.findByRole("button", { name: /TRAE Work CN/ }),
    );
    expect(
      screen.queryByRole("button", { name: "查看模型说明" }),
    ).not.toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "打开 TRAE Work CN 官方页面" }),
    );

    await user.click(screen.getByRole("button", { name: /WorkBuddy/ }));
    await user.click(
      screen.getByRole("button", { name: "打开 WorkBuddy 官方页面" }),
    );

    await user.click(screen.getByRole("button", { name: /Grok Build/ }));
    await user.click(
      screen.getByRole("button", { name: "打开 Grok Build 官方页面" }),
    );

    await user.click(screen.getByRole("button", { name: /Claude Code/ }));
    await user.click(
      screen.getByRole("button", { name: "打开 Claude Code CLI 官网" }),
    );
    await user.click(
      screen.getByRole("button", { name: "打开 Claude Desktop 官网" }),
    );

    await user.click(screen.getByRole("button", { name: /^Codex/ }));
    const codexDetail = screen.getByRole("region", { name: "Codex 详情" });
    expect(
      within(codexDetail).queryByRole("group", { name: "官方网站" }),
    ).not.toBeInTheDocument();
    expect(
      within(codexDetail).queryByRole("button", { name: /官方|CLI/ }),
    ).not.toBeInTheDocument();
    const installer = within(codexDetail).getByRole("region", {
      name: "Codex Desktop 安装器",
    });
    expect(
      installer.compareDocumentPosition(
        within(codexDetail).getByRole("heading", { name: "支持的功能" }),
      ) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    expect(ports.settings.openExternal).toHaveBeenCalledTimes(6);
    expect(ports.settings.openExternal).toHaveBeenNthCalledWith(
      1,
      "https://qoder.com.cn/qoderwork",
    );
    expect(ports.settings.openExternal).toHaveBeenNthCalledWith(
      2,
      "https://www.trae.cn/sem-work",
    );
    expect(ports.settings.openExternal).toHaveBeenNthCalledWith(
      3,
      "https://www.workbuddy.cn/",
    );
    expect(ports.settings.openExternal).toHaveBeenNthCalledWith(
      4,
      "https://x.ai/grok",
    );
    expect(ports.settings.openExternal).toHaveBeenNthCalledWith(
      5,
      "https://docs.anthropic.com/en/docs/claude-code/getting-started",
    );
    expect(ports.settings.openExternal).toHaveBeenNthCalledWith(
      6,
      "https://claude.com/download",
    );
  });

  it("keeps one open lock while showing pending state only on the active link", async () => {
    const user = userEvent.setup();
    let releaseOpen!: () => void;
    const opening = new Promise<void>((resolve) => {
      releaseOpen = resolve;
    });
    const ports = configuredPorts();
    ports.settings.openExternal = vi.fn(() => opening);
    renderPage(ports);

    await user.click(
      await screen.findByRole("button", { name: /Claude Code/ }),
    );
    const cliLink = screen.getByRole("button", {
      name: "打开 Claude Code CLI 官网",
    });
    const desktopLink = screen.getByRole("button", {
      name: "打开 Claude Desktop 官网",
    });
    await user.click(cliLink);
    expect(cliLink).toHaveTextContent("正在打开…");
    expect(desktopLink).toBeDisabled();
    expect(ports.settings.openExternal).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: /WorkBuddy/ }));
    const workBuddyLink = screen.getByRole("button", {
      name: "打开 WorkBuddy 官方页面",
    });
    expect(workBuddyLink).toBeDisabled();
    expect(workBuddyLink).toHaveTextContent("打开 WorkBuddy 官方页面");

    releaseOpen();
    await waitFor(() => expect(workBuddyLink).toBeEnabled());
  });

  it("navigates from Agent detail to models, Skills, and MCP for direct capabilities", async () => {
    const user = userEvent.setup();
    renderPage(configuredPorts());

    const qoderDetail = await screen.findByRole("region", {
      name: "QoderWork CN 详情",
    });
    expect(
      within(qoderDetail).getByRole("heading", { name: "支持的功能" }),
    ).toBeVisible();
    expect(
      within(qoderDetail).getByRole("region", { name: "产品介绍" }),
    ).toHaveTextContent("QoderWork CN 是阿里云");
    expect(qoderDetail).not.toHaveTextContent("FyAgent");
    expect(qoderDetail).not.toHaveTextContent("Hooks");
    expect(qoderDetail).not.toHaveTextContent("应用识别");
    expect(qoderDetail).not.toHaveTextContent("查看 Skills");
    expect(qoderDetail).not.toHaveTextContent("的目录说明");
    expect(qoderDetail).not.toHaveTextContent("应用状态");
    expect(qoderDetail).not.toHaveTextContent("配置概览");
    expect(qoderDetail).not.toHaveTextContent("不适用的功能");
    expect(qoderDetail).not.toHaveTextContent("使用说明");
    expect(
      within(qoderDetail).queryByRole("button", { name: "管理 Hooks" }),
    ).not.toBeInTheDocument();
    expect(
      within(qoderDetail).queryByRole("region", {
        name: "QoderWork Hooks 配置",
      }),
    ).not.toBeInTheDocument();
    expect(
      within(qoderDetail).queryByRole("region", { name: "MCP 配置检查" }),
    ).not.toBeInTheDocument();
    await user.click(
      within(qoderDetail).getByRole("button", { name: "打开 Skills" }),
    );
    expect(screen.getByTestId("location")).toHaveTextContent("/skills");

    await user.click(screen.getByRole("button", { name: /WorkBuddy/ }));
    const workBuddyDetail = screen.getByRole("region", {
      name: "WorkBuddy 详情",
    });
    await user.click(
      within(workBuddyDetail).getByRole("button", { name: "配置模型" }),
    );
    expect(screen.getByTestId("location")).toHaveTextContent(
      "/models?target=workbuddy",
    );

    await user.click(screen.getByRole("button", { name: /Claude Code/ }));
    const claudeDetail = screen.getByRole("region", {
      name: "Claude Code 详情",
    });
    await user.click(
      within(claudeDetail).getByRole("button", { name: "打开 MCP" }),
    );
    expect(screen.getByTestId("location")).toHaveTextContent("/mcp");

    await user.click(screen.getByRole("button", { name: /^Codex/ }));
    const codexDetail = screen.getByRole("region", { name: "Codex 详情" });
    expect(
      within(codexDetail).queryByRole("region", { name: "产品介绍" }),
    ).not.toBeInTheDocument();
    expect(codexDetail).not.toHaveTextContent("FyAgent");
    expect(codexDetail).not.toHaveTextContent("不适用的功能");
    expect(codexDetail).not.toHaveTextContent("项支持");
  });

  it("does not read WorkBuddy or Provider observation on the Agent page", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.workbuddy.getStatus = vi.fn(ports.workbuddy.getStatus);
    ports.providers.getSummary = vi.fn(ports.providers.getSummary);
    renderPage(ports);

    await user.click(await screen.findByRole("button", { name: /WorkBuddy/ }));
    expect(ports.workbuddy.getStatus).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: /Claude Code/ }));
    expect(ports.providers.getSummary).not.toHaveBeenCalled();
  });

  it("mounts the native installer only for Codex and cleans up on selection change", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const cleanup = vi.fn();
    ports.codexDesktop.getLocalStatus = vi.fn(
      ports.codexDesktop.getLocalStatus,
    );
    ports.codexDesktop.subscribeJobUpdates = vi.fn(async () => cleanup);
    renderPage(ports);

    await screen.findByRole("region", { name: "QoderWork CN 详情" });
    expect(ports.codexDesktop.getLocalStatus).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("region", { name: "Codex Desktop 安装器" }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /^Codex/ }));
    expect(
      await screen.findByRole("region", { name: "Codex Desktop 安装器" }),
    ).toBeVisible();
    await waitFor(() =>
      expect(ports.codexDesktop.getLocalStatus).toHaveBeenCalled(),
    );

    await user.click(screen.getByRole("button", { name: /WorkBuddy/ }));
    await waitFor(() => expect(cleanup).toHaveBeenCalledTimes(1));
    expect(
      screen.queryByRole("region", { name: "Codex Desktop 安装器" }),
    ).not.toBeInTheDocument();
  });

  it("does not replace an unavailable native catalog with static entries", async () => {
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
    expect(
      screen.queryByRole("button", { name: /QoderWork/ }),
    ).not.toBeInTheDocument();
    expect(ports.catalog.get).toHaveBeenCalledTimes(2);
  });
});
