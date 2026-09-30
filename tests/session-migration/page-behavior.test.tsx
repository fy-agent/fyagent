import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { SessionsPage } from "@/pages/sessions/Page";
import { FeatureProvider } from "@/shared/features/provider";
import {
  migratableSessionSchema,
  restoreAttemptSchema,
  sessionPackageSchema,
  type MigratableSession,
  type RestoreAttempt,
  type SessionMeta,
} from "@/shared/features/session-migration";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { restoreAttemptSample, sessionPackageSample } from "./samples";

vi.mock("@/shared/platform/runtime", () => ({
  detectRuntime: () => ({ isNative: true }),
}));

const sourceA = "/isolated/source-a.jsonl";
const sourceB = "/isolated/source-b.jsonl";

function sourceSession(
  providerId: string,
  sessionId: string,
  sourcePath: string,
  title: string,
): SessionMeta {
  return {
    providerId,
    sessionId,
    sourcePath,
    title,
    projectDir: "/tmp/current-workspace",
  };
}

function migrationPreview(
  providerId: string,
  sessionId: string,
  sourceIndex: number,
): MigratableSession {
  const raw = structuredClone(sessionPackageSample());
  const session = (raw.sessions as Array<Record<string, unknown>>)[0];
  const origin = session.origin as Record<string, unknown>;
  session.snapshotId = `fys1:${String(sourceIndex).repeat(64)}`;
  session.contentDigest = `fyc1:${String(sourceIndex + 2).repeat(64)}`;
  session.title = `预览 ${sessionId}`;
  origin.originId = `origin-${sessionId}`;
  origin.providerId = providerId;
  origin.sessionId = sessionId;
  return migratableSessionSchema.parse(session);
}

function pagePorts({
  preview,
}: {
  preview: (
    providerId: string,
    sourcePath: string,
  ) => Promise<MigratableSession>;
}) {
  const sessions = [
    sourceSession("codex", "session-a", sourceA, "来源 A"),
    sourceSession("codex", "session-b", sourceB, "来源 B"),
  ];
  const ports = createBrowserFeaturePorts();
  ports.sessions.listSessions = vi.fn(async () => sessions);
  ports.sessions.getSessionMessages = vi.fn(async () => []);
  ports.sessions.previewSessionMigration = vi.fn(preview);
  ports.sessions.listRestoreAttempts = vi.fn(async () => []);
  ports.sessions.probeLocalProvider = vi.fn(async (providerId) => ({
    providerId,
    installed: true,
    detectedVersion: providerId === "codex" ? "0.154.0" : "0.0.0",
    extractionSupported: providerId === "codex",
    writeSupported: false,
    reasonCode: "providerVersionUnsupported",
  }));
  ports.sessions.getReleaseCapabilityMatrix = vi.fn(async () => []);
  ports.sessions.exportSessionPackage = vi.fn(async (_items, targetPath) => ({
    path: targetPath,
    sessionCount: 2,
    byteLen: 512,
    packageFileDigest: "a".repeat(64),
  }));
  ports.sessions.pickExportPath = vi.fn(async () => "/tmp/frozen-export.json");
  return ports;
}

function renderPage(ports: ReturnType<typeof pagePorts>) {
  return render(
    <MemoryRouter>
      <FeatureProvider ports={ports}>
        <SessionsPage />
      </FeatureProvider>
    </MemoryRouter>,
  );
}

async function selectBothSessions(user: ReturnType<typeof userEvent.setup>) {
  await screen.findByText("来源 A", { exact: true });
  const selectButtons = screen.getAllByRole("button", {
    name: "勾选以导出",
  });
  await user.click(selectButtons[0]);
  await user.click(selectButtons[1]);
}

function configureSupportedImport(
  ports: ReturnType<typeof pagePorts>,
  attempts: RestoreAttempt[],
) {
  ports.sessions.probeLocalProvider = vi.fn(async (providerId) => ({
    providerId,
    installed: true,
    detectedVersion: providerId === "codex" ? "0.154.0" : "0.0.0",
    extractionSupported: providerId === "codex",
    writeSupported: providerId === "codex",
    reasonCode:
      providerId === "codex" ? undefined : "providerVersionUnsupported",
  }));
  ports.sessions.pickPackageFile = vi.fn(async () => "/tmp/package.json");
  ports.sessions.readSessionPackage = vi.fn(async () => ({
    package: sessionPackageSchema.parse(sessionPackageSample()),
    attempts: [],
  }));
  ports.sessions.restoreSessionPackage = vi.fn(async () => attempts);
}

