import { afterEach, describe, expect, it } from "vitest";
import { initializeAppearance } from "@/app/appearance";
import {
  persistTheme,
  readThemePreference,
  THEME_STORAGE_KEY,
} from "@/shared/design-system/appearance";

afterEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.theme;
});

describe("startup appearance restore", () => {
  it("applies a stored explicit preference before route loading", () => {
    persistTheme("dark");
    expect(readThemePreference()).toBe("dark");
    initializeAppearance();
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("dark");
  });

  it("falls back to light when storage is empty or invalid", () => {
    initializeAppearance();
    expect(document.documentElement.dataset.theme).toBe("light");
    localStorage.setItem(THEME_STORAGE_KEY, "not-a-theme");
    initializeAppearance();
    expect(document.documentElement.dataset.theme).toBe("light");
  });
});
