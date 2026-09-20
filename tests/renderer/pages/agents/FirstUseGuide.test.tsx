import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { FirstUseGuide } from "@/pages/agents/FirstUseGuide";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import type { FeaturePorts } from "@/shared/features/ports";
import type { AgentCatalogEntry } from "@/shared/features/types";

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
    ports.settings.getFirstUseGuideState = vi.fn(async () => "pending" as const);
    ports.settings.dismissFirstUseGuide = vi.fn(async () => "dismissed" as const);
    ports.providers.getSummary = vi.fn(async () => ({
      currentId: "",
      providers: {},
      writeTargets: [],
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
    expect(screen.getByRole("button", { name: "编程开发" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "日常办公" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "两者都用" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "跳过引导" })).toBeInTheDocument();
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
      screen.getByText(/检测到 Codex 已有已保存配置（当前已保存配置：公司内部专属模型）/),
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
        existingSummary: "当前已保存配置：公司内部专属模型",
      });
    });
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
        existingSummary: "当前已保存配置：公司内部专属模型",
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
      screen.getByText(/无法确认 Codex 的现有配置状态。当前不确定是否存在可用配置。/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Disk I\/O error reading provider file/),
    ).toBeInTheDocument();
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
      screen.getByText(/检测到 Codex 已有已保存配置（当前已保存配置：公司内部专属模型）/),
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
    });

    expect(onConfigure).not.toHaveBeenCalled();
    expect(screen.queryByText("确认 Codex 配置方式")).not.toBeInTheDocument();
  });

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
