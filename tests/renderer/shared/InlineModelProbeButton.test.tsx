import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { InlineModelProbeButton } from "@/shared/features/controls/InlineModelProbeButton";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

describe("InlineModelProbeButton", () => {
  it("displays latency on ok, diagnoses 401/404 without mutating state, and resets when target/model changes", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();

    ports.providers.probeSavedModel = vi
      .fn()
      .mockResolvedValueOnce({
        category: "ok",
        ok: true,
        latencyMs: 184,
        httpStatus: 200,
        message: "连接可用",
      })
      .mockResolvedValueOnce({
        category: "endpoint_or_model_mismatch",
        ok: false,
        latencyMs: 92,
        httpStatus: 404,
        message: "模型 ID 不匹配或接口路径不兼容（未改动已保存配置）",
      });

    const { rerender } = render(
      <FeatureProvider ports={ports}>
        <InlineModelProbeButton
          target="codex"
          providerId="deepseek"
          modelId="deepseek-chat"
        />
      </FeatureProvider>,
    );

    await user.click(
      screen.getByRole("button", { name: "测试模型 deepseek-chat 连通性" }),
    );
    expect(await screen.findByRole("status")).toHaveTextContent("✓ 184ms");
    expect(ports.providers.probeSavedModel).toHaveBeenCalledWith({
      target: "codex",
      providerId: "deepseek",
      modelId: "deepseek-chat",
    });

    // Switching model resets stale probe badge synchronously
    rerender(
      <FeatureProvider ports={ports}>
        <InlineModelProbeButton
          target="workbuddy"
          modelId="missing-model-v9"
        />
      </FeatureProvider>,
    );
    expect(screen.queryByRole("status")).not.toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "测试模型 missing-model-v9 连通性" }),
    );
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "模型/路径不匹配 (404)",
      ),
    );
    expect(screen.getByRole("status")).toHaveTextContent("未改动已保存配置");
  });
});
