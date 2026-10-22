import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { ModelsPage } from "@/pages/models/Page";
import {
  OmpPortError,
  type OmpConfigRead,
  type OmpProviderSummary,
  type OmpSavePreview,
  type OmpSaveRequest,
} from "@/shared/features/omp";
import type { FeaturePorts } from "@/shared/features/ports";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TooltipProvider } from "@/shared/ui/primitives";

const baseRead: OmpConfigRead = {
  profile: "default",
  modelsPath: "~/.omp/agent/models.yml",
  configPath: "~/.omp/agent/config.yml",
  modelsRevision: "models-rev",
  configRevision: "config-rev",
  providers: [
    {
      id: "team-gateway",
      baseUrl: "https://gateway.example/v1",
      api: "openai-completions",
      auth: null,
      hasApiKey: true,
      hasCommandKey: false,
      models: [{ id: "vendor/model-a", name: "Model A" }],
    },
  ],
  defaultModel: null,
  modelsFileStatus: "ok",
  configFileStatus: "ok",
};

/** Mirrors the contract: models merge by ID, null key/auth keep the file value. */
function mergeProvider(
  existing: OmpProviderSummary | undefined,
  request: OmpSaveRequest,
): OmpProviderSummary {
  const models = new Map(
    (existing?.models ?? []).map((model) => [model.id, model]),
  );
  for (const model of request.provider.models) {
    models.set(model.id, { id: model.id, name: model.name ?? null });
  }
  const apiKey = request.provider.apiKey ?? null;
  return {
    id: request.provider.id,
    baseUrl: request.provider.baseUrl,
    api: request.provider.api,
    auth: request.provider.auth ?? existing?.auth ?? null,
    hasApiKey: apiKey !== null || Boolean(existing?.hasApiKey),
    hasCommandKey:
      apiKey !== null
        ? apiKey.startsWith("!")
        : Boolean(existing?.hasCommandKey),
    models: [...models.values()],
  };
}

function createPorts(read: OmpConfigRead = baseRead) {
  const ports = createBrowserFeaturePorts();
  let current = read;
  let planned: OmpSaveRequest | null = null;
  ports.ompModels.readConfig = vi.fn(async () => structuredClone(current));
  ports.ompModels.discoverCli = vi.fn(async () => ({
    detected: true,
    version: "18.12.5",
    executablePath: "C:\\Users\\me\\.omp\\bin\\omp.exe",
  }));
  ports.ompModels.previewSave = vi.fn(
    async (request): Promise<OmpSavePreview> => {
      planned = request;
      const existing = current.providers.find(
        (provider) => provider.id === request.provider.id,
      );
      const requestedDefault = request.defaultModel ?? null;
      return {
        planId: "omp-plan:1",
        planDigest: "digest-1",
        profile: current.profile,
        modelsPath: current.modelsPath,
        configPath: current.configPath,
        provider: mergeProvider(existing, request),
        defaultModel: requestedDefault ?? current.defaultModel,
        changedFiles: requestedDefault ? ["models", "config"] : ["models"],
        warnings: [],
      };
    },
  );
  ports.ompModels.applySave = vi.fn(async () => {
    const request = planned!;
    const existing = current.providers.find(
      (provider) => provider.id === request.provider.id,
    );
    const provider = mergeProvider(existing, request);
    const requestedDefault = request.defaultModel ?? null;
    current = {
      ...current,
      modelsRevision: "models-rev-2",
      configRevision: requestedDefault
        ? "config-rev-2"
        : current.configRevision,
      providers: [
        ...current.providers.filter((item) => item.id !== provider.id),
        provider,
      ],
      defaultModel: requestedDefault ?? current.defaultModel,
    };
    return {
      modelsPath: current.modelsPath,
      configPath: current.configPath,
      modelsRevision: current.modelsRevision,
      configRevision: current.configRevision,
      provider,
      defaultModel: current.defaultModel,
      modelsWritten: true,
      configWritten: requestedDefault !== null,
    };
  });
  return ports;
}

function renderPage(ports: FeaturePorts) {
  return render(
    <StrictMode>
      <MemoryRouter initialEntries={["/models?target=omp"]}>
        <TooltipProvider delayDuration={0} skipDelayDuration={0}>
          <FeatureProvider ports={ports}>
            <ModelsPage />
          </FeatureProvider>
        </TooltipProvider>
      </MemoryRouter>
    </StrictMode>,
  );
}

