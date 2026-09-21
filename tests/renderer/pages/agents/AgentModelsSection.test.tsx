import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { AgentModelsSection } from "@/pages/agents/AgentModelsSection";
import { PRODUCT_DIRECTORY } from "@/shared/features/directory";
import { FeatureProvider } from "@/shared/features/provider";
import { featureKeys } from "@/shared/features/queries";
import type {
  AgentCatalogEntry,
  ProviderAppId,
  ProviderLiveSummary,
  ProviderSummaryQueryData,
} from "@/shared/features/types";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

const configuredLive: ProviderLiveSummary = {
  target: "codex",
  state: "configured",
  exists: true,
  connection: {
    modelId: "actual-file-model",
    baseUrl: "https://live.example.test/v1",
    protocol: "responses",
  },
};

function summary(
  live: ProviderLiveSummary | undefined,
): ProviderSummaryQueryData {
  return {
    currentId: "saved",
    providers: {
      saved: { id: "saved", name: "数据库方案", modelId: "saved-model" },
    },
    writeTargets: [
      {
        path: "~/.codex/config.toml",
        backupPath: "~/.codex/config.toml.backup",
        exists: true,
      },
    ],
    ...(live ? { live } : {}),
  };
}

function setup(
  data: ProviderSummaryQueryData,
  target: ProviderAppId = "codex",
  failed = false,
  cached = false,
) {
  const ports = createBrowserFeaturePorts();
  ports.providers.getSummary = vi.fn(async () => {
    if (failed) throw new Error("private-native-error-secret-do-not-render");
    return data;
  });
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  if (cached)
    queryClient.setQueryData(featureKeys.providerSummary(target), data);
  const agentId = target === "claude" ? "claude-code" : target;
  const entry = PRODUCT_DIRECTORY.find(
    (candidate) => candidate.agentId === agentId,
  )!;
  const catalogEntry: AgentCatalogEntry = {
    id: agentId,
    variantId: agentId,
    displayName: entry.displayName,
    description: "模型配置",
    officialLinks: [],
    capabilities: [
      {
        id: "models.write",
        mode: "direct",
        reasonCode: "dedicated_native_contract",
        evidenceIds: ["p0_scope"],
      },
    ],
  };
  const forbiddenCalls = [
    vi.spyOn(ports.providers, "fetchModels"),
    vi.spyOn(ports.providers, "checkModel"),
    vi.spyOn(ports.providers, "checkReachability"),
    vi.spyOn(ports.providers, "applyQuickSetupWithResult"),
    vi.spyOn(ports.changePlans, "applyChangePlan"),
  ];
  const onOpenManagement = vi.fn();
  render(
    <MemoryRouter>
      <FeatureProvider ports={ports}>
        <QueryClientProvider client={queryClient}>
          <AgentModelsSection
            entry={entry}
            catalogEntry={catalogEntry}
            onOpenManagement={onOpenManagement}
          />
        </QueryClientProvider>
      </FeatureProvider>
    </MemoryRouter>,
  );
  return { forbiddenCalls, ports, onOpenManagement };
}

describe("AgentModelsSection actual files and saved plans", () => {
  it.each(["codex", "claude", "grokbuild"] as const)(
    "shows %s actual connection independently of a different saved selection",
    async (target) => {
      const { forbiddenCalls, ports } = setup(
        summary({ ...configuredLive, target }),
        target,
      );
      const actual = await screen.findByRole("region", {
        name: "实际配置文件",
      });
      expect(actual).toHaveTextContent("实际配置文件已配置");
      expect(actual).toHaveTextContent("actual-file-model");
      expect(actual).toHaveTextContent("https://live.example.test/v1");
      expect(actual).not.toHaveTextContent("saved-model");
      expect(
        screen.getByRole("heading", { name: "FyAgent 已保存方案" }),
      ).toBeInTheDocument();
      expect(screen.getByText("已选方案 · 数据库方案")).toBeInTheDocument();
      expect(screen.getByText("saved-model")).toBeInTheDocument();
      expect(
        screen.queryByRole("heading", { name: "当前模型" }),
      ).not.toBeInTheDocument();
      expect(ports.providers.getSummary).toHaveBeenCalledWith(target);
      for (const call of forbiddenCalls) expect(call).not.toHaveBeenCalled();
    },
  );

  it.each(["not_configured", "missing"] as const)(
    "keeps %s files visibly unconfigured when a saved provider exists",
    async (state) => {
      const live: ProviderLiveSummary =
        state === "missing"
          ? { target: "codex", state, exists: false, connection: null }
          : { target: "codex", state, exists: true, connection: null };
      setup(summary(live));
      expect(
        await screen.findByRole("region", { name: "实际配置文件" }),
      ).toHaveTextContent("实际配置尚未配置");
      expect(screen.getByText("saved-model")).toBeInTheDocument();
      expect(screen.queryByText("实际配置文件已配置")).not.toBeInTheDocument();
    },
  );

  it("shows a configured live file even when no plan has been saved in FyAgent", async () => {
    setup({ ...summary(configuredLive), currentId: "", providers: {} });
    expect(await screen.findByText("实际配置文件已配置")).toBeInTheDocument();
    expect(screen.getByText("FyAgent 还没有保存方案")).toBeInTheDocument();
    expect(screen.getByText("~/.codex/config.toml")).toBeInTheDocument();
  });

  it.each([
    ["older host", undefined],
    [
      "unreadable",
      { target: "codex", state: "unreadable", exists: null, connection: null },
    ],
    ["mismatched target", { ...configuredLive, target: "claude" }],
  ] satisfies Array<[string, ProviderLiveSummary | undefined]>)(
    "retains the saved list while %s actual status is unknown",
    async (_label, live) => {
      const user = userEvent.setup();
      const { forbiddenCalls, onOpenManagement } = setup(summary(live));
      expect(await screen.findByText(/实际配置状态未知/)).toBeInTheDocument();
      expect(screen.getByText("saved-model")).toBeInTheDocument();
      expect(screen.queryByText("实际配置文件已配置")).not.toBeInTheDocument();
      await user.type(screen.getByRole("searchbox"), "saved-model");
      expect(screen.getByText("saved-model")).toBeInTheDocument();
      await user.click(screen.getByRole("button", { name: "管理模型" }));
      expect(onOpenManagement).toHaveBeenCalledOnce();
      for (const call of forbiddenCalls) expect(call).not.toHaveBeenCalled();
    },
  );

  it("does not reuse cached live state as current after a failed reread but preserves cached saved plans", async () => {
    setup(summary(configuredLive), "codex", true, true);
    expect(await screen.findByText(/实际配置状态未知/)).toBeInTheDocument();
    expect(screen.getByText("saved-model")).toBeInTheDocument();
    const actual = screen.getByRole("region", { name: "实际配置文件" });
    expect(
      within(actual).queryByText(/actual-file-model/),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText(/private-native-error-secret/),
    ).not.toBeInTheDocument();
  });

  it("shows a safe read error when no saved data is available", async () => {
    setup(summary(undefined), "codex", true);
    expect(await screen.findByText(/实际配置状态未知/)).toBeInTheDocument();
    expect(
      screen.getByText(/FyAgent 已保存方案暂时无法读取/),
    ).toBeInTheDocument();
    expect(screen.queryByText("saved-model")).not.toBeInTheDocument();
    expect(
      screen.queryByText(/private-native-error-secret/),
    ).not.toBeInTheDocument();
  });
});
