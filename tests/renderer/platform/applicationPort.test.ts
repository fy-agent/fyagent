import { beforeEach, describe, expect, it, vi } from "vitest";
import { createSimpleFeaturePorts } from "@/shared/platform/tauri/feature-ports/simple";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

const { getVersion } = vi.hoisted(() => ({ getVersion: vi.fn() }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion }));

describe("application version port", () => {
  beforeEach(() => getVersion.mockReset());

  it("reads the running package only when requested", async () => {
    getVersion.mockResolvedValue("9.8.7-preview.1");
    const { settings } = createSimpleFeaturePorts();
    expect(getVersion).not.toHaveBeenCalled();
    await expect(settings.getAppVersion()).resolves.toBe("9.8.7-preview.1");
    expect(getVersion).toHaveBeenCalledWith();
  });

  it.each([null, {}, "", "1.2", "01.2.3", "1.2.3\n", "private/path"])(
    "rejects malformed native version %j",
    async (value) => {
      getVersion.mockResolvedValue(value);
      await expect(
        createSimpleFeaturePorts().settings.getAppVersion(),
      ).rejects.toThrow("Application version is unavailable");
    },
  );

  it("does not invent a package version for browser preview", async () => {
    await expect(
      createBrowserFeaturePorts().settings.getAppVersion(),
    ).rejects.toThrow("桌面应用");
  });
});
