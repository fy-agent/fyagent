import { describe, expect, it, vi } from "vitest";

const { getCurrentWindowMock } = vi.hoisted(() => ({
  getCurrentWindowMock: vi.fn(() => ({
    setDecorations: vi.fn().mockResolvedValue(undefined),
    minimize: vi.fn().mockResolvedValue(undefined),
    toggleMaximize: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
  })),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: getCurrentWindowMock,
}));

import { createWindowFramePort } from "../../../src/v2/shared/platform/windowFrame";

describe("WindowFramePort factory", () => {
  it("selects the browser or native adapter from the runtime descriptor", () => {
    const browserPort = createWindowFramePort({
      isNative: false,
      platform: "browser",
    });
    const nativePort = createWindowFramePort({
      isNative: true,
      platform: "linux",
    });

    expect(browserPort).toMatchObject({
      isNative: false,
      platform: "browser",
    });
    expect(nativePort).toMatchObject({ isNative: true, platform: "linux" });
    expect(getCurrentWindowMock).toHaveBeenCalledTimes(1);
  });
});
