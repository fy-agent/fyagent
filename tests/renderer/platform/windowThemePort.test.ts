import { describe, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => undefined),
}));
import { invoke } from "@tauri-apps/api/core";
import { setNativeWindowTheme } from "@/shared/platform/tauri/theme";

describe("existing native theme command", () => {
  it.each(["light", "dark", "system"] as const)(
    "sends only the closed %s preference",
    async (theme) => {
      await setNativeWindowTheme(theme);
      expect(invoke).toHaveBeenLastCalledWith("set_window_theme", { theme });
    },
  );
});
