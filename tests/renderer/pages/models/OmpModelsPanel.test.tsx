import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { OmpModelsPanel } from "@/pages/models/OmpModelsPanel";
import type {
  OmpConfigRead,
  OmpSavePreview,
  OmpSaveRequest,
} from "@/shared/features/omp";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

function emptyOmpRead(overrides: Partial<OmpConfigRead> = {}): OmpConfigRead {
  return {
    profile: "default",
    modelsPath: "~/.omp/agent/models.yml",
    configPath: "~/.omp/agent/config.yml",
    modelsRevision: "rev-models-1",
    configRevision: "rev-config-1",
    providers: [],
    defaultModel: null,
    modelsFileStatus: "notFound",
    configFileStatus: "notFound",
    ...overrides,
  };
}

function renderPanel(read: OmpConfigRead = emptyOmpRead()) {
  const ports = createBrowserFeaturePorts();
  ports.ompModels.readConfig = vi.fn(async () => read);
  ports.ompModels.discoverCli = vi.fn(async () => ({
    detected: true,
    version: "18.12.5",
    executablePath: "~/.bun/bin/omp",
  }));
  ports.ompModels.previewSave = vi.fn(
    async (req: OmpSaveRequest): Promise<OmpSavePreview> => ({
      planId: "omp-plan:1",
      planDigest: "digest-1",
      profile: "default",
      modelsPath: "~/.omp/agent/models.yml",
      configPath: "~/.omp/agent/config.yml",
      provider: {
        id: req.provider.id,
        baseUrl: req.provider.baseUrl,
        api: req.provider.api,
        auth: req.provider.auth ?? "apiKey",
        hasApiKey: Boolean(req.provider.apiKey),
        hasCommandKey: Boolean(req.provider.apiKey?.startsWith("!")),
        models: req.provider.models.map((m) => ({
          id: m.id,
          name: m.name ?? null,
        })),
      },
      defaultModel: req.defaultModel ?? null,
      changedFiles: req.defaultModel ? ["models", "config"] : ["models"],
      warnings: [],
    }),
  );
  ports.ompModels.applySave = vi.fn(async () => ({
    modelsPath: "~/.omp/agent/models.yml",
    configPath: "~/.omp/agent/config.yml",
    modelsRevision: "rev-models-2",
    configRevision: "rev-config-2",
    provider: {
      id: "fyagent-deepseek",
      baseUrl: "https://api.deepseek.com/v1",
      api: "openai-completions",
      auth: "apiKey",
      hasApiKey: true,
      hasCommandKey: false,
      models: [{ id: "deepseek-chat", name: null }],
    },
    defaultModel: "fyagent-deepseek/deepseek-chat",
    modelsWritten: true,
    configWritten: true,
  }));

  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <MemoryRouter>
      <QueryClientProvider client={queryClient}>
        <FeatureProvider ports={ports}>
          <OmpModelsPanel active />
        </FeatureProvider>
      </QueryClientProvider>
    </MemoryRouter>,
  );
  return { ports };
}

describe("OmpModelsPanel", () => {
  it("blocks saving when legacy models.json is detected", async () => {
    renderPanel(emptyOmpRead({ modelsFileStatus: "legacyJsonOnly" }));
    expect(await screen.findByText("检测到旧版 models.json")).toBeVisible();
    expect(screen.getByRole("button", { name: "预览并保存" })).toBeDisabled();
  });

  it("previews and applies a new omp provider with default model role", async () => {
    const user = userEvent.setup();
    const { ports } = renderPanel();

    const idInput = await screen.findByPlaceholderText("例如 fyagent-deepseek");
    await user.clear(idInput);
    await user.type(idInput, "fyagent-deepseek");
    await user.type(
      screen.getByPlaceholderText("https://api.deepseek.com/v1"),
      "https://api.deepseek.com/v1",
    );
    await user.type(
      screen.getByPlaceholderText("例如 DEEPSEEK_API_KEY"),
      "DEEPSEEK_API_KEY",
    );
    await user.type(
      screen.getByPlaceholderText(/输入模型 ID/),
      "deepseek-chat{enter}",
    );
    await user.click(
      screen.getByRole("checkbox", {
        name: "设为 omp 默认模型",
      }),
    );

    await user.click(screen.getByRole("button", { name: "预览并保存" }));
    expect(await screen.findByText("确认写入 omp 配置")).toBeVisible();
    expect(screen.getByText("fyagent-deepseek/deepseek-chat")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "确认写入" }));
    await waitFor(() =>
      expect(ports.ompModels.applySave).toHaveBeenCalledWith({
        planId: "omp-plan:1",
        planDigest: "digest-1",
      }),
    );
    expect(await screen.findByText("已写入 omp 配置文件")).toBeVisible();
  });
});