async function executePageImport(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: /导入会话包/u }));
  const dialog = await screen.findByRole("dialog");
  await user.click(within(dialog).getByRole("button", { name: "选择文件" }));
  await user.click(within(dialog).getByRole("button", { name: "解析会话包" }));
  await within(dialog).findByText("会话包解析成功", { exact: true });
  await user.click(
    within(dialog).getByRole("button", {
      name: "确认恢复至目标软件",
    }),
  );
  return dialog;
}

describe("session migration page batch export behavior", () => {
  it("previews every frozen selection and exports that same set", async () => {
    const user = userEvent.setup();
    const previews = new Map([
      [sourceA, migrationPreview("codex", "session-a", 1)],
      [sourceB, migrationPreview("codex", "session-b", 2)],
    ]);
    const ports = pagePorts({
      preview: async (_providerId, sourcePath) => {
        const result = previews.get(sourcePath);
        if (!result) throw new Error(`unexpected source: ${sourcePath}`);
        return result;
      },
    });
    renderPage(ports);

    await selectBothSessions(user);
    vi.mocked(ports.sessions.previewSessionMigration).mockClear();
    await user.click(
      screen.getByRole("button", { name: "批量导出 (2 个会话)" }),
    );

    await waitFor(() => {
      expect(ports.sessions.previewSessionMigration).toHaveBeenCalledWith(
        "codex",
        sourceA,
      );
      expect(ports.sessions.previewSessionMigration).toHaveBeenCalledWith(
        "codex",
        sourceB,
      );
    });

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("2 个", { exact: true })).toBeVisible();

    // Mutating the live page selection after the preview opened must not alter
    // the set that the user reviewed and is about to export.
    const liveSelected = screen.getAllByRole("button", {
      name: "取消勾选",
      hidden: true,
    });
    fireEvent.click(liveSelected[0]);

    await user.click(
      within(dialog).getByRole("button", { name: "选择保存位置" }),
    );
    await user.click(
      within(dialog).getByRole("button", { name: "确认导出迁移包" }),
    );

    await waitFor(() =>
      expect(ports.sessions.exportSessionPackage).toHaveBeenCalledTimes(1),
    );
    const [items, targetPath] = vi.mocked(ports.sessions.exportSessionPackage)
      .mock.calls[0];
    expect(items).toEqual([
      { providerId: "codex", sourcePath: sourceA },
      { providerId: "codex", sourcePath: sourceB },
    ]);
    expect(targetPath).toBe("/tmp/frozen-export.json");
  });

  it("blocks the whole batch and names the session whose preview fails", async () => {
    const user = userEvent.setup();
    const ports = pagePorts({
      preview: async (_providerId, sourcePath) => {
        if (sourcePath === sourceB) {
          throw new Error("无法确认最终答复是否完整");
        }
        return migrationPreview("codex", "session-a", 1);
      },
    });
    renderPage(ports);

    await selectBothSessions(user);
    await user.click(
      screen.getByRole("button", { name: "批量导出 (2 个会话)" }),
    );

    const alert = await screen.findByRole("alert");
    expect(within(alert).getByText(/来源 B/u)).toBeVisible();
    expect(within(alert).getByText(/无法确认最终答复是否完整/u)).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "确认导出迁移包" }),
    ).not.toBeInTheDocument();
    expect(ports.sessions.exportSessionPackage).not.toHaveBeenCalled();
  });

  it("does not emit a success toast for an ambiguous restore result", async () => {
    const user = userEvent.setup();
    const ports = pagePorts({
      preview: async (_providerId, sourcePath) =>
        migrationPreview(
          "codex",
          sourcePath === sourceA ? "session-a" : "session-b",
          sourcePath === sourceA ? 1 : 2,
        ),
    });
    configureSupportedImport(ports, [
      restoreAttemptSchema.parse(restoreAttemptSample({ stage: "ambiguous" })),
    ]);
    renderPage(ports);
    await screen.findByText("来源 A", { exact: true });

    const dialog = await executePageImport(user);

    expect(
      await within(dialog).findByText("恢复操作待确认", { exact: true }),
    ).toBeVisible();
    expect(
      screen.queryByText("会话恢复写入完成", { exact: true }),
    ).not.toBeInTheDocument();
  });

  it("does not emit a success toast when restore returns no receipts", async () => {
    const user = userEvent.setup();
    const ports = pagePorts({
      preview: async () => migrationPreview("codex", "session-a", 1),
    });
    configureSupportedImport(ports, []);
    renderPage(ports);
    await screen.findByText("来源 A", { exact: true });

    const dialog = await executePageImport(user);
    expect(
      await within(dialog).findByText(/未收到.*回执|没有.*恢复记录/u),
    ).toBeVisible();
    expect(
      screen.queryByText("会话恢复写入完成", { exact: true }),
    ).not.toBeInTheDocument();
  });

  it("reprobes local readiness when import is opened after native first launch", async () => {
    const user = userEvent.setup();
    const ports = pagePorts({
      preview: async () => migrationPreview("codex", "session-a", 1),
    });
    let initialized = false;
    ports.sessions.probeLocalProvider = vi.fn(async (providerId) => ({
      providerId,
      installed: true,
      detectedVersion: "0.154.0",
      extractionSupported: true,
      writeSupported: initialized,
      reasonCode: initialized ? undefined : "targetStoreUnidentified",
    }));
    ports.sessions.pickPackageFile = vi.fn(async () => "/tmp/package.json");
    ports.sessions.readSessionPackage = vi.fn(async () => ({
      package: sessionPackageSchema.parse(sessionPackageSample()),
      attempts: [],
    }));
    renderPage(ports);
    await screen.findByText("来源 A", { exact: true });
    await waitFor(() =>
      expect(ports.sessions.probeLocalProvider).toHaveBeenCalledWith("codex"),
    );
    vi.mocked(ports.sessions.probeLocalProvider).mockClear();
    initialized = true;
    await user.click(screen.getByRole("button", { name: /导入会话包/u }));
    await waitFor(() =>
      expect(ports.sessions.probeLocalProvider).toHaveBeenCalledWith("codex"),
    );
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "选择文件" }));
    await user.click(
      within(dialog).getByRole("button", { name: "解析会话包" }),
    );
    await within(dialog).findByText("会话包解析成功", { exact: true });
    expect(
      within(dialog).getByRole("button", { name: "确认恢复至目标软件" }),
    ).toBeEnabled();
  });

  it("shows a probe failure as unknown instead of claiming not installed", async () => {
    const user = userEvent.setup();
    const ports = pagePorts({
      preview: async () => migrationPreview("codex", "session-a", 1),
    });
    ports.sessions.probeLocalProvider = vi.fn(async () => {
      throw new Error(
        JSON.stringify({
          code: "probeUnavailable",
          message: "探测服务暂时不可用",
        }),
      );
    });
    renderPage(ports);

    await screen.findByText("来源 A", { exact: true });
    await user.click(screen.getByText("来源 A", { exact: true }));
    expect(await screen.findByText(/探测服务暂时不可用/u)).toBeVisible();
    expect(screen.queryByText(/本地未安装/u)).toBeNull();
    expect(
      screen.getByRole("button", { name: "在目标软件中恢复" }),
    ).toBeDisabled();
  });

  it("prioritizes an unresolved receipt over an earlier success for the same origin", async () => {
    const user = userEvent.setup();
    const preview = migrationPreview("codex", "session-a", 1);
    const ports = pagePorts({ preview: async () => preview });
    const base = {
      origin: preview.origin,
      snapshotId: preview.snapshotId,
      targetProviderId: "codex",
      targetNativeId: undefined,
    };
    ports.sessions.listRestoreAttempts = vi.fn(async () => [
      restoreAttemptSchema.parse(
        restoreAttemptSample({
          ...base,
          attemptId: "attempt-success",
          stage: "nativeWritten",
        }),
      ),
      restoreAttemptSchema.parse(
        restoreAttemptSample({
          ...base,
          attemptId: "attempt-unresolved",
          stage: "needsReconciliation",
        }),
      ),
    ]);
    renderPage(ports);

    await screen.findByText("来源 A", { exact: true });
    await user.click(screen.getByText("来源 A", { exact: true }));
    expect(
      await screen.findByText("写入结果尚未确认", { exact: true }),
    ).toBeVisible();
    expect(
      screen.queryByText("已写入目标存储 · 待读回验证", { exact: true }),
    ).not.toBeInTheDocument();
  });
});
