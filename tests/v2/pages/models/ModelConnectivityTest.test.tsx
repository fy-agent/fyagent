import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { ModelConnectivityTest } from "@/v2/pages/models/ModelConnectivityTest";
import { TooltipProvider } from "@/v2/shared/ui/primitives";

function renderProbe(
  props: Partial<Parameters<typeof ModelConnectivityTest>[0]> = {},
) {
  const onProbe = vi.fn(async () => ({
    success: true,
    status: "operational" as const,
    message: "模型 gpt-test 已响应（12 ms）",
    responseTimeMs: 12,
    httpStatus: 200,
    modelUsed: "gpt-test",
    errorCategory: null,
  }));
  render(
    <TooltipProvider delayDuration={0} skipDelayDuration={0}>
      <ModelConnectivityTest
        searchId="probe-search"
        modelIds={["gpt-test", "claude-sonnet-4"]}
        onProbe={onProbe}
        {...props}
      />
    </TooltipProvider>,
  );
  return { onProbe };
}

describe("ModelConnectivityTest", () => {
  it("does not render the button when there are no models", () => {
    renderProbe({ modelIds: [] });
    expect(
      screen.queryByRole("button", { name: "测试连通" }),
    ).not.toBeInTheDocument();
  });

  it("lets the user search, filter by group, pick a model, and shows upstream errors", async () => {
    const user = userEvent.setup();
    const onProbe = vi.fn(async () => ({
      success: false,
      status: "failed" as const,
      message: 'HTTP 401: {"error":{"message":"invalid api key"}}',
      responseTimeMs: 40,
      httpStatus: 401,
      modelUsed: "gpt-test",
      errorCategory: null,
    }));
    renderProbe({ onProbe });

    await user.click(screen.getByRole("button", { name: "测试连通" }));
    expect(
      await screen.findByRole("heading", { name: "选择要测试的模型" }),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "开始测试" })).toBeDisabled();

    await user.click(
      within(screen.getByRole("toolbar", { name: "按分组过滤" })).getByRole(
        "button",
        { name: /claude/ },
      ),
    );
    expect(screen.queryByText("gpt-test")).not.toBeInTheDocument();
    expect(screen.getByText("claude-sonnet-4")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "全部" }));
    await user.type(screen.getByLabelText("搜索模型"), "gpt");
    expect(screen.getByText("gpt-test")).toBeVisible();
    expect(screen.queryByText("claude-sonnet-4")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "gpt-test" }));
    await user.click(screen.getByRole("button", { name: "开始测试" }));
    expect(onProbe).toHaveBeenCalledWith("gpt-test");
    expect(await screen.findByText("连通测试失败")).toBeVisible();
    expect(screen.getByText(/invalid api key/)).toBeVisible();
  });

  it("invalidates a stale probe result when the owning draft revision changes", async () => {
    const user = userEvent.setup();
    const onProbe = vi.fn(async () => ({
      success: false,
      status: "failed" as const,
      message: "HTTP 400: stale failure",
      responseTimeMs: 20,
      httpStatus: 400,
      modelUsed: "gpt-test",
      errorCategory: null,
    }));
    const view = render(
      <TooltipProvider delayDuration={0} skipDelayDuration={0}>
        <ModelConnectivityTest
          searchId="probe-reset-search"
          modelIds={["gpt-test"]}
          onProbe={onProbe}
          resetVersion="1:0"
        />
      </TooltipProvider>,
    );

    await user.click(screen.getByRole("button", { name: "测试连通" }));
    await user.click(screen.getByRole("button", { name: "gpt-test" }));
    await user.click(screen.getByRole("button", { name: "开始测试" }));
    expect(await screen.findByText("连通测试失败")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "关闭" }));
    expect(screen.getByText("连通测试失败")).toBeVisible();

    view.rerender(
      <TooltipProvider delayDuration={0} skipDelayDuration={0}>
        <ModelConnectivityTest
          searchId="probe-reset-search"
          modelIds={["gpt-test"]}
          onProbe={onProbe}
          resetVersion="1:1"
        />
      </TooltipProvider>,
    );
    expect(screen.queryByText("连通测试失败")).not.toBeInTheDocument();
  });
});
