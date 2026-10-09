import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type {
  ModelProbeResult,
  ModelProbeSnapshot,
} from "@/shared/features/models";
import { ModelConnectivityTest } from "@/pages/models/ModelConnectivityTest";
import { PersistentSurface } from "@/shared/ui/PersistentSurface";
import { TooltipProvider } from "@/shared/ui/primitives";

function renderProbe(
  props: Partial<Parameters<typeof ModelConnectivityTest>[0]> = {},
) {
  const onProbe = vi.fn(
    async (_modelId: string, requestId: string): Promise<ModelProbeResult> => ({
      success: true,
      status: "operational",
      message: "模型 gpt-test 已响应（12 ms）",
      responseTimeMs: 12,
      httpStatus: 200,
      modelUsed: "gpt-test",
      errorCategory: null,
      requestId,
      terminal: "completed",
      requestCount: 1,
      retryCount: 0,
      inputMode: "compatibility",
    }),
  );
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
    const onProbe = vi.fn(
      async (
        _modelId: string,
        requestId: string,
      ): Promise<ModelProbeResult> => ({
        success: false,
        status: "failed",
        message: 'HTTP 401: {"error":{"message":"invalid api key"}}',
        responseTimeMs: 40,
        httpStatus: 401,
        modelUsed: "gpt-test",
        errorCategory: null,
        requestId,
        terminal: "completed",
        requestCount: 1,
        retryCount: 0,
        inputMode: "compatibility",
      }),
    );
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
    expect(onProbe).toHaveBeenCalledWith(
      "gpt-test",
      expect.stringMatching(/^[0-9a-f-]{36}$/),
    );
    expect(await screen.findByText("连通测试失败")).toBeVisible();
    expect(screen.getByText(/invalid api key/)).toBeVisible();
  });

  it("invalidates a stale probe result when the owning draft revision changes", async () => {
    const user = userEvent.setup();
    const onProbe = vi.fn(
      async (
        _modelId: string,
        requestId: string,
      ): Promise<ModelProbeResult> => ({
        success: false,
        status: "failed",
        message: "HTTP 400: stale failure",
        responseTimeMs: 20,
        httpStatus: 400,
        modelUsed: "gpt-test",
        errorCategory: null,
        requestId,
        terminal: "completed",
        requestCount: 1,
        retryCount: 0,
        inputMode: "compatibility",
      }),
    );
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

describe("ModelConnectivityTest native cancellation", () => {
  it("shows request bounds and waits for the native terminal after cancel acknowledgement", async () => {
    const user = userEvent.setup();
    let finish!: (result: ModelProbeResult) => void;
    let requestId = "";
    const onProbe = vi.fn((_model: string, id: string) => {
      requestId = id;
      return new Promise<ModelProbeResult>((resolve) => {
        finish = resolve;
      });
    });
    const onCancel = vi.fn(async (id: string) => ({
      requestId: id,
      phase: "cancelling" as const,
      requestCount: 1,
      retryCount: 0,
    }));
    renderProbe({ onProbe, onCancel });
    await user.click(screen.getByRole("button", { name: "测试连通" }));
    expect(screen.getByText(/最多 2 次/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "gpt-test" }));
    await user.click(screen.getByRole("button", { name: "开始测试" }));
    expect(screen.getByRole("button", { name: "关闭" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "取消测试" }));
    expect(onCancel).toHaveBeenCalledWith(requestId);
    expect(screen.getByRole("button", { name: "正在取消…" })).toBeDisabled();
    expect(screen.queryByText("模型测试已取消")).not.toBeInTheDocument();
    await act(async () =>
      finish({
        success: false,
        status: "failed",
        message: "cancelled",
        modelUsed: "gpt-test",
        responseTimeMs: null,
        httpStatus: null,
        errorCategory: null,
        requestId,
        terminal: "cancelled",
        requestCount: 1,
        retryCount: 0,
        inputMode: "compatibility",
      }),
    );
    expect(await screen.findByText("模型测试已取消")).toBeVisible();
    expect(screen.getByText(/已发起 1 次请求/)).toBeVisible();
    expect(screen.getByRole("button", { name: "关闭" })).toBeEnabled();
  });

  it("keeps running after a failed cancellation instead of fabricating cancellation", async () => {
    const user = userEvent.setup();
    let finish!: (result: ModelProbeResult) => void;
    let requestId = "";
    const onProbe = vi.fn((_modelId: string, id: string) => {
      requestId = id;
      return new Promise<ModelProbeResult>((resolve) => {
        finish = resolve;
      });
    });
    renderProbe({
      onProbe,
      onCancel: vi.fn(async () => {
        throw new Error("not registered");
      }),
    });
    await user.click(screen.getByRole("button", { name: "测试连通" }));
    await user.click(screen.getByRole("button", { name: "gpt-test" }));
    await user.click(screen.getByRole("button", { name: "开始测试" }));
    await user.click(screen.getByRole("button", { name: "取消测试" }));
    expect(await screen.findByText("取消尚未确认")).toBeVisible();
    expect(screen.getByRole("button", { name: "取消测试" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "关闭" })).toBeDisabled();
    await act(async () =>
      finish({
        success: true,
        status: "operational",
        message: "responded",
        modelUsed: "gpt-test",
        responseTimeMs: 1,
        httpStatus: 200,
        errorCategory: null,
        requestId,
        terminal: "completed",
        requestCount: 1,
        retryCount: 0,
        inputMode: "compatibility",
      }),
    );
    expect(await screen.findByText("连通测试成功")).toBeVisible();
  });

  it("does not show a late result from a previous draft", async () => {
    const user = userEvent.setup();
    let finish!: (result: ModelProbeResult) => void;
    let requestId = "";
    const onProbe = vi.fn((_modelId: string, id: string) => {
      requestId = id;
      return new Promise<ModelProbeResult>((resolve) => {
        finish = resolve;
      });
    });
    const element = (version: number) => (
      <TooltipProvider delayDuration={0}>
        <ModelConnectivityTest
          searchId="late-probe"
          modelIds={["gpt-test"]}
          resetVersion={version}
          onProbe={onProbe}
        />
      </TooltipProvider>
    );
    const view = render(element(1));
    await user.click(screen.getByRole("button", { name: "测试连通" }));
    await user.click(screen.getByRole("button", { name: "gpt-test" }));
    await user.click(screen.getByRole("button", { name: "开始测试" }));
    view.rerender(element(2));
    expect(screen.queryByText(/等待模型首块响应/)).not.toBeInTheDocument();
    await act(async () =>
      finish({
        success: false,
        status: "failed",
        message: "late old draft error",
        modelUsed: "gpt-test",
        responseTimeMs: 1,
        httpStatus: 400,
        errorCategory: null,
        requestId,
        terminal: "completed",
        requestCount: 1,
        retryCount: 0,
        inputMode: "compatibility",
      }),
    );
    expect(screen.queryByText(/late old draft error/)).not.toBeInTheDocument();
  });
});

function completedProbe(requestId: string): ModelProbeResult {
  return {
    success: true,
    status: "operational",
    message: "responded",
    modelUsed: "gpt-test",
    responseTimeMs: 1,
    httpStatus: 200,
    errorCategory: null,
    requestId,
    terminal: "completed",
    requestCount: 1,
    retryCount: 0,
    inputMode: "compatibility",
  };
}

async function selectProbeModel() {
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "测试连通" }));
  await user.click(screen.getByRole("button", { name: "gpt-test" }));
}