async function openPreview(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "预览并保存" }));
  return screen.findByRole("dialog", { name: "保存前确认" });
}

describe("Oh My Pi models panel", () => {
  it("lists Oh My Pi as a model target and shows CLI, path, profile and files", async () => {
    const ports = createPorts();
    renderPage(ports);

    expect(screen.getByTestId("model-target-omp")).toHaveTextContent(
      "Oh My Pi",
    );
    const status = await screen.findByTestId("omp-status");
    await waitFor(() => expect(status).toHaveTextContent("已安装 · v18.12.5"));
    expect(status).toHaveTextContent("default");
    expect(screen.getByText("~/.omp/agent/models.yml")).toBeVisible();
    expect(screen.getByText("C:\\Users\\me\\.omp\\bin\\omp.exe")).toBeVisible();
    expect(screen.getByText(/在 omp 中执行 \/login/)).toBeVisible();
    expect(screen.getByLabelText("供应商")).toHaveValue(
      "existing:team-gateway",
    );
  });

  it("previews without writing, confirms only the plan ID and digest, and rereads", async () => {
    const user = userEvent.setup();
    const ports = createPorts();
    renderPage(ports);

    const baseUrl = await screen.findByLabelText("服务地址");
    await user.clear(baseUrl);
    await user.type(baseUrl, "https://new.example/v1");
    await user.click(screen.getByLabelText("同时设为 omp 默认模型"));
    const dialog = await openPreview(user);

    expect(ports.ompModels.applySave).not.toHaveBeenCalled();
    expect(within(dialog).getByText("~/.omp/agent/config.yml")).toBeVisible();
    expect(
      within(dialog).getByText("team-gateway/vendor/model-a"),
    ).toBeVisible();
    expect(within(dialog).getByText("环境变量或密钥引用")).toBeVisible();

    expect(vi.mocked(ports.ompModels.previewSave).mock.calls[0][0]).toEqual({
      provider: {
        id: "team-gateway",
        baseUrl: "https://new.example/v1",
        api: "openai-completions",
        apiKey: null,
        auth: null,
        models: [{ id: "vendor/model-a", name: "Model A" }],
      },
      defaultModel: "team-gateway/vendor/model-a",
      expectedModelsRevision: "models-rev",
      expectedConfigRevision: "config-rev",
    });

    await user.click(within(dialog).getByRole("button", { name: "确认保存" }));
    await screen.findByText("已保存到 omp 配置");
    expect(ports.ompModels.applySave).toHaveBeenCalledTimes(1);
    expect(vi.mocked(ports.ompModels.applySave).mock.calls[0]).toEqual([
      { planId: "omp-plan:1", planDigest: "digest-1" },
    ]);
    expect(
      screen.getByText(/请退出正在运行的 omp 会话后重新启动/),
    ).toBeVisible();
    expect(
      vi.mocked(ports.ompModels.readConfig).mock.calls.length,
    ).toBeGreaterThanOrEqual(2);
    expect(screen.getByTestId("omp-status")).toHaveTextContent(
      "team-gateway/vendor/model-a",
    );
    expect(screen.getByLabelText("omp 保存结果")).toHaveTextContent(
      "config：~/.omp/agent/config.yml 已写入",
    );
  });

  it("adds a model while keeping the existing key and default", async () => {
    const user = userEvent.setup();
    const ports = createPorts();
    renderPage(ports);

    await user.type(await screen.findByLabelText("模型 ID"), "vendor/model-b");
    await user.click(screen.getByRole("button", { name: "添加" }));
    const dialog = await openPreview(user);
    const request = vi.mocked(ports.ompModels.previewSave).mock.calls[0][0];
    expect(request.provider.apiKey).toBeNull();
    expect(request.defaultModel).toBeNull();
    expect(request.provider.models.map((model) => model.id)).toEqual([
      "vendor/model-b",
      "vendor/model-a",
    ]);
    await user.click(within(dialog).getByRole("button", { name: "确认保存" }));
    await screen.findByText("已保存到 omp 配置");
    expect(screen.getByLabelText("omp 保存结果")).toHaveTextContent(
      "config：~/.omp/agent/config.yml 保持不变",
    );
  });

  it("cancelling the preview writes nothing and keeps the draft", async () => {
    const user = userEvent.setup();
    const ports = createPorts();
    renderPage(ports);

    const baseUrl = await screen.findByLabelText("服务地址");
    await user.clear(baseUrl);
    await user.type(baseUrl, "https://draft.example/v1");
    const dialog = await openPreview(user);
    await user.click(within(dialog).getByRole("button", { name: "取消" }));

    expect(ports.ompModels.applySave).not.toHaveBeenCalled();
    expect(screen.getByLabelText("服务地址")).toHaveValue(
      "https://draft.example/v1",
    );
  });

  it("creates a provider with an environment reference; the fyagent- prefix is only a suggestion", async () => {
    const user = userEvent.setup();
    const ports = createPorts();
    renderPage(ports);

    await user.selectOptions(await screen.findByLabelText("供应商"), "new");
    const providerId = screen.getByLabelText("供应商 ID");
    expect(providerId).toHaveValue("fyagent-custom");
    await user.clear(providerId);
    await user.type(providerId, "team-gateway");
    await user.click(screen.getByRole("button", { name: "预览并保存" }));
    expect(
      await screen.findByText("该供应商已存在，请在上方选择它进行修改。"),
    ).toBeVisible();
    expect(ports.ompModels.previewSave).not.toHaveBeenCalled();

    await user.clear(providerId);
    await user.type(providerId, "lab");
    await user.type(
      screen.getByLabelText("服务地址"),
      "http://127.0.0.1:8080/v1",
    );
    await user.selectOptions(screen.getByLabelText("协议"), "openai-responses");
    await user.type(screen.getByLabelText("环境变量名"), "LAB_KEY");
    await user.type(screen.getByLabelText("模型 ID"), "org/model/v2");
    await user.click(screen.getByRole("button", { name: "添加" }));
    const dialog = await openPreview(user);

    expect(within(dialog).getByText("环境变量 LAB_KEY")).toBeVisible();
    expect(
      vi.mocked(ports.ompModels.previewSave).mock.calls[0][0].provider,
    ).toEqual({
      id: "lab",
      baseUrl: "http://127.0.0.1:8080/v1",
      api: "openai-responses",
      apiKey: "LAB_KEY",
      auth: "apiKey",
      models: [{ id: "org/model/v2", name: null }],
    });
    await user.click(within(dialog).getByRole("button", { name: "确认保存" }));
    await screen.findByText("已保存到 omp 配置");
    expect(screen.getByLabelText("供应商")).toHaveValue("existing:lab");
    expect(
      JSON.stringify(vi.mocked(ports.ompModels.applySave).mock.calls),
    ).not.toContain("LAB_KEY");
  });

  it("offers only the two implemented protocols", async () => {
    const ports = createPorts();
    renderPage(ports);
    const select = await screen.findByLabelText("协议");
    expect(
      within(select)
        .getAllByRole("option")
        .map((option) => option.getAttribute("value")),
    ).toEqual(["openai-completions", "openai-responses"]);
  });

  it("shows preview warnings before confirmation", async () => {
    const user = userEvent.setup();
    const ports = createPorts();
    const preview = ports.ompModels.previewSave;
    ports.ompModels.previewSave = vi.fn(async (request) => ({
      ...(await preview(request)),
      warnings: ["已有模型会保留"],
    }));
    renderPage(ports);

    await screen.findByLabelText("服务地址");
    const dialog = await openPreview(user);
    expect(within(dialog).getByText("已有模型会保留")).toBeVisible();
  });

  it("rereads after a stale preview and never applies", async () => {
    const user = userEvent.setup();
    const ports = createPorts();
    ports.ompModels.previewSave = vi.fn(async () => {
      throw new OmpPortError("stale");
    });
    renderPage(ports);

    await screen.findByLabelText("服务地址");
    const readsBefore = vi.mocked(ports.ompModels.readConfig).mock.calls.length;
    await user.click(screen.getByRole("button", { name: "预览并保存" }));

    await screen.findByText("配置文件已被其他程序修改，未写入");
    await waitFor(() =>
      expect(
        vi.mocked(ports.ompModels.readConfig).mock.calls.length,
      ).toBeGreaterThan(readsBefore),
    );
    expect(ports.ompModels.applySave).not.toHaveBeenCalled();
  });

  it("guides a reread after a stale apply without blocking writes", async () => {
    const user = userEvent.setup();
    const ports = createPorts();
    ports.ompModels.applySave = vi.fn(async () => {
      throw new OmpPortError("stale");
    });
    renderPage(ports);

    await screen.findByLabelText("服务地址");
    const dialog = await openPreview(user);
    await user.click(within(dialog).getByRole("button", { name: "确认保存" }));

    await screen.findByText("配置文件已被其他程序修改，未写入");
    expect(screen.queryByText("已保存到 omp 配置")).toBeNull();
    expect(
      screen.getByRole("button", { name: "预览并保存" }),
    ).not.toBeDisabled();
  });

  it("blocks further writes when the apply result cannot be classified", async () => {
    const user = userEvent.setup();
    const ports = createPorts();
    ports.ompModels.applySave = vi.fn(async () => {
      throw new OmpPortError("unknown");
    });
    renderPage(ports);

    await screen.findByLabelText("服务地址");
    const dialog = await openPreview(user);
    await user.click(within(dialog).getByRole("button", { name: "确认保存" }));

    await screen.findByText("无法确认保存结果");
    expect(
      screen.getByRole("button", { name: "暂时无法确认当前设置" }),
    ).toBeDisabled();
  });

  it("leaves legacy JSON-only configuration to omp migration", async () => {
    const ports = createPorts({
      ...baseRead,
      providers: [],
      modelsRevision: "missing",
      modelsFileStatus: "legacyJsonOnly",
    });
    renderPage(ports);

    expect(await screen.findByText("请先让 omp 迁移旧版配置")).toBeVisible();
    expect(screen.getByText("仅有旧版 models.json")).toBeVisible();
    expect(screen.getByRole("button", { name: "预览并保存" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "重新读取" })).toBeEnabled();
    expect(screen.queryByLabelText("服务地址")).toBeNull();
  });

  it("refuses to edit when a file cannot be parsed and offers a reread", async () => {
    const user = userEvent.setup();
    const ports = createPorts({ ...baseRead, configFileStatus: "invalidYaml" });
    renderPage(ports);

    expect(await screen.findByText("omp 配置文件无法解析")).toBeVisible();
    expect(screen.getByRole("button", { name: "预览并保存" })).toBeDisabled();
    expect(screen.queryByLabelText("服务地址")).toBeNull();
    const readsBefore = vi.mocked(ports.ompModels.readConfig).mock.calls.length;
    await user.click(screen.getByRole("button", { name: "重新读取" }));
    await waitFor(() =>
      expect(
        vi.mocked(ports.ompModels.readConfig).mock.calls.length,
      ).toBeGreaterThan(readsBefore),
    );
  });

  it("shows a closed message when the configuration cannot be read", async () => {
    const ports = createPorts();
    ports.ompModels.readConfig = vi.fn(async () => {
      throw new OmpPortError("config");
    });
    renderPage(ports);

    expect(
      await screen.findByText("omp 配置文件当前无法安全修改"),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "预览并保存" })).toBeDisabled();
  });

  it("keeps providers with unsupported protocols read-only", async () => {
    const ports = createPorts({
      ...baseRead,
      providers: [
        {
          ...baseRead.providers[0],
          id: "codex-proxy",
          api: "openai-codex-responses",
        },
      ],
    });
    renderPage(ports);
    const select = await screen.findByLabelText("供应商");
    expect(select).toHaveValue("new");
    await userEvent.setup().selectOptions(select, "existing:codex-proxy");
    expect(await screen.findByText(/FyAgent 暂不编辑这种格式/)).toBeVisible();
    expect(screen.getByRole("button", { name: "预览并保存" })).toBeDisabled();
  });

  it("shows install guidance without running anything when omp is absent", async () => {
    const ports = createPorts();
    ports.ompModels.discoverCli = vi.fn(async () => ({
      detected: false,
      version: null,
      executablePath: null,
    }));
    renderPage(ports);

    expect(
      await screen.findByText("irm https://omp.sh/install.ps1 | iex"),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "打开 omp 官方安装说明" }),
    ).toBeVisible();
    expect(screen.getByTestId("omp-status")).toHaveTextContent("未检测到");
  });
});
