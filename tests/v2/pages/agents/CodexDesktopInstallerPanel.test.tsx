import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { StrictMode } from "react";
import { describe, expect, it, vi } from "vitest";

import type {
  InstallerErrorDto,
  JobSnapshot,
  JobStage,
  LocalInstallStatus,
  ProgressPhase,
  RemoteReleaseStatus,
} from "@/shared/codex-desktop";
import { CodexDesktopInstallerPanel } from "@/v2/shared/codex-desktop/CodexDesktopInstallerPanel";
import type {
  CodexDesktopPort,
  FeaturePorts,
} from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";

const release: RemoteReleaseStatus = {
  releaseId: `v1:${"b".repeat(64)}`,
  displayVersion: "1.2.3.4",
  platformVersion: {
    kind: "windows_msix",
    major: 1,
    minor: 2,
    build: 3,
    revision: 4,
  },
  downloadSizeHint: 4096,
  checkedAt: "2026-08-14T00:00:00.000Z",
};

const notInstalled: LocalInstallStatus = {
  state: "not_installed",
  platform: "windows",
  architecture: "x86_64",
};

function snapshot(options: {
  sequence: number;
  stage: JobStage;
  updatedAt?: string;
  phase?: ProgressPhase;
  completed?: number | null;
  total?: number | null;
  percent?: number | null;
  error?: InstallerErrorDto | null;
  cancellable?: boolean;
}): JobSnapshot {
  return {
    jobId: "11111111-1111-4111-8111-111111111111",
    sequence: options.sequence,
    stage: options.stage,
    release,
    startedAt: "2026-08-14T00:00:00.000Z",
    updatedAt: options.updatedAt ?? "2026-08-14T00:00:01.000Z",
    progress: options.phase
      ? {
          phase: options.phase,
          completedBytes: options.completed ?? null,
          totalBytes: options.total ?? null,
          percent: options.percent ?? null,
        }
      : null,
    cancellable: options.cancellable ?? false,
    result: null,
    error: options.error ?? null,
  };
}

function createInstallerPort(
  overrides: Partial<CodexDesktopPort> = {},
): CodexDesktopPort {
  return {
    getLocalStatus: vi.fn(async () => notInstalled),
    checkLatest: vi.fn(async () => release),
    getJob: vi.fn(async () => null),
    startInstall: vi.fn(async () =>
      snapshot({ sequence: 1, stage: "checking" }),
    ),
    cancelInstall: vi.fn(async () =>
      snapshot({ sequence: 2, stage: "cancelled" }),
    ),
    launch: vi.fn(async () => undefined),
    openLogDirectory: vi.fn(async () => undefined),
    subscribeJobUpdates: vi.fn(async () => () => undefined),
    ...overrides,
  };
}

function renderPanel(port: CodexDesktopPort, strict = false) {
  const ports: FeaturePorts = createBrowserFeaturePorts();
  ports.codexDesktop = port;
  const panel = (
    <FeatureProvider ports={ports}>
      <CodexDesktopInstallerPanel />
    </FeatureProvider>
  );
  return render(strict ? <StrictMode>{panel}</StrictMode> : panel);
}

