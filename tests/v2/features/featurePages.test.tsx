import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { McpPage } from "@/v2/pages/mcp/Page";
import { SkillsPage } from "@/v2/pages/skills/Page";
import type { FeaturePorts } from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import {
  createAssignments,
  createMcpAssignments,
  MCP_TARGETS,
  SKILL_TARGETS,
  type InstalledSkill,
  type McpServer,
  type SkillHubSkill,
  type UnmanagedSkill,
} from "@/v2/shared/features/types";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";

function appearsBefore(first: HTMLElement, second: HTMLElement) {
  expect(
    first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING,
  ).not.toBe(0);
}

function renderFeature(page: React.ReactNode, ports: FeaturePorts) {
  return render(<FeatureProvider ports={ports}>{page}</FeatureProvider>);
}

function installedSkill(id: string, name: string): InstalledSkill {
  return {
    id,
    name,
    directory: id,
    apps: createAssignments(["claude"]),
    installedAt: 1,
    updatedAt: 1,
  };
}

function marketSkill(overrides: Partial<SkillHubSkill> = {}): SkillHubSkill {
  return {
    key: "skillhub:review-skill",
    slug: "review-skill",
    name: "Review Skill",
    description: "Review changes",
    directory: "review-skill",
    repoOwner: "skillhub.cn",
    repoName: "review-skill",
    repoBranch: "skillhub",
    version: "1.0.0",
    ownerName: "acme",
    homepageUrl: "https://skillhub.cn/skills/review-skill",
    readmeUrl: "https://skillhub.cn/skills/review-skill",
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

describe("V2 MCP management", () => {
  it("keeps secrets out of ordinary UI and preserves advanced extensions", async () => {
    const user = userEvent.setup();
    const secret = "ultra-private-token";
    const server: McpServer = {
      id: "docs",
      name: "Docs server",
      description: "Documentation helper",
      apps: {
        ...createMcpAssignments(["claude"]),
        gemini: true,
        hermes: true,
        hiddenClient: true,
      },
      server: {
        type: "stdio",
        command: "npx",
        env: { SECRET_TOKEN: secret },
        extension: { keep: true },
      },
    };
    const upsert = vi.fn(async (serverToSave: McpServer) => {
      void serverToSave;
    });
    const ports = createBrowserFeaturePorts();
    ports.mcp.getAll = async () => ({ docs: server });
    ports.mcp.upsert = upsert;

    renderFeature(<McpPage />, ports);

    expect(
      await screen.findByRole("heading", { name: "Docs server" }),
    ).toBeVisible();
    expect(document.body).not.toHaveTextContent(secret);
    expect(screen.getAllByRole("switch")).toHaveLength(7);
    expect(
      screen
        .getAllByRole("switch")
        .map((node) => node.getAttribute("aria-label")),
    ).toEqual(MCP_TARGETS.map((app) => `${app.label} MCP 分配`));
    expect(screen.getByText(/stdio · 1 Agent/)).toBeVisible();
    expect(screen.getByRole("region", { name: "安装来源" })).toHaveTextContent(
      "手动添加",
    );
    expect(screen.getByRole("region", { name: "安装来源" })).toHaveTextContent(
      "无本地安装目录",
    );
    expect(screen.getByRole("region", { name: "当前分配" })).toHaveTextContent(
      "Claude Code",
    );
    expect(screen.getByRole("region", { name: "安装信息" })).toHaveTextContent(
      "stdio",
    );
    appearsBefore(
      screen.getByRole("button", { name: "编辑" }),
      screen.getByRole("region", { name: "安装来源" }),
    );
    appearsBefore(
      screen.getByRole("button", { name: "删除" }),
      screen.getByRole("region", { name: "安装信息" }),
    );

    await user.click(screen.getByRole("button", { name: "编辑" }));
    const dialog = screen.getByRole("dialog", { name: "编辑 Docs server" });
    expect(within(dialog).getByDisplayValue(/SECRET_TOKEN/)).toHaveValue(
      `SECRET_TOKEN=${secret}`,
    );

    await user.click(within(dialog).getByRole("tab", { name: "JSON 编辑" }));
    const advanced = within(dialog).getByLabelText("单个服务配置（JSON）");
    const advancedValue = JSON.parse((advanced as HTMLTextAreaElement).value);
    advancedValue.secondExtension = "preserved";
    fireEvent.change(advanced, {
      target: { value: JSON.stringify(advancedValue) },
    });
    await user.click(within(dialog).getByRole("tab", { name: "快速配置" }));
    await user.click(within(dialog).getByRole("tab", { name: "JSON 编辑" }));
    expect((advanced as HTMLTextAreaElement).value).toContain(
      "secondExtension",
    );

    await user.click(within(dialog).getByRole("button", { name: "保存" }));
    await waitFor(() => expect(upsert).toHaveBeenCalledTimes(1));
    expect(upsert.mock.calls[0][0]).toMatchObject({
      apps: {
        claude: true,
        gemini: true,
        grokbuild: false,
        hermes: true,
        hiddenClient: true,
      },
      server: {
        env: { SECRET_TOKEN: secret },
        extension: { keep: true },
        secondExtension: "preserved",
      },
    });
  });

  it("distinguishes a zero-result import", async () => {
    const user = userEvent.setup();
    let reads = 0;
    const ports = createBrowserFeaturePorts();
    ports.mcp.getAll = vi.fn(async () => {
      reads += 1;
      if (reads === 1) return {};
      throw new Error("MCP refresh unavailable");
    });
    ports.mcp.importFromApps = vi.fn(async () => 0);

    renderFeature(<McpPage />, ports);
    await screen.findByText("还没有 MCP 服务");
    await user.click(screen.getAllByRole("button", { name: "导入现有" })[0]);

    expect(await screen.findByText("没有发现可导入的 MCP")).toBeVisible();
    expect(
      await screen.findByText(/刷新失败，正在显示上一次成功数据/, undefined, {
        timeout: 4_000,
      }),
    ).toHaveTextContent("请稍后重试。");
    expect(document.body).not.toHaveTextContent("MCP refresh unavailable");
    expect(screen.getByText("还没有 MCP 服务")).toBeVisible();
    expect(screen.queryByText("无法加载 MCP")).not.toBeInTheDocument();
  });

  it("keeps cached MCP data visible when a write-triggered refresh fails", async () => {
    const user = userEvent.setup();
    const server: McpServer = {
      id: "docs",
      name: "Docs server",
      apps: createAssignments(["claude"]),
      server: { type: "stdio", command: "npx" },
    };
    let reads = 0;
    const getAll = vi.fn(async () => {
      reads += 1;
      if (reads === 1) return { docs: server };
      throw new Error("MCP refresh unavailable");
    });
    const ports = createBrowserFeaturePorts();
    ports.mcp.getAll = getAll;
    ports.mcp.toggleApp = vi.fn(async () => undefined);

    renderFeature(<McpPage />, ports);
    expect(
      await screen.findByRole("heading", { name: "Docs server" }),
    ).toBeVisible();

    const assignment = screen.getByRole("switch", {
      name: "Claude Code MCP 分配",
    });
    await user.click(assignment);

    expect(
      await screen.findByText(/刷新失败，正在显示上一次成功数据/, undefined, {
        timeout: 4_000,
      }),
    ).toHaveTextContent("请稍后重试。");
    expect(document.body).not.toHaveTextContent("MCP refresh unavailable");
    expect(screen.getByRole("heading", { name: "Docs server" })).toBeVisible();
    expect(assignment).toBeChecked();
    expect(screen.queryByText("无法加载 MCP")).not.toBeInTheDocument();
  });

  it("redacts backend configuration details from import and toggle errors", async () => {
    const user = userEvent.setup();
    const secret = "sk-sentinel-secret";
    const server: McpServer = {
      id: "docs",
      name: "Docs server",
      apps: createAssignments(["claude"]),
      server: { type: "stdio", command: "npx" },
    };
    const ports = createBrowserFeaturePorts();
    ports.mcp.getAll = async () => ({ docs: server });
    ports.mcp.importFromApps = vi.fn(async () => {
      throw new Error(`parser source: OPENAI_API_KEY = ${secret}`);
    });
    ports.mcp.toggleApp = vi.fn(async () => {
      throw new Error(`parser source: OPENAI_API_KEY = ${secret}`);
    });

    renderFeature(<McpPage />, ports);
    await screen.findByRole("heading", { name: "Docs server" });
    await user.click(screen.getAllByRole("button", { name: "导入现有" })[0]);
    expect(
      await screen.findByText(
        "MCP 配置中的敏感字段未通过校验，请检查对应字段格式",
      ),
    ).toBeVisible();
    expect(document.body).not.toHaveTextContent(secret);

    await user.click(
      screen.getByRole("switch", { name: "Claude Code MCP 分配" }),
    );
    await waitFor(() => expect(ports.mcp.toggleApp).toHaveBeenCalledTimes(1));
    expect(document.body).not.toHaveTextContent(secret);
  });

  it("keeps cross-app import conflicts actionable without echoing the server ID", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.mcp.importFromApps = vi.fn(async () => {
      throw new Error(
        "MCP 服务器 'secret-shaped-server-id' 在多个应用中的配置冲突；未合并 codex 分配",
      );
    });

    renderFeature(<McpPage />, ports);
    await screen.findByText("还没有 MCP 服务");
    await user.click(screen.getAllByRole("button", { name: "导入现有" })[0]);

    expect(
      await screen.findByText(
        "检测到同名 MCP 服务器的配置冲突，未合并 Codex 分配；请统一两端配置或更改服务器 ID",
      ),
    ).toBeVisible();
    expect(document.body).not.toHaveTextContent("secret-shaped-server-id");
  });

  it("redacts secret-bearing MCP URLs and arguments in ordinary details", async () => {
    const secret = "amap-query-secret";
    const server: McpServer = {
      id: "amap",
      name: "高德地图 MCP",
      apps: createAssignments(["claude"]),
      server: {
        type: "http",
        url: `https://mcp.amap.com/mcp?key=${secret}`,
        args: ["mcp", "-s", "feishu-app-secret"],
      },
    };
    const ports = createBrowserFeaturePorts();
    ports.mcp.getAll = async () => ({ amap: server });
    renderFeature(<McpPage />, ports);

    expect(
      await screen.findByRole("heading", { name: "高德地图 MCP" }),
    ).toBeVisible();
    expect(document.body).not.toHaveTextContent(secret);
    expect(document.body).not.toHaveTextContent("feishu-app-secret");
    expect(document.body).toHaveTextContent(
      "https://mcp.amap.com/mcp?key=••••••",
    );
    expect(screen.getByRole("region", { name: "安装来源" })).toHaveTextContent(
      "精选目录",
    );
    expect(screen.getByRole("region", { name: "安装来源" })).toHaveTextContent(
      "无本地安装目录",
    );
    expect(screen.getByRole("region", { name: "当前分配" })).toHaveTextContent(
      "Claude Code",
    );
    appearsBefore(
      screen.getByRole("button", { name: "编辑" }),
      screen.getByRole("region", { name: "安装来源" }),
    );
  });

  it("shows a copyable MCP install directory for an absolute command", async () => {
    const user = userEvent.setup();
    const command =
      "C:\\Users\\xk\\AppData\\Local\\OpenAI\\Codex\\runtimes\\cua_node\\node.exe";
    const directory =
      "C:\\Users\\xk\\AppData\\Local\\OpenAI\\Codex\\runtimes\\cua_node";
    const server: McpServer = {
      id: "node_repl",
      name: "node_repl",
      apps: createAssignments(["codex"]),
      server: { type: "stdio", command },
    };
    const ports = createBrowserFeaturePorts();
    ports.mcp.getAll = async () => ({ node_repl: server });
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    renderFeature(<McpPage />, ports);

    expect(
      await screen.findByRole("heading", { name: "node_repl" }),
    ).toBeVisible();
    expect(screen.getByRole("region", { name: "安装来源" })).toHaveTextContent(
      directory,
    );
    expect(screen.queryByText("无本地安装目录")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "复制安装目录" }));
    expect(writeText).toHaveBeenCalledWith(directory);
  });

  it("installs a zero-config catalog item onto the default MCP targets", async () => {
    const user = userEvent.setup();
    const store: Record<string, McpServer> = {};
    const upsert = vi.fn(async (server: McpServer) => {
      store[server.id] = server;
    });
    const ports = createBrowserFeaturePorts();
    ports.mcp.getAll = vi.fn(async () => ({ ...store }));
    ports.mcp.upsert = upsert;
    renderFeature(<McpPage />, ports);

    await screen.findByText("还没有 MCP 服务");
    await user.click(screen.getByRole("tab", { name: "发现" }));
    const card = screen
      .getByRole("heading", { name: "Time" })
      .closest("article");
    expect(card).not.toBeNull();
    await user.click(
      within(card as HTMLElement).getByRole("button", { name: "安装" }),
    );

    await waitFor(() => expect(upsert).toHaveBeenCalledTimes(1));
    expect(upsert.mock.calls[0]?.[0]).toMatchObject({
      id: "time",
      apps: {
        claude: true,
        codex: true,
        opencode: true,
        workbuddy: true,
      },
      server: { type: "stdio", command: "uvx", args: ["mcp-server-time"] },
    });
  });

  it("opens discover docs through the shared external-link outlet", async () => {
    const user = userEvent.setup();
    const openExternal = vi.fn(async () => undefined);
    const ports = createBrowserFeaturePorts();
    ports.settings.openExternal = openExternal;
    renderFeature(<McpPage />, ports);

    await user.click(await screen.findByRole("tab", { name: "发现" }));
    const card = screen
      .getByRole("heading", { name: "Time" })
      .closest("article");
    expect(card).not.toBeNull();
    await user.click(
      within(card as HTMLElement).getByRole("button", { name: "文档" }),
    );
    expect(openExternal).toHaveBeenCalledWith(
      "https://github.com/modelcontextprotocol/servers/tree/main/src/time",
    );
  });

  it("does not silently overwrite a conflicting catalog id", async () => {
    const user = userEvent.setup();
    const existing: McpServer = {
      id: "time",
      name: "Custom time",
      apps: createAssignments(["claude"]),
      server: { type: "stdio", command: "python", args: ["time.py"] },
    };
    const upsert = vi.fn(async () => undefined);
    const ports = createBrowserFeaturePorts();
    ports.mcp.getAll = async () => ({ time: existing });
    ports.mcp.upsert = upsert;
    renderFeature(<McpPage />, ports);

    await screen.findByRole("heading", { name: "Custom time" });
    await user.click(screen.getByRole("tab", { name: "发现" }));
    const card = screen
      .getByRole("heading", { name: "Time" })
      .closest("article");
    expect(card).not.toBeNull();
    expect(
      within(card as HTMLElement).getByRole("button", { name: "已存在" }),
    ).toBeDisabled();
    expect(upsert).not.toHaveBeenCalled();

    await user.click(
      within(card as HTMLElement).getByRole("button", { name: "重新配置" }),
    );
    await user.click(screen.getByRole("button", { name: "确认" }));
    await waitFor(() =>
      expect(upsert).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "time",
          server: {
            type: "stdio",
            command: "uvx",
            args: ["mcp-server-time"],
          },
        }),
      ),
    );
  });

  it("keeps catalog install dialogs free of launch-command details", async () => {
    const user = userEvent.setup();
    const upsert = vi.fn(async () => undefined);
    const ports = createBrowserFeaturePorts();
    ports.mcp.upsert = upsert;
    renderFeature(<McpPage />, ports);

    await screen.findByText("还没有 MCP 服务");
    await user.click(screen.getByRole("tab", { name: "发现" }));
    const card = screen
      .getByRole("heading", { name: "高德地图 MCP" })
      .closest("article");
    expect(card).not.toBeNull();
    await user.click(
      within(card as HTMLElement).getByRole("button", { name: "配置并安装" }),
    );

    const dialog = await screen.findByRole("dialog", {
      name: "安装 高德地图 MCP",
    });
    expect(dialog).not.toHaveTextContent("npx");
    expect(dialog).not.toHaveTextContent("cmd");
    expect(dialog).not.toHaveTextContent("mcp.amap.com");
    await user.type(
      within(dialog).getByLabelText(/API Key/),
      "amap-query-secret",
    );
    await user.click(within(dialog).getByRole("button", { name: "安装" }));
    await waitFor(() =>
      expect(upsert).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "amap",
          server: {
            type: "http",
            url: "https://mcp.amap.com/mcp?key=amap-query-secret",
          },
        }),
      ),
    );
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: "安装 高德地图 MCP" }),
      ).not.toBeInTheDocument(),
    );
    expect(document.body).not.toHaveTextContent("amap-query-secret");
  });

  it("installs a zero-config China catalog item onto the default MCP targets", async () => {
    const user = userEvent.setup();
    const store: Record<string, McpServer> = {};
    const upsert = vi.fn(async (server: McpServer) => {
      store[server.id] = server;
    });
    const ports = createBrowserFeaturePorts();
    ports.mcp.getAll = vi.fn(async () => ({ ...store }));
    ports.mcp.upsert = upsert;
    renderFeature(<McpPage />, ports);

    await screen.findByText("还没有 MCP 服务");
    await user.click(screen.getByRole("tab", { name: "发现" }));
    const card = screen
      .getByRole("heading", { name: "AntV 图表 MCP" })
      .closest("article");
    expect(card).not.toBeNull();
    await user.click(
      within(card as HTMLElement).getByRole("button", { name: "安装" }),
    );

    await waitFor(() => expect(upsert).toHaveBeenCalledTimes(1));
    const installed = upsert.mock.calls[0]?.[0];
    expect(installed?.id).toBe("antv-chart");
    expect(installed?.server.type).toBe("stdio");
    expect(installed?.server.args).toEqual(
      expect.arrayContaining(["-y", "@antv/mcp-server-chart"]),
    );
  });

  it("filters the discovery catalog by install mode", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    renderFeature(<McpPage />, ports);

    await screen.findByText("还没有 MCP 服务");
    await user.click(screen.getByRole("tab", { name: "发现" }));
    await user.selectOptions(screen.getByLabelText("分类筛选"), "ready");
    expect(
      screen.getByRole("heading", { name: "Playwright MCP" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("heading", { name: "高德地图 MCP" }),
    ).not.toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("分类筛选"), "configure");
    expect(screen.getByRole("heading", { name: "高德地图 MCP" })).toBeVisible();
    expect(
      screen.queryByRole("heading", { name: "Playwright MCP" }),
    ).not.toBeInTheDocument();
  });
});

