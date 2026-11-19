import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { ModelHealthDialog } from "@/pages/models/ModelHealthDialog";
import { FeatureProvider } from "@/shared/features/provider";
import type {
  ApiKeyRotationApplyRequest,
  ApiKeyRotationPreviewRequest,
  CloneModelConfigRequest,
  ModelHealthOverview,
  SavedModelProbeRequest,
} from "@/shared/features/types";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

function createSampleOverview(): ModelHealthOverview {
  return {
    targets: [
      {
        target: "claude",
        configured: true,
        providerId: "p-claude",
        providerName: "DeepSeek",
        baseUrl: "https://api.deepseek.com/anthropic",
        endpointHost: "api.deepseek.com",
        modelId: "deepseek-chat",
        protocol: "anthropic",
        modelCount: 1,
        credentialSource: "api_key",
        canProbe: true,
        canCloneSource: true,
        canRotateKey: true,
      },
      {
        target: "codex",
        configured: true,
        providerId: "p-codex",
        providerName: "DeepSeek",
        baseUrl: "https://api.deepseek.com/v1",
        endpointHost: "api.deepseek.com",
        modelId: "deepseek-chat",
        protocol: "chat",
        modelCount: 1,
        credentialSource: "api_key",
        canProbe: true,
        canCloneSource: true,
        canRotateKey: true,
      },
      {
        target: "grokbuild",
        configured: false,
        providerId: null,
        providerName: null,
        baseUrl: null,
        endpointHost: null,
        modelId: null,
        protocol: null,
        modelCount: 0,
        credentialSource: "none",
        canProbe: false,
        canCloneSource: false,
        canRotateKey: false,
        statusNote: "尚未配置第三方模型",
      },
      {
        target: "opencode",
        configured: true,
        providerId: "openai",
        providerName: "OpenAI Official",
        baseUrl: null,
        endpointHost: null,
        modelId: "gpt-5.4",
        protocol: "responses",
        modelCount: 2,
        credentialSource: "oauth",
        canProbe: false,
        canCloneSource: false,
        canRotateKey: false,
        statusNote: "官方订阅账号",
      },
      {
        target: "omp",
        configured: true,
        providerId: "custom-gw",
        providerName: "Company GW",
        baseUrl: "https://gw.internal.example/v1",
        endpointHost: "gw.internal.example",
        modelId: "gpt-5.4-mini",
        protocol: "responses",
        modelCount: 1,
        credentialSource: "api_key",
        canProbe: true,
        canCloneSource: true,
        canRotateKey: true,
      },
      {
        target: "workbuddy",
        configured: false,
        providerId: null,
        providerName: null,
        baseUrl: null,
        endpointHost: null,
        modelId: null,
        protocol: null,
        modelCount: 0,
        credentialSource: "none",
        canProbe: false,
        canCloneSource: false,
        canRotateKey: false,
        statusNote: "仅配置了官方账号",
      },
    ],
  };
}

function renderDialog(
  ports: ReturnType<typeof createBrowserFeaturePorts>,
  onSaveAsTemplate = vi.fn(),
) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const onOpenChange = vi.fn();
  render(
    <MemoryRouter>
      <QueryClientProvider client={queryClient}>
        <FeatureProvider ports={ports}>
          <ModelHealthDialog
            open
            onOpenChange={onOpenChange}
            onSaveAsTemplate={onSaveAsTemplate}
          />
        </FeatureProvider>
      </QueryClientProvider>
    </MemoryRouter>,
  );
  return { onOpenChange, onSaveAsTemplate };
}

