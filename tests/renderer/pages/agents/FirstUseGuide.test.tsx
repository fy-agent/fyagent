import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { FirstUseGuide } from "@/pages/agents/FirstUseGuide";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import type { FeaturePorts } from "@/shared/features/ports";
import type {
  AgentCatalogEntry,
  ProviderAppId,
  ProviderLiveSummary,
} from "@/shared/features/types";

const configuredLive: ProviderLiveSummary = {
  target: "codex",
  state: "configured",
  exists: true,
  connection: {
    modelId: "file-model",
    baseUrl: "https://live.example.test/v1",
    protocol: "responses",
  },
};

describe("FirstUseGuide", () => {
  let queryClient: QueryClient;
  let ports: FeaturePorts;

  const mockCatalogEntries: AgentCatalogEntry[] = [
    {
      id: "codex",
      variantId: "codex",
      displayName: "Codex",
      description: "OpenAI Codex 智能编程助手",
      officialLinks: [],
      capabilities: [],
    },
    {
      id: "grokbuild",
      variantId: "grokbuild",
      displayName: "Grok Build",
      description: "xAI 命令行开发助手",
      officialLinks: [],
      capabilities: [],
    },
    {
      id: "claude-code",
      variantId: "claude-code",
      displayName: "Claude Code",
      description: "编程助手",
      officialLinks: [],
      capabilities: [],
    },
    {
      id: "workbuddy",
      variantId: "workbuddy",
      displayName: "WorkBuddy",
      description: "腾讯通用办公助手",
      officialLinks: [],
      capabilities: [],
    },
  ];

  function LocationProbe() {
    const location = useLocation();
    return <div data-testid="location">{location.search}</div>;
  }

  function renderGuide(
    props: Partial<React.ComponentProps<typeof FirstUseGuide>> = {},
  ) {
    return render(
      <MemoryRouter initialEntries={["/agents"]}>
        <QueryClientProvider client={queryClient}>
          <FeatureProvider ports={ports}>
            <FirstUseGuide entries={mockCatalogEntries} {...props} />
            <LocationProbe />
          </FeatureProvider>
        </QueryClientProvider>
      </MemoryRouter>,
    );
  }

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    ports = createBrowserFeaturePorts();
    ports.settings.getFirstUseGuideState = vi.fn(
      async () => "pending" as const,
    );
    ports.settings.dismissFirstUseGuide = vi.fn(
      async () => "dismissed" as const,
    );
    ports.providers.getSummary = vi.fn(async (app: ProviderAppId) => ({
      currentId: "",
      providers: {},
      writeTargets: [],
      live: {
        target: app,
        state: "missing" as const,
        exists: false as const,
        connection: null,
      },
    }));
    ports.workbuddy.getModelIds = vi.fn(async () => ({
      ids: [],
      revision: "",
    }));
    ports.opencodeModels.getSnapshot = vi.fn(async () => ({
      providers: [],
      revision: "",
      path: "",
      backupPath: "",
      exists: false,
    }));
  });

  it("renders two-step purpose choices initially", () => {
    renderGuide();

    expect(
      screen.getByRole("heading", { name: "你主要想用 AI 做什么？" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "编程开发" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "日常办公" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "两者都用" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "跳过引导" }),
    ).toBeInTheDocument();
  });

  it("navigates to recommendations with real starting instruction presets upon selecting purpose", async () => {
    const user = userEvent.setup();
    renderGuide();

    await user.click(screen.getByRole("button", { name: "编程开发" }));

    expect(
      screen.getByRole("heading", { name: "推荐你从这些软件开始" }),
    ).toBeInTheDocument();

    expect(screen.getByText("Codex")).toBeInTheDocument();
    expect(screen.getByText("Grok Build")).toBeInTheDocument();

    // Reuses real FDE presets lazily
    expect(screen.getAllByText(/建议起始指令：/).length).toBeGreaterThan(0);

    const configButtons = screen.getAllByRole("button", { name: "开始配置" });
    expect(configButtons.length).toBeGreaterThan(0);
  });

  it("directly triggers onConfigure with intent=replace when agent has no existing configuration", async () => {
    const user = userEvent.setup();
    const onConfigure = vi.fn();
    renderGuide({ onConfigure });

    await user.click(screen.getByRole("button", { name: "编程开发" }));

    const codexArticle = screen.getByText("Codex").closest("article")!;
    const startBtn = codexArticle.querySelector("button")!;
    await user.click(startBtn);

    await waitFor(() => {
      expect(onConfigure).toHaveBeenCalledWith({
        agentId: "codex",
        intent: "replace",
        existingSummary: undefined,
      });
    });
  });

  it("offers keep or replace dialog when real non-secret existing configuration is detected", async () => {
    const user = userEvent.setup();
    const onConfigure = vi.fn();
    const forbiddenCalls = [
      vi.spyOn(ports.providers, "fetchModels"),
      vi.spyOn(ports.providers, "checkModel"),
      vi.spyOn(ports.providers, "checkReachability"),
      vi.spyOn(ports.providers, "applyQuickSetupWithResult"),
      vi.spyOn(ports.changePlans, "createCodexProviderUpsertPlan"),
      vi.spyOn(ports.changePlans, "applyChangePlan"),
    ];

    // Mock real existing configuration summary
    ports.providers.getSummary = vi.fn(async () => ({
      currentId: "custom-gpt",
      providers: {
        "custom-gpt": {
          id: "custom-gpt",
          name: "公司内部专属模型",
          modelId: "gpt-4o",
        },
      },
      writeTargets: [],
      live: configuredLive,
    }));

    renderGuide({ onConfigure });

    await user.click(screen.getByRole("button", { name: "编程开发" }));

    const codexArticle = screen.getByText("Codex").closest("article")!;
    const startBtn = codexArticle.querySelector("button")!;
    await user.click(startBtn);

    // Dialog should appear with non-secret summary
    await waitFor(() => {
      expect(screen.getByText("确认 Codex 配置方式")).toBeInTheDocument();
    });
    expect(
      screen.getByText(
        /检测到 Codex 的实际配置文件中已有设置（模型：file-model；服务地址：https:\/\/live.example.test\/v1）/,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/保留现有配置不会发起模型网络请求/),
    ).toBeInTheDocument();

    // Click "保留现有配置"
    const keepButton = screen.getByRole("button", { name: "保留现有配置" });
    await user.click(keepButton);

    await waitFor(() => {
      expect(onConfigure).toHaveBeenCalledWith({
        agentId: "codex",
        intent: "keep",
        existingSummary:
          "模型：file-model；服务地址：https://live.example.test/v1",
      });
    });
    for (const call of forbiddenCalls) expect(call).not.toHaveBeenCalled();
    expect(screen.queryByText("公司内部专属模型")).not.toBeInTheDocument();
  });

  it("triggers replace intent when user chooses to replace existing configuration", async () => {
    const user = userEvent.setup();
    const onConfigure = vi.fn();

    ports.providers.getSummary = vi.fn(async () => ({
      currentId: "custom-gpt",
      providers: {
        "custom-gpt": {
          id: "custom-gpt",
          name: "公司内部专属模型",
          modelId: "gpt-4o",
        },
      },
      writeTargets: [],
      live: configuredLive,
    }));

    renderGuide({ onConfigure });

    await user.click(screen.getByRole("button", { name: "编程开发" }));

    const codexArticle = screen.getByText("Codex").closest("article")!;
    const startBtn = codexArticle.querySelector("button")!;
    await user.click(startBtn);

    await waitFor(() => {
      expect(screen.getByText("确认 Codex 配置方式")).toBeInTheDocument();
    });

    // Click "替换现有配置"
    const replaceButton = screen.getByRole("button", { name: "替换现有配置" });
    await user.click(replaceButton);

    await waitFor(() => {
      expect(onConfigure).toHaveBeenCalledWith({
        agentId: "codex",
        intent: "replace",
        existingSummary:
          "模型：file-model；服务地址：https://live.example.test/v1",
      });
    });
  });

  it("preserves unknown state when configuration check fails, offering retry and inspect without auto-replacing", async () => {
    const user = userEvent.setup();
    const onConfigure = vi.fn();

    ports.providers.getSummary = vi.fn(async () => {
      throw new Error("Disk I/O error reading provider file");
    });

    renderGuide({ onConfigure });

    await user.click(screen.getByRole("button", { name: "编程开发" }));
    const codexArticle = screen.getByText("Codex").closest("article")!;
    const startBtn = codexArticle.querySelector("button")!;
    await user.click(startBtn);

    await waitFor(() => {
      expect(screen.getByText("无法确认 Codex 配置状态")).toBeInTheDocument();
    });
    expect(
      screen.getByText(
        /无法确认 Codex 的现有配置状态。当前不确定是否存在可用配置。/,
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/读取当前配置失败/)).toBeInTheDocument();
    expect(onConfigure).not.toHaveBeenCalled();

    // Click "进入软件详情查看"
    const inspectBtn = screen.getByRole("button", { name: "进入软件详情查看" });
    await user.click(inspectBtn);

    await waitFor(() => {
      expect(onConfigure).toHaveBeenCalledWith({
        agentId: "codex",
        intent: "inspect",
      });
    });
  });

  it("allows retrying configuration check from unknown state", async () => {
    const user = userEvent.setup();
    const onConfigure = vi.fn();

    let attempt = 0;
    ports.providers.getSummary = vi.fn(async () => {
      attempt++;
      if (attempt === 1) {
        throw new Error("Temporary failure");
      }
      return {
        currentId: "custom-gpt",
        providers: {
          "custom-gpt": {
            id: "custom-gpt",
            name: "公司内部专属模型",
            modelId: "gpt-4o",
          },
        },
        writeTargets: [],
        live: configuredLive,
      };
    });

    renderGuide({ onConfigure });

    await user.click(screen.getByRole("button", { name: "编程开发" }));
    const codexArticle = screen.getByText("Codex").closest("article")!;
    const startBtn = codexArticle.querySelector("button")!;
    await user.click(startBtn);

    await waitFor(() => {
      expect(screen.getByText("无法确认 Codex 配置状态")).toBeInTheDocument();
    });

    // Click retry
    const retryBtn = screen.getByRole("button", { name: "重试" });
    await user.click(retryBtn);

    await waitFor(() => {
      expect(screen.getByText("确认 Codex 配置方式")).toBeInTheDocument();
    });
    expect(
      screen.getByText(
        /检测到 Codex 的实际配置文件中已有设置（模型：file-model；服务地址：https:\/\/live.example.test\/v1）/,
      ),
    ).toBeInTheDocument();
  });

  it("aborts entering agent if guide is unmounted while configuration check is in flight", async () => {
    const user = userEvent.setup();
    const onConfigure = vi.fn();

    type ProviderSummary = Awaited<
      ReturnType<typeof ports.providers.getSummary>
    >;
    let resolveSummary!: (value: ProviderSummary) => void;
    ports.providers.getSummary = vi.fn(
      async () =>
        new Promise<ProviderSummary>((resolve) => {
          resolveSummary = resolve;
        }),
    );

    const { unmount } = renderGuide({ onConfigure });

    await user.click(screen.getByRole("button", { name: "编程开发" }));
    const codexArticle = screen.getByText("Codex").closest("article")!;
    const startBtn = codexArticle.querySelector("button")!;
    await user.click(startBtn);

    // Unmount while check is in-flight
    unmount();

    // Now resolve the promise
    resolveSummary({
      currentId: "custom-gpt",
      providers: {
        "custom-gpt": {
          id: "custom-gpt",
          name: "公司内部专属模型",
          modelId: "gpt-4o",
        },
      },
      writeTargets: [],
      live: configuredLive,
    });

    expect(onConfigure).not.toHaveBeenCalled();
    expect(screen.queryByText("确认 Codex 配置方式")).not.toBeInTheDocument();
  });

  it.each([
    ["codex", "Codex"],
    ["claude", "Claude Code"],
    ["grokbuild", "Grok Build"],
  ] as const)(
    "detects %s live configuration even with an empty saved database",
    async (target, name) => {
      const user = userEvent.setup();
      ports.providers.getSummary = vi.fn(async () => ({
        currentId: "",
        providers: {},
        writeTargets: [],
        live: { ...configuredLive, target },
      }));
      renderGuide();
      await user.click(screen.getByRole("button", { name: "编程开发" }));
      await user.click(
        screen.getByText(name).closest("article")!.querySelector("button")!,
      );
      expect(
        await screen.findByRole("dialog", { name: `确认 ${name} 配置方式` }),
      ).toHaveTextContent("模型：file-model");
      expect(screen.getByRole("dialog")).toHaveTextContent(
        "https://live.example.test/v1",
      );
      expect(ports.providers.getSummary).toHaveBeenCalledWith(target);
    },
  );

  it.each(["missing", "not_configured"] as const)(
    "does not mistake a saved provider for live configuration when files are %s",
    async (state) => {
      const user = userEvent.setup();
      const onConfigure = vi.fn();
      const live: ProviderLiveSummary =
        state === "missing"
          ? { target: "codex", state, exists: false, connection: null }
          : { target: "codex", state, exists: true, connection: null };
      ports.providers.getSummary = vi.fn(async () => ({
        currentId: "db-only",
        providers: { "db-only": { id: "db-only", name: "仅数据库方案" } },
        writeTargets: [],
        live,
      }));
      renderGuide({ onConfigure });
      await user.click(screen.getByRole("button", { name: "编程开发" }));
      await user.click(
        screen.getByText("Codex").closest("article")!.querySelector("button")!,
      );
      await waitFor(() =>
        expect(onConfigure).toHaveBeenCalledWith({
          agentId: "codex",
          intent: "replace",
          existingSummary: undefined,
        }),
      );
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    },
  );

  it.each([
    ["older host", undefined],
    [
      "unreadable file",
      { target: "codex", state: "unreadable", exists: null, connection: null },
    ],
    ["wrong target", { ...configuredLive, target: "claude" }],
  ] satisfies Array<[string, ProviderLiveSummary | undefined]>)(
    "keeps %s unknown even when the database has a selected provider",
    async (_label, live) => {
      const user = userEvent.setup();
      const onConfigure = vi.fn();
      ports.providers.getSummary = vi.fn(async () => ({
        currentId: "db-only",
        providers: { "db-only": { id: "db-only", name: "仅数据库方案" } },
        writeTargets: [],
        ...(live ? { live } : {}),
      }));
      renderGuide({ onConfigure });
      await user.click(screen.getByRole("button", { name: "编程开发" }));
      await user.click(
        screen.getByText("Codex").closest("article")!.querySelector("button")!,
      );
      expect(
        await screen.findByRole("dialog", { name: "无法确认 Codex 配置状态" }),
      ).toBeInTheDocument();
      expect(
        screen.queryByRole("button", { name: "保留现有配置" }),
      ).not.toBeInTheDocument();
      expect(onConfigure).not.toHaveBeenCalled();
    },
  );

  it("supports skipping and dismissing the first-use guide", async () => {
    const user = userEvent.setup();
    renderGuide();

    const skipBtn = screen.getByRole("button", { name: "跳过引导" });
    await user.click(skipBtn);

    await waitFor(() => {
      expect(ports.settings.dismissFirstUseGuide).toHaveBeenCalledTimes(1);
    });
  });

  it("supports viewing all software and re-selecting purpose", async () => {
    const user = userEvent.setup();
    renderGuide();

    await user.click(screen.getByRole("button", { name: "编程开发" }));

    const reselectBtn = screen.getByRole("button", { name: "重新选择" });
    await user.click(reselectBtn);
    expect(
      screen.getByRole("heading", { name: "你主要想用 AI 做什么？" }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "编程开发" }));
    const viewAllBtn = screen.getByRole("button", { name: "查看全部软件" });
    await user.click(viewAllBtn);

    await waitFor(() => {
      expect(ports.settings.dismissFirstUseGuide).toHaveBeenCalledTimes(1);
    });
  });
});
