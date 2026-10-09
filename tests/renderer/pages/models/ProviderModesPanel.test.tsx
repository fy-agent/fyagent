import { useState } from "react";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ProviderModesPanel } from "@/pages/models/ProviderModesPanel";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import type {
  AppMode,
  AppModeView,
  ProxyStack,
} from "@/domain/configuration/types/proxy";
import type { Provider } from "@/domain/configuration/types";
import { TooltipProvider } from "@/shared/ui/primitives";

vi.mock("@/domain/configuration/presets/presetModelMetadata", () => ({
  codexPresetModelSources: () => [
    {
      baseUrl: "https://example.com",
      models: new Map([["model-a", { contextWindow: 4321 }]]),
    },
  ],
}));

const codexProvider: Provider = {
  id: "A",
  name: "A",
  category: "custom",
  settingsConfig: {
    auth: { OPENAI_API_KEY: "test-key" },
    config:
      'model_provider = "custom"\nmodel = "model-a"\n[model_providers.custom]\nbase_url = "https://example.com"\n',
    modelCatalog: {
      models: [
        {
          model: "model-a",
          reasoningLevels: ["low", "high"],
          defaultReasoningLevel: "high",
        },
      ],
    },
  },
};

const apiProvider = (id: string): Provider => ({
  id,
  name: id,
  category: "custom",
  settingsConfig: {
    env: {
      ANTHROPIC_BASE_URL: "https://example.com",
      ANTHROPIC_AUTH_TOKEN: "test-key",
      ANTHROPIC_MODEL: "model-a",
    },
  },
  meta: { stackModels: [{ model: "model-a" }] },
});

function setup(
  app: "claude" | "codex" = "claude",
  initial?: Partial<AppModeView>,
  stale = false,
  firstProvider?: Provider,
) {
  const ports = createBrowserFeaturePorts();
  let mode: AppModeView = {
    mode: "direct",
    attached: false,
    routeProviderId: "A",
    directProviderId: "A",
    ...initial,
  };
  let stack: ProxyStack = {
    active: mode.mode === "stack",
    members: [],
    ...(stale ? { staleClients: { daemon: true, others: true } } : {}),
  };
  const providers: Record<string, Provider> = {
    A: firstProvider ?? apiProvider("A"),
    B: apiProvider("B"),
    official: {
      id: "codex-official",
      name: "官方订阅",
      category: "official",
      settingsConfig: { auth: {}, config: "" },
    },
  };
  ports.providers.getAll = vi.fn(async () => providers);
  ports.providers.getMode = vi.fn(async () => mode);
  ports.providers.getStack = vi.fn(async () => stack);
  ports.providers.setMode = vi.fn(async (_app, enabled, isStack, route) => {
    mode = {
      ...mode,
      mode: enabled ? (isStack ? "stack" : "route") : "direct",
      attached: enabled,
      routeProviderId: route ?? mode.routeProviderId,
    };
    stack = { ...stack, active: isStack === true };
  });
  ports.providers.setStackMember = vi.fn(async (_app, providerId, enabled) => {
    stack = {
      ...stack,
      members: enabled
        ? [
            ...stack.members,
            { providerId, modelIds: ["ccs-test/model-a"], route: false },
          ]
        : stack.members.filter((member) => member.providerId !== providerId),
    };
    return null;
  });
  ports.providers.setRoute = vi.fn(async (_app, providerId) => {
    mode = { ...mode, routeProviderId: providerId };
    stack = {
      ...stack,
      members: [{ providerId, modelIds: ["model-a"], route: true }],
    };
  });
  ports.providers.restartCodexDaemon = vi.fn(async () => "restarted" as const);
  ports.providers.update = vi.fn(async (_app, provider) => {
    providers[provider.id] = provider;
    return true;
  });
  ports.providers.fetchModels = vi.fn(async () => [
    { id: "model-b" },
    { id: "model-c" },
  ]);
  function Harness() {
    const [view, setView] = useState<AppMode>("direct");
    return (
      <ProviderModesPanel
        app={app}
        active
        disabled={false}
        view={view}
        onViewChange={setView}
      />
    );
  }
  render(
    <TooltipProvider>
      <FeatureProvider ports={ports}>
        <Harness />
      </FeatureProvider>
    </TooltipProvider>,
  );
  return { ports };
}

