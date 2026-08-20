import { afterEach, describe, expect, it, vi } from "vitest";

async function loadPlatform(userAgent?: string, platform?: string) {
  vi.stubGlobal(
    "navigator",
    userAgent === undefined
      ? undefined
      : {
          userAgent,
          platform: platform ?? "",
        },
  );
  vi.resetModules();
  return import("@/lib/platform");
}

describe("desktop platform detection", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.resetModules();
  });

  it("enables desktop drag regions for positively identified Windows", async () => {
    const platform = await loadPlatform(
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
      "Win32",
    );

    expect(platform.detectDesktopPlatform()).toBe("windows");
    expect(platform.isWindows()).toBe(true);
    expect(platform.isMac()).toBe(false);
    expect(platform.DRAG_REGION_ENABLED).toBe(true);
  });

  it.each([
    ["Mozilla/5.0 (Macintosh; Intel Mac OS X 14_6)", "MacIntel"],
    ["Mozilla/5.0", "Darwin"],
  ])(
    "enables desktop drag regions for positively identified macOS",
    async (userAgent, navigatorPlatform) => {
      const platform = await loadPlatform(userAgent, navigatorPlatform);

      expect(platform.detectDesktopPlatform()).toBe("macos");
      expect(platform.isWindows()).toBe(false);
      expect(platform.isMac()).toBe(true);
      expect(platform.DRAG_REGION_ENABLED).toBe(true);
    },
  );

  it("fails closed for an unsupported host", async () => {
    const platform = await loadPlatform(
      "Mozilla/5.0 (UnsupportedOS x86_64)",
      "UnsupportedOS x86_64",
    );

    expect(platform.detectDesktopPlatform()).toBe("unknown");
    expect(platform.isWindows()).toBe(false);
    expect(platform.isMac()).toBe(false);
    expect(platform.DRAG_REGION_ENABLED).toBe(false);
    expect(platform.DRAG_REGION_ATTR).toEqual({});
    expect(platform.DRAG_REGION_STYLE).toEqual({});
  });

  it("fails closed when navigator is unavailable", async () => {
    const platform = await loadPlatform();

    expect(platform.detectDesktopPlatform()).toBe("unknown");
    expect(platform.DRAG_REGION_ENABLED).toBe(false);
  });
});
