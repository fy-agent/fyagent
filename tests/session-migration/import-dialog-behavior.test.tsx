import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { ImportPackageDialog } from "@/pages/sessions/components/ImportPackageDialog";
import {
  migratableSessionSchema,
  sessionPackageSchema,
  type LocalProviderProbe,
  type ReadSessionPackageResult,
  type RestoreAttempt,
  type RestoreRequest,
  type RestoreStage,
} from "@/shared/features/session-migration";
import { restoreAttemptSample, sessionPackageSample } from "./samples";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function multiProviderPackage(
  firstContentDigest = `fyc1:${"a".repeat(64)}`,
): ReadSessionPackageResult {
  const raw = structuredClone(sessionPackageSample());
  const first = (raw.sessions as Array<Record<string, unknown>>)[0];
  first.title = "Codex 会话";
  first.contentDigest = firstContentDigest;

  const second = structuredClone(first);
  second.snapshotId = `fys1:${"d".repeat(64)}`;
  second.contentDigest = `fyc1:${"e".repeat(64)}`;
  second.title = "Grok 会话";
  second.origin = {
    ...(second.origin as Record<string, unknown>),
    originId: "origin-grokbuild",
    providerId: "grokbuild",
    sessionId: "grok-session",
  };
  raw.sessions = [first, second];

  return {
    package: sessionPackageSchema.parse(raw),
    attempts: [],
  };
}

function probe(
  providerId: string,
  writeSupported: boolean,
): LocalProviderProbe {
  return {
    providerId,
    installed: true,
    detectedVersion: providerId === "codex" ? "0.154.0" : "1.0.34",
    extractionSupported: true,
    writeSupported,
    reasonCode: writeSupported ? undefined : "providerVersionUnsupported",
  };
}

function attempt(stage: RestoreStage): RestoreAttempt {
  return restoreAttemptSample({
    stage,
    lastError:
      stage === "failed"
        ? { code: "database_locked", detail: { path: "/tmp/provider.db" } }
        : undefined,
  }) as unknown as RestoreAttempt;
}

function renderDialog(
  overrides: Partial<{
    onOpenChange: (open: boolean) => void;
    onReadPackage: (path: string) => Promise<ReadSessionPackageResult>;
    onRestore: import("vitest").Mock<
      (request: RestoreRequest) => Promise<RestoreAttempt[]>
    >;
    onPickPackageFile: () => Promise<string | null>;
    onPickDirectory: () => Promise<string | null>;
    localProbes: Record<string, LocalProviderProbe>;
  }> = {},
) {
  const props = {
    open: true,
    onOpenChange: vi.fn(),
    onReadPackage: vi.fn(async () => multiProviderPackage()),
    onRestore: vi.fn(async () => [attempt("nativeWritten")]),
    onPickPackageFile: vi.fn(async () => "/tmp/multi-provider.json"),
    onPickDirectory: vi.fn(async () => "/tmp/target-workspace"),
    localProbes: {
      codex: probe("codex", true),
      grokbuild: probe("grokbuild", false),
    },
    initialTargetWorkspace: "/tmp/target-workspace",
    originRef: undefined,
    ...overrides,
  };
  return { ...render(<ImportPackageDialog {...props} />), props };
}

async function readPackage(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "选择文件" }));
  await user.click(screen.getByRole("button", { name: "解析会话包" }));
  await screen.findByText("会话包解析成功", { exact: true });
}