describe("V2 Skills management", () => {
  it("submits the supported foundIn intersection when importing unmanaged Skills", async () => {
    const user = userEvent.setup();
    const unmanaged: UnmanagedSkill = {
      directory: "review-skill",
      name: "Review Skill",
      foundIn: ["Claude", "CODEX", "openclaw"],
      path: "C:/tmp/review-skill",
    };
    const importFromApps = vi.fn(
      async (
        imports: Parameters<FeaturePorts["skills"]["importFromApps"]>[0],
      ) => {
        void imports;
        return [];
      },
    );
    let imported = false;
    const getRepos = vi.fn(async () =>
      imported
        ? [{ owner: "acme", name: "skills", branch: "main", enabled: true }]
        : [],
    );
    const ports = createBrowserFeaturePorts();
    ports.skills.scanUnmanaged = async () => [unmanaged];
    ports.skills.getRepos = getRepos;
    ports.skills.importFromApps = vi.fn(async (imports) => {
      imported = true;
      await importFromApps(imports);
      return [];
    });

    renderFeature(<SkillsPage />, ports);
    await screen.findByText("还没有安装 Skill");
    await user.click(screen.getByRole("button", { name: "更多" }));
    await user.click(screen.getByRole("button", { name: "导入本地 Skill" }));

    const dialog = await screen.findByRole("dialog", {
      name: "导入本地 Skills",
    });
    expect(
      within(dialog).getByRole("checkbox", { name: /Claude/ }),
    ).toBeChecked();
    expect(
      within(dialog).getByRole("checkbox", { name: /Codex/ }),
    ).toBeChecked();
    expect(
      within(dialog).getByRole("checkbox", { name: /OpenCode/ }),
    ).not.toBeChecked();

    await user.click(
      within(dialog).getByRole("button", { name: "导入所选 · 1" }),
    );
    await waitFor(() => expect(importFromApps).toHaveBeenCalledTimes(1));
    expect(importFromApps).toHaveBeenCalledWith([
      {
        directory: "review-skill",
        apps: expect.objectContaining({
          claude: true,
          codex: true,
          opencode: false,
          qoderwork: false,
          "trae-work": false,
          workbuddy: false,
        }),
      },
    ]);
    await waitFor(() => expect(getRepos).toHaveBeenCalledTimes(2));
    await user.click(screen.getByRole("tab", { name: "发现" }));
    expect(screen.queryByText("尚未配置仓库")).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "仓库" })).not.toBeInTheDocument();
  });

  it("keeps cached Skills visible when a write-triggered refresh fails", async () => {
    const user = userEvent.setup();
    const skill = installedSkill("review-skill", "Review Skill");
    let reads = 0;
    const getInstalled = vi.fn(async () => {
      reads += 1;
      if (reads === 1) return [skill];
      throw new Error("Skills refresh unavailable");
    });
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = getInstalled;
    ports.skills.toggleApp = vi.fn(async () => true);

    renderFeature(<SkillsPage />, ports);
    expect(
      await screen.findByRole("heading", { name: "Review Skill" }),
    ).toBeVisible();

    expect(
      screen
        .getAllByRole("switch")
        .map((node) => node.getAttribute("aria-label")),
    ).toEqual(SKILL_TARGETS.map((app) => `${app.label} Skill 分配`));

    const assignment = screen.getByRole("switch", {
      name: "Claude Code Skill 分配",
    });
    await user.click(assignment);

    expect(
      await screen.findByText(
        /刷新失败，正在显示上一次成功加载的数据/,
        undefined,
        { timeout: 4_000 },
      ),
    ).toHaveTextContent("请稍后重试。");
    expect(document.body).not.toHaveTextContent("Skills refresh unavailable");
    expect(screen.getByRole("heading", { name: "Review Skill" })).toBeVisible();
    expect(assignment).toBeChecked();
    expect(screen.queryByText("无法加载 Skills")).not.toBeInTheDocument();
  });

  it("refreshes update availability after a partially successful batch", async () => {
    const user = userEvent.setup();
    const alpha = installedSkill("alpha", "Alpha Skill");
    const beta = installedSkill("beta", "Beta Skill");
    let updateReads = 0;
    const checkUpdates = vi.fn(async () => {
      updateReads += 1;
      return updateReads === 1
        ? [
            { id: alpha.id, name: alpha.name, remoteHash: "alpha-next" },
            { id: beta.id, name: beta.name, remoteHash: "beta-next" },
          ]
        : [{ id: beta.id, name: beta.name, remoteHash: "beta-next" }];
    });
    const update = vi.fn(async (id: string) => {
      if (id === beta.id) throw new Error("Beta update failed");
      return alpha;
    });
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = async () => [alpha, beta];
    ports.skills.checkUpdates = checkUpdates;
    ports.skills.update = update;

    renderFeature(<SkillsPage />, ports);
    await screen.findByRole("heading", { name: "Alpha Skill" });
    await user.click(screen.getByRole("button", { name: "检查更新" }));
    const updateAll = await screen.findByRole("button", {
      name: "更新全部 · 2",
    });

    await user.click(updateAll);

    expect(
      await screen.findByRole("button", { name: "更新全部 · 1" }),
    ).toBeVisible();
    expect(update).toHaveBeenCalledTimes(2);
    expect(checkUpdates).toHaveBeenCalledTimes(2);
    expect(screen.getByText("批量更新完成失败")).toBeVisible();
    expect(screen.getByText("1 项失败，1 项成功")).toBeVisible();
  });

  it("shows download source and assigned apps in installed skill details", async () => {
    const user = userEvent.setup();
    const remote: InstalledSkill = {
      ...installedSkill("review-skill", "Review Skill"),
      description: "Review changes in pull requests",
      repoOwner: "acme",
      repoName: "skills",
      repoBranch: "main",
      apps: createAssignments(["claude", "codex"]),
      path: "C:\\Users\\xk\\AppData\\Roaming\\fyagent\\skills\\review-skill",
    };
    const local: InstalledSkill = {
      ...installedSkill("local-notes", "Local Notes"),
      apps: createAssignments(),
      installedAt: 0,
    };
    const openExternal = vi.fn(async () => undefined);
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = async () => [remote, local];
    ports.settings.openExternal = openExternal;

    renderFeature(<SkillsPage />, ports);

    expect(
      await screen.findByRole("region", { name: "下载来源" }),
    ).toHaveTextContent("GitHub 仓库");
    expect(screen.getByRole("region", { name: "下载来源" })).toHaveTextContent(
      "acme/skills",
    );
    expect(
      within(screen.getByRole("region", { name: "Skill 详情" })).getByText(
        "Review changes in pull requests",
      ),
    ).toBeVisible();
    const assignment = screen.getByRole("region", { name: "当前分配" });
    expect(assignment).toHaveTextContent("Claude Code");
    expect(assignment).toHaveTextContent("Codex");
    expect(assignment).not.toHaveTextContent("Gemini");

    await user.click(screen.getByRole("button", { name: "打开仓库" }));
    expect(openExternal).toHaveBeenCalledWith("https://github.com/acme/skills");

    const installPath =
      "C:\\Users\\xk\\AppData\\Roaming\\fyagent\\skills\\review-skill";
    expect(screen.getByRole("region", { name: "下载来源" })).toHaveTextContent(
      installPath,
    );
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    await user.click(screen.getByRole("button", { name: "复制安装目录" }));
    expect(writeText).toHaveBeenCalledWith(installPath);
    expect(
      screen.queryByRole("button", { name: "展开" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "收起" }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Local Notes/ }));
    expect(screen.getByRole("region", { name: "下载来源" })).toHaveTextContent(
      "本地导入",
    );
    expect(screen.getByRole("region", { name: "当前分配" })).toHaveTextContent(
      "尚未分配到任何应用",
    );
    expect(
      screen.getByRole("region", { name: "安装信息" }),
    ).not.toHaveTextContent("安装时间");
    appearsBefore(
      screen.getByRole("button", { name: "卸载" }),
      screen.getByRole("region", { name: "下载来源" }),
    );
  });

  it("keeps discovery installation locked until authority refresh completes", async () => {
    const user = userEvent.setup();
    const discoverable = marketSkill();
    const installed = {
      ...installedSkill("review-skill", "Review Skill"),
      repoOwner: discoverable.repoOwner,
      repoName: discoverable.repoName,
    };
    const refreshed = deferred<InstalledSkill[]>();
    let installedReads = 0;
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = vi.fn(() => {
      installedReads += 1;
      return installedReads === 1 ? Promise.resolve([]) : refreshed.promise;
    });
    ports.skills.searchSkillHub = async () => ({
      query: "",
      skills: [discoverable],
      totalCount: 1,
    });
    ports.skills.installSkillHub = vi.fn(async () => [installed]);

    renderFeature(<SkillsPage />, ports);
    await screen.findByText("还没有安装 Skill");
    await user.click(screen.getByRole("tab", { name: "发现" }));
    const install = await screen.findByRole("button", {
      name: "安装到 Claude Code",
    });

    await user.click(install);
    await waitFor(() => expect(install).toBeDisabled());
    fireEvent.click(install);
    expect(ports.skills.installSkillHub).toHaveBeenCalledTimes(1);

    refreshed.resolve([installed]);
    expect(
      await screen.findByRole("button", { name: "已安装" }),
    ).toBeDisabled();
    expect(ports.skills.getInstalled).toHaveBeenCalledTimes(2);
  });

  it("refreshes authority after a partially failed discovery installation", async () => {
    const user = userEvent.setup();
    const discoverable = marketSkill();
    const installed = {
      ...installedSkill("review-skill", "Review Skill"),
      repoOwner: discoverable.repoOwner,
      repoName: discoverable.repoName,
    };
    let backendInstalled = false;
    const getInstalled = vi.fn(async () =>
      backendInstalled ? [installed] : [],
    );
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = getInstalled;
    ports.skills.searchSkillHub = async () => ({
      query: "",
      skills: [discoverable],
      totalCount: 1,
    });
    ports.skills.installSkillHub = vi.fn(async () => {
      backendInstalled = true;
      throw new Error("partial install");
    });

    renderFeature(<SkillsPage />, ports);
    await screen.findByText("还没有安装 Skill");
    await user.click(screen.getByRole("tab", { name: "发现" }));
    await user.click(
      await screen.findByRole("button", { name: "安装到 Claude Code" }),
    );

    expect(await screen.findByText("请稍后重试。")).toBeVisible();
    expect(screen.queryByText("partial install")).not.toBeInTheDocument();
    expect(
      await screen.findByRole("button", { name: "已安装" }),
    ).toBeDisabled();
    expect(getInstalled).toHaveBeenCalledTimes(2);
  });

  it("renders Skill discovery as a marketplace card grid without select switchers", async () => {
    const user = userEvent.setup();
    const discoverable = marketSkill();
    const ports = createBrowserFeaturePorts();
    const openExternal = vi.fn(async () => undefined);
    ports.settings.openExternal = openExternal;
    ports.skills.searchSkillHub = async () => ({
      query: "",
      skills: [discoverable],
      totalCount: 1,
    });

    renderFeature(<SkillsPage />, ports);
    await screen.findByText("还没有安装 Skill");
    await user.click(screen.getByRole("tab", { name: "发现" }));

    expect(
      screen.queryByRole("combobox", { name: "安装目标" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("searchbox", { name: "搜索 Skill 市场" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("tab", { name: "Skill 市场" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "仓库" })).not.toBeInTheDocument();
    expect(
      screen.queryByText(/Skill 市场 · \d+ \/ \d+/),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("tablist", { name: "安装目标" })).toBeVisible();
    expect(
      within(screen.getByRole("tablist", { name: "安装目标" }))
        .getAllByRole("tab")
        .map((tab) => tab.textContent?.replace(/\s+/g, " ").trim()),
    ).toEqual(SKILL_TARGETS.map((app) => app.label));
    expect(
      screen.getByRole("tab", { name: "Claude Code", selected: true }),
    ).toBeVisible();
    expect(
      screen.queryByRole("tablist", { name: "仓库筛选" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("tablist", { name: "分类筛选" })).toBeVisible();
    expect(
      within(screen.getByRole("tablist", { name: "分类筛选" })).getByRole(
        "tab",
        { name: "全部", selected: true },
      ),
    ).toBeVisible();
    expect(screen.getByRole("tab", { name: "办公效率" })).toBeVisible();
    expect(screen.getByRole("tab", { name: "开发编程" })).toBeVisible();
    expect(screen.getByRole("tab", { name: "IT 运维与安全" })).toBeVisible();
    const card = await screen.findByRole("heading", { name: "Review Skill" });
    const article = card.closest("article");
    expect(article).not.toBeNull();
    expect(
      within(article as HTMLElement).getByText("Review changes"),
    ).toBeVisible();
    expect(
      within(article as HTMLElement).queryByText("acme/skills"),
    ).not.toBeInTheDocument();
    await user.click(
      within(article as HTMLElement).getByRole("button", { name: "主页" }),
    );
    expect(openExternal).toHaveBeenCalledWith(discoverable.homepageUrl);
    expect(
      await screen.findByRole("button", { name: "安装到 Claude Code" }),
    ).toBeVisible();
  });

  it("opens the full discovery description in a details dialog", async () => {
    const user = userEvent.setup();
    const longDescription =
      "Review changes across a long skill summary that must not stretch the discovery card. ".repeat(
        8,
      );
    const ports = createBrowserFeaturePorts();
    ports.skills.searchSkillHub = async () => ({
      query: "",
      skills: [{ ...marketSkill(), description: longDescription }],
      totalCount: 1,
    });

    renderFeature(<SkillsPage />, ports);
    await screen.findByText("还没有安装 Skill");
    await user.click(screen.getByRole("tab", { name: "发现" }));
    const card = (
      await screen.findByRole("heading", { name: "Review Skill" })
    ).closest("article");
    expect(card).not.toBeNull();
    expect(
      within(card as HTMLElement).getByRole("button", { name: "详情" }),
    ).toBeVisible();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await user.click(
      within(card as HTMLElement).getByRole("button", { name: "详情" }),
    );
    const dialog = await screen.findByRole("dialog", { name: "Review Skill" });
    expect(dialog).toHaveTextContent(longDescription.trim());
    expect(dialog).toHaveTextContent("Skill 市场");
    expect(dialog).toHaveTextContent("review-skill");
    await user.click(within(dialog).getByRole("button", { name: "关闭" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("renders Skill 市场 results with Chinese copy, author, and homepage", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    const openExternal = vi.fn(async () => undefined);
    const installSkillHub = vi.fn(async () => []);
    ports.settings.openExternal = openExternal;
    ports.skills.installSkillHub = installSkillHub;
    ports.skills.searchSkillHub = async () => ({
      query: "",
      totalCount: 48,
      skills: [
        {
          key: "skillhub:tencent-docs",
          slug: "tencent-docs",
          name: "腾讯文档",
          description: "腾讯文档在线云文档平台",
          directory: "tencent-docs",
          repoOwner: "skillhub.cn",
          repoName: "tencent-docs",
          repoBranch: "1.0.41",
          version: "1.0.41",
          ownerName: "tencent-adm",
          installs: 8107,
          homepageUrl: "https://skillhub.cn/skills/tencent-docs",
          readmeUrl: "https://skillhub.cn/skills/tencent-docs",
          category: "office-efficiency",
        },
      ],
    });

    renderFeature(<SkillsPage />, ports);
    await screen.findByText("还没有安装 Skill");
    await user.click(screen.getByRole("tab", { name: "发现" }));

    expect(
      screen.getByRole("searchbox", { name: "搜索 Skill 市场" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("tab", { name: "Skill 市场" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "仓库" })).not.toBeInTheDocument();
    expect(
      screen.queryByText(/Skill 市场 · \d+ \/ \d+/),
    ).not.toBeInTheDocument();
    const card = await screen.findByRole("heading", { name: "腾讯文档" });
    const article = card.closest("article");
    expect(article).not.toBeNull();
    expect(
      within(article as HTMLElement).getByText("腾讯文档在线云文档平台"),
    ).toBeVisible();
    expect(
      within(article as HTMLElement).queryByText(/来自 /),
    ).not.toBeInTheDocument();
    expect(
      within(article as HTMLElement).getByText(
        "办公效率 · v1.0.41 · tencent-adm",
      ),
    ).toBeVisible();
    expect(screen.getByText(/将安装到 Claude Code/)).toBeVisible();
    expect(
      screen.queryByRole("heading", { name: /skillhub\.cn/ }),
    ).not.toBeInTheDocument();
    await user.click(
      within(article as HTMLElement).getByRole("button", { name: "主页" }),
    );
    expect(openExternal).toHaveBeenCalledWith(
      "https://skillhub.cn/skills/tencent-docs",
    );
    await user.click(
      within(article as HTMLElement).getByRole("button", { name: "详情" }),
    );
    const dialog = await screen.findByRole("dialog", { name: "腾讯文档" });
    expect(dialog).toHaveTextContent("Skill 市场");
    expect(dialog).toHaveTextContent("办公效率");
    expect(dialog).toHaveTextContent("tencent-docs");
    expect(dialog).toHaveTextContent("tencent-adm");
    await user.click(within(dialog).getByRole("button", { name: "关闭" }));
    await user.click(
      within(article as HTMLElement).getByRole("button", {
        name: "安装到 Claude Code",
      }),
    );
    await waitFor(() =>
      expect(installSkillHub).toHaveBeenCalledWith("tencent-docs", "claude"),
    );
  });

  it("blocks discovery installation when installed authority is unavailable", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = vi.fn(async () => {
      throw new Error("installed authority unavailable");
    });
    ports.skills.searchSkillHub = async () => ({
      query: "",
      skills: [marketSkill()],
      totalCount: 1,
    });

    renderFeature(<SkillsPage />, ports);
    await user.click(screen.getByRole("tab", { name: "发现" }));

    expect(
      await screen.findByText("无法加载已安装 Skills", undefined, {
        timeout: 5_000,
      }),
    ).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "安装到 Claude Code" }),
    ).not.toBeInTheDocument();
  });

  it("paginates Skill 市场 discovery with page size 21", async () => {
    const user = userEvent.setup();
    const all = Array.from({ length: 60 }, (_, index) =>
      marketSkill({
        key: `skillhub:skill-${index}`,
        slug: `skill-${index}`,
        name: `Paged Skill ${index + 1}`,
        directory: `skill-${index}`,
        repoName: `skill-${index}`,
        homepageUrl: `https://skillhub.cn/skills/skill-${index}`,
        readmeUrl: `https://skillhub.cn/skills/skill-${index}`,
      }),
    );
    const searchSkillHub = vi.fn(
      async (_query: string, limit: number, offset: number) => ({
        query: _query,
        skills: all.slice(offset, offset + limit),
        totalCount: all.length,
      }),
    );
    const ports = createBrowserFeaturePorts();
    ports.skills.searchSkillHub = searchSkillHub;

    renderFeature(<SkillsPage />, ports);
    await screen.findByText("还没有安装 Skill");
    await user.click(screen.getByRole("tab", { name: "发现" }));
    expect(
      await screen.findByRole("heading", { name: "Paged Skill 1" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("heading", { name: "Paged Skill 22" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText(/Skill 市场 · \d+ \/ \d+/),
    ).not.toBeInTheDocument();

    const pagination = screen.getByRole("navigation", {
      name: "Skill 市场分页",
    });
    await user.click(within(pagination).getByRole("button", { name: "3" }));

    expect(
      await screen.findByRole("heading", { name: "Paged Skill 43" }),
    ).toBeVisible();
    expect(
      await screen.findByRole("heading", { name: "Paged Skill 51" }),
    ).toBeVisible();
    await waitFor(() =>
      expect(searchSkillHub).toHaveBeenCalledWith("", 21, 42, ""),
    );
  });

  it("filters Skill 市场 discovery by official category", async () => {
    const user = userEvent.setup();
    const searchSkillHub = vi.fn(
      async (
        _query: string,
        _limit: number,
        _offset: number,
        category = "",
      ) => ({
        query: _query,
        skills: [
          marketSkill({
            name: category === "office-efficiency" ? "办公 Skill" : "全部 Skill",
            category: category || undefined,
          }),
        ],
        totalCount: 1,
      }),
    );
    const ports = createBrowserFeaturePorts();
    ports.skills.searchSkillHub = searchSkillHub;

    renderFeature(<SkillsPage />, ports);
    await screen.findByText("还没有安装 Skill");
    await user.click(screen.getByRole("tab", { name: "发现" }));
    expect(
      await screen.findByRole("heading", { name: "全部 Skill" }),
    ).toBeVisible();
    await waitFor(() =>
      expect(searchSkillHub).toHaveBeenCalledWith("", 21, 0, ""),
    );

    await user.click(screen.getByRole("tab", { name: "办公效率" }));
    expect(
      await screen.findByRole("heading", { name: "办公 Skill" }),
    ).toBeVisible();
    await waitFor(() =>
      expect(searchSkillHub).toHaveBeenCalledWith(
        "",
        21,
        0,
        "office-efficiency",
      ),
    );
    expect(
      screen.queryByRole("tab", { name: "Skill 市场" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "仓库" })).not.toBeInTheDocument();
  });

  it("resets Skill 市场 discovery to page 1 when search changes", async () => {
    const user = userEvent.setup();
    const all = Array.from({ length: 22 }, (_, index) =>
      marketSkill({
        key: `skillhub:skill-${index}`,
        slug: `skill-${index}`,
        name: `Paged Skill ${index + 1}`,
        directory: `skill-${index}`,
        repoName: `skill-${index}`,
      }),
    );
    const searchSkillHub = vi.fn(
      async (query: string, limit: number, offset: number) => ({
        query,
        skills: all.slice(offset, offset + limit),
        totalCount: all.length,
      }),
    );
    const ports = createBrowserFeaturePorts();
    ports.skills.searchSkillHub = searchSkillHub;

    renderFeature(<SkillsPage />, ports);
    await screen.findByText("还没有安装 Skill");
    await user.click(screen.getByRole("tab", { name: "发现" }));
    await screen.findByRole("heading", { name: "Paged Skill 1" });
    await user.click(
      within(
        screen.getByRole("navigation", { name: "Skill 市场分页" }),
      ).getByRole("button", { name: "2" }),
    );
    await screen.findByRole("heading", { name: "Paged Skill 22" });

    await user.type(
      screen.getByRole("searchbox", { name: "搜索 Skill 市场" }),
      "paged",
    );

    await waitFor(
      () => expect(searchSkillHub).toHaveBeenCalledWith("paged", 21, 0, ""),
      { timeout: 2_000 },
    );
  });

  it("treats a cancelled ZIP picker as a no-op", async () => {
    const user = userEvent.setup();
    const getInstalled = vi.fn(async () => []);
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = getInstalled;
    ports.skills.pickZip = vi.fn(async () => null);
    ports.skills.installFromZip = vi.fn(async () => []);

    renderFeature(<SkillsPage />, ports);
    await screen.findByText("还没有安装 Skill");
    await user.click(screen.getByRole("button", { name: "更多" }));
    await user.click(screen.getByRole("button", { name: "从 ZIP 安装" }));

    await waitFor(() => expect(ports.skills.pickZip).toHaveBeenCalledTimes(1));
    expect(ports.skills.installFromZip).not.toHaveBeenCalled();
    expect(getInstalled).toHaveBeenCalledTimes(1);
    expect(screen.queryByText("ZIP 安装完成")).not.toBeInTheDocument();
  });
});
