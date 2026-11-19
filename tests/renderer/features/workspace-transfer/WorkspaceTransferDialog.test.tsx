import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { FeatureProvider } from "@/shared/features/provider";
import { WorkspaceTransferDialog } from "@/shared/features/workspace-transfer-ui/WorkspaceTransferDialog";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

function renderDialog(ports: ReturnType<typeof createBrowserFeaturePorts>) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <MemoryRouter>
      <QueryClientProvider client={queryClient}>
        <FeatureProvider ports={ports}>
          <WorkspaceTransferDialog open onOpenChange={vi.fn()} />
        </FeatureProvider>
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

describe("WorkspaceTransferDialog", () => {
  it("estimates <=100 MiB projection budget and exports an encrypted .fyagent-pack", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();

    ports.workspaceTransfer.previewExport = vi.fn(async () => ({
      counts: {
        models: 2,
        customTemplates: 1,
        mcpServers: 3,
        prompts: 2,
        skills: 4,
        sessionsIncluded: 42,
        sessionsExcluded: 5,
      },
      apiKeyCount: 2,
      breakdown: {
        modelsBytes: 1024,
        templatesBytes: 512,
        mcpBytes: 2048,
        promptsBytes: 4096,
        skillsBytes: 200_000,
        sessionsBytes: 4_000_000,
        totalUncompressedBytes: 4_207_680,
        estimatedCompressedBytes: 620_000,
        softWarningBytes: 80 * 1024 * 1024,
        hardLimitBytes: 100 * 1024 * 1024,
        withinHardLimit: true,
      },
      warnings: ["为控制在 100 MiB 硬上限内，已排除 5 条较旧会话。"],
    }));
    ports.workspaceTransfer.pickExportPath = vi.fn(
      async () => "/tmp/my-pc.fyagent-pack",
    );
    ports.workspaceTransfer.exportPack = vi.fn(async (req) => ({
      path: req.targetPath,
      packId: "fypk-unit-1",
      compressedBytes: 615_000,
      uncompressedBytes: 4_207_680,
      fileSha256: "f".repeat(64),
      hasEncryptedSecrets: Boolean(req.passphrase),
      counts: {
        models: 2,
        customTemplates: 1,
        mcpServers: 3,
        prompts: 2,
        skills: 4,
        sessionsIncluded: 42,
        sessionsExcluded: 5,
      },
      warnings: [],
    }));

    renderDialog(ports);

    await user.click(
      screen.getByRole("button", { name: /硬件精简投影/ }),
    );
    await user.click(screen.getByRole("button", { name: "估算体积预算" }));

    expect(await screen.findByText(/100\.00 MiB 硬上限/)).toBeVisible();
    expect(screen.getByText(/硬件载体友好/)).toBeVisible();
    expect(ports.workspaceTransfer.previewExport).toHaveBeenCalledWith(
      expect.objectContaining({ sessionMode: "compact_context" }),
    );

    await user.click(
      screen.getByRole("checkbox", {
        name: "包含 API Key（AES-256-GCM 个人口令加密）",
      }),
    );
    await user.type(
      screen.getByPlaceholderText("输入不少于 6 位的迁移口令"),
      "hardware-pass-2026",
    );
    await user.click(
      screen.getByRole("button", { name: "生成 .fyagent-pack 迁移包" }),
    );

    await waitFor(() =>
      expect(ports.workspaceTransfer.exportPack).toHaveBeenCalledWith(
        expect.objectContaining({
          targetPath: "/tmp/my-pc.fyagent-pack",
          passphrase: "hardware-pass-2026",
        }),
      ),
    );
    expect(await screen.findByText(/已生成迁移包/)).toBeVisible();
  });

  it("inspects, unlocks passphrase-encrypted secrets, and imports a .fyagent-pack", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();

    ports.workspaceTransfer.pickImportFile = vi.fn(
      async () => "/tmp/from-old-pc.fyagent-pack",
    );
    ports.workspaceTransfer.inspectPack = vi.fn(async (req) => {
      const unlocked = req.passphrase === "unlock-pass";
      return {
        path: req.path,
        manifest: {
          schema: "fyagent-workspace-pack/v1",
          packId: "fypk-old",
          exportedAt: 1763550000,
          appVersion: "0.4.16",
          platform: "macos",
          hasEncryptedSecrets: true,
          sessionProjectionMode: "full_dialogue" as const,
          payloadSha256: "c".repeat(64),
          uncompressedBytes: 500_000,
          compressedBytes: 90_000,
          budgetLimitBytes: 100 * 1024 * 1024,
          counts: {
            models: 2,
            customTemplates: 1,
            mcpServers: 2,
            prompts: 1,
            skills: 2,
            sessionsIncluded: 12,
            sessionsExcluded: 0,
          },
          warnings: [],
        },
        secretsLocked: !unlocked,
        decryptedApiKeyTargets: unlocked ? (["codex", "claude"] as const).slice() : [],
        modelTargets: (["codex", "claude"] as const).slice(),
        strictSessionCount: 7,
        portableSessionCount: 5,
        warnings: [],
      };
    });
    ports.workspaceTransfer.applyImport = vi.fn(async () => ({
      modelsApplied: 2,
      modelsSkipped: 0,
      templatesImported: 1,
      mcpImported: 2,
      promptsImported: 1,
      skillsImported: 2,
      sessionsArchived: 12,
      sessionArchiveDir: "~/.fyagent/imported-sessions/fypk-old",
      warnings: [],
    }));

    renderDialog(ports);

    await user.click(screen.getByRole("tab", { name: "导入迁移包" }));
    await user.click(
      screen.getByRole("button", { name: "选择 .fyagent-pack 迁移包" }),
    );

    expect(
      await screen.findByPlaceholderText("输入导出时设置的迁移口令"),
    ).toBeVisible();
    await user.type(
      screen.getByPlaceholderText("输入导出时设置的迁移口令"),
      "unlock-pass",
    );
    await user.click(screen.getByRole("button", { name: "解锁加密密钥" }));
    expect(await screen.findByText(/已解锁 2 端加密 API Key/)).toBeVisible();

    await user.click(screen.getByRole("button", { name: "确认导入所选内容" }));
    await waitFor(() =>
      expect(ports.workspaceTransfer.applyImport).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "/tmp/from-old-pc.fyagent-pack",
          passphrase: "unlock-pass",
        }),
      ),
    );
    expect(await screen.findByText(/导入完成：模型写入 2 端/)).toBeVisible();
  });

  it("discovers SSH config hosts, previews agentless target files, and pushes directly", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();

    ports.workspaceTransfer.listSshHosts = vi.fn(async () => [
      {
        alias: "dev-box",
        hostName: "10.0.0.12",
        user: "alice",
        port: 2222,
        identityFile: "~/.ssh/id_ed25519",
      },
    ]);
    ports.workspaceTransfer.testSshTarget = vi.fn(async () => ({
      ok: true,
      remoteHome: "/Users/alice",
      message: "SSH 连通正常",
    }));
    ports.workspaceTransfer.previewSshPush = vi.fn(async () => ({
      destination: "alice@dev-box",
      remoteHome: "/Users/alice",
      reachable: true,
      files: [
        {
          remotePath: "~/.codex/config.toml",
          category: "model_config" as const,
          bytes: 380,
          mode: "644",
          containsSecret: false,
          action: "update" as const,
        },
        {
          remotePath: "~/.codex/auth.json",
          category: "model_config" as const,
          bytes: 60,
          mode: "600",
          containsSecret: true,
          action: "create" as const,
        },
      ],
      totalBytes: 440,
      warnings: [],
    }));
    ports.workspaceTransfer.applySshPush = vi.fn(async () => ({
      ok: true,
      backupDir: "/Users/alice/.fyagent-remote-backups/20261119T143000Z",
      outcomes: [
        {
          remotePath: "~/.codex/config.toml",
          ok: true,
          action: "updated",
          message: "已备份原文件并更新生效",
        },
        {
          remotePath: "~/.codex/auth.json",
          ok: true,
          action: "created",
          message: "已创建并生效",
        },
      ],
      warnings: [],
    }));

    renderDialog(ports);

    await user.click(screen.getByRole("tab", { name: "推送到 SSH 远端" }));
    const hostChip = await screen.findByRole("button", {
      name: /dev-box \(10\.0\.0\.12\)/,
    });
    await user.click(hostChip);

    await user.click(screen.getByRole("button", { name: "测试 SSH 连通性" }));
    expect(await screen.findByText(/已连通 · \/Users\/alice/)).toBeVisible();

    await user.click(
      screen.getByRole("button", { name: "预览远端推送清单" }),
    );
    expect(await screen.findByText("~/.codex/auth.json")).toBeVisible();
    expect(screen.getByText("chmod 600")).toBeVisible();

    await user.click(
      screen.getByRole("button", { name: "确认推送配置文件" }),
    );
    await waitFor(() =>
      expect(ports.workspaceTransfer.applySshPush).toHaveBeenCalledWith(
        expect.objectContaining({
          target: expect.objectContaining({
            host: "dev-box",
            user: "alice",
            port: 2222,
          }),
        }),
      ),
    );
    expect(await screen.findByText(/推送已完成/)).toBeVisible();
  });
});
