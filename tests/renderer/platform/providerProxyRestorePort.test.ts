import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ProviderAppId } from "@/shared/features/models";
import { createTauriFeaturePorts } from "@/shared/platform/tauri/features";
import {
  createBrowserFeaturePorts,
  NATIVE_ONLY_ERROR,
} from "@/shared/platform/browser/features";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const preview = {
  app: "codex",
  enabled: true,
  canRestore: true,
  targets: [
    { path: "~/.codex/config.toml", exists: true },
    { path: "~/.codex/fyagent-model-catalog.json", exists: false },
  ],
};

describe("Provider proxy restoration boundary", () => {
  beforeEach(() => invoke.mockReset());

  it.each(["claude", "codex", "grokbuild"] as const)(
    "binds preview and exit to %s without renderer paths",
    async (app) => {
      const ports = createTauriFeaturePorts().providers;
      invoke
        .mockResolvedValueOnce({ ...preview, app })
        .mockResolvedValueOnce(null);
      await expect(ports.getProxyRestorePreview(app)).resolves.toEqual({
        ...preview,
        app,
      });
      await expect(ports.restoreManagedProxy(app)).resolves.toBeUndefined();
      expect(invoke.mock.calls).toEqual([
        ["get_proxy_restore_preview", { app }],
        ["set_proxy_takeover_for_app", { appType: app, enabled: false }],
      ]);
    },
  );

  it.each([
    { ...preview, app: "claude" },
    { ...preview, token: "fixture-secret" },
    { ...preview, canRestore: true, enabled: false },
    { ...preview, canRestore: true, targets: [] },
    {
      ...preview,
      targets: [{ ...preview.targets[0], backupPath: "invented" }],
    },
    { ...preview, targets: [{ path: "", exists: true }] },
    { ...preview, targets: [{ path: "unsafe\npath", exists: true }] },
    { ...preview, targets: [{ path: "x".repeat(4097), exists: true }] },
    { ...preview, targets: [{ path: "safe", exists: "yes" }] },
    { ...preview, targets: [preview.targets[0], preview.targets[0]] },
    {
      ...preview,
      targets: Array.from({ length: 9 }, (_, i) => ({
        path: `path${i}`,
        exists: true,
      })),
    },
    null,
  ])(
    "rejects malformed, wrong-target or secret-bearing preview %#",
    async (value) => {
      invoke.mockResolvedValue(value);
      await expect(
        createTauriFeaturePorts().providers.getProxyRestorePreview("codex"),
      ).rejects.toThrow();
    },
  );

  it.each([
    { ...preview, canRestore: false },
    { ...preview, enabled: false, canRestore: false, targets: [] },
  ])(
    "keeps unavailable and inactive preview states explicit",
    async (value) => {
      invoke.mockResolvedValue(value);
      await expect(
        createTauriFeaturePorts().providers.getProxyRestorePreview("codex"),
      ).resolves.toEqual(value);
    },
  );

  it.each([undefined, true, { restored: true }])(
    "rejects unconfirmed native exit response %#",
    async (value) => {
      invoke.mockResolvedValue(value);
      await expect(
        createTauriFeaturePorts().providers.restoreManagedProxy("codex"),
      ).rejects.toThrow("unconfirmed");
    },
  );

  it.each(["opencode", "../codex", "", null])(
    "rejects unsupported target %# before IPC",
    async (value) => {
      const ports = createTauriFeaturePorts().providers;
      await expect(
        ports.getProxyRestorePreview(value as ProviderAppId),
      ).rejects.toThrow("invalid");
      await expect(
        ports.restoreManagedProxy(value as ProviderAppId),
      ).rejects.toThrow("invalid");
      expect(invoke).not.toHaveBeenCalled();
    },
  );

  it("keeps both operations native-only in browser previews", async () => {
    const ports = createBrowserFeaturePorts().providers;
    await expect(ports.getProxyRestorePreview("codex")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.restoreManagedProxy("codex")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    expect(invoke).not.toHaveBeenCalled();
  });
});
