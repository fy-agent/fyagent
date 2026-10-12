import { beforeEach, describe, expect, it, vi } from "vitest";
import { createTauriFeaturePorts } from "@/shared/platform/tauri/features";

const { invoke, reportFrontendError } = vi.hoisted(() => ({
  invoke: vi.fn(),
  reportFrontendError: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@/shared/platform/tauri/frontendLogger", () => ({
  reportFrontendError,
}));

describe("upstream aggregation commands", () => {
  beforeEach(() => invoke.mockReset());
  it("preserves mode/member/default command arguments through the production facade", async () => {
    const port = createTauriFeaturePorts().providers;
    invoke.mockResolvedValue(undefined);
    await port.setMode("codex", true, true, "official");
    await port.setRoute("codex", "third-party");
    await port.setStackMember("codex", "third-party", true);
    await port.adoptCodexCatalog();
    await port.setMode("codex", false);
    expect(invoke.mock.calls).toEqual([
      [
        "set_proxy_takeover_for_app",
        { appType: "codex", enabled: true, stack: true, route: "official" },
      ],
      ["update_tray_menu"],
      ["set_proxy_route", { appType: "codex", providerId: "third-party" }],
      [
        "set_proxy_stack_member",
        { appType: "codex", providerId: "third-party", enabled: true },
      ],
      ["adopt_codex_stack_catalog"],
      [
        "set_proxy_takeover_for_app",
        {
          appType: "codex",
          enabled: false,
          stack: undefined,
          route: undefined,
        },
      ],
      ["update_tray_menu"],
    ]);
  });
  it("reads the upstream DTO and keeps existing provider metadata on round trip", async () => {
    const port = createTauriFeaturePorts().providers;
    const provider = {
      id: "p",
      name: "P",
      settingsConfig: { auth: { OPENAI_API_KEY: "key" }, custom: true },
      meta: { customFeature: "keep", stackModels: [{ model: "a" }] },
    };
    invoke.mockResolvedValueOnce({ p: provider });
    const providers = await port.getAll("claude");
    expect(providers.p).toEqual(provider);
    await port.update("claude", providers.p);
    expect(invoke).toHaveBeenLastCalledWith("update_provider", {
      app: "claude",
      provider,
    });
    invoke.mockResolvedValueOnce({
      active: true,
      members: [{ providerId: "p", route: true, modelIds: ["a"] }],
      staleClients: { daemon: false, others: true },
    });
    expect(await port.getStack("codex")).toMatchObject({
      active: true,
      staleClients: { daemon: false, others: true },
    });
    invoke.mockResolvedValueOnce("notRunning");
    expect(await port.restartCodexDaemon()).toBe("notRunning");
  });

  it("logs and retries a failed tray refresh without failing the saved mode", async () => {
    const error = new Error("托盘暂时不可用");
    invoke
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce(error)
      .mockResolvedValueOnce(undefined);
    await expect(
      createTauriFeaturePorts().providers.setMode("codex", true),
    ).resolves.toBeUndefined();
    expect(invoke.mock.calls.map(([command]) => command)).toEqual([
      "set_proxy_takeover_for_app",
      "update_tray_menu",
      "update_tray_menu",
    ]);
    expect(reportFrontendError).toHaveBeenCalledWith(
      "update_tray_menu：模式已保存，托盘刷新失败，将重试一次",
      error,
    );
  });

  it("keeps the saved mode successful and logs a failed bounded retry", async () => {
    const error = new Error("托盘不可用");
    invoke
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce(error)
      .mockRejectedValueOnce(error);
    await expect(
      createTauriFeaturePorts().providers.setMode("codex", true),
    ).resolves.toBeUndefined();
    expect(invoke).toHaveBeenCalledTimes(3);
    expect(reportFrontendError).toHaveBeenCalledWith(
      "update_tray_menu：托盘刷新重试失败，模式已保存",
      error,
    );
  });

  it("does not refresh the tray when saving the mode fails", async () => {
    const error = new Error("模式保存失败");
    invoke.mockRejectedValueOnce(error);
    await expect(
      createTauriFeaturePorts().providers.setMode("codex", true),
    ).rejects.toBe(error);
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});
