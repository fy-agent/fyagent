import { beforeEach, describe, expect, it, vi } from "vitest";
import { createTauriFeaturePorts } from "@/shared/platform/tauri/features";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

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
});
