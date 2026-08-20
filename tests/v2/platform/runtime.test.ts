import { describe, expect, it } from "vitest";

import {
  detectNativePlatform,
  detectRuntime,
  shouldShowMacOverlayDragStrip,
} from "../../../src/v2/shared/platform/runtime";

describe("V2 runtime detection", () => {
  it("keeps ordinary browser previews on the browser platform", () => {
    expect(
      detectRuntime({
        navigator: {
          platform: "Win32",
          userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
        },
      }),
    ).toEqual({ isNative: false, platform: "browser" });
  });

  it("detects the native platform from stable navigator hints", () => {
    expect(
      detectRuntime({
        isTauri: true,
        navigator: { platform: "Win32" },
      }),
    ).toEqual({ isNative: true, platform: "windows" });

    expect(detectNativePlatform({ platform: "MacIntel" })).toBe("macos");
    expect(detectNativePlatform({ userAgent: "UnsupportedOS x86_64" })).toBe(
      "unknown",
    );
  });

  it("recognizes the Tauri internals fallback without guessing a platform", () => {
    expect(detectRuntime({ __TAURI_INTERNALS__: {} })).toEqual({
      isNative: true,
      platform: "unknown",
    });
  });

  it("shows the Overlay drag strip only on native macOS", () => {
    expect(
      shouldShowMacOverlayDragStrip({
        isNative: true,
        platform: "macos",
      }),
    ).toBe(true);
    expect(
      shouldShowMacOverlayDragStrip({
        isNative: true,
        platform: "windows",
      }),
    ).toBe(false);
    expect(
      shouldShowMacOverlayDragStrip({
        isNative: false,
        platform: "browser",
      }),
    ).toBe(false);
  });
});
