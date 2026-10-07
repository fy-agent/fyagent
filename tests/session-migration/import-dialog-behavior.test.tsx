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
    onReviewRestore: () => Promise<RestoreAttempt[]>;
    onVerifyReadback: (attemptId: string) => Promise<RestoreAttempt>;
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

  it.each(["extractionRuleUnavailable", "extractionRuleVersionMismatch"])(
    "restores a validated package with a verified writer despite %s",
    async (reasonCode) => {
      const user = userEvent.setup();
      const { props } = renderDialog({
        localProbes: {
          codex: {
            ...probe("codex", true),
            extractionSupported: false,
            reasonCode,
          },
        },
      });
      await readPackage(user);
      const restore = screen.getByRole("button", {
        name: "确认恢复至目标软件",
      });
      expect(restore).toBeEnabled();
      await user.click(restore);
      await waitFor(() => expect(props.onRestore).toHaveBeenCalledTimes(1));
      expect(props.onRestore).toHaveBeenCalledWith(
        expect.objectContaining({
          targetProviderId: "codex",
          snapshotIds: [`fys1:${"b".repeat(64)}`],
        }),
      );
    },
  );

  it("keeps an unverified writer disabled even when extraction is supported", async () => {
    const user = userEvent.setup();
    const { props } = renderDialog({
      localProbes: {
        codex: { ...probe("codex", false), detectedVersion: "unknown" },
      },
    });
    await readPackage(user);
    expect(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    ).toBeDisabled();
    expect(props.onRestore).not.toHaveBeenCalled();
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

  it("blocks blind retries after a missing restore response, including changed copy parameters", async () => {
    const user = userEvent.setup();
    const onRestore = vi.fn(async () => {
      throw new Error("SECRET /private/provider.db");
    });
    const onReviewRestore = vi.fn(async () => []);
    const { props } = renderDialog({ onRestore, onReviewRestore });
    await readPackage(user);
    const restore = screen.getByRole("button", { name: "确认恢复至目标软件" });
    await user.click(restore);
    await screen.findByText("目标恢复调用 · 未收到完整结果");
    expect(restore).toBeDisabled();
    expect(screen.getByRole("button", { name: "上一步" })).toBeDisabled();
    expect(screen.getByLabelText("绑定目标机器工作区目录：")).toHaveValue(
      "/tmp/target-workspace",
    );
    await user.click(screen.getByRole("radio", { name: /另存为新副本/u }));
    await user.click(restore);
    await user.click(screen.getByRole("button", { name: "核对恢复回执" }));
    await waitFor(() => expect(onReviewRestore).toHaveBeenCalledTimes(1));
    expect(restore).toBeDisabled();
    expect(props.onRestore).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(/SECRET|private\/provider/u)).toBeNull();
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

  it("retries only the same complete request after receipts prove no side effect", async () => {
    const user = userEvent.setup();
    let count = 0;
    const onRestore = vi.fn(async (request: RestoreRequest) => [
      {
        ...attempt(count++ === 0 ? "failed" : "nativeWritten"),
        requestId: request.requestId,
        snapshotId: request.snapshotIds[0],
      },
    ]);
    renderDialog({ onRestore });
    await readPackage(user);
    await user.click(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    );
    await screen.findByText("本机权威回执已确认该会话没有写入副作用。");
    expect(screen.getByRole("button", { name: "重新配置" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "重试同一恢复" }));
    await screen.findByText("已写入，待读回验证", { exact: true });
    expect(onRestore.mock.calls[1][0]).toEqual(onRestore.mock.calls[0][0]);
  });

  it("resolves a missing response from the matching receipt instead of writing again", async () => {
    const user = userEvent.setup();
    const onRestore = vi.fn(
      async (_value: RestoreRequest): Promise<RestoreAttempt[]> => {
        throw new Error("timeout");
      },
    );
    const onReviewRestore = vi.fn(async () => {
      const request = onRestore.mock.calls[0][0];
      return [
        {
          ...attempt("nativeWritten"),
          requestId: request.requestId,
          snapshotId: request.snapshotIds[0],
        },
      ];
    });
    const onVerifyReadback = vi.fn(async () => {
      throw { code: "nativeProtocolFailed", detail: { reason: "SECRET" } };
    });
    renderDialog({ onRestore, onReviewRestore, onVerifyReadback });
    await readPackage(user);
    await user.click(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    );
    await screen.findByText("目标恢复调用 · 未收到完整结果");
    await user.click(screen.getByRole("button", { name: "核对恢复回执" }));
    await screen.findByText("已写入，待读回验证", { exact: true });
    await user.click(screen.getByRole("button", { name: "系统读回核验" }));
    await screen.findByText("目标读回调用失败");
    expect(
      screen.getAllByText("已写入目标会话，完整历史尚未通过目标读回。").length,
    ).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "重新配置" })).toBeDisabled();
    expect(onRestore).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(/SECRET/u)).toBeNull();
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

  it("preserves an unconfirmed request when the controlled dialog closes and reopens", async () => {
    const user = userEvent.setup();
    const onRestore = vi.fn(async () => {
      throw new Error("timeout");
    });
    const props = {
      onReadPackage: vi.fn(async () => multiProviderPackage()),
      onRestore,
      onPickPackageFile: vi.fn(async () => "/tmp/package.json"),
      onPickDirectory: vi.fn(async () => "/tmp/target-workspace"),
      localProbes: { codex: probe("codex", true) },
      initialTargetWorkspace: "/tmp/target-workspace",
      originRef: undefined,
    };
    function Harness() {
      const [open, setOpen] = useState(true);
      const [workspace, setWorkspace] = useState(props.initialTargetWorkspace);
      return (
        <>
          <button
            onClick={() => {
              setWorkspace("/tmp/other-session");
              setOpen(true);
            }}
          >
            重新打开
          </button>
          <ImportPackageDialog
            {...props}
            initialTargetWorkspace={workspace}
            open={open}
            onOpenChange={setOpen}
          />
        </>
      );
    }
    render(<Harness />);
    await readPackage(user);
    await user.click(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    );
    await screen.findByText("目标恢复调用 · 未收到完整结果");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await user.click(screen.getByRole("button", { name: "重新打开" }));
    await screen.findByRole("dialog");
    expect(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    ).toBeDisabled();
    expect(screen.getByLabelText("绑定目标机器工作区目录：")).toHaveValue(
      "/tmp/target-workspace",
    );
    expect(onRestore).toHaveBeenCalledTimes(1);
  });

  it("starts each fresh controlled opening from the current workspace and default copy policy", async () => {
    const user = userEvent.setup();
    const { props, rerender } = renderDialog();
    await readPackage(user);
    await user.click(screen.getByRole("radio", { name: /另存为新副本/u }));
    await user.clear(screen.getByLabelText("绑定目标机器工作区目录："));
    await user.type(
      screen.getByLabelText("绑定目标机器工作区目录："),
      "/tmp/discarded-draft",
    );
    await user.keyboard("{Escape}");
    expect(props.onOpenChange).toHaveBeenCalledWith(false);
    rerender(<ImportPackageDialog {...props} open={false} />);
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    rerender(
      <ImportPackageDialog
        {...props}
        initialTargetWorkspace="/tmp/current-session"
      />,
    );
    await readPackage(user);
    expect(screen.getByLabelText("绑定目标机器工作区目录：")).toHaveValue(
      "/tmp/current-session",
    );
    expect(screen.getByRole("radio", { name: /默认策略/u })).toBeChecked();
    expect(props.onRestore).not.toHaveBeenCalled();
  });

  it.each(["missingSnapshot", "foreignRequest", "differentKind"] as const)(
    "keeps the original request locked after %s receipts",
    async (mismatch) => {
      const user = userEvent.setup();
      const readResult = multiProviderPackage();
      const copy = structuredClone(readResult.package.sessions[0]);
      copy.snapshotId = `fys1:${"c".repeat(64)}`;
      readResult.package.sessions.push(copy);
      const onRestore = vi.fn(
        async (_request: RestoreRequest): Promise<RestoreAttempt[]> => {
          throw new Error("timeout");
        },
      );
      const onReviewRestore = vi.fn(async () => {
        const request = onRestore.mock.calls[0][0];
        const selected =
          mismatch === "missingSnapshot"
            ? request.snapshotIds.slice(0, 1)
            : request.snapshotIds;
        return selected.map((snapshotId, index) => ({
          ...attempt("failed"),
          attemptId: `receipt-${index}`,
          snapshotId,
          requestId:
            mismatch === "foreignRequest"
              ? "foreign-request"
              : request.requestId,
          requestKind:
            mismatch === "differentKind"
              ? ("saveAsNewCopy" as const)
              : request.requestKind,
        }));
      });
      renderDialog({
        onReadPackage: vi.fn(async () => readResult),
        onRestore,
        onReviewRestore,
      });
      await readPackage(user);
      await user.click(
        screen.getByRole("button", { name: "确认恢复至目标软件" }),
      );
      await screen.findByText("目标恢复调用 · 未收到完整结果");
      await user.click(screen.getByRole("button", { name: "核对恢复回执" }));
      await waitFor(() => expect(onReviewRestore).toHaveBeenCalledTimes(1));
      await waitFor(() =>
        expect(screen.queryByRole("button", { name: "正在核对…" })).toBeNull(),
      );
      expect(screen.queryByRole("button", { name: "重试同一恢复" })).toBeNull();
      const reconfigure = screen.queryByRole("button", { name: "重新配置" });
      if (reconfigure) expect(reconfigure).toBeDisabled();
      else
        expect(
          screen.getByRole("button", { name: "确认恢复至目标软件" }),
        ).toBeDisabled();
      expect(onRestore).toHaveBeenCalledTimes(1);
    },
  );

  it.each(["sourceUnreadable", "packageMalformed", "packageUnknownField"])(
    "keeps the path and returns focus for %s without a target write",
    async (code) => {
      const user = userEvent.setup();
      const onReadPackage = vi
        .fn<(path: string) => Promise<ReadSessionPackageResult>>()
        .mockRejectedValueOnce({
          code,
          detail: { reason: "SECRET", path: "/private/source" },
        })
        .mockResolvedValueOnce(multiProviderPackage());
      const { props } = renderDialog({ onReadPackage });
      await user.type(
        screen.getByLabelText("迁移包文件 (.json)："),
        "/tmp/broken.json",
      );
      await user.click(screen.getByRole("button", { name: "解析会话包" }));
      const alert = await screen.findByRole("alert");
      expect(alert).toHaveTextContent("未调用目标恢复，未写入目标会话");
      expect(alert).not.toHaveTextContent("SECRET");
      await user.click(screen.getByRole("button", { name: "返回编辑位置" }));
      const input = screen.getByLabelText("迁移包文件 (.json)：");
      expect(input).toHaveFocus();
      expect(input).toHaveValue("/tmp/broken.json");
      await user.clear(input);
      await user.type(input, "/tmp/fixed.json");
      await user.click(screen.getByRole("button", { name: "解析会话包" }));
      await screen.findByText("会话包解析成功", { exact: true });
      expect(onReadPackage).toHaveBeenLastCalledWith("/tmp/fixed.json");
      expect(props.onRestore).not.toHaveBeenCalled();
    },
  );

  it("keeps every partial result visible and refuses batch reconfiguration", async () => {
    const user = userEvent.setup();
    const readResult = multiProviderPackage();
    const copy = structuredClone(readResult.package.sessions[0]);
    copy.snapshotId = `fys1:${"c".repeat(64)}`;
    readResult.package.sessions.push(copy);
    renderDialog({
      onReadPackage: vi.fn(async () => readResult),
      onRestore: vi.fn(async () => [
        attempt("nativeReadbackVerified"),
        {
          ...attempt("needsReconciliation"),
          attemptId: "partial-2",
          snapshotId: copy.snapshotId,
        },
      ]),
    });
    await readPackage(user);
    await user.click(
      screen.getByRole("button", { name: "确认恢复至目标软件" }),
    );
    await screen.findByText("写入结果尚未确认", {
      exact: true,
      selector: "strong",
    });
    expect(
      screen.getByText("已写入，目标完整历史已通过系统读回。"),
    ).toBeVisible();
    expect(screen.getByText(/目标会话是否完整写入尚未确认/u)).toBeVisible();
    expect(screen.getByRole("button", { name: "重新配置" })).toBeDisabled();
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
      return (
        <>
          <button type="button" onClick={() => setOpen(true)}>
            重新打开导入
          </button>
          <ImportPackageDialog
            open={open}
            onOpenChange={setOpen}
            onReadPackage={onReadPackage}
            onRestore={vi.fn(async () => [attempt("nativeWritten")])}
            onPickPackageFile={vi.fn(async () => pickPaths.shift() ?? null)}
            onPickDirectory={vi.fn(async () => "/tmp/workspace")}
            localProbes={{ codex: probe("codex", true) }}
            initialTargetWorkspace="/tmp/workspace"
            originRef={undefined}
          />
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
