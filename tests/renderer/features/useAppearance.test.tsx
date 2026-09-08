import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useAppearance } from "@/shared/features/useAppearance";
import { THEME_STORAGE_KEY } from "@/shared/design-system/appearance";

vi.mock("@/shared/platform/theme", () => ({
  synchronizeWindowTheme: vi.fn(async () => undefined),
}));

afterEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.theme;
  vi.restoreAllMocks();
});

describe("shell appearance lifetime", () => {
  it("restores a stored explicit preference when the shell control mounts", () => {
    localStorage.setItem(THEME_STORAGE_KEY, "dark");
    const { result, unmount } = renderHook(useAppearance);
    expect(result.current.theme).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    unmount();
  });

  it("follows an existing system preference until the user makes an explicit choice", () => {
    let dark = false;
    const listeners = new Map<string, Set<() => void>>();
    vi.spyOn(window, "matchMedia").mockImplementation((query) => {
      const callbacks = listeners.get(query) ?? new Set<() => void>();
      listeners.set(query, callbacks);
      return {
        media: query,
        get matches() {
          return query.includes("color-scheme") && dark;
        },
        onchange: null,
        addEventListener: vi.fn((_type, listener: () => void) =>
          callbacks.add(listener),
        ),
        removeEventListener: vi.fn((_type, listener: () => void) =>
          callbacks.delete(listener),
        ),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(() => true),
      };
    });
    localStorage.setItem(THEME_STORAGE_KEY, "system");
    const { result, unmount } = renderHook(useAppearance);
    expect(result.current.theme).toBe("light");
    act(() => {
      dark = true;
      listeners
        .get("(prefers-color-scheme: dark)")
        ?.forEach((listener) => listener());
    });
    expect(result.current.theme).toBe("dark");
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("system");
    act(() => result.current.toggle(document.createElement("button")));
    expect(result.current.theme).toBe("light");
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("light");
    act(() => {
      dark = false;
      listeners
        .get("(prefers-color-scheme: dark)")
        ?.forEach((listener) => listener());
    });
    expect(result.current.theme).toBe("light");
    unmount();
    expect([...listeners.values()].every((set) => set.size === 0)).toBe(true);
  });

  it("reads storage updates without changing route state or persisting them again", () => {
    const { result } = renderHook(useAppearance);
    const write = vi.spyOn(Storage.prototype, "setItem");
    act(() =>
      window.dispatchEvent(
        new StorageEvent("storage", {
          key: THEME_STORAGE_KEY,
          newValue: "dark",
        }),
      ),
    );
    expect(result.current.theme).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    act(() =>
      window.dispatchEvent(
        new StorageEvent("storage", {
          key: "other-setting",
          newValue: "light",
        }),
      ),
    );
    expect(result.current.theme).toBe("dark");
    act(() =>
      window.dispatchEvent(
        new StorageEvent("storage", {
          key: THEME_STORAGE_KEY,
          newValue: "not-a-theme",
        }),
      ),
    );
    expect(result.current.theme).toBe("light");
    expect(write).not.toHaveBeenCalled();
  });
});