describe("ModelConnectivityTest observer lifetime", () => {
  it("admits one probe and one cancel in the same tick and ignores a late cancel acknowledgement", async () => {
    let finish!: (result: ModelProbeResult) => void;
    let acknowledge!: (snapshot: ModelProbeSnapshot) => void;
    let requestId = "";
    const onProbe = vi.fn((_model: string, id: string) => {
      requestId = id;
      return new Promise<ModelProbeResult>((resolve) => {
        finish = resolve;
      });
    });
    const onCancel = vi.fn(
      () =>
        new Promise<ModelProbeSnapshot>((resolve) => {
          acknowledge = resolve;
        }),
    );
    renderProbe({ onProbe, onCancel });
    await selectProbeModel();
    const start = screen.getByRole("button", { name: "开始测试" });
    act(() => {
      fireEvent.click(start);
      fireEvent.click(start);
    });
    expect(onProbe).toHaveBeenCalledTimes(1);
    expect(requestId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u,
    );
    const cancel = screen.getByRole("button", { name: "取消测试" });
    act(() => {
      fireEvent.click(cancel);
      fireEvent.click(cancel);
    });
    expect(onCancel).toHaveBeenCalledTimes(1);
    await act(async () => finish(completedProbe(requestId)));
    await act(async () =>
      acknowledge({
        requestId,
        phase: "cancelling",
        requestCount: 1,
        retryCount: 0,
      }),
    );
    expect(screen.getByText("连通测试成功")).toBeVisible();
    expect(screen.queryByText(/等待后台终态/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "关闭" })).toBeEnabled();
  });

  it("pauses hidden polling but accepts the original terminal promise", async () => {
    let finish!: (result: ModelProbeResult) => void;
    let requestId = "";
    const onProbe = vi.fn((_model: string, id: string) => {
      requestId = id;
      return new Promise<ModelProbeResult>((resolve) => {
        finish = resolve;
      });
    });
    const onStatus = vi.fn(
      async (id: string): Promise<ModelProbeSnapshot> => ({
        requestId: id,
        phase: "running",
        requestCount: 1,
        retryCount: 0,
      }),
    );
    const element = (active: boolean) => (
      <TooltipProvider delayDuration={0}>
        <PersistentSurface active={active}>
          <ModelConnectivityTest
            searchId="hidden-probe"
            modelIds={["gpt-test"]}
            onProbe={onProbe}
            onStatus={onStatus}
          />
        </PersistentSurface>
      </TooltipProvider>
    );
    const view = render(element(true));
    await selectProbeModel();
    vi.useFakeTimers();
    try {
      act(() =>
        fireEvent.click(screen.getByRole("button", { name: "开始测试" })),
      );
      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });
      expect(onStatus).toHaveBeenCalledTimes(1);
      view.rerender(element(false));
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1000);
      });
      expect(onStatus).toHaveBeenCalledTimes(1);
      await act(async () => finish(completedProbe(requestId)));
      view.rerender(element(true));
      expect(screen.getByText("连通测试成功")).toBeVisible();
      expect(screen.getByRole("button", { name: "关闭" })).toBeEnabled();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1000);
      });
      expect(onStatus).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it("cancels with the latest callback on unmount and ignores a late result", async () => {
    let finish!: (result: ModelProbeResult) => void;
    let requestId = "";
    const onProbe = vi.fn((_model: string, id: string) => {
      requestId = id;
      return new Promise<ModelProbeResult>((resolve) => {
        finish = resolve;
      });
    });
    const snapshot = async (id: string): Promise<ModelProbeSnapshot> => ({
      requestId: id,
      phase: "cancelling",
      requestCount: 1,
      retryCount: 0,
    });
    const originalCancel = vi.fn(snapshot);
    const latestCancel = vi.fn(snapshot);
    const onBusyChange = vi.fn();
    const element = (onCancel: typeof originalCancel) => (
      <TooltipProvider delayDuration={0}>
        <ModelConnectivityTest
          searchId="unmount-probe"
          modelIds={["gpt-test"]}
          onProbe={onProbe}
          onCancel={onCancel}
          onBusyChange={onBusyChange}
        />
      </TooltipProvider>
    );
    const view = render(element(originalCancel));
    await selectProbeModel();
    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "开始测试" }));
    view.rerender(element(latestCancel));
    view.unmount();
    expect(originalCancel).not.toHaveBeenCalled();
    expect(latestCancel).toHaveBeenCalledExactlyOnceWith(requestId);
    await act(async () => finish(completedProbe(requestId)));
    expect(onBusyChange.mock.calls).toEqual([[true]]);
    expect(screen.queryByText("连通测试成功")).not.toBeInTheDocument();
  });

  it("rejects a terminal result from a different request", async () => {
    const onProbe = vi.fn(
      async (): Promise<ModelProbeResult> =>
        completedProbe("00000000-0000-4000-8000-000000000000"),
    );
    renderProbe({ onProbe });
    await selectProbeModel();
    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "开始测试" }));
    expect(await screen.findByText("连通测试失败")).toBeVisible();
    expect(screen.getByText(/结果标识不匹配/)).toBeVisible();
    expect(screen.queryByText("连通测试成功")).not.toBeInTheDocument();
  });
});