describe("V2 Codex Desktop installer panel", () => {
  it("starts once under repeat clicks and passes only the release id", async () => {
    let resolveStart!: (value: JobSnapshot) => void;
    const pendingStart = new Promise<JobSnapshot>((resolve) => {
      resolveStart = resolve;
    });
    const port = createInstallerPort({
      startInstall: vi.fn(() => pendingStart),
    });
    renderPanel(port);

    const install = await screen.findByRole("button", {
      name: "安装 Codex Desktop",
    });
    fireEvent.click(install);
    fireEvent.click(install);

    expect(port.startInstall).toHaveBeenCalledTimes(1);
    expect(port.startInstall).toHaveBeenCalledWith(release.releaseId);
    expect(port.startInstall).not.toHaveBeenCalledWith(
      expect.objectContaining({ url: expect.anything() }),
    );

    await act(async () => {
      resolveStart(snapshot({ sequence: 1, stage: "checking" }));
      await pendingStart;
    });
  });

  it("rejects stale snapshots and labels bytes and speed only while downloading", async () => {
    let listener: ((value: JobSnapshot) => void) | undefined;
    const port = createInstallerPort({
      subscribeJobUpdates: vi.fn(async (next) => {
        listener = next;
        return () => undefined;
      }),
    });
    renderPanel(port);
    await waitFor(() => expect(listener).toBeDefined());

    act(() => {
      listener?.(
        snapshot({
          sequence: 1,
          stage: "downloading",
          phase: "download",
          completed: 1024,
          total: 4096,
          percent: 25,
          updatedAt: "2026-08-14T00:00:01.000Z",
          cancellable: true,
        }),
      );
      listener?.(
        snapshot({
          sequence: 2,
          stage: "downloading",
          phase: "download",
          completed: 2048,
          total: 4096,
          percent: 50,
          updatedAt: "2026-08-14T00:00:02.000Z",
          cancellable: true,
        }),
      );
    });

    expect(screen.getByText(/已下载 2\.00 KB \/ 4\.00 KB/)).toHaveTextContent(
      "1.00 KB/s",
    );
    expect(screen.getByRole("progressbar")).toHaveAttribute(
      "aria-valuenow",
      "50",
    );

    act(() => {
      listener?.(
        snapshot({
          sequence: 1,
          stage: "downloading",
          phase: "download",
          completed: 4096,
          total: 4096,
          percent: 99,
          updatedAt: "2026-08-14T00:00:03.000Z",
        }),
      );
    });
    expect(screen.getByRole("progressbar")).toHaveAttribute(
      "aria-valuenow",
      "50",
    );

    act(() => {
      listener?.(
        snapshot({
          sequence: 3,
          stage: "installing",
          phase: "installation",
          completed: 50,
          total: 100,
          percent: 50,
          updatedAt: "2026-08-14T00:00:03.000Z",
        }),
      );
    });
    expect(screen.getByRole("progressbar")).toHaveAttribute(
      "aria-label",
      "安装进度",
    );
    expect(document.body).not.toHaveTextContent("已下载");
    expect(document.body).not.toHaveTextContent("/s");
  });

  it("explains the post-download wait and only offers logs after failure", async () => {
    let listener: ((value: JobSnapshot) => void) | undefined;
    const port = createInstallerPort({
      subscribeJobUpdates: vi.fn(async (next) => {
        listener = next;
        return () => undefined;
      }),
    });
    renderPanel(port);
    await waitFor(() => expect(listener).toBeDefined());

    act(() => {
      listener?.(
        snapshot({
          sequence: 1,
          stage: "downloading",
          phase: "download",
          completed: 1024,
          total: 4096,
          percent: 25,
          updatedAt: "2026-08-14T00:00:01.000Z",
          cancellable: true,
        }),
      );
      listener?.(
        snapshot({
          sequence: 2,
          stage: "downloading",
          phase: "download",
          completed: 2048,
          total: 4096,
          percent: 50,
          updatedAt: "2026-08-14T00:00:02.000Z",
          cancellable: true,
        }),
      );
    });

    expect(screen.getByText("正在下载 Codex Desktop。")).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "打开日志目录" }),
    ).not.toBeInTheDocument();

    act(() => {
      listener?.(
        snapshot({
          sequence: 3,
          stage: "downloading",
          phase: "download",
          completed: 4096,
          total: 4096,
          percent: 100,
          updatedAt: "2026-08-14T00:00:03.000Z",
          cancellable: true,
        }),
      );
    });

    expect(screen.getByText("下载已完成，正在校验并准备安装。")).toBeVisible();
    expect(
      screen.getByRole("status", {
        name: "正在校验并准备安装 Codex Desktop",
      }),
    ).toBeVisible();
    expect(screen.getByText(/已下载 4\.00 KB \/ 4\.00 KB/)).toHaveTextContent(
      "文件较大，校验可能需要一点时间。",
    );
    expect(document.body).not.toHaveTextContent("/s");
    expect(
      screen.queryByRole("button", { name: "打开日志目录" }),
    ).not.toBeInTheDocument();

    act(() => {
      listener?.(
        snapshot({
          sequence: 4,
          stage: "failed",
          error: {
            code: "DOWNLOAD_FAILED",
            stage: "failed",
            messageKey: "secret.backend.message.must.not.render",
            retryable: true,
            suggestedAction: "retry",
            details: {
              endpointKind: "artifact",
              attempt: 1,
              maxAttempts: 3,
              httpStatus: null,
              platformErrorCode: null,
              redactedMessage: "sensitive path C:/Users/private",
              context: {},
            },
          },
        }),
      );
    });

    expect(screen.getByRole("button", { name: "打开日志目录" })).toBeVisible();
    expect(document.body).not.toHaveTextContent("C:/Users/private");
  });

  it("refreshes METADATA_CHANGED without automatically retrying install", async () => {
    const metadataChanged: InstallerErrorDto = {
      code: "METADATA_CHANGED",
      stage: "failed",
      messageKey: "secret.backend.message.must.not.render",
      retryable: true,
      suggestedAction: "refresh",
      details: {
        endpointKind: null,
        attempt: null,
        maxAttempts: null,
        httpStatus: null,
        platformErrorCode: null,
        redactedMessage: "sensitive path C:/Users/private",
        context: {},
      },
    };
    const failed = snapshot({
      sequence: 7,
      stage: "failed",
      error: metadataChanged,
    });
    const port = createInstallerPort({ getJob: vi.fn(async () => failed) });
    renderPanel(port);

    const refresh = await screen.findByRole("button", {
      name: "刷新状态",
    });
    expect(
      await screen.findByRole("button", { name: "打开日志目录" }),
    ).toBeVisible();
    expect(document.body).toHaveTextContent(
      "版本信息已更新，请刷新后重新确认安装。",
    );
    expect(document.body).not.toHaveTextContent("METADATA_CHANGED");
    expect(document.body).not.toHaveTextContent("secret.backend");
    expect(document.body).not.toHaveTextContent("C:/Users/private");

    fireEvent.click(refresh);
    await waitFor(() => expect(port.checkLatest).toHaveBeenCalledWith(true));
    expect(port.startInstall).not.toHaveBeenCalled();
    const install = await screen.findByRole("button", {
      name: "安装 Codex Desktop",
    });

    fireEvent.click(install);
    await waitFor(() => expect(port.startInstall).toHaveBeenCalledTimes(1));
  });

  it("redacts raw native failures and never invents an installed state", async () => {
    const rawFailure = async () => {
      throw new Error("sk-secret C:/private/install.msix");
    };
    const port = createInstallerPort({
      getLocalStatus: vi.fn(rawFailure),
      checkLatest: vi.fn(rawFailure),
      getJob: vi.fn(rawFailure),
      subscribeJobUpdates: vi.fn(rawFailure),
    });
    renderPanel(port);

    expect(await screen.findByText("暂时无法读取安装状态。")).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "打开日志目录" }),
    ).not.toBeInTheDocument();
    expect(document.body).not.toHaveTextContent("sk-secret");
    expect(document.body).not.toHaveTextContent("C:/private");
    expect(screen.getByText("无法确认")).toBeVisible();
    expect(document.body).not.toHaveTextContent("已安装");
  });

  it("keeps one active listener in StrictMode and releases it on unmount", async () => {
    const listeners = new Set<(value: JobSnapshot) => void>();
    const cleanup = vi.fn((listener: (value: JobSnapshot) => void) => {
      listeners.delete(listener);
    });
    const port = createInstallerPort({
      subscribeJobUpdates: vi.fn(async (listener) => {
        listeners.add(listener);
        return () => cleanup(listener);
      }),
    });
    const view = renderPanel(port, true);

    await waitFor(() => expect(listeners.size).toBe(1));
    expect(port.subscribeJobUpdates).toHaveBeenCalledTimes(1);
    view.unmount();
    await waitFor(() => expect(listeners.size).toBe(0));
    expect(cleanup).toHaveBeenCalledTimes(1);
  });
});