describe("ModelHealthDialog", () => {
  it("renders all 6 targets, runs batch health probes, and hands off save-as-template draft", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getModelHealthOverview = vi.fn(async () =>
      createSampleOverview(),
    );
    ports.providers.probeSavedModel = vi.fn(
      async (req: SavedModelProbeRequest) => ({
        ok: req.target !== "omp",
        category: req.target === "omp" ? ("invalid_key" as const) : ("ok" as const),
        httpStatus: req.target === "omp" ? 401 : 200,
        latencyMs: req.target === "omp" ? 95 : 180,
        message: req.target === "omp" ? "API Key 无效或已过期" : "连接正常",
      }),
    );

    const { onSaveAsTemplate, onOpenChange } = renderDialog(ports);

    expect(await screen.findByTestId("health-row-claude")).toBeVisible();
    expect(screen.getByTestId("health-row-opencode")).toHaveTextContent(
      "官方订阅",
    );

    const probeAllBtn = screen.getByRole("button", {
      name: "一键全端测速 (3)",
    });
    await user.click(probeAllBtn);

    await waitFor(() =>
      expect(ports.providers.probeSavedModel).toHaveBeenCalledTimes(3),
    );
    expect(
      await within(screen.getByTestId("health-row-claude")).findByText(
        "可用 · 180 ms",
      ),
    ).toBeVisible();
    expect(
      await within(screen.getByTestId("health-row-omp")).findByText(
        "Key 无效 (401)",
      ),
    ).toBeVisible();

    const saveBtn = within(screen.getByTestId("health-row-omp")).getByRole(
      "button",
      { name: "存为模板" },
    );
    await user.click(saveBtn);
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onSaveAsTemplate).toHaveBeenCalledWith({
      name: "Company GW",
      baseUrl: "https://gw.internal.example/v1",
      defaultModelId: "gpt-5.4-mini",
      codexProtocol: "responses",
    });
  });

  it("previews cross-app clone, applies, and retries only failed targets", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getModelHealthOverview = vi.fn(async () =>
      createSampleOverview(),
    );
    ports.providers.previewCloneModelConfig = vi.fn(
      async (req: CloneModelConfigRequest) => ({
        items: req.destinationTargets.map((target) => ({
          target,
          paths: [`~/.config/${target}/config`],
          baseUrl: "https://api.deepseek.com/v1",
          modelId: req.modelIdOverride || "deepseek-chat",
          ready: true,
        })),
      }),
    );

    let attempt = 0;
    ports.providers.applyCloneModelConfig = vi.fn(
      async (req: CloneModelConfigRequest) => {
        attempt += 1;
        return {
          outcomes: req.destinationTargets.map((target) => ({
            target,
            status:
              attempt === 1 && target === "workbuddy"
                ? ("failed" as const)
                : ("applied" as const),
            message:
              attempt === 1 && target === "workbuddy"
                ? "写入失败，已自动回滚"
                : "已克隆并生效",
            paths: [`~/.config/${target}/config`],
          })),
        };
      },
    );

    renderDialog(ports);

    const claudeRow = await screen.findByTestId("health-row-claude");
    await user.click(
      within(claudeRow).getByRole("button", { name: "克隆到其他端" }),
    );

    // Unconfigured non-opt-in target (workbuddy) is pre-selected by default; grokbuild is opt-in.
    await user.click(screen.getByRole("checkbox", { name: "Grok Build" }));
    await user.click(screen.getByRole("button", { name: "预览克隆范围" }));
    expect(await screen.findByText("即将改动的配置文件")).toBeVisible();
    expect(ports.providers.previewCloneModelConfig).toHaveBeenCalledWith(
      expect.objectContaining({
        sourceTarget: "claude",
        destinationTargets: ["workbuddy", "grokbuild"],
      }),
    );

    await user.click(screen.getByRole("button", { name: "确认克隆写入" }));
    expect(await screen.findByText("失败已回滚")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "重试失败项 (1)" }));
    await waitFor(() =>
      expect(ports.providers.applyCloneModelConfig).toHaveBeenCalledTimes(2),
    );
    expect(
      vi.mocked(ports.providers.applyCloneModelConfig).mock.calls[1][0]
        .destinationTargets,
    ).toEqual(["workbuddy"]);
  });

  it("pre-selects same-host targets for API key rotation, trims new key, and applies", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.providers.getModelHealthOverview = vi.fn(async () =>
      createSampleOverview(),
    );
    ports.providers.previewApiKeyRotation = vi.fn(
      async (req: ApiKeyRotationPreviewRequest) => ({
        items: req.targets.map((target) => ({
          target,
          paths: [`~/.config/${target}/config`],
          baseUrl: "https://api.deepseek.com/v1",
          modelId: "deepseek-chat",
          ready: true,
        })),
      }),
    );
    ports.providers.applyApiKeyRotation = vi.fn(
      async (req: ApiKeyRotationApplyRequest) => ({
        outcomes: req.targets.map((target) => ({
          target,
          status: "applied" as const,
          message: "API Key 已轮换并生效",
          paths: [`~/.config/${target}/config`],
        })),
      }),
    );

    renderDialog(ports);

    const claudeRow = await screen.findByTestId("health-row-claude");
    await user.click(
      within(claudeRow).getByRole("button", { name: "轮换 Key" }),
    );

    await user.type(
      screen.getByPlaceholderText("粘贴新的 API Key"),
      "  sk-rotated-999  ",
    );

    await user.click(screen.getByRole("button", { name: "预览轮换范围" }));
    expect(await screen.findByText("即将更新密钥的配置文件")).toBeVisible();
    // Both claude and codex share endpointHost "api.deepseek.com", while omp uses gw.internal.example.
    expect(ports.providers.previewApiKeyRotation).toHaveBeenCalledWith({
      targets: ["claude", "codex"],
    });

    await user.click(screen.getByRole("button", { name: "确认一键轮换" }));
    await waitFor(() =>
      expect(ports.providers.applyApiKeyRotation).toHaveBeenCalledWith({
        targets: ["claude", "codex"],
        apiKey: "sk-rotated-999",
      }),
    );
    expect(await screen.findAllByText("已生效")).toHaveLength(2);
  });
});
