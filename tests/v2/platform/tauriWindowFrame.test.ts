import { beforeEach, describe, expect, it, vi } from "vitest";

const { getCurrentWindowMock } = vi.hoisted(() => ({
  getCurrentWindowMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: getCurrentWindowMock,
}));

import { createTauriWindowFramePort } from "../../../src/v2/shared/platform/tauri/windowFrame";

function createNativeWindowMock() {
  return {
    setDecorations: vi.fn().mockResolvedValue(undefined),
    minimize: vi.fn().mockResolvedValue(undefined),
    toggleMaximize: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
  };
}

describe("Tauri WindowFramePort", () => {
  beforeEach(() => {
    getCurrentWindowMock.mockReset();
  });

  it("removes Windows decorations only once, including repeated mount calls", async () => {
    const appWindow = createNativeWindowMock();
    getCurrentWindowMock.mockReturnValue(appWindow);
    const port = createTauriWindowFramePort("windows");

    const firstPreparation = port.prepareFrame();
    const repeatedPreparation = port.prepareFrame();

    expect(port.isNative).toBe(true);
    expect(port.platform).toBe("windows");
    expect(firstPreparation).toBe(repeatedPreparation);
    await expect(firstPreparation).resolves.toBeUndefined();
    expect(getCurrentWindowMock).toHaveBeenCalledTimes(1);
    expect(appWindow.setDecorations).toHaveBeenCalledTimes(1);
    expect(appWindow.setDecorations).toHaveBeenCalledWith(false);
  });

  it("delegates each native window action to the current Tauri window", async () => {
    const appWindow = createNativeWindowMock();
    const port = createTauriWindowFramePort("windows", appWindow);

    await port.minimize();
    await port.toggleMaximize();
    await port.close();

    expect(appWindow.minimize).toHaveBeenCalledTimes(1);
    expect(appWindow.toggleMaximize).toHaveBeenCalledTimes(1);
    expect(appWindow.close).toHaveBeenCalledTimes(1);
  });

  it("leaves non-Windows native decorations unchanged", async () => {
    const appWindow = createNativeWindowMock();
    const port = createTauriWindowFramePort("macos", appWindow);

    await port.prepareFrame();

    expect(appWindow.setDecorations).not.toHaveBeenCalled();
  });
});
