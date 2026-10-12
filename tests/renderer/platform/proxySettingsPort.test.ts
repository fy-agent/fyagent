import { beforeEach, describe, expect, it, vi } from "vitest";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
import { createTauriFeaturePorts } from "@/shared/platform/tauri/features";

describe("proxy settings native port", () => {
  beforeEach(() => invoke.mockReset());
  it("reads and saves configuration and regenerates through the registered commands", async () => {
    const settings = createTauriFeaturePorts().settings;
    const config = {
      proxyEnabled: false,
      listenAddress: "0.0.0.0",
      listenPort: 15721,
      enableLogging: true,
    };
    invoke.mockResolvedValueOnce(config);
    expect(await settings.getProxyConfig()).toEqual(config);
    expect(invoke).toHaveBeenLastCalledWith("get_global_proxy_config");
    invoke.mockResolvedValueOnce(undefined);
    await settings.saveProxyConfig(config);
    expect(invoke).toHaveBeenLastCalledWith("update_global_proxy_config", {
      config,
    });
    invoke.mockResolvedValueOnce(null);
    expect(await settings.getProxyAccessKey()).toBeNull();
    invoke.mockResolvedValueOnce("new-key");
    expect(await settings.regenerateProxyAccessKey()).toBe("new-key");
    expect(invoke).toHaveBeenLastCalledWith("regenerate_proxy_access_key");
  });
  it("rejects invalid wire data without echoing credentials", async () => {
    const settings = createTauriFeaturePorts().settings;
    invoke.mockResolvedValueOnce({ key: "private-key" });
    await expect(settings.getProxyAccessKey()).rejects.toThrow(
      "无法读取代理访问密钥",
    );
    invoke.mockResolvedValueOnce("");
    await expect(settings.regenerateProxyAccessKey()).rejects.toThrow(
      "无法重新生成代理访问密钥",
    );
    invoke.mockResolvedValueOnce({ listenAddress: "0.0.0.0", listenPort: -1 });
    await expect(settings.getProxyConfig()).rejects.toThrow("无法读取代理设置");
  });
});