describe("session package import behavior", () => {
  it("filters snapshots by provider and disables an unsupported local target", async () => {
    const user = userEvent.setup();
    const { props } = renderDialog();
    await readPackage(user);

    const restore = screen.getByRole("button", {
      name: "确认恢复至目标软件",
    });
    expect(restore).toBeEnabled();

    await user.selectOptions(
      screen.getByLabelText("恢复至目标软件（与包内来源一一对应）："),
      "grokbuild",
    );
    expect(restore).toBeDisabled();
    expect(
      screen.getByText("当前客户端版本尚未支持恢复迁移", { exact: true }),
    ).toBeVisible();
    expect(screen.getByText("Grok 会话", { exact: false })).toBeVisible();
    expect(screen.queryByText("Codex 会话", { exact: false })).toBeNull();
    expect(screen.getByText(/2 条消息/u)).toBeVisible();
    expect(screen.queryByText(/2 轮问答/u)).toBeNull();
    expect(props.onRestore).not.toHaveBeenCalled();

    await user.selectOptions(
      screen.getByLabelText("恢复至目标软件（与包内来源一一对应）："),
      "codex",
    );
    await user.click(restore);
    await waitFor(() => expect(props.onRestore).toHaveBeenCalledTimes(1));
    expect(props.onRestore).toHaveBeenCalledWith(
      expect.objectContaining({
        targetProviderId: "codex",
        snapshotIds: [`fys1:${"b".repeat(64)}`],
      }),
    );
  });

  it("keeps restore disabled when no local probe result exists", async () => {
    const user = userEvent.setup();
    renderDialog({ localProbes: {} });
    await readPackage(user);

    expect(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    ).toBeDisabled();
    expect(
      screen.getByText("尚未探测到该客户端的安装状态", { exact: true }),
    ).toBeVisible();
  });

  it("reuses one request id for retry and changes it only after binding changes", async () => {
    const user = userEvent.setup();
    const onRestore = vi.fn(async () => {
      throw new Error("请求超时，请确认目标状态后重试");
    });
    const { props } = renderDialog({ onRestore });
    await readPackage(user);

    const restore = screen.getByRole("button", {
      name: "确认恢复至目标软件",
    });
    await user.click(restore);
    await screen.findByText("请求超时，请确认目标状态后重试");
    await user.click(restore);
    await waitFor(() => expect(props.onRestore).toHaveBeenCalledTimes(2));

    const firstRequest = props.onRestore.mock.calls[0][0];
    const retryRequest = props.onRestore.mock.calls[1][0];
    expect(retryRequest.requestId).toBe(firstRequest.requestId);

    await user.click(screen.getByRole("radio", { name: /另存为新副本/u }));
    await user.click(restore);
    await waitFor(() => expect(props.onRestore).toHaveBeenCalledTimes(3));
    expect(props.onRestore.mock.calls[2][0].requestId).not.toBe(
      firstRequest.requestId,
    );
  });

  it("coalesces repeated clicks while one restore request is pending", async () => {
    const user = userEvent.setup();
    const pending = deferred<RestoreAttempt[]>();
    const onRestore = vi.fn(() => pending.promise);
    const { props } = renderDialog({ onRestore });
    await readPackage(user);

    const restore = screen.getByRole("button", {
      name: "确认恢复至目标软件",
    });
    await user.dblClick(restore);
    expect(props.onRestore).toHaveBeenCalledTimes(1);
    expect(restore).toBeDisabled();
    expect(screen.getByRole("button", { name: "上一步" })).toBeDisabled();
    await user.keyboard("{Escape}");
    expect(props.onOpenChange).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeVisible();

    pending.resolve([attempt("nativeWritten")]);
    await screen.findByText("已写入，待读回验证", { exact: true });
  });

  it.each([
    ["needsReconciliation", "写入结果尚未确认"],
    ["ambiguous", "恢复操作待确认"],
    ["failed", "恢复未完全成功"],
  ] as const)(
    "does not present %s as a green success",
    async (stage, expectedHeading) => {
      const user = userEvent.setup();
      renderDialog({
        onRestore: vi.fn(async () => [attempt(stage)]),
      });
      await readPackage(user);
      await user.click(
        screen.getByRole("button", { name: "确认恢复至目标软件" }),
      );

      expect(
        await screen.findByText(expectedHeading, {
          exact: true,
          selector: "strong",
        }),
      ).toBeVisible();
      expect(
        screen.queryByText("恢复执行完成", { exact: true }),
      ).not.toBeInTheDocument();
      if (stage === "failed") {
        expect(screen.getByText(/目标数据库正在被占用/u)).toBeInTheDocument();
        expect(screen.queryByText(/"database_locked"/u)).toBeNull();
      }
    },
  );

  it("renders a JSON-string native error as a readable message", async () => {
    const user = userEvent.setup();
    renderDialog({
      onRestore: vi.fn(async () => {
        throw new Error(
          JSON.stringify({
            code: "database_locked",
            detail: { path: "/tmp/provider.db" },
          }),
        );
      }),
    });
    await readPackage(user);
    await user.click(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    );

    const alert = await screen.findByRole("alert");
    expect(
      within(alert).getByText("目标数据库正在被占用，请先关闭目标客户端", {
        exact: true,
      }),
    ).toBeVisible();
    expect(alert).not.toHaveTextContent("database_locked");
    expect(alert).not.toHaveTextContent('{"code"');
  });

  it("rejects an empty restore receipt list instead of showing success", async () => {
    const user = userEvent.setup();
    renderDialog({ onRestore: vi.fn(async () => []) });
    await readPackage(user);
    await user.click(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    );

    expect(
      await screen.findByText(/未收到.*回执|没有.*恢复记录/u),
    ).toBeVisible();
    expect(
      screen.queryByText("恢复执行完成", { exact: true }),
    ).not.toBeInTheDocument();
  });

  it("keeps the request id when only an unselected snapshot body changes", async () => {
    const user = userEvent.setup();
    const onRestore = vi.fn(async () => {
      throw new Error("请求超时");
    });
    const onReadPackage = vi
      .fn<(path: string) => Promise<ReadSessionPackageResult>>()
      .mockResolvedValueOnce(multiProviderPackage(`fyc1:${"a".repeat(64)}`))
      .mockResolvedValueOnce(multiProviderPackage(`fyc1:${"f".repeat(64)}`));
    const { props } = renderDialog({
      onReadPackage,
      onRestore,
      localProbes: {
        codex: probe("codex", true),
        grokbuild: probe("grokbuild", true),
      },
    });
    await readPackage(user);
    await user.selectOptions(
      screen.getByLabelText("恢复至目标软件（与包内来源一一对应）："),
      "grokbuild",
    );
    await user.click(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    );
    await screen.findByText("请求超时", { exact: true });
    const firstRequestId = props.onRestore.mock.calls[0][0].requestId;

    await user.click(screen.getByRole("button", { name: "上一步" }));
    await user.click(screen.getByRole("button", { name: "解析会话包" }));
    await screen.findByText("会话包解析成功", { exact: true });
    await user.selectOptions(
      screen.getByLabelText("恢复至目标软件（与包内来源一一对应）："),
      "grokbuild",
    );
    await user.click(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    );
    await waitFor(() => expect(props.onRestore).toHaveBeenCalledTimes(2));

    expect(props.onRestore.mock.calls[1][0].requestId).toBe(firstRequestId);
    expect(props.onRestore.mock.calls[1][0].snapshotIds).toEqual([
      `fys1:${"d".repeat(64)}`,
    ]);
  });

  it("ignores an old package read after close and reopen", async () => {
    const user = userEvent.setup();
    const stale = deferred<ReadSessionPackageResult>();
    const current = deferred<ReadSessionPackageResult>();
    const onReadPackage = vi.fn((path: string) =>
      path.includes("stale") ? stale.promise : current.promise,
    );
    const pickPaths = ["/tmp/stale.json", "/tmp/current.json"];

    function Harness() {
      const [open, setOpen] = useState(true);
      const [sessionKey, setSessionKey] = useState(0);
      return (
        <>
          <button
            type="button"
            onClick={() => {
              setSessionKey((current) => current + 1);
              setOpen(true);
            }}
          >
            重新打开导入
          </button>
          {open && (
            <ImportPackageDialog
              key={sessionKey}
              open
              onOpenChange={setOpen}
              onReadPackage={onReadPackage}
              onRestore={vi.fn(async () => [attempt("nativeWritten")])}
              onPickPackageFile={vi.fn(async () => pickPaths.shift() ?? null)}
              onPickDirectory={vi.fn(async () => "/tmp/workspace")}
              localProbes={{ codex: probe("codex", true) }}
              initialTargetWorkspace="/tmp/workspace"
              originRef={undefined}
            />
          )}
        </>
      );
    }

    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "选择文件" }));
    await user.click(screen.getByRole("button", { name: "解析会话包" }));
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());

    await user.click(screen.getByRole("button", { name: "重新打开导入" }));
    await user.click(screen.getByRole("button", { name: "选择文件" }));
    await user.click(screen.getByRole("button", { name: "解析会话包" }));
    current.resolve(multiProviderPackage());
    await screen.findByText("会话包解析成功", { exact: true });

    const staleResult = multiProviderPackage();
    staleResult.package.sessions = [
      migratableSessionSchema.parse({
        ...staleResult.package.sessions[0],
        title: "过期异步结果",
      }),
    ];
    stale.resolve(staleResult);

    await waitFor(() =>
      expect(
        screen.queryByText("过期异步结果", { exact: false }),
      ).not.toBeInTheDocument(),
    );
    expect(screen.getByText("Codex 会话", { exact: false })).toBeVisible();
  });
});