describe("upstream provider modes", () => {
  it("creates a Codex supplier with image extension explicitly off", async () => {
    const user = userEvent.setup();
    const { ports } = setup("codex");
    ports.providers.add = vi.fn(async () => true);
    await screen.findByText("A");
    await user.click(screen.getByRole("button", { name: "添加供应商" }));
    const editor = screen.getByRole("dialog", { name: "添加供应商" });
    await user.type(within(editor).getByLabelText("供应商名称"), "New Codex");
    await user.type(
      within(editor).getByLabelText("API 地址"),
      "https://example.com",
    );
    await user.type(within(editor).getByLabelText("API Key"), "fixture-key");
    await user.click(within(editor).getByRole("button", { name: "手动添加" }));
    await user.type(within(editor).getByLabelText("模型 ID"), "model-a");
    await user.click(
      within(editor).getByRole("button", { name: "保存供应商" }),
    );
    await waitFor(() => expect(ports.providers.add).toHaveBeenCalledOnce());
    const [app, saved] = vi.mocked(ports.providers.add).mock.calls[0];
    expect(app).toBe("codex");
    expect(saved.meta).toMatchObject({
      apiFormat: "openai_responses",
      imageExtensionConfigured: true,
    });
    expect(saved.settingsConfig.config).toContain("requires_openai_auth = true");
    expect(saved.settingsConfig.config).not.toContain(
      "x-openai-actor-authorization",
    );
  });

  it("ignores fetched models from a connection edited while the request was running", async () => {
    const user = userEvent.setup();
    const { ports } = setup();
    let respond: (models: { id: string }[]) => void = () => undefined;
    ports.providers.fetchModels = vi.fn(
      () =>
        new Promise<{ id: string }[]>((resolve) => {
          respond = resolve;
        }),
    );
    await screen.findByText("A");
    await user.click(screen.getByRole("button", { name: "添加供应商" }));
    const editor = screen.getByRole("dialog", { name: "添加供应商" });
    await user.type(within(editor).getByLabelText("供应商名称"), "New");
    const address = within(editor).getByLabelText("API 地址");
    await user.type(address, "https://first.example.com");
    await user.type(within(editor).getByLabelText("API Key"), "fixture-key");
    await user.click(within(editor).getByRole("button", { name: "获取模型" }));
    await user.clear(address);
    await user.type(address, "https://second.example.com");
    await act(async () => respond([{ id: "old-connection-model" }]));
    expect(
      within(editor).queryByRole("checkbox", {
        name: "选择 old-connection-model",
      }),
    ).toBeNull();
    expect(
      within(editor).getByRole("button", { name: "获取模型" }),
    ).toBeEnabled();
  });

  it("allows clearing a Claude 1M marker carried in the model ID", async () => {
    const user = userEvent.setup();
    const provider = {
      ...apiProvider("A"),
      meta: { stackModels: [{ model: "model-a[1M]" }] },
    };
    const { ports } = setup("claude", undefined, false, provider);
    await screen.findByText("A");
    await user.click(screen.getAllByRole("button", { name: "编辑模型" })[0]);
    const editor = screen.getByRole("dialog", { name: "编辑 A 的模型" });
    const oneM = within(editor).getByRole("checkbox", {
      name: "model-a[1M] 支持 1M 上下文",
    });
    expect(oneM).toBeChecked();
    await user.click(oneM);
    expect(within(editor).getByLabelText("模型 ID")).toHaveValue("model-a");
    await user.click(
      within(editor).getByRole("button", { name: "保存供应商" }),
    );
    await waitFor(() =>
      expect(ports.providers.update).toHaveBeenCalledWith(
        "claude",
        expect.objectContaining({
          meta: { stackModels: [{ model: "model-a" }] },
        }),
      ),
    );
  });

  it("reports a failed reread after saving without inviting a duplicate save", async () => {
    const user = userEvent.setup();
    const { ports } = setup();
    await screen.findByText("A");
    await user.click(screen.getAllByRole("button", { name: "编辑模型" })[0]);
    const editor = screen.getByRole("dialog", { name: "编辑 A 的模型" });
    vi.mocked(ports.providers.getAll).mockRejectedValue(
      new Error("read failed"),
    );
    await user.click(
      within(editor).getByRole("button", { name: "保存供应商" }),
    );
    await waitFor(
      () =>
        expect(
          screen.queryByRole("dialog", { name: "编辑 A 的模型" }),
        ).toBeNull(),
      { timeout: 3000 },
    );
    expect(
      screen.getByText("模型已保存，但无法刷新列表，请重试读取。"),
    ).toBeVisible();
    expect(ports.providers.update).toHaveBeenCalledOnce();
  });
  it("only shows reload warnings for clients using an old catalog", async () => {
    const user = userEvent.setup();
    setup("codex");
    await screen.findByText("A");
    await user.click(screen.getByRole("tab", { name: "聚合" }));
    expect(screen.queryByText(/旧模型列表/)).toBeNull();
    expect(screen.queryByText(/桌面版请完全退出/)).toBeNull();
  });

  it("clears the default reasoning level when that level is removed", async () => {
    const user = userEvent.setup();
    const { ports } = setup("codex", undefined, false, codexProvider);
    await screen.findByText("A");
    await user.click(screen.getAllByRole("button", { name: "编辑模型" })[0]);
    const editor = screen.getByRole("dialog", { name: "编辑 A 的模型" });
    expect(within(editor).getByLabelText("默认思考档位")).toHaveValue("high");
    await user.click(
      within(editor).getByRole("checkbox", { name: "model-a high" }),
    );
    expect(within(editor).getByLabelText("默认思考档位")).toHaveValue("");
    await user.click(
      within(editor).getByRole("button", { name: "保存供应商" }),
    );
    await waitFor(() => expect(ports.providers.update).toHaveBeenCalledOnce());
    expect(ports.providers.update).toHaveBeenCalledWith(
      "codex",
      expect.objectContaining({
        settingsConfig: expect.objectContaining({
          modelCatalog: {
            models: [
              {
                model: "model-a",
                reasoningLevels: ["low"],
                defaultReasoningLevel: undefined,
              },
            ],
          },
        }),
      }),
    );
  });

  it("fills local presets immediately and keeps later user edits when models.dev arrives", async () => {
    const user = userEvent.setup();
    let respond: (value: Response) => void = () => undefined;
    vi.spyOn(globalThis, "fetch").mockImplementation(
      () =>
        new Promise((resolve) => {
          respond = resolve;
        }),
    );
    setup("codex", undefined, false, codexProvider);
    await screen.findByText("A");
    await user.click(screen.getAllByRole("button", { name: "编辑模型" })[0]);
    const editor = screen.getByRole("dialog", { name: "编辑 A 的模型" });
    await user.click(
      within(editor).getByRole("button", { name: "补全空白参数" }),
    );
    const context = within(editor).getByLabelText("上下文窗口");
    expect(context).toHaveValue(4321);
    expect(
      within(editor).getByText("已补全空白参数（来源：同地址预设），请核对。"),
    ).toBeVisible();
    await user.clear(context);
    await user.type(context, "9000");
    await act(async () => {
      respond(
        new Response(
          JSON.stringify({
            fixture: {
              api: "https://example.com",
              models: {
                "model-a": {
                  limit: { context: 5678 },
                  modalities: { input: ["text", "image"] },
                },
              },
            },
          }),
        ),
      );
    });
    await waitFor(() =>
      expect(
        within(editor).getByRole("checkbox", { name: "model-a 支持图片" }),
      ).toBeChecked(),
    );
    expect(context).toHaveValue(9000);
  });
  it("browses without enabling, adds members and confirms a selected default", async () => {
    const user = userEvent.setup();
    const { ports } = setup();
    await screen.findByText("A");
    await user.click(screen.getByRole("tab", { name: "聚合" }));
    expect(ports.providers.setMode).not.toHaveBeenCalled();
    await user.click(screen.getAllByRole("button", { name: "加入聚合" })[1]);
    await waitFor(() =>
      expect(ports.providers.setStackMember).toHaveBeenCalledWith(
        "claude",
        "B",
        true,
      ),
    );
    await user.click(screen.getByRole("button", { name: "启用聚合…" }));
    const dialog = screen.getByRole("dialog", { name: "启用聚合" });
    expect(
      within(dialog).queryByRole("option", { name: "官方订阅" }),
    ).toBeNull();
    await user.selectOptions(within(dialog).getByRole("combobox"), "B");
    await user.click(within(dialog).getByRole("button", { name: "确认启用" }));
    await waitFor(() =>
      expect(ports.providers.setMode).toHaveBeenCalledWith(
        "claude",
        true,
        true,
        "B",
      ),
    );
  });

  it("keeps Codex official subscription eligible as default and confirms daemon interruption", async () => {
    const user = userEvent.setup();
    const { ports } = setup("codex", undefined, true);
    await screen.findByText("A");
    await user.click(screen.getByRole("tab", { name: "聚合" }));
    await user.click(screen.getByRole("button", { name: "启用聚合…" }));
    const enter = screen.getByRole("dialog", { name: "启用聚合" });
    expect(
      within(enter).getByRole("option", { name: "官方订阅" }),
    ).toBeInTheDocument();
    await user.click(within(enter).getByRole("button", { name: "取消" }));
    await user.click(
      screen.getByRole("button", { name: "重启 Codex 命令行服务…" }),
    );
    expect(ports.providers.restartCodexDaemon).not.toHaveBeenCalled();
    const restart = screen.getByRole("dialog", {
      name: "重启 Codex 命令行服务",
    });
    expect(within(restart).getByText(/会中断/)).toBeVisible();
    await user.click(within(restart).getByRole("button", { name: "确认重启" }));
    await waitFor(() =>
      expect(ports.providers.restartCodexDaemon).toHaveBeenCalledOnce(),
    );
  });

  it("batch-adds models and saves the Claude default as first row without changing credentials", async () => {
    const user = userEvent.setup();
    const { ports } = setup();
    await screen.findByText("A");
    await user.click(screen.getAllByRole("button", { name: "编辑模型" })[0]);
    const editor = screen.getByRole("dialog", { name: "编辑 A 的模型" });
    await user.click(within(editor).getByRole("button", { name: "获取模型" }));
    await within(editor).findByRole("checkbox", { name: "选择 model-b" });
    await user.click(
      within(editor).getByRole("button", { name: "选择全部搜索结果" }),
    );
    await user.click(
      within(editor).getByRole("button", { name: "添加选中的 2 个模型" }),
    );
    await user.click(
      within(editor).getByRole("button", { name: "将 model-c 设为默认模型" }),
    );
    await user.click(
      within(editor).getByRole("button", { name: "保存供应商" }),
    );
    await waitFor(() => expect(ports.providers.update).toHaveBeenCalledOnce());
    expect(ports.providers.update).toHaveBeenCalledWith(
      "claude",
      expect.objectContaining({
        settingsConfig: apiProvider("A").settingsConfig,
        meta: {
          stackModels: [
            { model: "model-c" },
            { model: "model-a" },
            { model: "model-b" },
          ],
        },
      }),
    );
  });
});
