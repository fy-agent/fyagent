import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { PresetBatchDrawer } from "@/pages/models/PresetBatchDrawer";
import { FeatureProvider } from "@/shared/features/provider";
import type {
  BatchProviderApplyRequest,
  BatchProviderPreviewRequest,
} from "@/shared/features/types";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

describe("PresetBatchDrawer", () => {
  it("previews, trims pasted API key, applies across selected targets, and allows retrying isolated failures", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();

    ports.providers.previewBatchSetup = vi.fn(
      async (req: BatchProviderPreviewRequest) => ({
        items: req.targets.map((t) => ({
          target: t.target,
          paths: [`~/.config/${t.target}/config`],
          baseUrl: t.baseUrl,
          modelId: t.modelId,
          ready: true,
        })),
      }),
    );

    let attempt = 0;
    ports.providers.applyBatchSetup = vi.fn(
      async (req: BatchProviderApplyRequest) => {
        attempt += 1;
        return {
          outcomes: req.targets.map((t) => ({
            target: t.target,
            status:
              attempt === 1 && t.target === "omp"
                ? ("failed" as const)
                : ("applied" as const),
            message:
              attempt === 1 && t.target === "omp"
                ? "omp 配置写入失败，原 YAML 文件未改动"
                : "已写入并生效",
            paths: [`~/.config/${t.target}/config`],
          })),
        };
      },
    );

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <MemoryRouter>
        <QueryClientProvider client={queryClient}>
          <FeatureProvider ports={ports}>
            <PresetBatchDrawer open onOpenChange={vi.fn()} />
          </FeatureProvider>
        </QueryClientProvider>
      </MemoryRouter>,
    );

    // Select Zhipu GLM
    await user.click(screen.getByRole("radio", { name: /智谱 GLM/ }));
    await user.type(
      screen.getByPlaceholderText(/粘贴 智谱 GLM/),
      "  sk-zhipu-key-123  ",
    );

    await user.click(screen.getByRole("button", { name: "预览改动范围" }));
    expect(
      await screen.findByText(/即将改动的配置文件/),
    ).toBeVisible();
    expect(
      screen.getAllByText("https://open.bigmodel.cn/api/paas/v4").length,
    ).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "确认一键写入" }));
    await waitFor(() =>
      expect(ports.providers.applyBatchSetup).toHaveBeenCalledTimes(1),
    );
    expect(
      vi.mocked(ports.providers.applyBatchSetup).mock.calls[0][0].apiKey,
    ).toBe("sk-zhipu-key-123");

    expect(await screen.findByText("失败已回滚")).toBeVisible();
    const retryBtn = screen.getByRole("button", { name: "重试失败项 (1)" });
    await user.click(retryBtn);

    await waitFor(() =>
      expect(ports.providers.applyBatchSetup).toHaveBeenCalledTimes(2),
    );
    expect(
      vi.mocked(ports.providers.applyBatchSetup).mock.calls[1][0].targets.map(
        (t) => t.target,
      ),
    ).toEqual(["omp"]);
  });
});
