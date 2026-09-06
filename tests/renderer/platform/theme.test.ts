import { describe, expect, it, vi } from "vitest";
vi.mock("@/shared/platform/runtime", () => ({
  detectRuntime: () => ({ isNative: true }),
}));
vi.mock("@/shared/platform/tauri/theme", () => ({
  setNativeWindowTheme: vi.fn(),
}));
import { synchronizeWindowTheme } from "@/shared/platform/theme";
import { setNativeWindowTheme } from "@/shared/platform/tauri/theme";

describe("native appearance transport", () => {
  it("serializes/coalesces updates so an old native completion cannot win", async () => {
    let complete!: () => void;
    vi.mocked(setNativeWindowTheme)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            complete = resolve;
          }),
      )
      .mockResolvedValue(undefined);
    const first = synchronizeWindowTheme("dark");
    await synchronizeWindowTheme("system");
    await synchronizeWindowTheme("light");
    expect(setNativeWindowTheme).toHaveBeenCalledTimes(1);
    complete();
    await first;
    expect(vi.mocked(setNativeWindowTheme).mock.calls).toEqual([
      ["dark"],
      ["light"],
    ]);
  });
});
